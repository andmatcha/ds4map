use crate::compact::{self, CompactReport};
use crate::ds4_hid;
use crate::live_monitor::{DisplayMode, MonitorFrame, MonitorUi};
use crate::output_format::{OutputDriver, OutputFormat};
use crate::serial_out::{SerialConfig, SerialOutput};
use serialport::SerialPortType;
use std::env;
use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitCode;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};

static LIVE_MONITOR_STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

#[cfg(unix)]
const SIGINT: i32 = 2;
const BACKGROUND_RUN_ENV: &str = "DS4MAP_BACKGROUND_RUN";
const RUNTIME_STATE_DIR: &str = "ds4map";
const RUN_PID_FILE: &str = "run.pid";
const RUN_INFO_FILE: &str = "run.info";
const STOP_REQUEST_FILE: &str = "run.stop";

#[derive(Debug, Clone)]
struct RunCommandConfig {
    display_mode: DisplayMode,
    output_format: OutputFormat,
    serial: Option<SerialConfig>,
}

struct RunOutput {
    driver: Box<dyn OutputDriver>,
    serial: SerialOutput,
}

impl RunOutput {
    fn open(config: &RunCommandConfig) -> Result<Option<Self>, String> {
        if config.serial.is_none() {
            return Ok(None);
        }

        let serial_config = config
            .serial
            .as_ref()
            .expect("serial config must exist when output is enabled");
        let serial = SerialOutput::open(serial_config).map_err(|error| {
            format!(
                "failed to open serial port {} at {} baud: {}",
                serial_config.port, serial_config.baud_rate, error
            )
        })?;

        Ok(Some(Self {
            driver: config.output_format.create_driver(),
            serial,
        }))
    }

    fn write_compact_report(&mut self, compact_report: &CompactReport) -> Result<(), String> {
        let bytes = self.driver.encode(compact_report)?;
        self.serial.write_bytes(&bytes).map_err(|error| {
            format!(
                "failed to write {} output: {}",
                self.driver.format_name(),
                error
            )
        })
    }
}

pub fn run() -> ExitCode {
    let mut args = env::args();
    let bin_name = args.next().unwrap_or_else(|| String::from("ds4"));

    match args.next().as_deref() {
        Some("--help") | Some("-h") => {
            print_help(&bin_name);
            ExitCode::SUCCESS
        }
        Some("help") => print_help_topic(&bin_name, args.next().as_deref()),
        Some("devices") => list_devices(),
        Some("ports") => list_output_ports(),
        Some("run") => run_live_monitor(args.collect(), &bin_name),
        Some("status") => show_running_action_status(),
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
        Some("stop") => stop_running_action(),
        Some(command) => {
            eprintln!("unknown subcommand: {command}");
            print_usage(&bin_name);
            ExitCode::from(2)
        }
        None => {
            print_help(&bin_name);
            ExitCode::from(2)
        }
    }
}

