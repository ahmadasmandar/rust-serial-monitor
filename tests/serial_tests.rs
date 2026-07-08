#[path = "../src/config.rs"]
mod config;
#[path = "../src/serial_types.rs"]
mod serial_types;
#[path = "../src/terminal_buffer.rs"]
mod terminal_buffer;

use chrono::Local;
use config::AppConfig;
use serial_types::{DeviceInfo, LineEnding, TranslationFormat, TxMode};
use terminal_buffer::TerminalBuffer;

#[test]
fn test_line_endings() {
    assert_eq!(LineEnding::None.as_str(), "");
    assert_eq!(LineEnding::CR.as_str(), "\r");
    assert_eq!(LineEnding::LF.as_str(), "\n");
    assert_eq!(LineEnding::CRLF.as_str(), "\r\n");
}

#[test]
fn test_config_serialization() {
    let mut config = AppConfig::default();
    config.serial.baud_rate = 9600;
    config.serial.port_name = "COM3".to_string();
    config.max_buffer_size = 500;
    config.unlimited_buffer = true;
    config.tx_mode = TxMode::Hex;

    let serialized = serde_json::to_string(&config).unwrap();
    let deserialized: AppConfig = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.serial.baud_rate, 9600);
    assert_eq!(deserialized.serial.port_name, "COM3");
    assert_eq!(deserialized.max_buffer_size, 500);
    assert!(deserialized.unlimited_buffer);
    assert_eq!(deserialized.tx_mode, TxMode::Hex);
}

#[test]
fn test_device_info_serialization() {
    let info = DeviceInfo {
        port_name: "COM15".to_string(),
        manufacturer: "FTDI".to_string(),
        device_id: "FTDIBUS\\VID_0403+PID_6001\\0000".to_string(),
        service: "FTSER2K".to_string(),
        driver_provider: "FTDI".to_string(),
        driver_version: "2.12.36.20".to_string(),
        driver_date: "2024-10-28".to_string(),
        port_type: "USB".to_string(),
        vid: "0x0403".to_string(),
        pid: "0x6001".to_string(),
        serial_number: "A5069RR4".to_string(),
        product: "USB Serial Converter".to_string(),
    };

    let serialized = serde_json::to_string(&info).unwrap();
    let deserialized: DeviceInfo = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.port_name, "COM15");
    assert_eq!(deserialized.manufacturer, "FTDI");
    assert_eq!(deserialized.device_id, "FTDIBUS\\VID_0403+PID_6001\\0000");
    assert_eq!(deserialized.service, "FTSER2K");
    assert_eq!(deserialized.driver_provider, "FTDI");
    assert_eq!(deserialized.driver_version, "2.12.36.20");
    assert_eq!(deserialized.driver_date, "2024-10-28");
    assert_eq!(deserialized.port_type, "USB");
    assert_eq!(deserialized.vid, "0x0403");
    assert_eq!(deserialized.pid, "0x6001");
    assert_eq!(deserialized.serial_number, "A5069RR4");
    assert_eq!(deserialized.product, "USB Serial Converter");
}

#[test]
fn test_terminal_buffer_limits() {
    let mut buffer = TerminalBuffer::new(3);

    buffer.push_bytes_and_truncate(b"Line 1\n", Local::now());
    buffer.push_bytes_and_truncate(b"Line 2\n", Local::now());
    buffer.push_bytes_and_truncate(b"Line 3\n", Local::now());
    buffer.push_bytes_and_truncate(b"Line 4\n", Local::now());

    assert_eq!(buffer.entries().len(), 3);

    let formatted = buffer.export_to_string(false, false, true, TranslationFormat::Hex);
    assert!(formatted.contains("Line 2"));
    assert!(formatted.contains("Line 3"));
    assert!(formatted.contains("Line 4"));
    assert!(!formatted.contains("Line 1"));
}

#[test]
fn test_unlimited_terminal_buffer() {
    let mut buffer = TerminalBuffer::new(0);

    for i in 0..100 {
        let line = format!("Line {}\n", i);
        buffer.push_bytes_and_truncate(line.as_bytes(), Local::now());
    }

    assert_eq!(buffer.entries().len(), 100);
}

