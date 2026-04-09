mod commands;
mod help;
mod run_command;
mod run_logger;
mod runtime;

use std::env;
use std::process::ExitCode;

pub fn run() -> ExitCode {
    let mut args = env::args();
    let bin_name = args.next().unwrap_or_else(|| String::from("ds4"));

    match args.next().as_deref() {
        Some("--help") | Some("-h") => {
            help::print_help(&bin_name);
            ExitCode::SUCCESS
        }
        Some("help") => help::print_help_topic(&bin_name, args.next().as_deref()),
        Some("devices") => commands::list_devices(),
        Some("ports") => commands::list_output_ports(),
        Some("run") => run_command::run_live_monitor(args.collect(), &bin_name),
        Some("status") => commands::show_running_action_status(),
        Some("stop") => commands::stop_running_action(),
        Some("arm9") => {
            eprintln!(
                "`arm9` was replaced by `ds4 run --format arm9 --port <PORT> --baud <BAUD_RATE>`"
            );
            ExitCode::from(2)
        }
        Some(command @ "live-monitor") | Some(command @ "monitor") => {
            eprintln!("`{command}` was removed. Use `ds4 run` instead.");
            ExitCode::from(2)
        }
        Some(command) => {
            eprintln!("unknown subcommand: {command}");
            help::print_usage(&bin_name);
            ExitCode::from(2)
        }
        None => {
            help::print_help(&bin_name);
            ExitCode::from(2)
        }
    }
}
