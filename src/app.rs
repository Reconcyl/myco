use rand::{SeedableRng as _, Rng as _};
use rand::rngs::StdRng;

use termion::event::Key;

use std::collections::HashMap;
use std::io::{Read, Write};
use std::rc::Rc;

/// The instruction enum.
mod instruction;
/// The data structure storing organisms.
mod organism;
/// Parsing logic related to commands.
mod command;
/// Implementation code for commands.
mod commands;
/// Logic for rendering the UI.
mod ui;

use super::Options;
use crate::grid::{Grid, Point, ORIGIN, Dir};
use instruction::Instruction;
use organism::{OrganismCollection, OrganismState, OrganismId};
use command::{CommandHandler, Args};
use ui::UI;

/// General-purpose app error enum.
#[derive(Clone, Copy)]
pub enum Error {
    BadWidth,
    BadHeight,
}

impl Error {
    pub fn description(self) -> std::borrow::Cow<'static, str> {
        match self {
            Error::BadWidth => "Width cannot be 0.".into(),
            Error::BadHeight => "Height cannot be 0.".into(),
        }
    }
}

/// Rarely- or never- modified configuration information for the app.
struct Config {
    /// The seed for the RNG. This is never changed during execution.
    rng_seed: u64,
    /// How many milliseconds to wait between cycles.
    cycle_frequency: u32,
    /// The number of cosmic rays per cycle.
    cosmic_ray_rate: u32,
    /// The maximum number of organisms, if any before reproduction doesn't work.
    max_organisms: Option<usize>,
    /// How many cycles to wait between dedup passes. If zero, then never
    /// perform dedup passes.
    dedup_rate: usize,
}

impl Config {
    fn new(rng_seed: u64) -> Self {
        Self {
            rng_seed,
            cycle_frequency: 100,
            cosmic_ray_rate: 0,
            max_organisms: None,
            dedup_rate: 0,
        }
    }   
}

struct Commands<W> {
    /// The last valid command executed. This is used by the '.' key to
    /// re-execute commands.
    last: Option<String>,
    /// Handlers for various commands.
    handlers: HashMap<String, Rc<dyn CommandHandler<W>>>,
}

impl<W: Write> Commands<W> {
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
        result.register("cosmic-ray-rate", commands::cosmic_ray_rate());
        result.register_aliases(&["c", "cycle"], commands::cycle());
        result.register_aliases(&["p", "pause"], commands::pause());
        result.register("move", commands::move_());
        result.register_aliases(&["w", "write"], commands::write());
        result.register("|", commands::insert_line());
        result.register("byte", commands::byte());
        result.register("spawn", commands::spawn());
        result.register("dedup", commands::dedup());
        result.register("auto-dedup", commands::auto_dedup());
        result.register_aliases(&["f", "focus"], commands::focus());
        result.register("view", commands::view());
        result.register("ip", commands::move_ip());
        result.register_aliases(&["r", "run"], commands::run());
        result.register("kill", commands::kill());
        result
    }
    fn register(&mut self, name: &str, handler: Rc<dyn CommandHandler<W>>) {
        self.handlers.insert(String::from(name), handler);
    }
    fn register_aliases(&mut self, names: &[&str], handler: Rc<dyn CommandHandler<W>>) {
        for name in names {
            self.register(name, Rc::clone(&handler));
        }
    }
}

