use crate::serial_types::{Direction, TranslationFormat, TxMode};
use chrono::{DateTime, Local};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct BufferEntry {
    pub timestamp: DateTime<Local>,
    pub direction: Direction,
    pub data: Vec<u8>,
    pub formatted_len: usize,
}

pub struct TerminalBuffer {
    entries: VecDeque<BufferEntry>,
    pending: Vec<u8>,
    max_entries: usize,
    version: usize,
    total_lines_received: usize,

    // Incremental String caching parameters
    cached_text: String,
    cached_show_timestamps: bool,
    cached_enable_translation: bool,
    cached_translation_format: TranslationFormat,
    cached_show_line_numbers: bool,
}

impl TerminalBuffer {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            pending: Vec::new(),
            max_entries,
            version: 0,
            total_lines_received: 0,
            cached_text: String::new(),
            cached_show_timestamps: true,
            cached_enable_translation: true,
            cached_translation_format: TranslationFormat::Hex,
            cached_show_line_numbers: true,
        }
    }

    pub fn set_max_entries(&mut self, max: usize) {
        self.max_entries = max;
        self.truncate_internal();
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.pending.clear();
        self.cached_text.clear();
        self.total_lines_received = 0;
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
                let mut line_data = std::mem::replace(&mut self.pending, Vec::with_capacity(256));
                line_data.extend_from_slice(&bytes[start..=i]);

                self.total_lines_received += 1;
                let line_num = if self.cached_show_line_numbers { Some(self.total_lines_received) } else { None };

                let mut entry = BufferEntry {
                    timestamp,
                    direction,
                    data: line_data,
                    formatted_len: 0,
                };
                let entry_str = Self::format_entry(
                    &entry,
                    line_num,
                    self.cached_show_timestamps,
                    self.cached_enable_translation,
                    self.cached_translation_format,
                );
                entry.formatted_len = entry_str.len();
                self.cached_text.push_str(&entry_str);
                self.entries.push_back(entry);

                start = i + 1;
            }
        }

        // Keep remaining bytes in pending
        if start < bytes.len() {
            self.pending.extend_from_slice(&bytes[start..]);
        }

        // If pending buffer grows too large without newlines (e.g. 4KB), flush it as a line anyway
        if self.pending.len() >= 4096 {
            let line_data = std::mem::replace(&mut self.pending, Vec::with_capacity(4096));
            self.total_lines_received += 1;
            let line_num = if self.cached_show_line_numbers { Some(self.total_lines_received) } else { None };

            let mut entry = BufferEntry {
                timestamp,
                direction,
                data: line_data,
                formatted_len: 0,
            };
            let entry_str = Self::format_entry(
                &entry,
                line_num,
                self.cached_show_timestamps,
                self.cached_enable_translation,
                self.cached_translation_format,
            );
            entry.formatted_len = entry_str.len();
            self.cached_text.push_str(&entry_str);
            self.entries.push_back(entry);
        }

        self.truncate_internal();
        self.version += 1;
    }

    fn truncate_internal(&mut self) {
        let max = if self.max_entries == 0 { 20_000 } else { self.max_entries };
        while self.entries.len() > max {
            if let Some(popped) = self.entries.pop_front() {
                if popped.formatted_len <= self.cached_text.len() {
                    self.cached_text.drain(..popped.formatted_len);
                } else {
                    self.cached_text.clear();
                }
            }
        }
    }

    pub fn push_bytes_and_truncate(&mut self, bytes: &[u8], timestamp: DateTime<Local>) {
        self.push_bytes(bytes, Direction::Rx, timestamp);
        self.truncate_internal();
    }

    pub fn push_tx_entry(&mut self, bytes: &[u8], mode: TxMode, timestamp: DateTime<Local>) {
        self.total_lines_received += 1;
        let line_num = if self.cached_show_line_numbers { Some(self.total_lines_received) } else { None };

        let mut entry = BufferEntry {
            timestamp,
            direction: Direction::Tx(mode),
            data: bytes.to_vec(),
            formatted_len: 0,
        };
        let entry_str = Self::format_entry(
            &entry,
            line_num,
            self.cached_show_timestamps,
            self.cached_enable_translation,
            self.cached_translation_format,
        );
        entry.formatted_len = entry_str.len();
        self.cached_text.push_str(&entry_str);
        self.entries.push_back(entry);
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
        line_number: Option<usize>,
        show_timestamps: bool,
        enable_translation: bool,
        translation_format: TranslationFormat,
    ) -> String {
        let mut result = String::new();

        // Line number prefix
        if let Some(num) = line_number {
            result.push_str(&format!("[{}] ", num));
        }

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
        &mut self,
        show_line_numbers: bool,
        show_timestamps: bool,
        enable_translation: bool,
        translation_format: TranslationFormat,
    ) -> String {
        // If formatting parameters changed, rebuild the cache
        if self.cached_show_timestamps != show_timestamps
            || self.cached_enable_translation != enable_translation
            || self.cached_translation_format != translation_format
            || self.cached_show_line_numbers != show_line_numbers
            || (self.cached_text.is_empty() && !self.entries.is_empty())
        {
            self.cached_show_timestamps = show_timestamps;
            self.cached_enable_translation = enable_translation;
            self.cached_translation_format = translation_format;
            self.cached_show_line_numbers = show_line_numbers;
            self.cached_text.clear();

            let mut current_num = self.total_lines_received - self.entries.len();
            for entry in &mut self.entries {
                current_num += 1;
                let line_num = if show_line_numbers { Some(current_num) } else { None };
                let entry_str = Self::format_entry(
                    entry,
                    line_num,
                    show_timestamps,
                    enable_translation,
                    translation_format,
                );
                entry.formatted_len = entry_str.len();
                self.cached_text.push_str(&entry_str);
            }
        }

        let mut final_text = self.cached_text.clone();
        if !self.pending.is_empty() {
            let temp_entry = BufferEntry {
                timestamp: Local::now(),
                direction: Direction::Rx,
                data: self.pending.clone(),
                formatted_len: 0,
            };
            let line_num = if show_line_numbers { Some(self.total_lines_received + 1) } else { None };
            final_text.push_str(&Self::format_entry(
                &temp_entry,
                line_num,
                show_timestamps,
                enable_translation,
                translation_format,
            ));
        }
        final_text
    }
}
