use rand::{SeedableRng as _, Rng as _};
use rand::rngs::StdRng;

use termion::{color, clear, cursor::{self, Goto}};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode as _;

use std::collections::{VecDeque, HashSet};
use std::io::{self, Read, Write};
use std::fmt::Write as _;
use std::time::Duration;
use std::cmp::max;

mod instruction;
mod grid;
mod organism;

use instruction::Instruction;
use grid::{Grid, ORIGIN, Point, Dir};
use organism::{Organism, Response, get_points_for_selection};

type OrganismId = u64;

struct OrganismState {
    id: u64,
    delay_cycles: u8,

    organism: Organism,
}

struct RenderState {}

impl RenderState {
    fn compute() -> Self {
        Self {}
    }
}

struct AppState<R, W> {
    /// The seed for the RNG.
    rng_seed: u64,
    /// The RNG.
    rng: StdRng,
    /// Iterator of keys from STDIN.
    key_input: termion::input::Keys<R>,
    /// Handle to raw mode STDOUT.
    stdout: W,
    /// How many milliseconds to wait between cycles.
    cycle_frequency: u32,
    /// The maximum number of organisms before reproduction doesn't work.
    max_organisms: usize,
    /// The set of organisms.
    organisms: VecDeque<OrganismState>,
    /// The instruction grid.
    grid: Grid<StdRng>,
    /// The number of cycles that have passed.
    cycles: usize,
    /// The ID of the organism currently being watched.
    focus: Option<OrganismId>,
    /// Information related to rendering the view.
    render_state: RenderState,
    /// The position of the point currently selected.
    selected: Option<Point>,
    /// The width of the viewing window, separate from the grid itself.
    view_width: usize,
    /// The height of the viewing window, separate from the grid itself.
    view_height: usize,
    /// The offset of the viewing window into the grid.
    view_offset: Point,
    /// Whether execution is paused.
    paused: bool,
    /// The number of lines currently taken up by the info box.
    info_lines: usize,
    /// The number of lines currently taken up by the status box.
    status_lines: u16,
    /// The last valid command.
    last_command: Option<String>,
    /// The inverse probability of a cosmic ray ocurring on a given cycle.
    /// This is set to 0 if the probability is 0.
    cosmic_ray_chance: u32,
    /// Whether the app should quit next frame.
    quit: bool,
}