fn run_live_monitor(args: Vec<String>, bin_name: &str) -> ExitCode {
    let original_args = args.clone();
    if args.iter().any(|arg| is_help_flag(arg)) {
        print_run_help(bin_name);
        return ExitCode::SUCCESS;
    }

    let config = match parse_run_args(args) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            print_run_help(bin_name);
            return ExitCode::from(2);
        }
    };

    if should_background_run(&config) {
        return spawn_background_run(&original_args);
    }

    LIVE_MONITOR_STOP_REQUESTED.store(false, Ordering::SeqCst);
    install_live_monitor_sigint_handler();
    let _run_state_guard = match RunStateGuard::acquire(&config) {
        Ok(guard) => guard,
        Err(error) => {
            eprintln!("failed to prepare runtime state: {error}");
            return ExitCode::from(1);
        }
    };

    let mut output = match RunOutput::open(&config) {
        Ok(output) => output,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    if config.display_mode == DisplayMode::None {
        return run_output_only(output);
    }

    let (monitor_result, render_error) = {
        let mut ui = match MonitorUi::new(config.display_mode) {
            Ok(ui) => ui,
            Err(error) => {
                eprintln!("failed to initialize live monitor UI: {error}");
                return ExitCode::from(1);
            }
        };
        let mut last_frame = MonitorFrame::idle();
        let mut render_error = None;

        if let Err(error) = ui.render(&last_frame, Some("waiting")) {
            eprintln!("failed to render live monitor UI: {error}");
            return ExitCode::from(1);
        }

        let monitor_result = ds4_hid::monitor_input_reports_until(
            |event| {
                if render_error.is_some() {
                    return;
                }

                match compact::convert_input_report(&event.report) {
                    Ok(compact_report) => {
                        last_frame = monitor_frame_from_event(&event, compact_report);

                        let output_status = match output.as_mut() {
                            Some(output) => output
                                .write_compact_report(&compact_report)
                                .err()
                                .map(|_| String::from("output error")),
                            None => None,
                        };

                        if let Err(error) = ui.render(&last_frame, output_status.as_deref()) {
                            render_error = Some(error.to_string());
                        }
                    }
                    Err(error) => {
                        let _ = error;
                        let status = String::from("unsupported report");

                        if let Err(render_issue) = ui.render(&last_frame, Some(&status)) {
                            render_error = Some(render_issue.to_string());
                        }
                    }
                }
            },
            should_stop_running,
        );

        (monitor_result, render_error)
    };

    if let Some(error) = render_error {
        eprintln!("failed to update live monitor UI: {error}");
        return ExitCode::from(1);
    }

    match monitor_result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("failed to run DS4 monitor: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_output_only(mut output: Option<RunOutput>) -> ExitCode {
    let monitor_result = ds4_hid::monitor_input_reports_until(
        |event| match compact::convert_input_report(&event.report) {
            Ok(compact_report) => {
                if let Some(output) = output.as_mut() {
                    if let Err(error) = output.write_compact_report(&compact_report) {
                        eprintln!("output error: {error}");
                    }
                }
            }
            Err(_) => {}
        },
        should_stop_running,
    );

    match monitor_result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("failed to run DS4 output: {error}");
            ExitCode::from(1)
        }
    }
}

fn monitor_frame_from_event(
    event: &ds4_hid::InputReportEvent,
    compact_report: CompactReport,
) -> MonitorFrame {
    MonitorFrame {
        sequence: event.sequence,
        transport: event.device.transport,
        report_len: event.report.len(),
        device_name: event
            .device
            .product_name
            .clone()
            .unwrap_or_else(|| String::from("unknown")),
        vendor_id: event.device.vendor_id,
        product_id: event.device.product_id,
        interface_number: event.device.interface_number,
        raw_report: event.report.clone(),
        compact: compact_report,
    }
}

fn parse_run_args(args: Vec<String>) -> Result<RunCommandConfig, String> {
    let mut display_mode = DisplayMode::Full;
    let mut output_format = OutputFormat::Arm9;
    let mut format_specified = false;
    let mut port = None;
    let mut baud_rate = None;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--monitor" | "-m" => {
                let value = iter
                    .next()
                    .ok_or_else(|| String::from("missing value for --monitor"))?;
                display_mode = parse_display_mode(&value)?;
            }
            "--format" | "-f" => {
                let value = iter
                    .next()
                    .ok_or_else(|| String::from("missing value for --format"))?;
                output_format = OutputFormat::parse(&value)?;
                format_specified = true;
            }
            "--port" | "-p" => {
                let value = iter
                    .next()
                    .ok_or_else(|| String::from("missing value for --port"))?;
                port = Some(value);
            }
            "--baud" | "-b" => {
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

    match (port, baud_rate) {
        (Some(port), Some(baud_rate)) => {
            return Ok(RunCommandConfig {
                display_mode,
                output_format,
                serial: Some(SerialConfig { port, baud_rate }),
            });
        }
        (Some(_), None) => {
            return Err(String::from(
                "missing required option for serial output: --baud",
            ));
        }
        (None, Some(_)) => {
            return Err(String::from(
                "missing required option for serial output: --port",
            ));
        }
        (None, None) => {}
    }

    if format_specified {
        return Err(String::from(
            "--format requires serial output via --port and --baud",
        ));
    }

    if display_mode == DisplayMode::None {
        return Err(String::from(
            "--monitor none requires serial output via --port and --baud",
        ));
    }

    Ok(RunCommandConfig {
        display_mode,
        output_format,
        serial: None,
    })
}

fn parse_display_mode(value: &str) -> Result<DisplayMode, String> {
    match value {
        "graphic" => Ok(DisplayMode::Full),
        "raw" => Ok(DisplayMode::Raw),
        "compact" => Ok(DisplayMode::Compact),
        "none" => Ok(DisplayMode::None),
        other => Err(format!(
            "invalid monitor mode: {other} (expected graphic/raw/compact/none)"
        )),
    }
}

