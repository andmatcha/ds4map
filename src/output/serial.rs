use serialport::{ClearBuffer, Error as SerialError, SerialPort, new};
use std::fmt;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const READ_TIMEOUT_MILLIS: u64 = 50;
const WRITE_TIMEOUT_MILLIS: u64 = 1_000;
const READ_BUFFER_SIZE: usize = 256;

pub type PortRxCallback = Arc<dyn Fn(String, Vec<u8>) + Send + Sync>;

#[derive(Debug, Clone)]
pub struct SerialConfig {
    pub port: String,
    pub baud_rate: u32,
}

#[derive(Debug)]
pub enum SerialOutputError {
    Open(SerialError),
    Configure(SerialError),
    Write(io::Error),
}

impl fmt::Display for SerialOutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open(error) => write!(f, "{error}"),
            Self::Configure(error) => write!(f, "{error}"),
            Self::Write(error) => write!(f, "{error}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PortRxSnapshot {
    pub status: String,
    pub bytes: Vec<u8>,
}

impl PortRxSnapshot {
    fn waiting() -> Self {
        Self {
            status: String::from("waiting"),
            bytes: Vec::new(),
        }
    }
}

pub struct SerialOutput {
    port: Box<dyn SerialPort>,
    port_rx: Arc<Mutex<PortRxSnapshot>>,
    reader_stop_requested: Arc<AtomicBool>,
    reader_thread: Option<JoinHandle<()>>,
}

impl SerialOutput {
    pub fn open(
        config: &SerialConfig,
        port_rx_callback: Option<PortRxCallback>,
    ) -> Result<Self, SerialOutputError> {
        let port = new(&config.port, config.baud_rate)
            .timeout(Duration::from_millis(WRITE_TIMEOUT_MILLIS))
            .open()
            .map_err(SerialOutputError::Open)?;
        port.clear(ClearBuffer::Input)
            .map_err(SerialOutputError::Configure)?;
        let mut reader = port.try_clone().map_err(SerialOutputError::Configure)?;
        reader
            .set_timeout(Duration::from_millis(READ_TIMEOUT_MILLIS))
            .map_err(SerialOutputError::Configure)?;
        let port_rx = Arc::new(Mutex::new(PortRxSnapshot::waiting()));
        let reader_stop_requested = Arc::new(AtomicBool::new(false));
        let reader_thread = Some(spawn_reader_thread(
            reader,
            Arc::clone(&port_rx),
            Arc::clone(&reader_stop_requested),
            port_rx_callback,
        ));

        Ok(Self {
            port,
            port_rx,
            reader_stop_requested,
            reader_thread,
        })
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), SerialOutputError> {
        self.port.write_all(bytes).map_err(SerialOutputError::Write)
    }

    pub fn port_rx_snapshot(&self) -> PortRxSnapshot {
        self.port_rx
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl Drop for SerialOutput {
    fn drop(&mut self) {
        self.reader_stop_requested.store(true, Ordering::SeqCst);
        if let Some(reader_thread) = self.reader_thread.take() {
            let _ = reader_thread.join();
        }
    }
}

fn spawn_reader_thread(
    mut reader: Box<dyn SerialPort>,
    port_rx: Arc<Mutex<PortRxSnapshot>>,
    stop_requested: Arc<AtomicBool>,
    port_rx_callback: Option<PortRxCallback>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0u8; READ_BUFFER_SIZE];

        while !stop_requested.load(Ordering::SeqCst) {
            match reader.read(&mut buffer) {
                Ok(0) => {}
                Ok(count) => {
                    let mut received = Vec::from(&buffer[..count]);
                    if let Err(error) =
                        drain_available_bytes(&mut *reader, &mut received, &mut buffer)
                    {
                        let status = format!("error: {error}");
                        notify_port_rx_callback(
                            port_rx_callback.as_ref(),
                            status.clone(),
                            Vec::new(),
                        );
                        update_port_rx_status(&port_rx, status, None);
                        break;
                    }
                    notify_port_rx_callback(
                        port_rx_callback.as_ref(),
                        String::from("received"),
                        received.clone(),
                    );
                    update_port_rx_status(&port_rx, String::from("received"), Some(received));
                }
                Err(error) if is_transient_read_error(&error) => {}
                Err(error) => {
                    let status = format!("error: {error}");
                    notify_port_rx_callback(port_rx_callback.as_ref(), status.clone(), Vec::new());
                    update_port_rx_status(&port_rx, status, None);
                    break;
                }
            }
        }
    })
}

fn notify_port_rx_callback(callback: Option<&PortRxCallback>, status: String, bytes: Vec<u8>) {
    if let Some(callback) = callback {
        callback(status, bytes);
    }
}

fn drain_available_bytes(
    reader: &mut dyn SerialPort,
    received: &mut Vec<u8>,
    buffer: &mut [u8; READ_BUFFER_SIZE],
) -> Result<(), SerialError> {
    loop {
        let available = reader.bytes_to_read()? as usize;
        if available == 0 {
            return Ok(());
        }

        let chunk_len = available.min(buffer.len());
        match reader.read(&mut buffer[..chunk_len]) {
            Ok(0) => return Ok(()),
            Ok(count) => received.extend_from_slice(&buffer[..count]),
            Err(error) if is_transient_read_error(&error) => return Ok(()),
            Err(error) => return Err(error.into()),
        }
    }
}

fn update_port_rx_status(
    port_rx: &Arc<Mutex<PortRxSnapshot>>,
    status: String,
    bytes: Option<Vec<u8>>,
) {
    let mut snapshot = port_rx
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    snapshot.status = status;
    if let Some(bytes) = bytes {
        snapshot.bytes = bytes;
    }
}

fn is_transient_read_error(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
    )
}
