use crate::arm9::ManualPacketEncoder;
use crate::compact;
use crate::ds4_hid;
use crate::mode_sound::ModeSoundPlayer;
use crate::serial_out::{SerialConfig, SerialOutput};
use std::env;
use std::process::ExitCode;

struct Arm9CommandConfig {
    monitor_only: bool,
    serial: Option<SerialConfig>,
}

pub fn run() -> ExitCode {
    let mut args = env::args();
    let bin_name = args.next().unwrap_or_else(|| String::from("ds4"));

    match args.next().as_deref() {
        Some("list") => list_devices(),
        Some("run") => run_compact_output(args.collect()),
        Some("arm9") => run_manual_output(args.collect()),
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

fn run_compact_output(args: Vec<String>) -> ExitCode {
    let config = match parse_serial_args(args) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            eprintln!("Usage: ds4 run --port <PORT> --baud <BAUD_RATE>");
            return ExitCode::from(2);
        }
    };

    let mut serial = match SerialOutput::open(&config) {
        Ok(serial) => serial,
        Err(error) => {
            eprintln!(
                "failed to open serial port {} at {} baud: {}",
                config.port, config.baud_rate, error
            );
            return ExitCode::from(1);
        }
    };

    match ds4_hid::monitor_input_reports(|event| {
        match compact::convert_input_report(&event.report) {
            Ok(compact_report) => {
                if let Err(error) = serial.write_report(&compact_report) {
                    eprintln!(
                        "[#{}] failed to write compact report to serial: {}",
                        event.sequence, error
                    );
                }
            }
            Err(error) => {
                eprintln!(
                    "[#{}] skipped unsupported report (transport={}, bytes={}): {}",
                    event.sequence,
                    event.device.transport,
                    event.report.len(),
                    error
                );
            }
        }
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("failed to run compact output: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_manual_output(args: Vec<String>) -> ExitCode {
    let config = match parse_arm9_args(args) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            eprintln!("Usage: ds4 arm9 [--monitor] [--port <PORT> --baud <BAUD_RATE>]");
            return ExitCode::from(2);
        }
    };

    let mut encoder = ManualPacketEncoder::new();
    let mut sound_player = ModeSoundPlayer::new();

    if config.monitor_only {
        return match ds4_hid::monitor_input_reports(|event| {
            match compact::convert_input_report(&event.report) {
                Ok(compact_report) => {
                    let update = encoder.encode_compact_report_update(&compact_report);
                    if update.profile_changed {
                        sound_player.play(update.profile.as_str());
                    }
                    println!(
                        "[#{}] transport={} profile={} compact={} arm9={}",
                        event.sequence,
                        event.device.transport,
                        update.profile.as_str(),
                        ds4_hid::format_report_hex(&compact_report),
                        ds4_hid::format_report_hex(&update.packet)
                    );
                }
                Err(error) => {
                    eprintln!(
                        "[#{}] skipped unsupported report (transport={}, bytes={}): {}",
                        event.sequence,
                        event.device.transport,
                        event.report.len(),
                        error
                    );
                }
            }
        }) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("failed to monitor ARM9 output: {error}");
                ExitCode::from(1)
            }
        };
    }

    let serial_config = config
        .serial
        .expect("serial config must exist when not in monitor mode");
    let mut serial = match SerialOutput::open(&serial_config) {
        Ok(serial) => serial,
        Err(error) => {
            eprintln!(
                "failed to open serial port {} at {} baud: {}",
                serial_config.port, serial_config.baud_rate, error
            );
            return ExitCode::from(1);
        }
    };

    match ds4_hid::monitor_input_reports(|event| {
        match compact::convert_input_report(&event.report) {
            Ok(compact_report) => {
                let update = encoder.encode_compact_report_update(&compact_report);
                if update.profile_changed {
                    sound_player.play(update.profile.as_str());
                }
                if let Err(error) = serial.write_bytes(&update.packet) {
                    eprintln!(
                        "[#{}] failed to write MANUAL packet to serial: {}",
                        event.sequence, error
                    );
                }
            }
            Err(error) => {
                eprintln!(
                    "[#{}] skipped unsupported report (transport={}, bytes={}): {}",
                    event.sequence,
                    event.device.transport,
                    event.report.len(),
                    error
                );
            }
        }
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("failed to run MANUAL output: {error}");
            ExitCode::from(1)
        }
    }
}

fn parse_arm9_args(args: Vec<String>) -> Result<Arm9CommandConfig, String> {
    let mut monitor_only = false;
    let mut port = None;
    let mut baud_rate = None;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--monitor" => monitor_only = true,
            "--port" => {
                let value = iter
                    .next()
                    .ok_or_else(|| String::from("missing value for --port"))?;
                port = Some(value);
            }
            "--baud" => {
                let value = iter
                    .next()
                    .ok_or_else(|| String::from("missing value for --baud"))?;
                let parsed = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid baud rate: {value}"))?;
                baud_rate = Some(parsed);
            }
            other => return Err(format!("unknown arm9 option: {other}")),
        }
    }

    let serial = match (port, baud_rate) {
        (Some(port), Some(baud_rate)) => Some(SerialConfig { port, baud_rate }),
        (None, None) if monitor_only => None,
        (None, None) => {
            return Err(String::from(
                "missing required options: --port and --baud (or use --monitor)",
            ));
        }
        (None, Some(_)) => return Err(String::from("missing required option: --port")),
        (Some(_), None) => return Err(String::from("missing required option: --baud")),
    };

    Ok(Arm9CommandConfig {
        monitor_only,
        serial,
    })
}

fn parse_serial_args(args: Vec<String>) -> Result<SerialConfig, String> {
    let mut port = None;
    let mut baud_rate = None;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--port" => {
                let value = iter
                    .next()
                    .ok_or_else(|| String::from("missing value for --port"))?;
                port = Some(value);
            }
            "--baud" => {
                let value = iter
                    .next()
                    .ok_or_else(|| String::from("missing value for --baud"))?;
                let parsed = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid baud rate: {value}"))?;
                baud_rate = Some(parsed);
            }
            other => return Err(format!("unknown run option: {other}")),
        }
    }

    Ok(SerialConfig {
        port: port.ok_or_else(|| String::from("missing required option: --port"))?,
        baud_rate: baud_rate.ok_or_else(|| String::from("missing required option: --baud"))?,
    })
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
    eprintln!("  list       List connected DUALSHOCK 4 devices");
    eprintln!("  run        Stream DS4_COMPACT_V1 reports to a serial port");
    eprintln!("  arm9       Stream ARM9 MANUAL AC packets, or monitor them with --monitor");
    eprintln!("  monitor    Continuously monitor HID input reports from a DUALSHOCK 4");
    eprintln!("  stop       Stop the running action");
}
