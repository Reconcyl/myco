use rand::{SeedableRng as _, Rng as _};
use rand::rngs::StdRng;

use termion::event::Key;
use termion::input::TermRead;

use std::collections::{HashMap, VecDeque, HashSet};
use std::io::{Read, Write};
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

/// The instruction enum.
mod instruction;
/// Grid and geometric utilities (directions, points, etc.).
mod grid;
/// The actual organism logic (implementing instructions).
mod organism;
/// Parsing logic related to commands.
mod command;
/// Implementation code for commands.
mod commands;
/// Logic for rendering the UI.
mod ui;

use super::Options;
use instruction::Instruction;
use grid::{Grid, Point, ORIGIN, Dir};
use organism::{Organism, Response};
use command::{CommandHandler, Args};
use ui::UI;

type OrganismId = u64;

struct OrganismState {
    id: u64,
    delay_cycles: u8,
    organism: Organism,
}

type OrganismQueue = VecDeque<OrganismState>;

/// Rarely- or never- modified configuration information for the app.
struct Config {
    /// The seed for the RNG. This is never changed during execution.
    rng_seed: u64,
    /// How many milliseconds to wait between cycles.
    cycle_frequency: u32,
    /// The maximum number of organisms, if any before reproduction doesn't work.
    max_organisms: Option<usize>,
    /// How many cycles to wait between dedup passes.
    dedup_rate: Option<usize>,
}

impl Config {
    fn new(rng_seed: u64) -> Self {
        Self {
            rng_seed,
            cycle_frequency: 100,
            max_organisms: None,
            dedup_rate: None,
        }
    }   
}

struct Commands<R, W> {
    /// The last valid command executed. This is used by the '.' key to
    /// re-execute commands.
    last: Option<String>,
    /// Handlers for various commands.
    handlers: HashMap<String, Rc<dyn CommandHandler<R, W>>>,
}

impl<R: Read, W: Write> Commands<R, W> {
    fn new() -> Self {
        let mut result = Self {
            last: None,
            handlers: HashMap::new(),
        };
        result.register_aliases(&["q", "quit"], commands::quit());
        result.register_aliases(&["l", "list"], commands::list());
        result.register("max", commands::max());
        result.register("set-max", commands::set_max());
        result.register("speed", commands::speed());
        result.register("seed", commands::seed());
        result.register("source", commands::source());
        result.register("write-error-chance", commands::write_error_chance());
        result.register_aliases(&["c", "cycle"], commands::cycle());
        result.register_aliases(&["p", "pause"], commands::pause());
        result.register("move", commands::move_());
        result.register("write", commands::write());
        result.register("|", commands::insert_line());
        result.register("byte", commands::byte());
        result.register("spawn", commands::spawn());
        result.register("dedup", commands::dedup());
        result.register("auto-dedup", commands::auto_dedup());
        result.register("set-auto-dedup", commands::set_auto_dedup());
        result.register_aliases(&["f", "focus"],    commands::focus());
        result.register_aliases(&["uf", "unfocus"], commands::unfocus());
        result.register("view", commands::view());
        result.register("ip", commands::move_ip());
        result.register_aliases(&["r", "run"], commands::run());
        result.register("kill", commands::kill());
        result
    }
    fn register(&mut self, name: &str, handler: Rc<dyn CommandHandler<R, W>>) {
        self.handlers.insert(String::from(name), handler);
    }
    fn register_aliases(&mut self, names: &[&str], handler: Rc<dyn CommandHandler<R, W>>) {
        for name in names {
            self.register(name, Rc::clone(&handler));
        }
    }
}

pub struct AppState<R, W> {
    /// The total number of cycles that have passed.
    total_cycles: usize,
    /// How many cycles have passed since a dedup occurred.
    cycles_since_dedup: usize,
    /// The RNG.
    rng: StdRng,
    /// Iterator of keys from STDIN.
    key_input: termion::input::Keys<R>,
    /// The set of organisms.
    organisms: OrganismQueue,
    /// The instruction grid.
    grid: Grid<StdRng>,
    /// Configuration information.
    config: Config,
    /// Command-line parsing information.
    commands: Commands<R, W>,
    /// UI information.
    ui: UI<W>,
    /// The ID of the organism, if any, that is currently being focused.
    focus: Option<OrganismId>,
    /// Whether execution is paused.
    paused: bool,
    /// Whether the app should quit next frame.
    quit: bool,
}

