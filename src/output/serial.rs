use serialport::{Error as SerialError, SerialPort, new};
use std::fmt;
use std::io::{self, Write};
use std::time::Duration;

const WRITE_TIMEOUT_MILLIS: u64 = 1_000;

#[derive(Debug, Clone)]
pub struct SerialConfig {
    pub port: String,
    pub baud_rate: u32,
}

#[derive(Debug)]
pub enum SerialOutputError {
    Open(SerialError),
    Write(io::Error),
}

impl fmt::Display for SerialOutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open(error) => write!(f, "{error}"),
            Self::Write(error) => write!(f, "{error}"),
        }
    }
}

pub struct SerialOutput {
    port: Box<dyn SerialPort>,
}

impl SerialOutput {
    pub fn open(config: &SerialConfig) -> Result<Self, SerialOutputError> {
        let port = new(&config.port, config.baud_rate)
            .timeout(Duration::from_millis(WRITE_TIMEOUT_MILLIS))
            .open()
            .map_err(SerialOutputError::Open)?;

        Ok(Self { port })
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), SerialOutputError> {
        self.port.write_all(bytes).map_err(SerialOutputError::Write)
    }
}
