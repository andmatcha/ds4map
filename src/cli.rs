use crate::ds4_hid;
use std::env;
use std::process::ExitCode;

pub fn run() -> ExitCode {
    let mut args = env::args();
    let bin_name = args.next().unwrap_or_else(|| String::from("ds4"));

    match args.next().as_deref() {
        Some("list") => list_devices(),
        Some("run") => {
            println!("run command is not implemented yet");
            ExitCode::SUCCESS
        }
        Some("monitor") => monitor_reports(),
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

fn list_devices() -> ExitCode {
    match ds4_hid::list_devices() {
        Ok(devices) if devices.is_empty() => {
            println!("No DUALSHOCK 4 devices found");
            ExitCode::SUCCESS
        }
        Ok(devices) => {
            for (index, device) in devices.iter().enumerate() {
                println!(
                    "[{index}] transport={} vid=0x{:04x} pid=0x{:04x} interface={} product={} path={}",
                    device.transport,
                    device.vendor_id,
                    device.product_id,
                    device.interface_number,
                    device.product_name.as_deref().unwrap_or("unknown"),
                    device.path
                );
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to list DUALSHOCK 4 devices: {error}");
            ExitCode::from(1)
        }
    }
}

fn monitor_reports() -> ExitCode {
    match ds4_hid::monitor_input_reports(|event| {
        println!(
            "[#{}] transport={} bytes={} report={}",
            event.sequence,
            event.device.transport,
            event.report.len(),
            ds4_hid::format_report_hex(&event.report)
        );
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("failed to monitor DUALSHOCK 4 HID reports: {error}");
            ExitCode::from(1)
        }
    }
}

fn print_usage(bin_name: &str) {
    eprintln!("Usage: {bin_name} <COMMAND>");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  list    List connected DUALSHOCK 4 devices");
    eprintln!("  run     Placeholder for future execution behavior");
    eprintln!("  monitor Continuously monitor HID input reports from a DUALSHOCK 4");
    eprintln!("  stop    Stop the running action");
}
