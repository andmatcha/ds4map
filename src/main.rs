mod app;
mod input;
mod output;
mod ui;

use std::process::ExitCode;

fn main() -> ExitCode {
    app::cli::run()
}