impl<R: Read, W: Write> AppState<R, W> {
    fn init(seed: Option<u64>, stdin: R, stdout: W) -> Self {
        let rng_seed = seed.unwrap_or_else(|| rand::random());
        let mut rng = StdRng::seed_from_u64(rng_seed);
        let grid_rng = StdRng::seed_from_u64(rng.gen());
        Self {
            rng_seed,
            rng,
            key_input: stdin.keys(),
            stdout,
            cycle_frequency: 100,
            max_organisms: 50,
            organisms: VecDeque::new(),
            view_width: 50,
            view_height: 50,
            view_offset: ORIGIN,
            grid: Grid::init(500, 500, grid_rng, 0),
            cycles: 0,
            focus: None,
            render_state: RenderState::compute(),
            selected: None,
            paused: false,
            info_lines: 0,
            status_lines: 0,
            last_command: None,
            cosmic_ray_chance: 0,
            quit: false,
        }
    }
    fn make_organism_state(&mut self, o: Organism) -> OrganismState {
        OrganismState {
            id: self.rng.gen(),
            delay_cycles: 0,
            organism: o,
        }
    }
    fn add_organism(&mut self, o: Organism) {
        let o_state = self.make_organism_state(o);
        self.organisms.push_back(o_state);
    }
    fn get_organism_locations(&self) -> (Option<&OrganismState>, HashSet<Point>) {
        let mut focused = None;
        let mut locations = HashSet::new();
        for o in &self.organisms {
            if Some(o.id) == self.focus {
                focused = Some(o);
            }
            locations.insert(o.organism.ip);
        }
        (focused, locations)
    }
    fn get_focused(&self) -> Option<&OrganismState> {
        self.organisms.iter().find(|o| Some(o.id) == self.focus)
    }
    fn get_focused_mut_from(organisms: &mut VecDeque<OrganismState>, focused: OrganismId)
        -> &mut OrganismState
    {
        organisms.iter_mut().find(|o| o.id == focused).unwrap()
    }
    fn handle_offset(&self, p: Point) -> Point {
        let x = (self.view_offset.x + p.x) % self.grid.width();
        let y = (self.view_offset.y + p.y) % self.grid.height();
        Point { x, y }
    }
    /// Perform a cycle for a single organism.
    fn cycle_organism(&mut self) {
        let mut o_state = self.organisms.pop_front().unwrap();
        if o_state.delay_cycles > 0 {
            // If the organism is still being delayed, do nothing.
            o_state.delay_cycles -= 1;
            self.organisms.push_back(o_state);
        } else {
            // Otherwise, read and execute an instruction.
            let organism = &mut o_state.organism;
            let instruction = self.grid[organism.ip];
            let instruction = Instruction::from_byte(instruction);
            let mut dead = false;
            match organism.run(&mut self.grid, instruction) {
                Response::Delay(delay) => o_state.delay_cycles = delay,
                Response::Fork(mut new) =>
                    // We add one because `o_state` counts but is not present
                    if self.organisms.len() + 1 < self.max_organisms {
                        new.advance(&self.grid);
                        self.add_organism(new);
                    }
                Response::Die => {
                    if Some(o_state.id) == self.focus {
                        self.focus = None;
                    }
                    if self.organisms.is_empty() {
                        self.alert_no_organisms();
                    }
                    dead = true;
                }
            }
            if !dead {
                organism.advance(&self.grid);
                self.organisms.push_back(o_state);
            }
        }
    }
    /// Perform a cycle for all organisms.
    fn cycle(&mut self) {
        for _ in 0..self.organisms.len() {
            self.cycle_organism();
        }
        if self.cosmic_ray_chance > 0 && self.rng.gen_ratio(1, self.cosmic_ray_chance) {
            let x = self.rng.gen_range(0, self.grid.width());
            let y = self.rng.gen_range(0, self.grid.height());
            let val = self.rng.gen();
            self.grid.set(Point { x, y }, val);
        }
        self.cycles += 1;
    }
    
