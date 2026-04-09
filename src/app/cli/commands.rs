use super::runtime;
use crate::input::ds4_hid;
use serialport::SerialPortType;
use std::process::ExitCode;

pub(crate) fn stop_running_action() -> ExitCode {
    match runtime::read_runtime_status() {
        Some(status) if status.is_running => {}
        Some(_) | None => {
            eprintln!("no running action found");
            runtime::cleanup_runtime_state();
            return ExitCode::from(1);
        }
    }

    match runtime::request_stop() {
        Ok(()) => {
            println!("stop requested");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

pub(crate) fn show_running_action_status() -> ExitCode {
    match runtime::read_runtime_status() {
        Some(status) if status.is_running => {
            println!("running");
            println!("pid: {}", status.pid);
            println!("monitor: {}", status.display_mode.as_str());
            match status.output_format {
                Some(format) => println!("format: {}", format.as_str()),
                None => println!("format: (none)"),
            }
            match status.port {
                Some(port) => println!("port: {port}"),
                None => println!("port: (none)"),
            }
            match status.baud_rate {
                Some(baud_rate) => println!("baud: {baud_rate}"),
                None => println!("baud: (none)"),
            }
            match status.log_file {
                Some(log_file) => println!("log_file: {log_file}"),
                None => println!("log_file: (none)"),
            }
            println!(
                "state: {}",
                if runtime::stop_requested() {
                    "stopping"
                } else {
                    "running"
                }
            );
            ExitCode::SUCCESS
        }
        Some(_) | None => {
            runtime::cleanup_runtime_state();
            println!("no running action");
            ExitCode::SUCCESS
        }
    }
}

pub(crate) fn list_devices() -> ExitCode {
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

pub(crate) fn list_output_ports() -> ExitCode {
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
