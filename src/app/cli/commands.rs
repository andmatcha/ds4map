use super::runtime;
use crate::input::ds4_hid;
use serialport::{SerialPortInfo, SerialPortType};
use std::process::ExitCode;

const CYAN: &str = "\x1b[36m";
const RED: &str = "\x1b[31m";
const BOLD: &str = "\x1b[1m";
const NORMAL_INTENSITY: &str = "\x1b[22m";
const RESET: &str = "\x1b[0m";

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
            println!("{}", format_no_devices_found_message());
            ExitCode::SUCCESS
        }
        Ok(devices) => {
            println!("{}", format_device_count(devices.len()));
            for (index, device) in devices.iter().enumerate() {
                println!("{}", format_device_line(index, device));
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to list DUALSHOCK 4 devices: {error}");
            ExitCode::from(1)
        }
    }
}

fn format_no_devices_found_message() -> String {
    format!("{RED}No {BOLD}DUALSHOCK 4{NORMAL_INTENSITY} devices found{RESET}")
}

fn format_device_count(count: usize) -> String {
    format!("{CYAN}DS4 devices found: {count}{RESET}")
}

fn format_device_line(index: usize, device: &ds4_hid::Ds4DeviceInfo) -> String {
    format!(
        "[{index}] {} transport={} vid=0x{:04x} pid=0x{:04x} interface={} product={} path={}",
        format_transport_label(device.transport),
        device.transport,
        device.vendor_id,
        device.product_id,
        device.interface_number,
        device.product_name.as_deref().unwrap_or("unknown"),
        device.path
    )
}

fn format_transport_label(transport: &str) -> String {
    format!("{CYAN}{BOLD}{}{RESET}", transport_display_name(transport))
}

fn transport_display_name(transport: &str) -> &str {
    match transport {
        "usb" => "USB",
        "bluetooth" => "Bluetooth",
        "i2c" => "I2C",
        "spi" => "SPI",
        "unknown" => "Unknown",
        other => other,
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
                println!("{}", format_output_port(index, port));
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to list serial output ports: {error}");
            ExitCode::from(1)
        }
    }
}

fn format_output_port(index: usize, port: &SerialPortInfo) -> String {
    let port_name = format!("{CYAN}{}{RESET}", port.port_name);
    let port_type = match &port.port_type {
        SerialPortType::UsbPort(info) => {
            format!(
                "type=usb vid=0x{:04x} pid=0x{:04x} manufacturer={} product={CYAN}{}{RESET} serial={}",
                info.vid,
                info.pid,
                info.manufacturer.as_deref().unwrap_or("unknown"),
                info.product.as_deref().unwrap_or("unknown"),
                info.serial_number.as_deref().unwrap_or("unknown")
            )
        }
        SerialPortType::BluetoothPort => String::from("type=bluetooth"),
        SerialPortType::PciPort => String::from("type=pci"),
        SerialPortType::Unknown => String::from("type=unknown"),
    };

    format!("[{index}]  {port_name}  {port_type}")
}

#[cfg(test)]
mod tests {
    use super::{
        format_device_count, format_device_line, format_no_devices_found_message,
        format_output_port,
    };
    use crate::input::ds4_hid::Ds4DeviceInfo;
    use serialport::{SerialPortInfo, SerialPortType, UsbPortInfo};

    #[test]
    fn format_no_devices_found_message_is_red_with_bold_device_name() {
        assert_eq!(
            format_no_devices_found_message(),
            "\x1b[31mNo \x1b[1mDUALSHOCK 4\x1b[22m devices found\x1b[0m"
        );
    }

    #[test]
    fn format_device_count_is_cyan() {
        assert_eq!(
            format_device_count(2),
            "\x1b[36mDS4 devices found: 2\x1b[0m"
        );
    }

    #[test]
    fn format_device_line_places_cyan_bold_transport_after_index() {
        let device = Ds4DeviceInfo {
            path: String::from("IOService:/example"),
            vendor_id: 0x054c,
            product_id: 0x09cc,
            interface_number: 0,
            product_name: Some(String::from("Wireless Controller")),
            transport: "bluetooth",
        };

        assert_eq!(
            format_device_line(1, &device),
            "[1] \x1b[36m\x1b[1mBluetooth\x1b[0m transport=bluetooth vid=0x054c pid=0x09cc interface=0 product=Wireless Controller path=IOService:/example"
        );
    }

    #[test]
    fn format_output_port_colors_usb_port_name_and_keeps_it_space_delimited() {
        let port = SerialPortInfo {
            port_name: String::from("/dev/cu.usbserial-1234"),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x2341,
                pid: 0x0043,
                serial_number: Some(String::from("ABC")),
                manufacturer: Some(String::from("Arduino")),
                product: Some(String::from("Uno")),
            }),
        };

        assert_eq!(
            format_output_port(0, &port),
            "[0]  \x1b[36m/dev/cu.usbserial-1234\x1b[0m  type=usb vid=0x2341 pid=0x0043 manufacturer=Arduino product=\x1b[36mUno\x1b[0m serial=ABC"
        );
    }

    #[test]
    fn format_output_port_colors_non_usb_port_name_cyan() {
        let port = SerialPortInfo {
            port_name: String::from("COM3"),
            port_type: SerialPortType::BluetoothPort,
        };

        assert_eq!(
            format_output_port(1, &port),
            "[1]  \x1b[36mCOM3\x1b[0m  type=bluetooth"
        );
    }
}
