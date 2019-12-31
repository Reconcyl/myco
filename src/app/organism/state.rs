use rand::Rng;

use std::mem::swap;

use crate::grid::{Grid, Point, Dir};
use super::Instruction;

// Return the square root of an odd square number between 1 and 441.
fn isqrt(n: usize) -> u8 {
    match n {
        1 => 1,
        9 => 3,
        25 => 5,
        49 => 7,
        81 => 9,
        121 => 11,
        169 => 13,
        225 => 15,
        289 => 17,
        361 => 19,
        441 => 21,
        n => panic!("{} is not a valid square number", n),
    }
}

pub enum Response {
    Delay(u8),
    Fork(OrganismState),
    Die,
}

fn selection_radius(selection: &[u8]) -> u8 {
    (isqrt(selection.len()) - 1) / 2
}

pub fn get_points_for_selection<R>(
    cursor: Point,
    r: u8,
    grid: &Grid<R>
) -> impl Iterator<Item=Point> {
    let r = r as isize;
    let width = grid.width();
    let height = grid.height();
    (-r..=r).flat_map(move |dx| (-r..=r).map(move |dy| {
        Point::from_modular(
            cursor.x as isize + dx,
            cursor.y as isize + dy,
            width, height)
    }))
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct OrganismState {
    /// Instruction pointer
    pub ip: Point,
    /// IP direction
    pub dir: Dir,
    /// Cursor position
    pub cursor: Point,
    /// Clipboard data (square matrix of odd size 1..=21)
    clipboard: Vec<u8>,
    /// Selection radius (0..=10)
    pub r: u8,
    /// General-purpose control flow flag
    pub flag: bool,
    /// General-purpose register AX
    pub ax: u8,
    /// General-purpose register BX
    pub bx: u8,
    /// Storage array data.
    storage: Vec<u8>,
    /// Memory pointer.
    mp: usize,
}

impl std::fmt::Display for OrganismState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}({}, {})\tax={} bx={}",
            self.dir.to_char(),
            self.ip.x,
            self.ip.y,
            self.ax,
            self.bx
        )
    }
}

