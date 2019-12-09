//! Functionality relating to commands overall.
//! 

use std::borrow::Cow;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::path::PathBuf;

use super::AppState;
use super::grid::Dir;
use super::instruction::Instruction;

/// Generic error enum for invalid arguments.
pub enum Error {
    NoDirection,
    BadDirection,
    NoInstruction,
    BadInstruction,
    NoNumber,
    BadNumber,
    ZeroSpeed,
    NoPath,
    Extra(String),
}

impl Error {
    pub fn description(&self) -> Cow<'static, str> {
        match self {
            Error::NoDirection  => "Expected one of < > ^ v.".into(),
            Error::BadDirection => "Expected one of < > ^ v.".into(),
            Error::NoInstruction  => "Expected instruction.".into(),
            Error::BadInstruction => "Expected instruction.".into(),
            Error::NoNumber  => "Expected number.".into(),
            Error::BadNumber => "Invalid number.".into(),
            Error::ZeroSpeed => "Speed cannot be set to 0.".into(),
            Error::NoPath => "Expected filepath.".into(),
            Error::Extra(s) => format!("Unexpected argument '{}'.", s).into(),
        }
    }
}

/// Tracks the command's arguments and its position within them.
pub struct Args<'a> {
    args: Vec<&'a str>,
    pos: usize,
}

impl<'a> Args<'a> {
    /// Create the argument list from the original command by splitting on whitespace.
    pub fn from_command(c: &'a str) -> Self {
        Self {
            args: c.split_whitespace().collect(),
            pos: 0,
        }
    }
    /// Return the next argument as a string.
    pub fn next_raw(&mut self) -> Option<&str> {
        let result = self.args.get(self.pos);
        if result.is_some() {
            self.pos += 1;
        }
        result.copied()
    }
    /// Return the next argument in some parsed form.
    fn next<T: ParseArgs>(&mut self) -> Result<T, Error> {
        T::from_args(self)
    }
    /// Determine whether there are any arguments remaining.
    fn is_end(&self) -> bool {
        self.args.get(self.pos).is_none()
    }
    /// Return an error if there are arguments remaining.
    fn ensure_final(&self) -> Result<(), Error> {
        match self.args.get(self.pos) {
            Some(s) => Err(Error::Extra(s.to_string())),
            None => Ok(())
        }
    }
}

/// Represents types that can be parsed from (possibly multiple) arguments.
pub trait ParseArgs: Sized {
    fn from_args(args: &mut Args) -> Result<Self, Error>;
}

impl ParseArgs for Dir {
    fn from_args(args: &mut Args) -> Result<Self, Error> {
        Dir::from_str(args.next_raw().ok_or(Error::NoDirection)?)
            .ok_or(Error::BadDirection)
    }
}

impl ParseArgs for Instruction {
    fn from_args(args: &mut Args) -> Result<Self, Error> {
        Instruction::from_symbol(args.next_raw().ok_or(Error::NoInstruction)?)
            .ok_or(Error::BadInstruction)
    }
}

impl ParseArgs for PathBuf {
    fn from_args(args: &mut Args) -> Result<Self, Error> {
        Ok(Self::from(args.next_raw().ok_or(Error::NoPath)?))
    }
}

macro_rules! impl_ParseArgs_for_number {
    ($t:ty) => {
        impl ParseArgs for $t {
            fn from_args(args: &mut Args) -> Result<Self, Error> {
                args.next_raw().ok_or(Error::NoNumber)?
                    .parse().map_err(|_| Error::BadNumber)
            }
        }
    }
}

impl_ParseArgs_for_number!(usize);
impl_ParseArgs_for_number!(u32);
impl_ParseArgs_for_number!(u16);
impl_ParseArgs_for_number!(u8);

/// A value can optionally be parsed by returning `None` if there are no
/// arguments remaining.
impl<T: ParseArgs> ParseArgs for Option<T> {
    fn from_args(args: &mut Args) -> Result<Self, Error> {
        if args.is_end() {
            Ok(None)
        } else {
            args.next().map(Some)
        }
    }
}

/// Unit is parsed by doing nothing.
impl ParseArgs for () {
    fn from_args(_: &mut Args) -> Result<Self, Error> {
        Ok(())
    }
}

/// A pair of values can be parsed from arguments by parsing the first
/// and then the second.
impl<T: ParseArgs, U: ParseArgs> ParseArgs for (T, U) {
    fn from_args(args: &mut Args) -> Result<Self, Error> {
        Ok((args.next()?, args.next()?))
    }
}

/// A list of values can be parsed by repeatedly parsing until there are no
/// arguments remaining.
impl<T: ParseArgs> ParseArgs for Vec<T> {
    fn from_args(args: &mut Args) -> Result<Self, Error> {
        let mut result = Vec::new();
        while !args.is_end() {
            result.push(args.next()?);
        }
        Ok(result)
    }
}

/// A trait representing command handlers that take an argument.
pub(super) trait CommandHandler<R: Read, W: Write> {
    fn run(&self, app: &mut AppState<R, W>, args: Args) -> Result<(), Error>;
}

/// A struct that implements `CommandHandler` by forwarding to another function.
pub(super) struct ClosureHandler<A, F> {
    f: F,
    _marker: PhantomData<fn(A)>
}

impl<A, F> ClosureHandler<A, F> {
    pub fn new(f: F) -> Self {
        Self { f, _marker: PhantomData }
    }
}

impl<A, R, W, F> CommandHandler<R, W> for ClosureHandler<A, F>
    where A: ParseArgs, R: Read, W: Write, F: Fn(&mut AppState<R, W>, A) -> Result<(), Error>
{
    fn run(&self, app: &mut AppState<R, W>, mut args: Args) -> Result<(), Error> {
        let arg = args.next()?;
        args.ensure_final()?;
        (self.f)(app, arg)
    }
}