// Custom mock functions matching the parsers in app.rs
fn parse_hex(input: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    for token in input.split_whitespace() {
        if token.len() == 1 {
            let padded = format!("0{}", token);
            let byte = u8::from_str_radix(&padded, 16)
                .map_err(|_| format!("Invalid HEX token: '{}'", token))?;
            bytes.push(byte);
        } else if token.len() == 2 {
            let byte = u8::from_str_radix(token, 16)
                .map_err(|_| format!("Invalid HEX token: '{}'", token))?;
            bytes.push(byte);
        } else if token.len() % 2 == 0 {
            for chunk in token.as_bytes().chunks(2) {
                let s = std::str::from_utf8(chunk).unwrap();
                let byte =
                    u8::from_str_radix(s, 16).map_err(|_| format!("Invalid HEX chunk: '{}'", s))?;
                bytes.push(byte);
            }
        } else {
            return Err(format!("HEX token '{}' has invalid odd length", token));
        }
    }
    if bytes.is_empty() && !input.trim().is_empty() {
        return Err("No valid HEX content found".to_string());
    }
    Ok(bytes)
}

fn parse_binary(input: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    for token in input.split_whitespace() {
        if token.len() != 8 {
            return Err(format!("Binary token '{}' must be exactly 8 bits", token));
        }
        if !token.chars().all(|c| c == '0' || c == '1') {
            return Err(format!(
                "Binary token '{}' must contain only 0 and 1",
                token
            ));
        }
        let byte = u8::from_str_radix(token, 2)
            .map_err(|_| format!("Failed to parse binary token: '{}'", token))?;
        bytes.push(byte);
    }
    if bytes.is_empty() && !input.trim().is_empty() {
        return Err("No valid binary content found".to_string());
    }
    Ok(bytes)
}

#[test]
fn test_ascii_mode_hex_translation() {
    let raw_ascii = b"Hello\r\n";
    let hex_view: String = raw_ascii
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(hex_view, "48 65 6C 6C 6F 0D 0A");
}

#[test]
fn test_hex_send_parsing() {
    // Test valid inputs
    assert_eq!(
        parse_hex("AA 03 10 FF 55").unwrap(),
        vec![0xAA, 0x03, 0x10, 0xFF, 0x55]
    );
    assert_eq!(parse_hex("aa 03 10").unwrap(), vec![0xaa, 0x03, 0x10]);
    assert_eq!(parse_hex("AA0310").unwrap(), vec![0xAA, 0x03, 0x10]);
    assert_eq!(parse_hex("A B").unwrap(), vec![0x0A, 0x0B]);

    // Test invalid inputs
    assert!(parse_hex("AA 0G FF").is_err());
    assert!(parse_hex("AA 035").is_err());
}

#[test]
fn test_binary_send_parsing() {
    // Test valid inputs
    assert_eq!(
        parse_binary("10101010 00000011 00010000").unwrap(),
        vec![0xAA, 0x03, 0x10]
    );

    // Test invalid inputs
    assert!(parse_binary("10101010 0000001").is_err()); // Too short
    assert!(parse_binary("10101010 00000012").is_err()); // Invalid character
}

#[test]
fn test_non_utf8_rx_safe_formatting() {
    let mut buffer = TerminalBuffer::new(10);
    // Non-printable UTF-8 sequence [0xFF, 0x00, 0x7F, b'A']
    let bytes = &[0xFF, 0x00, 0x7F, b'A', b'\n'];
    buffer.push_bytes_and_truncate(bytes, Local::now());

    let formatted = buffer.export_to_string(false, false, true, TranslationFormat::Hex);

    // Check Direction indicator
    assert!(formatted.contains("[RX]"));
    // Check printable chars preserved and non-printables safely replaced by '.'
    assert!(formatted.contains("...A"));
    // Check HEX representation correctly formatted
    assert!(formatted.contains("[HEX] FF 00 7F 41"));
}