fn stop_running_action() -> ExitCode {
    match read_runtime_status() {
        Some(status) if status.is_running => {}
        Some(_) | None => {
            eprintln!("no running action found");
            cleanup_runtime_state();
            return ExitCode::from(1);
        }
    }

    if let Err(error) = fs::create_dir_all(runtime_state_dir()) {
        eprintln!("failed to prepare runtime state: {error}");
        return ExitCode::from(1);
    }

    if let Err(error) = fs::write(stop_request_path(), b"stop\n") {
        eprintln!("failed to request stop: {error}");
        return ExitCode::from(1);
    }

    println!("stop requested");
    ExitCode::SUCCESS
}

fn show_running_action_status() -> ExitCode {
    match read_runtime_status() {
        Some(status) if status.is_running => {
            println!("running");
            println!("pid: {}", status.pid);
            println!("monitor: {}", status.display_mode.as_str());
            println!("format: {}", status.output_format.as_str());
            match status.port {
                Some(port) => println!("port: {port}"),
                None => println!("port: (none)"),
            }
            match status.baud_rate {
                Some(baud_rate) => println!("baud: {baud_rate}"),
                None => println!("baud: (none)"),
            }
            println!(
                "state: {}",
                if stop_request_path().exists() {
                    "stopping"
                } else {
                    "running"
                }
            );
            ExitCode::SUCCESS
        }
        Some(_) | None => {
            cleanup_runtime_state();
            println!("no running action");
            ExitCode::SUCCESS
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

fn list_output_ports() -> ExitCode {
    match serialport::available_ports() {
        Ok(ports) if ports.is_empty() => {
            println!("No serial output ports found");
            ExitCode::SUCCESS
        }
        Ok(ports) => {
            for (index, port) in ports.iter().enumerate() {
                match &port.port_type {
                    SerialPortType::UsbPort(info) => {
                        println!(
                            "[{index}] port={} type=usb vid=0x{:04x} pid=0x{:04x} manufacturer={} product={} serial={}",
                            port.port_name,
                            info.vid,
                            info.pid,
                            info.manufacturer.as_deref().unwrap_or("unknown"),
                            info.product.as_deref().unwrap_or("unknown"),
                            info.serial_number.as_deref().unwrap_or("unknown"),
                        );
                    }
                    SerialPortType::BluetoothPort => {
                        println!("[{index}] port={} type=bluetooth", port.port_name);
                    }
                    SerialPortType::PciPort => {
                        println!("[{index}] port={} type=pci", port.port_name);
                    }
                    SerialPortType::Unknown => {
                        println!("[{index}] port={} type=unknown", port.port_name);
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to list serial output ports: {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(unix)]
fn install_live_monitor_sigint_handler() {
    type SignalHandler = unsafe extern "C" fn(i32);

    unsafe extern "C" fn handle_sigint(_: i32) {
        LIVE_MONITOR_STOP_REQUESTED.store(true, Ordering::SeqCst);
    }

    unsafe extern "C" {
        fn signal(signum: i32, handler: SignalHandler) -> SignalHandler;
    }

    unsafe {
        signal(SIGINT, handle_sigint);
    }
}

#[cfg(not(unix))]
fn install_live_monitor_sigint_handler() {}

fn print_usage(bin_name: &str) {
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

fn print_help(bin_name: &str) {
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

fn print_help_topic(bin_name: &str, topic: Option<&str>) -> ExitCode {
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

fn print_run_usage() {
    eprintln!("Run usage:");
    eprintln!("  ds4 run");
    eprintln!("  ds4 run --monitor graphic");
    eprintln!("  ds4 run --monitor raw");
    eprintln!("  ds4 run --monitor compact");
    eprintln!("  ds4 run --monitor none --format arm9 --port <PORT> --baud <BAUD_RATE>");
    eprintln!("  ds4 run --format arm9 --port <PORT> --baud <BAUD_RATE>");
}

fn print_run_help(bin_name: &str) {
    println!("Usage: {bin_name} run [OPTIONS]");
    println!();
    println!("Shows the fixed real-time DS4 monitor.");
    println!("`--monitor none` runs in the background.");
    println!();
    println!("Options:");
    println!("  -m, --monitor <graphic|raw|compact|none>  Select the monitor display mode");
    println!("  -f, --format <arm9>                  Select the output format");
    println!("  -p, --port <PORT>                    Enable serial output and choose the port");
    println!("  -b, --baud <BAUD_RATE>               Serial baud rate for output");
    println!("  -h, --help                           Show this help");
    println!();
    println!("Examples:");
    println!("  {bin_name} run");
    println!("  {bin_name} run --monitor raw");
    println!("  {bin_name} run --monitor compact");
    println!("  {bin_name} run --monitor none --format arm9 --port <PORT> --baud <BAUD_RATE>");
    println!("  {bin_name} run --monitor graphic --format arm9 --port <PORT> --baud <BAUD_RATE>");
}

fn is_help_flag(arg: &str) -> bool {
    matches!(arg, "-h" | "--help")
}

fn should_background_run(config: &RunCommandConfig) -> bool {
    config.display_mode == DisplayMode::None && env::var_os(BACKGROUND_RUN_ENV).is_none()
}

fn spawn_background_run(args: &[String]) -> ExitCode {
    let exe = match env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("failed to resolve executable path: {error}");
            return ExitCode::from(1);
        }
    };

    let mut command = Command::new(exe);
    command
        .arg("run")
        .args(args)
        .env(BACKGROUND_RUN_ENV, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    configure_background_command(&mut command);

    match command.spawn() {
        Ok(_) => {
            println!("started in background");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to start background run: {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(unix)]
fn configure_background_command(command: &mut Command) {
    unsafe {
        command.pre_exec(|| detach_background_process());
    }
}

#[cfg(not(unix))]
fn configure_background_command(_: &mut Command) {}

#[cfg(unix)]
fn detach_background_process() -> io::Result<()> {
    unsafe extern "C" {
        fn setsid() -> i32;
    }

    let result = unsafe { setsid() };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

fn should_stop_running() -> bool {
    LIVE_MONITOR_STOP_REQUESTED.load(Ordering::SeqCst) || stop_request_path().exists()
}

fn runtime_state_dir() -> PathBuf {
    env::temp_dir().join(RUNTIME_STATE_DIR)
}

fn run_pid_path() -> PathBuf {
    runtime_state_dir().join(RUN_PID_FILE)
}

fn run_info_path() -> PathBuf {
    runtime_state_dir().join(RUN_INFO_FILE)
}

fn stop_request_path() -> PathBuf {
    runtime_state_dir().join(STOP_REQUEST_FILE)
}

struct RunStateGuard {
    pid_path: PathBuf,
    info_path: PathBuf,
    stop_path: PathBuf,
}

impl RunStateGuard {
    fn acquire(config: &RunCommandConfig) -> Result<Self, std::io::Error> {
        let state_dir = runtime_state_dir();
        fs::create_dir_all(&state_dir)?;
        let pid_path = state_dir.join(RUN_PID_FILE);
        let info_path = state_dir.join(RUN_INFO_FILE);
        let stop_path = state_dir.join(STOP_REQUEST_FILE);
        let _ = fs::remove_file(&stop_path);
        let pid = std::process::id();
        fs::write(&pid_path, format!("{pid}\n"))?;
        fs::write(
            &info_path,
            format!(
                "pid={pid}\nmonitor={}\nformat={}\nport={}\nbaud={}\n",
                config.display_mode.as_str(),
                config.output_format.as_str(),
                config
                    .serial
                    .as_ref()
                    .map(|serial| serial.port.as_str())
                    .unwrap_or(""),
                config
                    .serial
                    .as_ref()
                    .map(|serial| serial.baud_rate.to_string())
                    .unwrap_or_default(),
            ),
        )?;
        Ok(Self {
            pid_path,
            info_path,
            stop_path,
        })
    }
}

impl Drop for RunStateGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.pid_path);
        let _ = fs::remove_file(&self.info_path);
        let _ = fs::remove_file(&self.stop_path);
    }
}

#[derive(Debug, Clone)]
struct RuntimeStatus {
    pid: u32,
    display_mode: DisplayMode,
    output_format: OutputFormat,
    port: Option<String>,
    baud_rate: Option<u32>,
    is_running: bool,
}

fn read_runtime_status() -> Option<RuntimeStatus> {
    let info = fs::read_to_string(run_info_path()).ok()?;
    let mut pid = None;
    let mut display_mode = None;
    let mut output_format = None;
    let mut port = None;
    let mut baud_rate = None;

    for line in info.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "pid" => pid = value.parse::<u32>().ok(),
            "monitor" => display_mode = parse_display_mode(value).ok(),
            "format" => output_format = OutputFormat::parse(value).ok(),
            "port" if !value.is_empty() => port = Some(String::from(value)),
            "baud" if !value.is_empty() => baud_rate = value.parse::<u32>().ok(),
            _ => {}
        }
    }

    let pid = pid?;
    Some(RuntimeStatus {
        pid,
        display_mode: display_mode?,
        output_format: output_format?,
        port,
        baud_rate,
        is_running: is_process_running(pid),
    })
}

fn cleanup_runtime_state() {
    let _ = fs::remove_file(run_pid_path());
    let _ = fs::remove_file(run_info_path());
    let _ = fs::remove_file(stop_request_path());
}

#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    use std::ffi::c_int;

    unsafe extern "C" {
        fn kill(pid: c_int, sig: c_int) -> c_int;
    }

    let result = unsafe { kill(pid as c_int, 0) };
    if result == 0 {
        return true;
    }

    let error = io::Error::last_os_error();
    matches!(error.raw_os_error(), Some(1))
}

#[cfg(not(unix))]
fn is_process_running(_: u32) -> bool {
    run_pid_path().exists()
}

#[cfg(test)]
mod tests {
    use super::{OutputFormat, parse_display_mode, parse_run_args};
    use crate::live_monitor::DisplayMode;

    #[test]
    fn parse_run_args_defaults_to_monitor_only() {
        let config = parse_run_args(vec![]).expect("run config should parse");
        assert_eq!(config.display_mode, DisplayMode::Full);
        assert!(config.serial.is_none());
        assert_eq!(config.output_format, OutputFormat::Arm9);
    }

    #[test]
    fn parse_run_args_supports_arm9_output() {
        let config = parse_run_args(vec![
            String::from("-f"),
            String::from("arm9"),
            String::from("-p"),
            String::from("/dev/ttyUSB0"),
            String::from("-b"),
            String::from("115200"),
        ])
        .expect("run config should parse");

        assert_eq!(config.display_mode, DisplayMode::Full);
        assert_eq!(config.output_format, OutputFormat::Arm9);
        let serial = config.serial.expect("serial config should exist");
        assert_eq!(serial.port, "/dev/ttyUSB0");
        assert_eq!(serial.baud_rate, 115200);
    }

    #[test]
    fn parse_run_args_rejects_format_without_serial_output() {
        let error = parse_run_args(vec![String::from("--format"), String::from("arm9")])
            .expect_err("parse should fail");
        assert!(error.contains("--format requires serial output"));
    }

    #[test]
    fn parse_run_args_rejects_baud_without_port() {
        let error = parse_run_args(vec![String::from("--baud"), String::from("115200")])
            .expect_err("parse should fail");
        assert!(error.contains("--port"));
    }

    #[test]
    fn parse_run_args_supports_raw_display_mode() {
        let config = parse_run_args(vec![String::from("-m"), String::from("raw")])
            .expect("run config should parse");
        assert_eq!(config.display_mode, DisplayMode::Raw);
    }

    #[test]
    fn parse_run_args_supports_compact_display_mode() {
        let config = parse_run_args(vec![String::from("--monitor"), String::from("compact")])
            .expect("run config should parse");
        assert_eq!(config.display_mode, DisplayMode::Compact);
    }

    #[test]
    fn parse_run_args_supports_none_display_mode_with_serial_output() {
        let config = parse_run_args(vec![
            String::from("--monitor"),
            String::from("none"),
            String::from("--format"),
            String::from("arm9"),
            String::from("--port"),
            String::from("/dev/ttyUSB0"),
            String::from("--baud"),
            String::from("115200"),
        ])
        .expect("run config should parse");
        assert_eq!(config.display_mode, DisplayMode::None);
        assert!(config.serial.is_some());
    }

    #[test]
    fn parse_run_args_rejects_none_display_mode_without_serial_output() {
        let error = parse_run_args(vec![String::from("--monitor"), String::from("none")])
            .expect_err("parse should fail");
        assert!(error.contains("--monitor none requires serial output"));
    }

    #[test]
    fn parse_display_mode_supports_graphic() {
        assert_eq!(
            parse_display_mode("graphic").expect("display mode should parse"),
            DisplayMode::Full
        );
    }

    #[test]
    fn help_flags_are_detected() {
        assert!(super::is_help_flag("--help"));
        assert!(super::is_help_flag("-h"));
        assert!(!super::is_help_flag("--monitor"));
    }
}
