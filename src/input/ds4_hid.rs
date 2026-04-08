use hidapi::{BusType, HidApi, HidDevice, HidError};
use std::fmt;

const SONY_VENDOR_ID: u16 = 0x054c;
const DUALSHOCK_4_PRODUCT_IDS: [u16; 2] = [0x05c4, 0x09cc];
const READ_TIMEOUT_MILLIS: i32 = 1_000;
// USB reports fit in 64 bytes, while Bluetooth input reports are larger.
const MAX_REPORT_SIZE: usize = 128;

#[derive(Debug, Clone)]
pub struct Ds4DeviceInfo {
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub interface_number: i32,
    pub product_name: Option<String>,
    pub transport: &'static str,
}

#[derive(Debug, Clone)]
pub struct InputReportEvent {
    pub sequence: u64,
    pub device: Ds4DeviceInfo,
    pub report: Vec<u8>,
}

#[derive(Debug)]
pub enum Ds4Error {
    Hid(HidError),
    DeviceNotFound,
}

impl fmt::Display for Ds4Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hid(error) => write!(f, "{error}"),
            Self::DeviceNotFound => write!(f, "no compatible DUALSHOCK 4 was found"),
        }
    }
}

impl From<HidError> for Ds4Error {
    fn from(value: HidError) -> Self {
        Self::Hid(value)
    }
}

pub fn list_devices() -> Result<Vec<Ds4DeviceInfo>, Ds4Error> {
    let api = HidApi::new()?;
    Ok(api
        .device_list()
        .filter(|device| is_dualshock_4(device.vendor_id(), device.product_id()))
        .map(map_device_info)
        .collect())
}

pub fn ensure_device_ready() -> Result<(), Ds4Error> {
    let api = HidApi::new()?;
    let device_info = find_best_device(&api).ok_or(Ds4Error::DeviceNotFound)?;
    let _device = device_info.open_device(&api)?;
    Ok(())
}

pub fn monitor_input_reports_until<F, S>(mut on_report: F, should_stop: S) -> Result<(), Ds4Error>
where
    F: FnMut(InputReportEvent),
    S: Fn() -> bool,
{
    let api = HidApi::new()?;
    let device_info = find_best_device(&api).ok_or(Ds4Error::DeviceNotFound)?;
    let device = device_info.open_device(&api)?;
    let metadata = map_device_info(device_info);
    let mut sequence = 0u64;

    loop {
        if should_stop() {
            return Ok(());
        }

        let Some(report) = read_next_report(&device)? else {
            continue;
        };

        sequence += 1;
        on_report(InputReportEvent {
            sequence,
            device: metadata.clone(),
            report,
        });
    }
}

fn find_best_device(api: &HidApi) -> Option<&hidapi::DeviceInfo> {
    api.device_list()
        .filter(|device| is_dualshock_4(device.vendor_id(), device.product_id()))
        .max_by_key(|device| device_priority(device))
}

fn read_next_report(device: &HidDevice) -> Result<Option<Vec<u8>>, Ds4Error> {
    let mut buffer = [0u8; MAX_REPORT_SIZE];
    let bytes_read = device.read_timeout(&mut buffer, READ_TIMEOUT_MILLIS)?;

    if bytes_read == 0 {
        return Ok(None);
    }

    Ok(Some(buffer[..bytes_read].to_vec()))
}

fn map_device_info(device: &hidapi::DeviceInfo) -> Ds4DeviceInfo {
    Ds4DeviceInfo {
        path: device.path().to_string_lossy().into_owned(),
        vendor_id: device.vendor_id(),
        product_id: device.product_id(),
        interface_number: device.interface_number(),
        product_name: device.product_string().map(ToOwned::to_owned),
        transport: bus_type_label(device.bus_type()),
    }
}

fn device_priority(device: &hidapi::DeviceInfo) -> (u8, bool, i32) {
    let transport_score = match device.bus_type() {
        BusType::Usb | BusType::Bluetooth => 2,
        _ => 1,
    };
    let named_score = device.product_string().is_some();

    (transport_score, named_score, -device.interface_number())
}

fn bus_type_label(bus_type: BusType) -> &'static str {
    match bus_type {
        BusType::Usb => "usb",
        BusType::Bluetooth => "bluetooth",
        BusType::I2c => "i2c",
        BusType::Spi => "spi",
        BusType::Unknown => "unknown",
    }
}

fn is_dualshock_4(vendor_id: u16, product_id: u16) -> bool {
    vendor_id == SONY_VENDOR_ID && DUALSHOCK_4_PRODUCT_IDS.contains(&product_id)
}
