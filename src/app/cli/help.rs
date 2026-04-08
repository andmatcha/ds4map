use std::process::ExitCode;

pub(crate) fn print_usage(bin_name: &str) {
    eprintln!("Usage: {bin_name} <COMMAND>");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  devices    List connected DUALSHOCK 4 devices");
    eprintln!("  ports      List candidate serial output ports");
    eprintln!("  run        Show the fixed real-time DS4 monitor");
    eprintln!("  status     Show the current running action");
    eprintln!("  stop       Stop the running action");
    eprintln!();
    print_run_usage();
}

pub(crate) fn print_help(bin_name: &str) {
    println!("Usage: {bin_name} <COMMAND>");
    println!();
    println!("Commands:");
    println!("  devices    List connected DUALSHOCK 4 devices");
    println!("  ports      List candidate serial output ports");
    println!("  run        Show the fixed real-time DS4 monitor");
    println!("  status     Show the current running action");
    println!("  stop       Stop the running action");
    println!("  help       Show help for a command");
    println!();
    println!("Run the monitor with:");
    println!("  {bin_name} run");
    println!("  {bin_name} run --monitor graphic");
    println!("  {bin_name} run --monitor raw");
    println!("  {bin_name} run --monitor compact");
    println!("  {bin_name} run --monitor none --format arm9 --port <PORT> --baud <BAUD_RATE>");
    println!("  {bin_name} run --format arm9 --port <PORT> --baud <BAUD_RATE>");
    println!();
    println!("Use `{bin_name} help run` for run-specific options.");
}

pub(crate) fn print_help_topic(bin_name: &str, topic: Option<&str>) -> ExitCode {
    match topic {
        None => {
            print_help(bin_name);
            ExitCode::SUCCESS
        }
        Some("run") => {
            print_run_help(bin_name);
            ExitCode::SUCCESS
        }
        Some("devices") => {
            println!("Usage: {bin_name} devices");
            println!();
            println!(
                "Lists connected DUALSHOCK 4 devices and their transport, VID/PID, interface, product name, and path."
            );
            ExitCode::SUCCESS
        }
        Some("ports") => {
            println!("Usage: {bin_name} ports");
            println!();
            println!("Lists candidate serial ports that can be used as output destinations.");
            ExitCode::SUCCESS
        }
        Some("stop") => {
            println!("Usage: {bin_name} stop");
            println!();
            println!("Requests the currently running `ds4 run` action to stop.");
            ExitCode::SUCCESS
        }
        Some("status") => {
            println!("Usage: {bin_name} status");
            println!();
            println!(
                "Shows the currently running `ds4 run` action, including PID and output settings."
            );
            ExitCode::SUCCESS
        }
        Some("arm9") => {
            println!(
                "`arm9` was replaced by `{bin_name} run --format arm9 --port <PORT> --baud <BAUD_RATE>`"
            );
            ExitCode::SUCCESS
        }
        Some("monitor") | Some("live-monitor") => {
            println!("This command was removed. Use `{bin_name} run` instead.");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown help topic: {other}");
            print_help(bin_name);
            ExitCode::from(2)
        }
    }
}

pub(crate) fn print_run_help(bin_name: &str) {
    println!("Usage: {bin_name} run [OPTIONS]");
    println!();
    println!("Shows the fixed real-time DS4 monitor.");
    println!("`--monitor none` runs in the background.");
    println!();
    println!("Options:");
    println!("  -m, --monitor <graphic|raw|compact|none>  Select the monitor display mode");
    println!("  -f, --format <arm9>                       Select the output format");
    println!(
        "  -p, --port <PORT>                         Enable serial output and choose the port"
    );
    println!("  -b, --baud <BAUD_RATE>                    Serial baud rate for output");
    println!("  -h, --help                                Show this help");
    println!();
    println!("Examples:");
    println!("  {bin_name} run");
    println!("  {bin_name} run --monitor raw");
    println!("  {bin_name} run --monitor compact");
    println!("  {bin_name} run --monitor none --format arm9 --port <PORT> --baud <BAUD_RATE>");
    println!("  {bin_name} run --monitor graphic --format arm9 --port <PORT> --baud <BAUD_RATE>");
}

pub(crate) fn is_help_flag(arg: &str) -> bool {
    matches!(arg, "-h" | "--help")
}

fn print_run_usage() {
    eprintln!("Run usage:");
    eprintln!("  ds4 run");
    eprintln!("  ds4 run --monitor graphic");
    eprintln!("  ds4 run --monitor raw");
    eprintln!("  ds4 run --monitor compact");
    eprintln!("  ds4 run --monitor none --format arm9 --port <PORT> --baud <BAUD_RATE>");
    eprintln!("  ds4 run --format arm9 --port <PORT> --baud <BAUD_RATE>");
}