// Utility methods.
impl<R: Read, W: Write> AppState<R, W> {
    /// Create a new state wrapper with a random ID for an organism.
    fn make_organism_state(&mut self, o: Organism) -> OrganismState {
        OrganismState {
            id: self.rng.gen(),
            delay_cycles: 0,
            organism: o,
        }
    }
    /// Add an organism to the list, assigning it a random ID.
    fn add_organism(&mut self, o: Organism) {
        let o_state = self.make_organism_state(o);
        self.organisms.push_back(o_state);
    }
    /// Create an organism and add it to the list.
    fn spawn_organism(&mut self) {
        let pos = self.absolute(self.ui.selection().unwrap_or(ORIGIN));
        self.add_organism(Organism::init(pos));
    }
    /// Return the state of the focused organism, if any, as well as a set
    /// of points of all organisms.
    fn get_focused_and_all_locations(
        focus: Option<OrganismId>,
        organisms: &OrganismQueue
    ) -> (Option<&OrganismState>, HashSet<Point>) {
        let mut focused = None;
        let mut locations = HashSet::new();
        for o in organisms {
            if Some(o.id) == focus {
                focused = Some(o);
            }
            locations.insert(o.organism.ip);
        }
        (focused, locations)
    }
    /// Return the state of the focused organism, if any.
    fn get_focused(&self) -> Option<&OrganismState> {
        self.focus.map(|focus|
            self.organisms.iter()
                .find(|o| o.id == focus).unwrap())
    }
    /// Return the state of the focused organism mutably.
    fn get_focused_mut(&mut self) -> Option<&mut OrganismState> {
        self.focus.map(move |focus|
            self.organisms.iter_mut()
                .find(|o| o.id == focus).unwrap())
    }
    /// Turn a point relative to the view into a point relative to the grid.
    fn absolute(&self, p: Point) -> Point {
        let offset = self.ui.view_offset;
        let x = (offset.x + p.x) % self.grid.width();
        let y = (offset.y + p.y) % self.grid.height();
        Point { x, y }
    }
    /// Get the value of the byte that is currently selected.
    fn get_selected_byte(&self) -> Option<u8> {
        self.ui.selection()
            .map(|p| self.grid[self.absolute(p)])
    }
    /// Remove organisms that are exact duplicates.
    /// For simplicity, we also remove the focus.
    fn dedup_organisms(&mut self) {
        self.focus = None;
        let mut organisms = HashSet::<(u8, Organism)>::new();
        let mut new_organisms = VecDeque::new();
        for o in self.organisms.drain(..) {
            if organisms.insert((o.delay_cycles, o.organism.clone())) {
                new_organisms.push_back(o);
            }
        }
        self.organisms = new_organisms;
    }
}

// The main simulation loop.
impl<R: Read, W: Write> AppState<R, W> {
    /// Advance the old organism and add the new one (as long as we stay under
    /// the maximum).
    fn fork(&mut self, mut old: OrganismState, mut new: Organism) {
        // The resulting number of organisms, counting `old` and `new`.
        let new_organism_len = self.organisms.len() + 2;
        // Would this number be too many?
        let valid = match self.config.max_organisms {
            Some(max) => new_organism_len <= max,
            None => true,
        };
        old.organism.advance(&self.grid);
        self.organisms.push_back(old);
        if valid {
            new.advance(&self.grid);
            self.add_organism(new);
        }
    }
    /// Perform a cycle for a single organism. Panic if there are no organisms.
    fn cycle_organism(&mut self) {
        let mut state = self.organisms.pop_front().unwrap();
        if state.delay_cycles > 0 {
            // If the organism is still being delayed, do nothing.
            state.delay_cycles -= 1;
            self.organisms.push_back(state);
        } else {
            // Otherwise, read and execute an instruction.
            let instruction = self.grid[state.organism.ip];
            let instruction = Instruction::from_byte(instruction);
            match state.organism.run(&mut self.grid, instruction) {
                Response::Delay(delay) => {
                    state.delay_cycles = delay;
                    state.organism.advance(&self.grid);
                    self.organisms.push_back(state);
                }
                Response::Fork(new) => self.fork(state, new),
                Response::Die => {
                    if Some(state.id) == self.focus {
                        self.focus = None;
                    }
                    if self.organisms.is_empty() {
                        self.ui.alert_no_organisms();
                    }
                }
            }
        }
    }
    /// Perform a cycle for all organisms.
    fn cycle(&mut self) {
        for _ in 0..self.organisms.len() {
            self.cycle_organism();
        }
        self.total_cycles += 1;
        self.cycles_since_dedup += 1;
        if let Some(rate) = self.config.dedup_rate {
            if self.cycles_since_dedup >= rate {
                self.cycles_since_dedup = 0;
                self.dedup_organisms();
            }
        }
    }
}

