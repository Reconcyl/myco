use rand::Rng;

use std::io::{Read, Write};
use std::collections::HashSet;

use super::{Organism, OrganismState, OrganismId, OrganismQueue};
use super::grid::{Grid, Dir, Point, ORIGIN};
use super::instruction::Instruction;
use super::organism::get_points_for_selection;

/// Enum representing different colors.
#[derive(Clone, Copy)]
pub enum Color {
    LightMagenta,
    LightRed,
    LightGreen,
    LightCyan,
    LightBlue,
    Red,
    Yellow,
    Blue,
    Gray,
    Reset,
    None,
}

impl Color {
    pub fn fg(self) -> String {
        use termion::color;
        match self {
            Color::LightMagenta => format!("{}", color::Fg(color::LightMagenta)),
            Color::LightRed     => format!("{}", color::Fg(color::LightRed)),
            Color::LightGreen   => format!("{}", color::Fg(color::LightGreen)),
            Color::LightCyan    => format!("{}", color::Fg(color::LightCyan)),
            Color::LightBlue    => format!("{}", color::Fg(color::LightBlue)),
            Color::Red          => format!("{}", color::Fg(color::Red)),
            Color::Yellow       => format!("{}", color::Fg(color::Yellow)),
            Color::Blue         => format!("{}", color::Fg(color::Blue)),
            Color::Gray         => format!("{}", color::Fg(color::AnsiValue::grayscale(4))),
            Color::Reset        => format!("{}", color::Fg(color::Reset)),
            Color::None         => String::new(),
        }
    }
    pub fn bg(self) -> String {
        use termion::color;
        match self {
            Color::LightMagenta => format!("{}", color::Bg(color::LightMagenta)),
            Color::LightRed     => format!("{}", color::Bg(color::LightRed)),
            Color::LightGreen   => format!("{}", color::Bg(color::LightGreen)),
            Color::LightCyan    => format!("{}", color::Bg(color::LightCyan)),
            Color::LightBlue    => format!("{}", color::Bg(color::LightBlue)),
            Color::Red          => format!("{}", color::Bg(color::Red)),
            Color::Yellow       => format!("{}", color::Bg(color::Yellow)),
            Color::Blue         => format!("{}", color::Bg(color::Blue)),
            Color::Gray         => format!("{}", color::Bg(color::AnsiValue::grayscale(4))),
            Color::Reset        => format!("{}", color::Bg(color::Reset)),
            Color::None         => String::new(),
        }
    }
}

/// General information relevant to the UI but not the simulation.
pub(super) struct UI<W> {
    /// Handle to raw mode STDOUT.
    stdout: W,
    /// The position of the point currently selected.
    selection: Option<Point>,
    /// The width of the viewing window, separate from the grid itself.
    view_width: u16,
    /// The height of the viewing window, separate from the grid itself.
    view_height: u16,
    /// The offset of the viewing window into the grid.
    pub view_offset: Point,
    /// The current lines of the info box.
    info_box: Vec<String>,
    /// The number of lines of the info box that can be displayed.
    info_box_view_height: u16,
    /// The index of the first line in the info box that is being displayed.
    /// At most, this should be max(0, info_box.len() - info_box_view_height).
    info_box_scroll_offset: usize,
    /// The number of lines currently taken up by the status box on the right.
    status_box_height: u16,
}

/// Convenience macro to write to STDOUT.
macro_rules! print {
    ($self:expr, $s:expr) => {
        print!($self, "{}", $s);
    };
    ($self:expr, $fmt:literal $(, $args:expr)*) => {
        write!($self.stdout, $fmt $(, $args)*).unwrap();
    };
}

// Methods relating to rendering bits of the UI.
impl<W: Write> UI<W> {
    /// Move the cursor to a given 1-based position.
    fn go_to(&mut self, term_x: u16, term_y: u16) {
        print!(self, termion::cursor::Goto(term_x, term_y));
    }
    /// Clear everything from the cursor until a newline.
    fn clear_right(&mut self) {
        print!(self, termion::clear::UntilNewline);
    }
    /// Clear the entire current line.
    fn clear_line(&mut self) {
        print!(self, termion::clear::CurrentLine);
    }
    /// Show the cursor.
    fn show_cursor(&mut self) {
        print!(self, termion::cursor::Show);
    }
    /// Hide the cursor.
    fn hide_cursor(&mut self) {
        print!(self, termion::cursor::Hide);
    }
    /// Move the cursor one character to the left.
    fn back(&mut self) {
        print!(self, 8 as char);
    }
    /// Render the contents of the info box.
    fn render_info_box(&mut self) {
        // The info box is placed 3 lines below the view window, but we add 1
        // to handle the gutter above the view window and 1 to compensate for
        // the coordinates being 1-based.
        let start_y = (self.view_height as u16) + 5;
        for line_no in 0..self.info_box_view_height {
            let term_y = start_y + line_no;
            // Clear the previous line
            self.go_to(2, term_y);
            self.clear_right();
            // Render the new one if necessary
            let line_idx = self.info_box_scroll_offset + line_no as usize;
            if let Some(line) = self.info_box.get(line_idx) {
                print!(self, line);
            }
        }
    }
    /// Render two given characters around a point.
    fn render_delimiters(&mut self, p: Point, start: char, end: char) {
        let term_x = (p.x as u16) * 3 + 2;
        let term_y = (p.y as u16) + 2;
        self.go_to(term_x,     term_y);
        print!(self, start);
        self.go_to(term_x + 3, term_y);
        print!(self, end);
    }
}

