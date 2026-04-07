mod cli;
mod compact;
mod ds4_hid;
mod serial_out;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
