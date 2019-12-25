use termion::cursor;
use termion::raw::IntoRawMode as _;
use termion::input::TermRead;

use structopt::StructOpt;

use std::io;

/// Actual app state and rendering logic.
mod app;
/// Grid and geometric utilities (directions, points, etc.).
mod grid;

#[derive(Debug, StructOpt)]
#[structopt(
    name="myco",
    no_version,
    author="Reconcyl",
    about="An artificial life experiment."
)]
struct Options {
    #[structopt(long="width", name="grid width", default_value="500")]
    grid_width: usize,
    #[structopt(long="height", name="grid height", default_value="500")]
    grid_height: usize,
    #[structopt(long="view-width", name="view width", default_value="35")]
    view_width: u16,
    #[structopt(long="view-height", name="view height", default_value="35")]
    view_height: u16,
    #[structopt(long="write-error-chance", name="initial write error chance", default_value="1")]
    write_error_chance: u32,
    #[structopt(long="seed", name="RNG seed")]
    rng_seed: Option<u64>,
    #[structopt(long="profile")]
    ignore_io: bool,
    #[structopt(name="initialization file")]
    initial_file: Option<String>,
}

fn main() {
    let options = Options::from_args();
    let ignore_io = options.ignore_io;

    let stdout = io::stdout();
    let stdout = if ignore_io {
        None
    } else {
        let stdout = stdout.into_raw_mode().unwrap();
        let stdout = termion::screen::AlternateScreen::from(stdout);
        let stdout = cursor::HideCursor::from(stdout);
        Some(stdout)
    };
    
    match app::AppState::init(options, stdout) {
        Ok(mut app) => if !ignore_io {
            app.run(termion::async_stdin().keys())
        },
        Err(e) => eprintln!("{}", e.description()),
    }
}