use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum LineEnding {
    None,
    CR,
    LF,
    CRLF,
}

impl LineEnding {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::None => "",
            LineEnding::CR => "\r",
            LineEnding::LF => "\n",
            LineEnding::CRLF => "\r\n",
        }
    }
}

impl fmt::Display for LineEnding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineEnding::None => write!(f, "None"),
            LineEnding::CR => write!(f, "CR (\\r)"),
            LineEnding::LF => write!(f, "LF (\\n)"),
            LineEnding::CRLF => write!(f, "CRLF (\\r\\n)"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayMode {
    Ascii,
    Hex,
}

impl fmt::Display for DisplayMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisplayMode::Ascii => write!(f, "ASCII"),
            DisplayMode::Hex => write!(f, "HEX"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxMode {
    Ascii,
    Hex,
    Binary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TranslationFormat {
    Hex,
    Binary,
}

impl fmt::Display for TranslationFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TranslationFormat::Hex => write!(f, "HEX"),
            TranslationFormat::Binary => write!(f, "Binary"),
        }
    }
}

impl fmt::Display for TxMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TxMode::Ascii => write!(f, "ASCII"),
            TxMode::Hex => write!(f, "HEX"),
            TxMode::Binary => write!(f, "Binary"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Rx,
    Tx(TxMode),
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::Rx => write!(f, "RX"),
            Direction::Tx(m) => write!(f, "TX ({})", m),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataBits {
    Five,
    Six,
    Seven,
    Eight,
}

impl DataBits {
    pub fn to_serialport(self) -> serialport::DataBits {
        match self {
            DataBits::Five => serialport::DataBits::Five,
            DataBits::Six => serialport::DataBits::Six,
            DataBits::Seven => serialport::DataBits::Seven,
            DataBits::Eight => serialport::DataBits::Eight,
        }
    }
}

impl fmt::Display for DataBits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataBits::Five => write!(f, "5"),
            DataBits::Six => write!(f, "6"),
            DataBits::Seven => write!(f, "7"),
            DataBits::Eight => write!(f, "8"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Parity {
    None,
    Odd,
    Even,
}

impl Parity {
    pub fn to_serialport(self) -> serialport::Parity {
        match self {
            Parity::None => serialport::Parity::None,
            Parity::Odd => serialport::Parity::Odd,
            Parity::Even => serialport::Parity::Even,
        }
    }
}

impl fmt::Display for Parity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Parity::None => write!(f, "None"),
            Parity::Odd => write!(f, "Odd"),
            Parity::Even => write!(f, "Even"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopBits {
    One,
    Two,
}

impl StopBits {
    pub fn to_serialport(self) -> serialport::StopBits {
        match self {
            StopBits::One => serialport::StopBits::One,
            StopBits::Two => serialport::StopBits::Two,
        }
    }
}

impl fmt::Display for StopBits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StopBits::One => write!(f, "1"),
            StopBits::Two => write!(f, "2"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowControl {
    None,
    Software,
    Hardware,
}

impl FlowControl {
    pub fn to_serialport(self) -> serialport::FlowControl {
        match self {
            FlowControl::None => serialport::FlowControl::None,
            FlowControl::Software => serialport::FlowControl::Software,
            FlowControl::Hardware => serialport::FlowControl::Hardware,
        }
    }
}

impl fmt::Display for FlowControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlowControl::None => write!(f, "None"),
            FlowControl::Software => write!(f, "XON/XOFF"),
            FlowControl::Hardware => write!(f, "RTS/CTS"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialSettings {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub flow_control: FlowControl,
    pub poll_interval_ms: u64,
}

impl Default for SerialSettings {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
            poll_interval_ms: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub port_name: String,
    pub manufacturer: String,
    pub device_id: String,
    pub service: String,
    pub driver_provider: String,
    pub driver_version: String,
    pub driver_date: String,

    // basic serialport metadata
    pub port_type: String,
    pub vid: String,
    pub pid: String,
    pub serial_number: String,
    pub product: String,
}
