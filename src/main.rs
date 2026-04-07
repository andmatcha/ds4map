use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args();
    let bin_name = args.next().unwrap_or_else(|| String::from("ds4"));

    match args.next().as_deref() {
        Some("list") => {
            println!("list command is not implemented yet");
            ExitCode::SUCCESS
        }
        Some("run") => {
            println!("run command is not implemented yet");
            ExitCode::SUCCESS
        }
        Some("stop") => {
            println!("stop command is not implemented yet");
            ExitCode::SUCCESS
        }
        Some(command) => {
            eprintln!("unknown subcommand: {command}");
            print_usage(&bin_name);
            ExitCode::from(2)
        }
        None => {
            print_usage(&bin_name);
            ExitCode::from(2)
        }
    }
}

fn print_usage(bin_name: &str) {
    eprintln!("Usage: {bin_name} <COMMAND>");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  list    List available items");
    eprintln!("  run     Run the selected action");
    eprintln!("  stop    Stop the running action");
}
