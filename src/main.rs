use termion::cursor;
use termion::raw::IntoRawMode as _;

use structopt::StructOpt;

use std::io;

mod app;

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
    #[structopt(long="seed", name="RNG seed")]
    rng_seed: Option<u64>,
    #[structopt(name="initialization file")]
    initial_file: Option<String>,
}

fn main() {
    let options = Options::from_args();

    let stdout = io::stdout();
    let stdout = stdout.into_raw_mode().unwrap();
    let stdout = termion::screen::AlternateScreen::from(stdout);

    let stdout = cursor::HideCursor::from(stdout);
    let stdin = termion::async_stdin();
    
    match app::AppState::init(options, stdin, stdout) {
        Ok(mut app) => app.run(),
        Err(e) => eprintln!("{}", e.description()),
    }
}