mod arm9;
mod cli;
mod compact;
mod ds4_hid;
mod live_monitor;
mod mode_sound;
mod serial_out;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
