use crate::serial_types::{Direction, TranslationFormat, TxMode};
use chrono::{DateTime, Local};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct BufferEntry {
    pub timestamp: DateTime<Local>,
    pub direction: Direction,
    pub data: Vec<u8>,
}

pub struct TerminalBuffer {
    entries: VecDeque<BufferEntry>,
    pending: Vec<u8>,
    max_entries: usize,
    version: usize,
}

impl TerminalBuffer {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            pending: Vec::new(),
            max_entries,
            version: 0,
        }
    }

    pub fn set_max_entries(&mut self, max: usize) {
        self.max_entries = max;
        self.truncate_internal();
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.pending.clear();
        self.version += 1;
    }

    pub fn push_bytes(&mut self, bytes: &[u8], direction: Direction, timestamp: DateTime<Local>) {
        if bytes.is_empty() {
            return;
        }

        // Split incoming bytes by newline to keep lines clean with accurate timestamps
        let mut start = 0;
        for i in 0..bytes.len() {
            if bytes[i] == b'\n' {
                // We found a newline. Combine pending with the new slice up to the newline.
                let mut line_data = std::mem::take(&mut self.pending);
                line_data.extend_from_slice(&bytes[start..=i]);

                self.entries.push_back(BufferEntry {
                    timestamp,
                    direction,
                    data: line_data,
                });
                start = i + 1;
            }
        }

        // Keep remaining bytes in pending
        if start < bytes.len() {
            self.pending.extend_from_slice(&bytes[start..]);
        }

        // If pending buffer grows too large without newlines (e.g. 4KB), flush it as a line anyway
        if self.pending.len() >= 4096 {
            let line_data = std::mem::take(&mut self.pending);
            self.entries.push_back(BufferEntry {
                timestamp,
                direction,
                data: line_data,
            });
        }

        self.truncate_internal();
        self.version += 1;
    }

    fn truncate_internal(&mut self) {
        if self.max_entries == 0 {
            return;
        }
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    pub fn push_bytes_and_truncate(&mut self, bytes: &[u8], timestamp: DateTime<Local>) {
        self.push_bytes(bytes, Direction::Rx, timestamp);
        self.truncate_internal();
    }

    pub fn push_tx_entry(&mut self, bytes: &[u8], mode: TxMode, timestamp: DateTime<Local>) {
        self.entries.push_back(BufferEntry {
            timestamp,
            direction: Direction::Tx(mode),
            data: bytes.to_vec(),
        });
        self.truncate_internal();
        self.version += 1;
    }

    pub fn version(&self) -> usize {
        self.version
    }

    #[allow(dead_code)]
    pub fn entries(&self) -> &VecDeque<BufferEntry> {
        &self.entries
    }

    #[allow(dead_code)]
    pub fn pending(&self) -> &[u8] {
        &self.pending
    }

    // Helper to format a single entry showing ASCII on first line and optional translation underneath
    pub fn format_entry(
        entry: &BufferEntry,
        show_timestamps: bool,
        enable_translation: bool,
        translation_format: TranslationFormat,
    ) -> String {
        let mut result = String::new();

        // --- 1. Original ASCII line ---
        if show_timestamps {
            result.push_str(
                &entry
                    .timestamp
                    .format("[%Y-%m-%d %H:%M:%S%.3f] ")
                    .to_string(),
            );
        }

        // Direction and mode indicator
        match entry.direction {
            Direction::Rx => result.push_str("[RX] "),
            Direction::Tx(m) => result.push_str(&format!("[TX-{}] ", m)),
        }

        // ASCII representation: replace non-printable control characters with dots '.' (excluding newlines)
        let ascii_view: String = entry
            .data
            .iter()
            .filter(|&&b| b != b'\r' && b != b'\n')
            .map(|&b| {
                if (32..=126).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();

        result.push_str(&format!("{}\n", ascii_view));

        // --- 2. Translated line (under it, indented) ---
        if enable_translation {
            result.push_str("    ↳ ");
            match translation_format {
                TranslationFormat::Hex => {
                    let hex_view: String = entry
                        .data
                        .iter()
                        .map(|b| format!("{:02X}", b))
                        .collect::<Vec<_>>()
                        .join(" ");
                    result.push_str(&format!("[HEX] {}\n", hex_view));
                }
                TranslationFormat::Binary => {
                    let bin_view: String = entry
                        .data
                        .iter()
                        .map(|b| format!("{:08b}", b))
                        .collect::<Vec<_>>()
                        .join(" ");
                    result.push_str(&format!("[BIN] {}\n", bin_view));
                }
            }
        }

        result
    }

    pub fn export_to_string(
        &self,
        show_timestamps: bool,
        enable_translation: bool,
        translation_format: TranslationFormat,
    ) -> String {
        let mut result = String::new();
        for entry in &self.entries {
            result.push_str(&Self::format_entry(
                entry,
                show_timestamps,
                enable_translation,
                translation_format,
            ));
        }
        if !self.pending.is_empty() {
            let temp_entry = BufferEntry {
                timestamp: Local::now(),
                direction: Direction::Rx,
                data: self.pending.clone(),
            };
            result.push_str(&Self::format_entry(
                &temp_entry,
                show_timestamps,
                enable_translation,
                translation_format,
            ));
        }
        result
    }
}