impl<R: Read, W: Write> AppState<R, W> {
    /// Initialize the AppState by creating the grid and executing commands
    /// from the initialization file.
    pub(super) fn init(options: Options, stdin: R, stdout: W) -> Self {
        // Initialize the RNGs.
        let rng_seed = options.rng_seed.unwrap_or_else(rand::random);
        let mut rng = StdRng::seed_from_u64(rng_seed);
        let grid_rng = StdRng::seed_from_u64(rng.gen());
        // Create the app.
        let mut app = Self {
            total_cycles: 0,
            cycles_since_dedup: 0,
            rng,
            key_input: stdin.keys(),
            organisms: VecDeque::new(),
            grid: Grid::init(500, 500, grid_rng, 0),
            config: Config::new(rng_seed),
            commands: Commands::new(),
            ui: UI::new(stdout),
            focus: None,
            paused: false,
            quit: false,
        };
        app.ui.clear();
        // Run commands in an initialization file if one was passed.
        if let Some(f) = options.initial_file {
            app.run_commands_in_file(&f);
        }
        app
    }
    fn run_commands_in_file(&mut self, path: impl AsRef<Path>) {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            for command in contents.lines() {
                self.run_command(command);
            }
        } else {
            self.ui.info1(format!("Cannot read file '{}'.", path.as_ref().display()));
        }
    }
    fn run_command(&mut self, command: &str) {
        let command = command.trim();
        // Do nothing if it's a comment
        if command.as_bytes().get(0) == Some(&b'#') {
            return;
        }
        let mut args = Args::from_command(command);
        match args.next_raw() {
            None => {}
            Some(head) => {
                let handler = self.commands.handlers.get(head);
                if let Some(handler) = handler {
                    match Rc::clone(handler).run(self, args) {
                        Ok(()) => self.commands.last = Some(command.to_string()),
                        Err(e) => self.ui.info1(e.description()),
                    }
                } else {
                    self.ui.info1(format!("Command '{}' does not exist.", head))
                }
            }
        }
    }
    fn handle_key(&mut self, key: Key) {
        let grid_width = self.grid.width();
        let grid_height = self.grid.height();
        match key {
            Key::Char(':') => if let Some(cmd) = self.ui.input_command(&mut self.key_input) {
                self.run_command(&cmd);
            }
            Key::Char('.') => if let Some(cmd) = &self.commands.last {
                let cmd = cmd.clone();
                self.run_command(&cmd);
            }
            Key::Char(' ') => if self.paused { self.cycle() }
            Key::Char('h') => self.ui.move_view_offset(Dir::L, grid_width, grid_height),
            Key::Char('j') => self.ui.move_view_offset(Dir::D, grid_width, grid_height),
            Key::Char('k') => self.ui.move_view_offset(Dir::U, grid_width, grid_height),
            Key::Char('l') => self.ui.move_view_offset(Dir::R, grid_width, grid_height),
            Key::Char('w') => self.ui.info_scroll_up(),
            Key::Char('s') => self.ui.info_scroll_down(),
            Key::Right => self.ui.move_selection(Dir::R),
            Key::Left  => self.ui.move_selection(Dir::L),
            Key::Down  => self.ui.move_selection(Dir::D),
            Key::Up    => self.ui.move_selection(Dir::U),
            Key::Char('p') => self.toggle_pause(),
            Key::Esc => self.ui.select(None),
            _ => {}
        }
    }
    fn check_inputs(&mut self) {
        // Read key presses since the last update.
        while let Some(key) = self.key_input.next() {
            self.handle_key(key.unwrap());
            if self.quit {
                break;
            }
        }
    }
    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        self.ui.info1(
            if self.paused { "Paused." }
            else { "Unpaused." });
    }
    pub fn run(&mut self) {
        let frame_frequency_ms = 16u64;
        let frame_frequency = Duration::from_millis(frame_frequency_ms);
        let mut time_since_last_cycle = 0;
        while !self.quit {
            if !self.paused {
                time_since_last_cycle += frame_frequency_ms;
                let cycle_frequency = self.config.cycle_frequency as u64;
                while time_since_last_cycle > cycle_frequency {
                    self.cycle();
                    time_since_last_cycle -= cycle_frequency;
                }
            }
            let (focused, occupied) = Self::get_focused_and_all_locations(
                self.focus,
                &self.organisms,
            );
            self.ui.render_grid(&self.grid, focused, occupied);
            self.ui.render_status_box(
                self.total_cycles,
                self.organisms.len(),
                self.get_selected_byte(),
                focused.map(|o| &o.organism),
            );
            self.ui.flush();
            self.check_inputs();
            std::thread::sleep(frame_frequency);
        }
    }
}