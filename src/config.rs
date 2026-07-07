use crate::serial_types::{DisplayMode, LineEnding, SerialSettings, TranslationFormat, TxMode};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub serial: SerialSettings,
    pub line_ending: LineEnding,
    pub display_mode: DisplayMode,
    pub show_timestamps: bool,
    pub auto_scroll: bool,
    pub max_buffer_size: usize,
    pub font_size: f32,
    pub font_color: [u8; 4],
    pub last_export_dir: Option<String>,
    pub unlimited_buffer: bool,
    pub tx_mode: TxMode,
    pub enable_translation: bool,
    pub translation_format: TranslationFormat,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            serial: SerialSettings::default(),
            line_ending: LineEnding::None,
            display_mode: DisplayMode::Ascii,
            show_timestamps: true,
            auto_scroll: true,
            max_buffer_size: 10_000,
            font_size: 13.0,
            font_color: [220, 220, 220, 255],
            last_export_dir: None,
            unlimited_buffer: false,
            tx_mode: TxMode::Ascii,
            enable_translation: true,
            translation_format: TranslationFormat::Hex,
        }
    }
}

impl AppConfig {
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Self {
        if !path.as_ref().exists() {
            return Self::default();
        }
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return Self::default(),
        };
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_err() {
            return Self::default();
        }
        serde_json::from_str(&contents).unwrap_or_else(|_| Self::default())
    }

    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let serialized = serde_json::to_string_pretty(self)?;
        let mut file = File::create(path)?;
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }
}