// Public getters and setters.
impl<W> UI<W> {
    pub fn selection(&self) -> Option<Point> {
        self.selection
    }
}

// Public methods related to UI rendering.
impl<W: Write> UI<W> {
    pub fn new(stdout: W) -> Self {
        // TODO: compute view_width, view_height, and info_box_view_height
        // based on the data termion provides about the width and height
        // of the terminal.
        let mut ui = Self {
            stdout,
            selection: None,
            view_width: 50,
            view_height: 50,
            view_offset: ORIGIN,
            info_box: Vec::new(),
            info_box_view_height: 10,
            info_box_scroll_offset: 0,
            status_box_height: 0,
        };
        ui.clear();
        ui
    }
    /// Flush STDOUT.
    pub fn flush(&mut self) {
        self.stdout.flush().unwrap();
    }
    /// Clear the screen.
    pub fn clear(&mut self) {
        print!(self, termion::clear::All);
    }
    /// Replace and redraw the existing info message.
    pub fn info(&mut self, info: Vec<String>) {
        self.info_box_scroll_offset = 0;
        self.info_box = info;
        self.render_info_box();
    }
    /// Replace the existing info message with a single-line one.
    pub fn info1<S: Into<String>>(&mut self, info: S) {
        self.info(vec![info.into()]);
    }
    /// Display an alert about there being no living organisms.
    pub fn alert_no_organisms(&mut self) {
        self.info1("There are no living organisms.");
    }
    /// Scroll the info box upwards one line and redraw.
    pub fn info_scroll_up(&mut self) {
        self.info_box_scroll_offset = self.info_box_scroll_offset.saturating_sub(1);
        self.render_info_box();
    }
    /// Scroll the info box downwards one line and redraw.
    pub fn info_scroll_down(&mut self) {
        // The maximum permissible height for info_box_scroll_offset.
        let max_offset = self.info_box.len().saturating_sub(self.info_box_view_height as usize);
        if self.info_box_scroll_offset < max_offset {
            self.info_box_scroll_offset += 1;
        }
        self.render_info_box();
    }
    /// Display a color-coded list of living organisms in the info box.
    pub fn list_organisms(&mut self, organisms: &OrganismQueue, focus: Option<OrganismId>) {
        if organisms.is_empty() {
            self.alert_no_organisms();
        } else {
            let mut lines = vec![String::from("Organisms:")];
            for (i, o) in organisms.iter().enumerate() {
                let color = if Some(o.id) == focus { Color::Yellow } else { Color::Blue };
                lines.push(format!("{color}{i}: {o}{reset}",
                    color = color.fg(),
                    i = i,
                    o = o.organism,
                    reset = Color::Reset.fg()
                ));
            }
            self.info(lines);   
        }
    }
    /// Replace the previous selection with a new selection and redraw it.
    pub fn select(&mut self, new_selection: Option<Point>) {
        if let Some(p) = self.selection {
            self.render_delimiters(p, ' ', ' ');
        }
        if let Some(p) = new_selection {
            self.render_delimiters(p, '[', ']');
        }
        self.selection = new_selection;
    }
    /// Move the selection in a particular direction and redraw it.
    pub fn move_selection(&mut self, dir: Dir) {
        let pos = self.selection.map(|p| p.move_in(
            dir,
            self.view_width as usize,
            self.view_height as usize,
        )).unwrap_or(ORIGIN);
        self.select(Some(pos));
    }
    /// Move the view offset in a particular direction. There is no need to redraw it because that
    /// is already done at frequent intervals.
    pub fn move_view_offset(&mut self, dir: Dir, grid_width: usize, grid_height: usize) {
        self.view_offset = self.view_offset.move_in(dir, grid_width, grid_height);
    }
    /// Render the status box data.
    pub fn render_status_box(
        &mut self,
        total_cycles: usize,
        num_organisms: usize,
        selected_byte: Option<u8>,
        focused_organism: Option<&Organism>,
    ) {
        let term_x = self.view_width as u16 * 3 + 3;
        let term_y = 2;
        // Clear the previous status box
        for i in 0..self.status_box_height {
            self.go_to(term_x, term_y + i);
            self.clear_right();
        }
        let mut status_lines = 0;
        /// Convenience macro to write a line of status.
        macro_rules! write_line {
            () => {
                status_lines += 1;
            };
            ($fmt:literal$(, $fmt_arg:expr)*) => {
                self.go_to(term_x + 1, term_y + status_lines);
                write!(self.stdout, $fmt $(, $fmt_arg)*).unwrap();
                status_lines += 1;
            }
        }
        write_line!("{:10}", total_cycles);
        write_line!("#{:9}", num_organisms);
        if let Some(byte) = selected_byte {
            write_line!("byte   {:3}", byte);
        }
        if let Some(o) = focused_organism {
            let Organism { dir, ax, bx, flag, .. } = o;
            let (first_row, column, bytes) = o.local_memory();
            write_line!("dir      {}", dir.to_char());
            write_line!("ax     {:3}", ax);
            write_line!("bx     {:3}", bx);
            write_line!("flag     {}", if *flag { 't' } else { 'f' });
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
                    self.go_to(term_x,     term_y);
                    print!(self, '[');
                    self.go_to(term_x + 3, term_y);
                    print!(self, ']');
                }
            }
            write_line!(" ...");
        }
        self.status_box_height = status_lines;
    }
    /// Render the colored cells in the grid.
    pub fn render_grid<R: Rng>(
        &mut self,
        grid: &Grid<R>,
        focused: Option<&OrganismState>,
        occupied: HashSet<Point>,
    ) {
        // Determine the position of the focused organism and the points in
        // the square that it is selecting.
        let (focused_pos, selected) = match focused {
            Some(o) => (
                Some(o.organism.ip),
                get_points_for_selection(
                    o.organism.cursor,
                    o.organism.r,
                    grid
                ).collect()
            ),
            None => (None, HashSet::new()),
        };
        // Get the points that should be viewed.
        let view = grid.view(
            self.view_offset,
            self.view_width as usize,
            self.view_height as usize
        );
        for (vis_y, row) in view.enumerate() {
            for (vis_x, (pos, byte)) in row.enumerate() {
                // Go to the correct position.
                let term_x = (vis_x as u16) * 3 + 3;
                let term_y = (vis_y as u16) + 2;
                self.go_to(term_x, term_y);
                // The focused IP is highlighted yellow; the focused organism's
                // selection is highlighted red, and non-focused IPs are
                // highlighted blue.
                let bg_color = if occupied.contains(&pos) {
                    if focused_pos == Some(pos) { Color::Yellow } else { Color::Blue }
                } else if selected.contains(&pos) {
                    Color::Red
                } else {
                    Color::None
                };
                let ins = Instruction::from_byte(byte);
                let fg_color = ins.category().color();
                // Write the instruction with the appropriate foreground and background colors.
                print!(self, "{}{}{}{}{}",
                    bg_color.bg(),
                    fg_color.fg(),
                    ins,
                    Color::Reset.fg(),
                    Color::Reset.bg()
                );
            }
        }
    }
    /// Display a command line that allows the user to enter a string.
    pub fn input_command<R: Read>(
        &mut self,
        key_input: &mut termion::input::Keys<R>,
    ) -> Option<String> {
        let mut command = String::new();
        let term_y = (self.view_height as u16) + 3;
        let term_x = 2;
        self.go_to(term_x, term_y);
        self.clear_right();
        self.show_cursor();
        print!(self, ": ");
        self.flush();
        loop {
            if let Some(key) = key_input.next() {
                use termion::event::Key;
                match key.unwrap() {
                    Key::Char('\n') => {
                        self.hide_cursor();
                        self.flush();
                        return Some(command);
                    }
                    Key::Char(c) => {
                        command.push(c);
                        write!(self.stdout, "{}", c).unwrap();
                        self.flush();
                    }
                    Key::Backspace => if command.pop().is_some() {
                        self.back();
                        print!(self, ' ');
                        self.back();
                        self.flush();
                    }
                    Key::Esc => {
                        self.clear_line();
                        self.hide_cursor();
                        self.flush();
                        return None;
                    }
                    _ => {}
                }
            }
        };
    }
}