pub struct AppState<W> {
    /// The total number of cycles that have passed.
    total_cycles: usize,
    /// How many cycles have passed since a dedup occurred.
    cycles_since_dedup: usize,
    /// The RNG used to generate cosmic rays.
    cosmic_ray_rng: StdRng,
    /// The collection of organisms.
    organisms: OrganismCollection,
    /// The instruction grid.
    grid: Grid<StdRng>,
    /// Configuration information.
    config: Config,
    /// Command-line parsing information.
    commands: Commands<W>,
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
impl<W: Write> AppState<W> {
    /// Create an organism and add it to the list.
    fn spawn_organism(&mut self) {
        let pos = self.absolute(self.ui.selection().unwrap_or(ORIGIN));
        self.organisms.insert(OrganismState::init(pos));
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
    /// Repeatedly make random modifications to the grid.
    fn cosmic_rays(&mut self) {
        for _ in 0..self.config.cosmic_ray_rate {
            let x = self.cosmic_ray_rng.gen_range(0, self.grid.width());
            let y = self.cosmic_ray_rng.gen_range(0, self.grid.height());
            let val = self.cosmic_ray_rng.gen();
            self.grid.set(Point { x, y }, val);
        }
    }
}

// The main simulation loop.
impl<W: Write> AppState<W> {
    /// Perform a cycle for all organisms.
    fn cycle(&mut self) {
        self.organisms.run_cycle(&mut self.grid, self.config.max_organisms);
        self.cosmic_rays();
        // If the focused organism is no longer alive, set it to `None`.
        if let Some(id) = self.focus {
            if self.organisms.alive(id) {
                self.focus = None;
            }
        }
        self.total_cycles += 1;
        self.cycles_since_dedup += 1;
        let rate = self.config.dedup_rate;
        if rate != 0 && self.cycles_since_dedup >= rate {
            self.cycles_since_dedup = 0;
            self.organisms.dedup();
        }
    }
}

impl<W: Write> AppState<W> {
    /// Initialize the AppState by creating the grid and executing commands
    /// from the initialization file.
    pub(super) fn init(options: Options, stdout: Option<W>) -> Result<Self, Error> {
        if options.grid_width == 0 {
            return Err(Error::BadWidth);
        }
        if options.grid_height == 0 {
            return Err(Error::BadHeight);
        }
        // Initialize the RNGs.
        let rng_seed = options.rng_seed.unwrap_or_else(rand::random);
        let mut rng  = StdRng::seed_from_u64(rng_seed);
        let grid_rng = StdRng::seed_from_u64(rng.gen());
        let kill_rng = StdRng::seed_from_u64(rng.gen());
        // Create the app.
        let mut app = Self {
            total_cycles: 0,
            cycles_since_dedup: 0,
            cosmic_ray_rng: rng,
            organisms: OrganismCollection::new(kill_rng),
            grid: Grid::init(
                options.grid_width,
                options.grid_height,
                grid_rng,
                Instruction::Nop as u8,
                options.write_error_chance,
            ),
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
        Ok(app)
    }
    fn run_commands_in_file(&mut self, path: impl AsRef<std::path::Path>) {
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
    fn handle_key<R: Read>(&mut self, key: Key, key_input: &mut termion::input::Keys<R>) {
        let grid_width = self.grid.width();
        let grid_height = self.grid.height();
        match key {
            Key::Char(':') => if let Some(cmd) = self.ui.input_command(key_input) {
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
    fn check_inputs<R: Read>(&mut self, key_input: &mut termion::input::Keys<R>) {
        // Read key presses since the last update.
        while let Some(key) = key_input.next() {
            self.handle_key(key.unwrap(), key_input);
            if self.quit {
                break;
            }
        }
    }
    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        self.ui.info1(
            if self.paused {
                "Paused."
            } else {
                "Unpaused."
            }
        );
    }
    pub fn run<R: Read>(&mut self, mut key_input: termion::input::Keys<R>) {
        use std::time::Duration;
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
            let focused = self.organisms.get_opt(self.focus).map(|ctx| &ctx.organism);
            let occupied = self.organisms.iter().map(|ctx| ctx.organism.ip).collect();
            self.ui.render_grid(&self.grid, focused, occupied);
            self.ui.render_status_box(
                self.total_cycles,
                self.organisms.len(),
                self.get_selected_byte(),
                focused,
            );
            self.ui.flush();
            self.check_inputs(&mut key_input);
            std::thread::sleep(frame_frequency);
        }
    }
}