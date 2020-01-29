use rand::Rng;

/// Sort a pair of values.
fn min_max<T: Ord>(a: T, b: T) -> (T, T) {
    if a > b {
        (b, a)
    } else {
        (a, b)
    }
}

/// Return the minimum number of increments or decrements (modulo a given base)
/// required to turn one number into another.
fn dist_modular(a: usize, b: usize, rem: usize) -> usize {
    let (min, max) = min_max(a, b);
    std::cmp::min(max - min, min + rem - max)
}

/// Return the minimum number of decrements (modulo a given base) required to turn
/// one number into another.
fn sub_modular(a: usize, b: usize, rem: usize) -> usize {
    if b > a {
        rem + a - b
    } else {
        a - b
    }
}

pub struct Grid<R> {
    width: usize,
    height: usize,
    // invariant: data.len() == width * height
    data: Vec<u8>,
    rng: R,
    /// The inverse probability of a cosmic ray occuring on a given cycle.
    /// This is set to 0 if the probability is 0.
    pub write_error_chance: u32,
    /// The inverse probability that an attempt to write to a wall will succeed.
    /// This is set to 0 if the probability is 0.
    pub wall_pierce_chance: u32,
}

impl<R> Grid<R> {
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    fn get_ref(&self, p: Point) -> Option<&u8> {
        if p.x < self.width && p.y < self.height {
            Some(&self.data[p.y * self.width + p.x])
        } else {
            None
        }
    }
    pub fn get(&self, p: Point) -> Option<u8> {
        self.get_ref(p).copied()
    }
    pub fn view<'a>(&'a self, start: Point, width: usize, height: usize)
        -> impl Iterator<Item=impl Iterator<Item=(Point, u8)> + 'a> + 'a
    {
        (0..height).map(move |i| {
            let y = (start.y + i) % self.height;
            (0..width).map(move |j| {
                let x = (start.x + j) % self.width;
                let point = Point { x, y };
                (point, self.get(point).unwrap())
            })
        })
    }
    pub fn view_all<'a>(&'a self)
        -> impl Iterator<Item=impl Iterator<Item=(Point, u8)> + 'a> + 'a
    {
        self.view(ORIGIN, self.width, self.height)
    }
}

impl<'a, R: Rng> Grid<R> {
    pub fn init(
        width: usize,
        height: usize,
        mut rng: R,
        fill: u8,
        write_error_chance: u32
    ) -> Self {
        assert_ne!(width * height, 0);
        let mut data = Vec::new();
        for _ in 0..width * height {
            if write_error_chance != 0
              && rng.gen_ratio(1, write_error_chance) {
                data.push(rng.gen());
            } else {
                data.push(fill);
            }
        }
        Self {
            width, height,
            data,
            rng,
            write_error_chance,
            wall_pierce_chance: 0,
        }
    }
    pub fn pierce_wall(&mut self) -> bool {
        self.wall_pierce_chance != 0
            && self.rng.gen_ratio(1, self.wall_pierce_chance)
    }
    pub fn set(&mut self, p: Point, new: u8) {
        if p.x < self.width && p.y < self.height {
            let wrong = self.rng.gen();
            self.data[p.y * self.width + p.x] =
                if self.write_error_chance > 0
                    && self.rng.gen_ratio(1, self.write_error_chance)
                { wrong } else { new };
        } else {
            panic!("{:?} is out of bounds", p);
        }
    }
}