    fn alert_no_organisms(&mut self) {
        self.render_info(&["There are no living organisms."]);
    }
    fn list_organisms(&mut self) {
        if self.organisms.is_empty() {
            self.alert_no_organisms();
        } else {
            let mut lines = vec![String::from("Organisms:")];
            for (i, o) in self.organisms.iter().enumerate() {
                let mut line = if Some(o.id) == self.focus {
                    format!("{}", color::Fg(color::Yellow))
                } else {
                    format!("{}", color::Fg(color::Blue))
                };
                write!(&mut line, "{}: {}", i, o.organism).unwrap();
                write!(&mut line, "{}", color::Fg(color::Reset)).unwrap();
                lines.push(line);
            }
            self.render_info(&lines);    
        }
    }
    fn render_selection(&mut self, new_selection: Option<Point>) {
        let stdout = &mut self.stdout;
        let mut set_char = |term_x, term_y, c| {
            write!(stdout, "{}{}", Goto(term_x, term_y), c).unwrap();
        };
        if let Some(p) = self.selected {
            let term_x = (p.x as u16) * 3 + 2;
            let term_y = (p.y as u16) + 2;
            set_char(term_x,     term_y, ' ');
            set_char(term_x + 3, term_y, ' ');
        }
        if let Some(p) = new_selection {
            let term_x = (p.x as u16) * 3 + 2;
            let term_y = (p.y as u16) + 2;
            set_char(term_x,     term_y, '[');
            set_char(term_x + 3, term_y, ']');
        }
        self.selected = new_selection;
    }
    fn move_selection(&mut self, dir: Dir) {
        let new_selection = match self.selected {
            Some(p) => p.move_in(dir, self.view_width, self.view_height),
            None => self.get_focused().map_or(ORIGIN,
                |p| p.organism.ip.move_in(dir, self.view_width, self.view_height)),
        };
        self.render_selection(Some(new_selection));
    }
    fn move_offset(&mut self, dir: Dir) {
        self.view_offset = self.view_offset.move_in(dir, self.grid.width(), self.grid.height());
    }
    fn render_status_box(&mut self) {
        let term_x = self.view_width as u16 * 3 + 4 - 1;
        let term_y = 2;
        let mut status_lines = 0;
        // Clear the previous status box
        for i in 0..self.status_lines {
            write!(self.stdout, "{}{}", Goto(term_x, term_y + i), clear::UntilNewline).unwrap();
        }
        macro_rules! write_line {
            () => {
                status_lines += 1;
            };
            ($fmt:literal$(, $fmt_arg:expr)*) => {
                write!(self.stdout, "{}", Goto(term_x + 1, term_y + status_lines)).unwrap();
                write!(self.stdout, $fmt $(, $fmt_arg)*).unwrap();
                status_lines += 1;
            }
        }
        write_line!("{:10}", self.cycles);
        write_line!("#{:9}", self.organisms.len());
        if let Some(pos) = self.selected {
            write_line!("byte   {:3}", self.grid[self.handle_offset(pos)]);
        }
        if let Some(o) = self.get_focused() {
            let (dir, ax, bx, flag) = (
                o.organism.dir, o.organism.ax, o.organism.bx, o.organism.flag);
            let (first_row, column, bytes) = o.organism.local_memory();
            write_line!("dir      {}", dir.to_char());
            write_line!("ax     {:3}", ax);
            write_line!("bx     {:3}", bx);
            write_line!("flag     {}", if flag { 't' } else { 'f' });
            if first_row {
                write_line!();
            } else {
                write_line!(" ...");
            }
            for row in 0..3 {
                let offset = row*3;
                write_line!("{:02x} {:02x} {:02x} {:02x}",
                    bytes[offset],
                    bytes[offset+1],
                    bytes[offset+2],
                    bytes[offset+3]
                );
                if row == (!first_row as usize) {
                    let term_y = term_y + status_lines - 1;
                    let term_x = term_x + column as u16 * 3;
                    write!(self.stdout, "{}[{}]",
                        Goto(term_x, term_y),
                        Goto(term_x+3, term_y),
                    ).unwrap();
                }
            }
            write_line!(" ...");
        }
        self.status_lines = status_lines;
    }
    fn render_info<S: AsRef<str>>(&mut self, new_info: &[S]) {
        let start_y = (self.view_height as u16) + 5;
        for line_no in 0..max(new_info.len(), self.info_lines) {
            let term_y = start_y + line_no as u16;
            write!(self.stdout, "{}", Goto(2, term_y)).unwrap();
            if line_no < self.info_lines {
                write!(self.stdout, "{}", clear::CurrentLine).unwrap();
            }
            if let Some(line) = new_info.get(line_no) {
                write!(self.stdout, "{}", line.as_ref()).unwrap();
            }
        }
        self.info_lines = new_info.len();
    }
    fn render_grid(&mut self) {
        let (focused, occupied) = self.get_organism_locations();
        let (focused_pos, selected) = match focused {
            Some(o) => (
                Some(o.organism.ip),
                get_points_for_selection(
                    o.organism.cursor,
                    o.organism.r,
                    &self.grid
                ).collect()
            ),
            None => (None, HashSet::new()),
        };
        let view = self.grid.view(self.view_offset, self.view_width, self.view_height);
        for (vis_y, row) in view.enumerate() {
            for (vis_x, (pos, byte)) in row.enumerate() {
                // Go to the correct position.
                let term_x = (vis_x as u16) * 3 + 3;
                let term_y = (vis_y as u16) + 2;
                write!(self.stdout, "{}", Goto(term_x, term_y)).unwrap();
                // Set background color if necessary.
                let mut is_bg_colored = false;
                if occupied.contains(&pos) {
                    if Some(pos) == focused_pos {
                        write!(self.stdout, "{}", color::Bg(color::Yellow)).unwrap();
                    } else {
                        write!(self.stdout, "{}", color::Bg(color::Blue)).unwrap();
                    }
                    is_bg_colored = true;
                } else if selected.contains(&pos) {
                    write!(self.stdout, "{}", color::Bg(color::Red)).unwrap();
                    is_bg_colored = true;
                }
                let ins = Instruction::from_byte(byte);
                // Write the instruction.
                write!(self.stdout, "{}{}{}",
                    ins.category().fg_color_sequence(),
                    ins,
                    color::Fg(color::Reset)
                ).unwrap();
                // Clear the colors if necessary.
                if is_bg_colored {
                    write!(self.stdout, "{}", color::Bg(color::Reset)).unwrap();
                }
            }
        }
        self.stdout.flush().unwrap();
    }
    fn run_commands_in_file(&mut self, path: &str) {
        if let Ok(contents) = std::fs::read_to_string(path) {
            for command in contents.lines() {
                self.run_command(command);
            }
        } else {
            self.render_info(&[format!("Cannot read file '{}'.", path)]);
        }
    }
    fn run_command(&mut self, command: &str) {
        let command = command.trim();
        // comment
        if command.as_bytes().get(0) == Some(&b'#') {
            return;
        }
        let mut valid_command = false;
        let parts: Vec<_> = command.split_whitespace().collect();
        if parts.get(0) == Some(&"|") {
            // special writing syntax
            let instructions: Option<Vec<_>> = parts.iter().skip(1)
                .map(|s| Instruction::from_symbol(s)).collect();
            if let Some(instructions) = instructions {
                if let Some(selected) = self.selected {
                    let mut pos = selected;
                    for ins in instructions {
                        self.grid.set(pos, ins as u8);
                        pos = pos.right(self.grid.width());
                    }
                    self.move_selection(Dir::D);
                }
                valid_command = true;
            }
        } else {
            match &parts[..] {
                [] => {
                    self.render_info::<&str>(&[]);
                    valid_command = true;
                }
                // Info and configuration
                ["l"] | ["list"] => {
                    self.list_organisms();
                    valid_command = true;
                }
                ["max"] => {
                    self.render_info(&[
                        format!("The current organism maximum is {}.", self.max_organisms)]);
                    valid_command = true;
                }
                ["max", n] => {
                    if let Ok(n) = n.parse() {
                        self.max_organisms = n;
                        self.render_info(&[format!("Set organism maximum to {}.", n)]);
                        valid_command = true;
                    }
                }
                ["speed"] => {
                    self.render_info(&[
                        format!("The current simulation speed is {}ms/cycle.", self.cycle_frequency)]);
                    valid_command = true;
                }
                ["speed", n] => {
                    if let Ok(n) = n.parse() {
                        if n != 0 {
                            self.cycle_frequency = n;
                            self.render_info(&[format!("Set the simulation speed to {}ms/cycle.", n)]);
                            valid_command = true;
                        }
                    }
                }
                ["seed"] => {
                    self.render_info(&[
                        format!("The RNG seed is {}.", self.rng_seed)]);
                    valid_command = true;
                }
                ["source", path] => {
                    self.run_commands_in_file(path);
                    valid_command = true;
                }
                ["cosmic-ray-chance"] => {
                    let chance = self.cosmic_ray_chance;
                    self.render_info(&[
                        if chance > 0 {
                            format!("The current cosmic ray chance is 1/{}", chance)
                        } else {
                            String::from("The current cosmic ray chance is 0.")
                        }
                    ]);
                    valid_command = true;
                }
                ["cosmic-ray-chance", chance] => {
                    if let Ok(chance) = chance.parse() {
                        self.cosmic_ray_chance = chance;
                        self.render_info(&[
                            if chance > 0 {
                                format!("Set the cosmic ray chance to 1/{}.", chance)
                            } else {
                                String::from("Set the cosmic ray chance to 0.")
                            }
                        ]);
                        valid_command = true;
                    }
                }
                ["write-error-chance"] => {
                    let chance = self.grid.write_error_chance;
                    self.render_info(&[
                        if chance > 0 {
                            format!("The current write error chance is 1/{}", chance)
                        } else {
                            String::from("The current write error chance is 0.")
                        }
                    ]);
                    valid_command = true;
                }
                ["write-error-chance", chance] => {
                    if let Ok(chance) = chance.parse() {
                        self.grid.write_error_chance = chance;
                        self.render_info(&[
                            if chance > 0 {
                                format!("Set the write error chance to 1/{}.", chance)
                            } else {
                                String::from("Set the write error chance to 0.")
                            }
                        ])
                    }
                    valid_command = true;
                }
                // Simulation
                ["c"] | ["cycle"] => {
                    self.cycle();
                    self.render_info(&["Ran a cycle."]);
                    valid_command = true;
                }
                ["c", n] | ["cycle", n] => {
                    if let Ok(n) = n.parse::<u32>() {
                        for _ in 0..n {
                            self.cycle();
                        }
                        self.render_info(&[format!("Ran {} cycles.", n)]);
                        valid_command = true;
                    }
                }
                ["p"] | ["pause"] => {
                    self.toggle_pause();
                    valid_command = true;
                }
                ["move", dir] => {
                    if let Some(dir) = Dir::from_str(dir) {
                        self.move_selection(dir);
                        valid_command = true;
                    }
                }
                ["move", dir, n] => {
                    if let (Some(dir), Ok(n)) = (Dir::from_str(dir), n.parse::<u16>()) {
                        for _ in 0..n {
                            self.move_selection(dir);
                        }
                        valid_command = true;
                    }
                }
                ["write", ins] => {
                    if let Some(ins) = Instruction::from_symbol(ins) {
                        if let Some(selected) = self.selected {
                            self.grid.set(self.handle_offset(selected), ins as u8);
                        } else {
                            self.render_info(&["Nowhere to write."]);
                        };
                        valid_command = true;
                    }
                }
                ["byte", byte] => {
                    if let Ok(byte) = byte.parse() {
                        let msg = if let Some(selected) = self.selected {
                            self.grid.set(self.handle_offset(selected), byte);
                            format!("Wrote byte {} at cursor.", byte)
                        } else {
                            String::from("Nothing to write.")
                        };
                        self.render_info(&[msg]);
                        valid_command = true;
                    }
                }
                ["spawn"] => {
                    self.add_organism(Organism::init(self.selected.unwrap_or(ORIGIN)));
                    valid_command = true;
                }
                ["q"] | ["quit"] => {
                    self.quit = true;
                    valid_command = true;
                }
                // Focus
                ["f", idx] | ["focus", idx] => {
                    if let Ok(idx) = idx.parse() {
                        if let Some(o) = self.organisms.get(idx) {
                            self.focus = Some(o.id);
                            self.render_info(&[
                                format!("Set focus to organism {}.", idx)]);
                            valid_command = true;
                        }
                    }
                }
                ["uf"] | ["unfocus"] => {
                    self.focus = None;
                    valid_command = true;
                }
                ["view"] => {
                    if let Some(o) = self.get_focused() {
                        self.view_offset = o.organism.ip;
                    } else {
                        self.render_info(&["No organism is selected."])
                    }
                    valid_command = true;
                }
                ["ip", dir] => {
                    if let Some(dir) = Dir::from_str(dir) {
                        if let Some(id) = self.focus {
                            let o = Self::get_focused_mut_from(&mut self.organisms, id);
                            o.organism.ip = o.organism.ip.move_in(
                                dir, self.grid.width(), self.grid.height());
                        } else {
                            self.render_info(&["No organism is selected."]);
                        };
                        valid_command = true;
                    }                
                }
                ["ip", dir, n] => {
                    if let (Some(dir), Ok(n)) = (Dir::from_str(dir), n.parse::<u16>()) {
                        if let Some(id) = self.focus {
                            let o = Self::get_focused_mut_from(&mut self.organisms, id);
                            for _ in 0..n {
                                o.organism.ip = o.organism.ip.move_in(
                                    dir, self.grid.width(), self.grid.height());
                            }
                        } else {
                            self.render_info(&["No organism is selected."]);
                        };
                        valid_command = true;
                    }
                }
                ["r", ins] | ["run", ins] => {
                    if let Some(ins) = Instruction::from_symbol(ins) {
                        let msg = if let Some(id) = self.focus {
                            let o = Self::get_focused_mut_from(&mut self.organisms, id);
                            match o.organism.run(&mut self.grid, ins) {
                                Response::Delay(_) => {
                                    "Executed."
                                }
                                Response::Fork(new) => {
                                    self.add_organism(new);
                                    "Executed."
                                }
                                Response::Die => {
                                    "Use the :kill command instead."
                                }
                            }
                        } else {
                            "No organism is selected."
                        };
                        self.render_info(&[msg]);
                        valid_command = true;
                    }
                }
                ["kill"] => {
                    if let Some(id) = self.focus.take() {
                        self.organisms.retain(|o| o.id != id);
                    } else {
                        self.render_info(&["No organism is selected."])
                    };
                    valid_command = true;
                }
                _ => {}
            }
        }

        if valid_command {
            self.last_command = Some(command.to_string());
        } else {
            self.render_info(&[
                String::from("Malformed command:"),
                format!(" {}", command),
            ]);
        }
    }
    fn command_line(&mut self) {
        let mut command = String::new();
        let term_y = (self.view_height as u16) + 3;
        let term_x = 2;
        write!(self.stdout, "{}{}{}: ",
            Goto(term_x, term_y),
            clear::CurrentLine,
            cursor::Show,
        ).unwrap();
        self.stdout.flush().unwrap();
        let do_execute = loop {
            if let Some(key) = self.key_input.next() {
                match key.unwrap() {
                    Key::Char('\n') => break true,
                    Key::Char(c) => {
                        command.push(c);
                        write!(self.stdout, "{}", c).unwrap();
                        self.stdout.flush().unwrap();
                    }
                    /*
                    Key::Up => {
                        let history_len = self.command_history.len();
                        if command_edit_idx < history_len {
                            command_edit_idx += 1;
                            if command_edit_idx == editable_commands.len() {
                                editable_commands.push(
                                    self.command_history[history_len - command_edit_idx].clone());
                            }
                            write!(self.stdout, "{}{}: {}",
                                Goto(term_x, term_y),
                                clear::CurrentLine,
                                editable_commands[command_edit_idx],
                            ).unwrap();
                            self.stdout.flush().unwrap();
                        }
                    }
                    Key::Down => {
                        if command_edit_idx > 0 {
                            command_edit_idx -= 1;
                            write!(self.stdout, "{}{}: {}",
                                Goto(term_x, term_y),
                                clear::CurrentLine,
                                editable_commands[command_edit_idx],
                            ).unwrap();
                            self.stdout.flush().unwrap();
                        }
                    }
                    */
                    Key::Backspace => {
                        if command.pop().is_some() {
                            write!(self.stdout, "{} {0}", 8 as char).unwrap();
                            self.stdout.flush().unwrap();
                        }
                    }
                    Key::Esc => break false,
                    _ => {}
                }
            }
        };
        if do_execute {
            self.run_command(&command);
        } else {
            write!(self.stdout, "{}", clear::CurrentLine).unwrap();
        }
        write!(self.stdout, "{}", cursor::Hide).unwrap();
        self.stdout.flush().unwrap();
    }
    fn handle_key(&mut self, key: Key) {
        match key {
            Key::Char(':') => self.command_line(),
            Key::Char('.') => if let Some(cmd) = &self.last_command {
                let cmd = cmd.clone();
                self.run_command(&cmd);
            }
            Key::Char(' ') => if self.paused { self.cycle() }
            Key::Char('h') => self.move_offset(Dir::L),
            Key::Char('j') => self.move_offset(Dir::D),
            Key::Char('k') => self.move_offset(Dir::U),
            Key::Char('l') => self.move_offset(Dir::R),
            Key::Right => self.move_selection(Dir::R),
            Key::Left  => self.move_selection(Dir::L),
            Key::Down  => self.move_selection(Dir::D),
            Key::Up    => self.move_selection(Dir::U),
            Key::Char('p') => self.toggle_pause(),
            Key::Esc => self.render_selection(None),
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
        self.render_info(&[
            if self.paused { "Paused." }
            else { "Unpaused." }
        ]);
    }
    fn run(&mut self) {
        write!(self.stdout, "{}", clear::All).unwrap();
        let frame_frequency_ms = 16u64;
        let frame_frequency = Duration::from_millis(frame_frequency_ms);
        let mut time_since_last_cycle = 0;
        while !self.quit {
            if !self.paused {
                time_since_last_cycle += frame_frequency_ms;
                while time_since_last_cycle > self.cycle_frequency as u64 {
                    self.cycle();
                    time_since_last_cycle -= self.cycle_frequency as u64;
                }
            }
            self.render_grid();
            self.render_status_box();
            self.check_inputs();
            std::thread::sleep(frame_frequency);
        }
    }
}

pub fn main() {
    let stdout = io::stdout();
    let stdout = stdout.into_raw_mode().unwrap();
    let stdout = termion::screen::AlternateScreen::from(stdout);
    let stdout = cursor::HideCursor::from(stdout);

    let stdin = termion::async_stdin();
    let mut app = AppState::init(Some(4), stdin, stdout);
    app.add_organism(Organism::init(ORIGIN));
    app.run_commands_in_file("init.myco");
    app.run();
}