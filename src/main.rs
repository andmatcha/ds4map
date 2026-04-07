mod cli;
mod ds4_hid;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