impl OrganismState {
    pub fn init(pos: Point) -> Self {
        Self {
            ip: pos,
            dir: Dir::R,
            cursor: pos,
            clipboard: vec![0],
            r: 0,
            flag: false,
            ax: 0,
            bx: 0,
            storage: Vec::new(),
            mp: 0,
        }
    }
    // Return the following information:
    // - mp < 4
    // - mp % 4
    // - the three rows of storage surrounding mp
    pub fn local_memory(&self) -> (bool, u8, [u8; 12]) {
        let first_row = self.mp < 4;
        let column = self.mp % 4;
        let byte_start = if first_row { 0 } else { self.mp - column - 4 };
        let mut bytes = [0; 12];
        for i in 0..12 {
            if let Some(&byte) = self.storage.get(byte_start + i) {
                bytes[i] = byte;
            }
        }
        (first_row, column as u8, bytes)
    }
    pub fn advance<R>(&mut self, grid: &Grid<R>) {
        self.ip = self.ip.move_in(self.dir, grid.width(), grid.height());
    }
    /// Attempt to set the selection radius. Do nothing if the proposed value is out of bounds.
    fn set_r(&mut self, new: u8) {
        if (0..=10).contains(&new) {
            self.r = new;
        }
    }
    fn get_stored(&mut self) -> u8 {
        self.storage.get(self.mp).copied().unwrap_or(0)
    }
    fn get_stored_mut(&mut self) -> &mut u8 {
        while self.storage.len() <= self.mp {
            self.storage.push(0);
        }
        &mut self.storage[self.mp]
    }
    fn set_dir(&mut self, dir: Dir) {
        self.dir = dir;
    }
    fn try_set_cursor<R: Rng>(&mut self, new_pos: Point, grid: &Grid<R>) -> bool {
        let do_set = grid[new_pos] != Instruction::Wall as u8;
        if do_set {
            self.cursor = new_pos;
        }
        do_set
    }
    fn paste<R: Rng>(&mut self, grid: &mut Grid<R>) -> u8 {
        let r = selection_radius(&self.clipboard);
        let width = r * 2 + 1;
        let low_corner = self.cursor
            .left_n(r as usize, grid.width())
            .up_n(r as usize, grid.height());
        // Fill in the region using a flood fill to select relevant points.
        let mut frontier = vec![self.cursor];
        let mut modified = Vec::new();
        while let Some(p) = frontier.pop() {
            if modified.contains(&p) {
                continue;
            }
            if p.dist_to(self.cursor, grid.width(), grid.height()) > r as usize {
                continue;
            }
            modified.push(p);
            if grid[p] == Instruction::Wall as u8 {
                if !grid.pierce_wall() {
                    continue;
                }
            }
            let relative_pos = p.sub(low_corner, grid.width(), grid.height());
            let idx = relative_pos.x * (width as usize) + relative_pos.y;
            grid.set(p, self.clipboard[idx]);
            frontier.push(p.up(grid.height()));
            frontier.push(p.down(grid.height()));
            frontier.push(p.left(grid.width()));
            frontier.push(p.right(grid.width()));
        }
        width
    }
    /// Execute the instruction. Return the number of additional cycles to delay
    /// (usually 0). Return `None` if the organism should die.
    pub fn run<R: Rng>(&mut self, grid: &mut Grid<R>, instruction: Instruction) -> Response {
        use Instruction::*;
        macro_rules! return_repeat_move {
            ($register:ident, $dir:ident) => {{
                let width = grid.width();
                let height = grid.height();
                let mut i = 0;
                while i < self.$register {
                    i += 1;
                    if !self.try_set_cursor(self.cursor.move_in(Dir::$dir, width, height), grid) {
                        break;
                    }
                }
                return Response::Delay(i);
            }}
        }
        match instruction {
            Halt | Wall => return Response::Die,
            Nop => {}
            FlagFork => {
                let mut new = self.clone();
                new.flag = true;
                self.flag = false;
                return Response::Fork(new);
            },
            CursorFork => {
                let mut new = self.clone();
                new.ip = new.cursor;
                return Response::Fork(new);
            },

            ZeroA => self.ax = 0,
            ZeroB => self.bx = 0,
            CopyA => self.bx = self.ax,
            CopyB => self.ax = self.bx,
            SwapAB => swap(&mut self.ax, &mut self.bx),
            SumA => self.ax = self.ax.wrapping_add(self.bx),
            SumB => self.bx = self.ax.wrapping_add(self.bx),
            NegateA => self.ax = self.ax.wrapping_neg(),
            NegateB => self.bx = self.bx.wrapping_neg(),
            IncA => self.ax = self.ax.wrapping_add(1),
            IncB => self.bx = self.bx.wrapping_add(1),
            DecA => self.ax = self.ax.wrapping_sub(1),
            DecB => self.bx = self.bx.wrapping_sub(1),
            MulA => self.ax = self.ax.wrapping_mul(self.bx),
            MulB => self.bx = self.ax.wrapping_mul(self.bx),
            DoubleA => self.ax = self.ax.wrapping_mul(2),
            DoubleB => self.bx = self.bx.wrapping_mul(2),
            HalveA => self.ax /= 2,
            HalveB => self.bx /= 2,
            Mod2A => self.ax %= 2,
            Mod2B => self.bx %= 2,
            BitAndA => self.ax &= self.bx,
            BitAndB => self.bx &= self.ax,
            BitOrA => self.ax |= self.bx,
            BitOrB => self.bx |= self.ax,
            BitXorA => self.ax ^= self.bx,
            BitXorB => self.bx ^= self.ax,
            EqA => self.ax = (self.ax == self.bx) as u8,
            EqB => self.bx = (self.ax == self.bx) as u8,
            NeqA => self.ax = (self.ax != self.bx) as u8,
            NeqB => self.bx = (self.ax != self.bx) as u8,
            NonzeroA => self.ax = (self.ax != 0) as u8,
            NonzeroB => self.bx = (self.bx != 0) as u8,
            IsZeroA => self.ax = (self.ax == 0) as u8,
            IsZeroB => self.bx = (self.bx == 0) as u8,

            WaitA => return Response::Delay(self.ax),
            WaitB => return Response::Delay(self.bx),
            MoveL => self.dir = Dir::L,
            MoveR => self.dir = Dir::R,
            MoveU => self.dir = Dir::U,
            MoveD => self.dir = Dir::D,
            CondMoveL => if self.flag { self.dir = Dir::L }
            CondMoveR => if self.flag { self.dir = Dir::R }
            CondMoveU => if self.flag { self.dir = Dir::U }
            CondMoveD => if self.flag { self.dir = Dir::D }
            CondHalt => if self.flag { return Response::Die }
            ReflectAll => self.set_dir(self.dir.reverse()),
            ReflectX => self.set_dir(self.dir.reflect_x()),
            ReflectY => self.set_dir(self.dir.reflect_y()),
            ReflectFwd => self.set_dir(self.dir.reflect_fwd()),
            ReflectBwd => self.set_dir(self.dir.reflect_bwd()),
            SetFlag => self.flag = true,
            ClearFlag => self.flag = false,
            FlagZeroA => self.flag = self.ax == 0,
            FlagNonzeroA => self.flag = self.ax != 0,
            FlagZeroB => self.flag = self.bx == 0,
            FlagNonzeroB => self.flag = self.bx != 0,
            FlagEq => self.flag = self.ax == self.bx,
            FlagNeq => self.flag = self.ax != self.bx,
            FlagNot => self.flag = !self.flag,
            FlagToA => self.ax = self.flag as u8,
            FlagToB => self.bx = self.flag as u8,

            CursorL => { self.try_set_cursor(self.cursor.left(grid.width()), grid); }
            CursorR => { self.try_set_cursor(self.cursor.right(grid.width()), grid); }
            CursorU => { self.try_set_cursor(self.cursor.up(grid.height()), grid); }
            CursorD => { self.try_set_cursor(self.cursor.down(grid.height()), grid); }
            CursorLTimesA => return_repeat_move!(ax, L),
            CursorRTimesA => return_repeat_move!(ax, R),
            CursorUTimesA => return_repeat_move!(ax, U),
            CursorDTimesA => return_repeat_move!(ax, D),
            CursorLTimesB => return_repeat_move!(bx, L),
            CursorRTimesB => return_repeat_move!(bx, R),
            CursorUTimesB => return_repeat_move!(bx, U),
            CursorDTimesB => return_repeat_move!(bx, D),
            CursorHome => { self.try_set_cursor(self.ip, grid); }

            RadiusA => self.set_r(self.ax),
            RadiusB => self.set_r(self.bx),
            RadiusReset => self.r = 0,
            RadiusToA => self.ax = self.r,
            RadiusToB => self.bx = self.r,
            IncRadius => self.set_r(self.r + 1),
            DecRadius => self.set_r(self.r.saturating_sub(1)),
            CursorA => grid.set(self.cursor, self.ax),
            CursorB => grid.set(self.cursor, self.bx),
            CursorToA => self.ax = grid[self.cursor],
            CursorToB => self.bx = grid[self.cursor],
            Copy => self.clipboard = get_points_for_selection(self.cursor, self.r, grid)
                .map(|p| grid[p]).collect(),
            Paste => return Response::Delay(self.paste(grid)),
            
            Pointer0 => self.mp = 0,
            PointerA => self.mp = self.ax as usize,
            PointerB => self.mp = self.bx as usize,
            PointerToA => self.ax = self.mp as u8,
            PointerToB => self.bx = self.mp as u8,
            PointerL => self.mp = self.mp.saturating_sub(1),
            PointerR => self.mp += 1,
            PointerLTimesA => self.mp = self.mp.saturating_sub(self.ax as usize),
            PointerRTimesA => self.mp += self.ax as usize,
            PointerLTimesB => self.mp = self.mp.saturating_sub(self.bx as usize),
            PointerRTimesB => self.mp += self.bx as usize,
            Pointee0 => *self.get_stored_mut() = 0,
            PointeeA => *self.get_stored_mut() = self.ax,
            PointeeB => *self.get_stored_mut() = self.bx,
            PointeeToA => self.ax = self.get_stored(),
            PointeeToB => self.bx = self.get_stored(),
            IncPointee => {
                let stored = self.get_stored_mut();
                *stored = stored.wrapping_add(1);
            }
            DecPointee => {
                let stored = self.get_stored_mut();
                *stored = stored.wrapping_sub(1);
            }
            IncPointeeA => {
                let ax = self.ax;
                let stored = self.get_stored_mut();
                *stored = stored.wrapping_add(ax);
            }
            DecPointeeA => {
                let ax = self.ax;
                let stored = self.get_stored_mut();
                *stored = stored.wrapping_sub(ax);
            }
            IncPointeeB => {
                let bx = self.bx;
                let stored = self.get_stored_mut();
                *stored = stored.wrapping_add(bx);
            }
            DecPointeeB => {
                let bx = self.bx;
                let stored = self.get_stored_mut();
                *stored = stored.wrapping_sub(bx);
            }
        }
        Response::Delay(0)
    }
}