impl<'a, R> std::ops::Index<Point> for Grid<R> {
    type Output = u8;
    fn index(&self, p: Point) -> &u8 {
        self.get_ref(p).unwrap()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Point { pub x: usize, pub y: usize }

pub const ORIGIN: Point = Point { x: 0, y: 0 };

impl Point {
    pub fn from_modular(x: isize, y: isize, width: usize, height: usize) -> Self {
        Self {
            x: x.rem_euclid(width as isize) as usize,
            y: y.rem_euclid(height as isize) as usize,
        }
    }
    pub fn up(self, height: usize) -> Self {
        assert!(self.y < height);
        let y = if self.y == 0 { height - 1 } else { self.y - 1 };
        Self { y, ..self }
    }
    pub fn up_n(self, n: usize, height: usize) -> Self {
        assert!(self.y < height);
        let n = n % height;
        let y = if self.y < n { height + self.y - n } else { self.y - n };
        Self { y, ..self }
    }
    pub fn down(self, height: usize) -> Self {
        assert!(self.y < height);
        let y = if self.y == height - 1 { 0 } else { self.y + 1 };
        Self { y, ..self }
    }
    pub fn down_n(self, n: usize, height: usize) -> Self {
        assert!(self.y < height);
        let y = (self.y + n) % height;
        Self { y, ..self }
    }
    pub fn left(self, width: usize) -> Self {
        assert!(self.x < width);
        let x = if self.x == 0 { width - 1 } else { self.x - 1 };
        Self { x, ..self }
    }
    pub fn left_n(self, n: usize, width: usize) -> Self {
        assert!(self.x < width);
        let n = n % width;
        let x = if self.x < n { width + self.x - n } else { self.x - n };
        Self { x, ..self }
    }
    pub fn right(self, width: usize) -> Self {
        assert!(self.x < width);
        let x = if self.x == width - 1 { 0 } else { self.x + 1 };
        Self { x, ..self }
    }
    pub fn right_n(self, n: usize, width: usize) -> Self {
        assert!(self.x < width);
        let x = (self.x + n) % width;
        Self { x, ..self }
    }
    pub fn move_in(self, dir: Dir, width: usize, height: usize) -> Self {
        match dir {
            Dir::L => self.left(width),
            Dir::R => self.right(width),
            Dir::U => self.up(height),
            Dir::D => self.down(height),
        }
    }
    pub fn move_in_n(self, dir: Dir, n: usize, width: usize, height: usize) -> Self {
        match dir {
            Dir::L => self.left_n(n, width),
            Dir::R => self.right_n(n, width),
            Dir::U => self.up_n(n, height),
            Dir::D => self.down_n(n, height),
        }
    }
    /// Modular taxicab distance.
    pub fn dist_to(self, other: Point, width: usize, height: usize) -> usize {
        std::cmp::max(
            dist_modular(self.x, other.x, width),
            dist_modular(self.y, other.y, height))
    }
    /// Modular componentwise subtraction.
    pub fn sub(self, other: Point, width: usize, height: usize) -> Self {
        Self {
            x: sub_modular(self.x, other.x, width),
            y: sub_modular(self.y, other.y, height),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Dir { L, R, U, D }

impl Dir {
    pub fn to_char(self) -> char {
        match self {
            Dir::L => '<',
            Dir::R => '>',
            Dir::U => '^',
            Dir::D => 'v',
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "<" => Some(Dir::L),
            ">" => Some(Dir::R),
            "^" => Some(Dir::U),
            "v" => Some(Dir::D),
            _ => None
        }
    }
    /// Reflect as in '#'.
    pub fn reverse(self) -> Self {
        match self {
            Dir::L => Dir::R,
            Dir::R => Dir::L,
            Dir::U => Dir::D,
            Dir::D => Dir::U,
        }
    }
    /// Reflect as in '|'.
    pub fn reflect_x(self) -> Self {
        match self {
            Dir::L => Dir::R,
            Dir::R => Dir::L,
            d => d
        }
    }
    /// Reflect as in '-'.
    pub fn reflect_y(self) -> Self {
        match self {
            Dir::U => Dir::D,
            Dir::D => Dir::U,
            d => d
        }
    }
    /// Reflect as in '/'.
    pub fn reflect_fwd(self) -> Self {
        match self {
            Dir::L => Dir::D,
            Dir::R => Dir::U,
            Dir::U => Dir::R,
            Dir::D => Dir::L,
        }
    }
    /// Reflect as in '\'.
    pub fn reflect_bwd(self) -> Self {
        match self {
            Dir::L => Dir::U,
            Dir::R => Dir::D,
            Dir::U => Dir::L,
            Dir::D => Dir::R,
        }
    }
}