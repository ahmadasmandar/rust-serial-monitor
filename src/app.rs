use crate::config::AppConfig;
use crate::serial_types::{
    DataBits, DeviceInfo, FlowControl, LineEnding, Parity, StopBits, TranslationFormat, TxMode,
};
use crate::serial_worker::{WorkerCommand, WorkerEvent};
use crate::terminal_buffer::TerminalBuffer;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct PortDetails {
    pub port_name: String,
    pub display_name: String,
    pub is_low_info_or_microsoft: bool,
}

pub struct SerialApp {
    config: AppConfig,
    config_path: PathBuf,

    cmd_tx: Sender<WorkerCommand>,
    event_rx: Receiver<WorkerEvent>,

    available_ports: Vec<PortDetails>,
    exclude_low_info: bool,
    is_paused: bool,
    is_connected: bool,
    status_message: String,
    error_message: Option<(String, Instant)>,

    terminal_buffer: TerminalBuffer,
    tx_input: String,

    command_history: Vec<String>,
    history_index: Option<usize>,
    show_about_dialog: bool,
    show_device_info_dialog: bool,
    device_info_loading: bool,
    device_info: Option<DeviceInfo>,
    terminal_text_cache: String,
    last_buffer_version: usize,
}

impl SerialApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        cmd_tx: Sender<WorkerCommand>,
        event_rx: Receiver<WorkerEvent>,
    ) -> Self {
        // Use user's profile directory to save config
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_path = config_dir.join("serial_monitor_config.json");
        let config = AppConfig::load_from_path(&config_path);

        let terminal_buffer = TerminalBuffer::new(if config.unlimited_buffer {
            0
        } else {
            config.max_buffer_size
        });

        // ── Modern Dark Theme ──────────────────────────────────────────────
        let mut visuals = egui::Visuals::dark();

        // Deep, warm-neutral background tones (easy on the eyes)
        let bg_darkest   = egui::Color32::from_rgb(16, 18, 24);   // main bg
        let bg_dark      = egui::Color32::from_rgb(22, 24, 32);   // panels
        let bg_mid       = egui::Color32::from_rgb(30, 33, 44);   // interactive idle
        let bg_hover     = egui::Color32::from_rgb(40, 44, 60);   // hover
        let bg_active    = egui::Color32::from_rgb(52, 58, 80);   // active/pressed
        let accent       = egui::Color32::from_rgb(86, 140, 245); // primary accent (soft blue)
        let accent_dim   = egui::Color32::from_rgb(60, 105, 200); // accent dimmed
        let text_primary = egui::Color32::from_rgb(210, 215, 225);// main text
        let text_muted   = egui::Color32::from_rgb(130, 138, 160);// secondary text
        let border_color = egui::Color32::from_rgb(42, 46, 62);   // subtle borders
        let separator_col= egui::Color32::from_rgb(38, 42, 56);   // separators

        // Panel backgrounds
        visuals.panel_fill = bg_dark;
        visuals.window_fill = bg_mid;
        visuals.extreme_bg_color = bg_darkest; // TextEdit backgrounds
        visuals.faint_bg_color = egui::Color32::from_rgb(26, 28, 38);

        // Widget styling
        visuals.widgets.noninteractive.bg_fill = bg_dark;
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text_muted);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, border_color);
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(6.0);

        visuals.widgets.inactive.bg_fill = bg_mid;
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text_primary);
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(0.5, border_color);
        visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);

        visuals.widgets.hovered.bg_fill = bg_hover;
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent_dim);
        visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);

        visuals.widgets.active.bg_fill = bg_active;
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent);
        visuals.widgets.active.rounding = egui::Rounding::same(6.0);

        visuals.widgets.open.bg_fill = bg_hover;
        visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, text_primary);
        visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, accent_dim);
        visuals.widgets.open.rounding = egui::Rounding::same(6.0);

        // Selection highlight (accent-tinted)
        visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(86, 140, 245, 80);
        visuals.selection.stroke = egui::Stroke::new(1.0, accent);

        // Window styling
        visuals.window_rounding = egui::Rounding::same(10.0);
        visuals.window_shadow = egui::epaint::Shadow {
            offset: egui::vec2(0.0, 4.0),
            blur: 16.0,
            spread: 2.0,
            color: egui::Color32::from_black_alpha(100),
        };
        visuals.window_stroke = egui::Stroke::new(1.0, border_color);

        // Popup menus
        visuals.popup_shadow = egui::epaint::Shadow {
            offset: egui::vec2(0.0, 2.0),
            blur: 10.0,
            spread: 1.0,
            color: egui::Color32::from_black_alpha(80),
        };

        // Separator color
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, separator_col);


        // Hyperlinks
        visuals.hyperlink_color = accent;

        // Striped backgrounds for tables/grids
        visuals.striped = true;

        cc.egui_ctx.set_visuals(visuals);

        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.item_spacing = egui::vec2(12.0, 10.0);
        style.spacing.window_margin = egui::Margin::same(16.0);

        use egui::TextStyle::*;
        style.text_styles = [
            (
                Heading,
                egui::FontId::new(20.0, egui::FontFamily::Proportional),
            ),
            (
                Body,
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
            ),
            (
                Monospace,
                egui::FontId::new(13.0, egui::FontFamily::Monospace),
            ),
            (
                Button,
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
            ),
            (
                Small,
                egui::FontId::new(11.0, egui::FontFamily::Proportional),
            ),
        ]
        .into();
        cc.egui_ctx.set_style(style);

        let mut app = Self {
            config,
            config_path,
            cmd_tx,
            event_rx,
            available_ports: Vec::new(),
            exclude_low_info: true,
            is_paused: false,
            is_connected: false,
            status_message: "Disconnected".to_string(),
            error_message: None,
            terminal_buffer,
            tx_input: String::new(),
            command_history: Vec::new(),
            history_index: None,
            show_about_dialog: false,
            show_device_info_dialog: false,
            device_info_loading: false,
            device_info: None,
            terminal_text_cache: String::new(),
            // Set to MAX so that it forces a cache rebuild on first draw
            last_buffer_version: usize::MAX,
        };
        app.refresh_ports();
        app
    }

    fn refresh_ports(&mut self) {
        match serialport::available_ports() {
            Ok(ports) => {
                self.available_ports = ports.into_iter().map(|p| {
                    let port_name = p.port_name.clone();
                    let mut product_name = None;
                    let mut manufacturer = None;
                    let mut is_low_info_or_microsoft = false;

                    match &p.port_type {
                        serialport::SerialPortType::UsbPort(usb) => {
                            product_name = usb.product.clone();
                            manufacturer = usb.manufacturer.clone();
                            
                            if let Some(ref mtf) = manufacturer {
                                let mtf_lower = mtf.to_lowercase();
                                if mtf_lower.contains("microsoft") {
                                    is_low_info_or_microsoft = true;
                                }
                            } else {
                                is_low_info_or_microsoft = true;
                            }
                            
                            let has_vid_pid = usb.vid != 0 || usb.pid != 0;
                            let has_serial = usb.serial_number.is_some() && usb.serial_number.as_ref().unwrap() != "Not available";
                            let has_product = usb.product.is_some() && usb.product.as_ref().unwrap() != "Not available";
                            
                            if !has_vid_pid || !has_serial || !has_product {
                                is_low_info_or_microsoft = true;
                            }
                        }
                        _ => {
                            is_low_info_or_microsoft = true;
                        }
                    }

                    let name_part = product_name.as_ref()
                        .or(manufacturer.as_ref())
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty() && s.to_lowercase() != "not available");

                    let display_name = if let Some(name) = name_part {
                        format!("{}_{}", port_name, name)
                    } else {
                        port_name.clone()
                    };

                    PortDetails {
                        port_name,
                        display_name,
                        is_low_info_or_microsoft,
                    }
                }).collect();

                let has_current = self.available_ports.iter().any(|p| {
                    p.port_name == self.config.serial.port_name && 
                    (!self.exclude_low_info || !p.is_low_info_or_microsoft)
                });

                if !has_current {
                    if let Some(first_valid) = self.available_ports.iter().find(|p| {
                        !self.exclude_low_info || !p.is_low_info_or_microsoft
                    }) {
                        self.config.serial.port_name = first_valid.port_name.clone();
                    } else {
                        self.config.serial.port_name.clear();
                    }
                }
            }
            Err(e) => {
                self.set_error(format!("Failed to scan COM ports: {}", e));
            }
        }
    }

    fn set_error(&mut self, msg: String) {
        self.error_message = Some((msg, Instant::now()));
    }

    fn send_data(&mut self) {
        if self.tx_input.is_empty() {
            return;
        }

        let bytes = match self.config.tx_mode {
            TxMode::Ascii => {
                let mut data = self.tx_input.clone();
                data.push_str(self.config.line_ending.as_str());
                Ok(data.into_bytes())
            }
            TxMode::Hex => parse_hex(&self.tx_input),
            TxMode::Binary => parse_binary(&self.tx_input),
        };

        match bytes {
            Ok(bytes_to_send) => {
                if let Err(e) = self
                    .cmd_tx
                    .send(WorkerCommand::WriteData(bytes_to_send.clone()))
                {
                    self.set_error(format!("Failed to send data: {}", e));
                } else {
                    // Record in terminal buffer
                    self.terminal_buffer.push_tx_entry(
                        &bytes_to_send,
                        self.config.tx_mode,
                        chrono::Local::now(),
                    );

                    // Add to command history
                    if self.command_history.last() != Some(&self.tx_input) {
                        self.command_history.push(self.tx_input.clone());
                    }
                    self.history_index = None;
                    self.tx_input.clear();
                }
            }
            Err(e) => {
                self.set_error(format!("Invalid input: {}", e));
            }
        }
    }

    fn save_log_to_file(&mut self) {
        let text = self.terminal_buffer.export_to_string(
            self.config.show_line_numbers,
            self.config.show_timestamps,
            self.config.enable_translation,
            self.config.translation_format,
        );

        let path = rfd::FileDialog::new()
            .add_filter("Log Files", &["log", "txt"])
            .set_file_name("serial_log.txt")
            .save_file();

        if let Some(p) = path {
            if let Some(parent) = p.parent() {
                self.config.last_export_dir = Some(parent.to_string_lossy().into_owned());
                let _ = self.config.save_to_path(&self.config_path);
            }
            match File::create(&p) {
                Ok(mut f) => {
                    if let Err(e) = f.write_all(text.as_bytes()) {
                        self.set_error(format!("Failed to write log file: {}", e));
                    }
                }
                Err(e) => self.set_error(format!("Failed to create log file: {}", e)),
            }
        }
    }
}

impl eframe::App for SerialApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for incoming serial worker events
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                WorkerEvent::Connected(port) => {
                    self.is_connected = true;
                    self.status_message = format!("Connected: {}", port);
                    self.terminal_buffer.clear();
                }
                WorkerEvent::Disconnected => {
                    self.is_connected = false;
                    self.status_message = "Disconnected".to_string();
                }
                WorkerEvent::DataReceived(data) => {
                    if !self.is_paused {
                        self.terminal_buffer
                            .push_bytes_and_truncate(&data, chrono::Local::now());
                    }
                }
                WorkerEvent::ErrorOccurred(err) => {
                    self.set_error(err);
                }
                WorkerEvent::DeviceInfo(info) => {
                    self.device_info_loading = false;
                    self.device_info = Some(*info);
                    self.show_device_info_dialog = true;
                }
            }
        }

        let current_version = self.terminal_buffer.version();
        if current_version != self.last_buffer_version {
            self.terminal_text_cache = self.terminal_buffer.export_to_string(
                self.config.show_line_numbers,
                self.config.show_timestamps,
                self.config.enable_translation,
                self.config.translation_format,
            );
            self.last_buffer_version = current_version;
        }

        // Top Toolbar
        egui::TopBottomPanel::top("top_bar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(18, 20, 28))
                    .inner_margin(egui::Margin::symmetric(15.0, 10.0))
                    .stroke(egui::Stroke::new(0.5, egui::Color32::from_rgb(38, 42, 56))),
            )
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);
                    ui.spacing_mut().interact_size.y = 28.0; // Enforce identical height for all controls

                    // 1. Port selector
                    ui.label("Port:");
                    let mut selected_display = self.config.serial.port_name.clone();
                    if let Some(p) = self.available_ports.iter().find(|p| p.port_name == self.config.serial.port_name) {
                        selected_display = p.display_name.clone();
                    }
                    egui::ComboBox::from_id_source("port_combo")
                        .selected_text(&selected_display)
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            for port in &self.available_ports {
                                if self.exclude_low_info && port.is_low_info_or_microsoft {
                                    continue;
                                }
                                ui.selectable_value(
                                    &mut self.config.serial.port_name,
                                    port.port_name.clone(),
                                    &port.display_name,
                                );
                            }
                        });

                    // 2. Refresh Button (Centered icon, same height)
                    let refresh_btn = egui::Button::new("🔄").min_size(egui::vec2(28.0, 28.0));
                    if ui.add(refresh_btn).on_hover_text("Refresh Ports").clicked() {
                        self.refresh_ports();
                    }

                    // 2b. Device Info Button
                    let info_btn = egui::Button::new("ℹ Info").min_size(egui::vec2(60.0, 28.0));
                    let info_btn_enabled = !self.config.serial.port_name.is_empty();
                    if ui
                        .add_enabled(info_btn_enabled, info_btn)
                        .on_hover_text("Read Device Information")
                        .clicked()
                    {
                        self.device_info_loading = true;
                        let _ = self.cmd_tx.send(WorkerCommand::GetDeviceInfo(
                            self.config.serial.port_name.clone(),
                        ));
                    }

                    if self.device_info_loading {
                        ui.spinner();
                    }

                    if ui.checkbox(&mut self.exclude_low_info, "Filter MS/Low-Info").changed() {
                        let has_current = self.available_ports.iter().any(|p| {
                            p.port_name == self.config.serial.port_name && 
                            (!self.exclude_low_info || !p.is_low_info_or_microsoft)
                        });

                        if !has_current {
                            if let Some(first_valid) = self.available_ports.iter().find(|p| {
                                !self.exclude_low_info || !p.is_low_info_or_microsoft
                            }) {
                                self.config.serial.port_name = first_valid.port_name.clone();
                            } else {
                                self.config.serial.port_name.clear();
                            }
                        }
                    }

                    ui.add_space(2.0);

                    // 3. Baud selector
                    ui.label("Baud:");
                    let baudrates = [9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600];
                    egui::ComboBox::from_id_source("baud_combo")
                        .selected_text(self.config.serial.baud_rate.to_string())
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for baud in baudrates {
                                ui.selectable_value(
                                    &mut self.config.serial.baud_rate,
                                    baud,
                                    baud.to_string(),
                                );
                            }
                        });

                    ui.add_space(6.0);

                    // 4. Open/Close button (Width: 150px)
                    if self.is_connected {
                        let btn = egui::Button::new("⏹ Disconnect")
                            .fill(egui::Color32::from_rgb(160, 55, 55))
                            .min_size(egui::vec2(150.0, 28.0));
                        if ui.add(btn).clicked() {
                            let _ = self.cmd_tx.send(WorkerCommand::Disconnect);
                        }
                    } else {
                        let btn = egui::Button::new("▶ Connect")
                            .fill(egui::Color32::from_rgb(40, 140, 90))
                            .min_size(egui::vec2(150.0, 28.0));
                        if ui.add(btn).clicked() {
                            if !self.config.serial.port_name.is_empty() {
                                self.terminal_buffer.clear();
                                let _ = self
                                    .cmd_tx
                                    .send(WorkerCommand::Connect(self.config.serial.clone()));
                            } else {
                                self.set_error("No serial port selected".to_string());
                            }
                        }
                    }

                    // 5. Status and text (Right-aligned with flexible spacing)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(&self.status_message);

                        // Status Light
                        let (light_color, glow_color) = if self.is_connected {
                            (egui::Color32::from_rgb(60, 210, 120), egui::Color32::from_rgba_unmultiplied(60, 210, 120, 40))
                        } else {
                            (egui::Color32::from_rgb(100, 105, 120), egui::Color32::TRANSPARENT)
                        };
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                        // Glow ring
                        ui.painter().circle_filled(rect.center(), 7.0, glow_color);
                        ui.painter().circle_filled(rect.center(), 5.0, light_color);
                        ui.add_space(6.0);
                    });
                });
            });

        // Settings Panel (Right side)
        egui::SidePanel::right("settings_panel")
            .resizable(false)
            .default_width(220.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(20, 22, 30))
                    .inner_margin(egui::Margin::same(15.0))
                    .stroke(egui::Stroke::new(0.5, egui::Color32::from_rgb(38, 42, 56))),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.heading("Settings");
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    if ui
                        .checkbox(&mut self.config.show_line_numbers, "Show Line Numbers")
                        .changed()
                    {
                        self.last_buffer_version = usize::MAX;
                    }
                    if ui
                        .checkbox(&mut self.config.show_timestamps, "Show Timestamps")
                        .changed()
                    {
                        self.last_buffer_version = usize::MAX;
                    }
                    ui.checkbox(&mut self.config.auto_scroll, "Auto-scroll");
                    if ui
                        .checkbox(&mut self.config.enable_translation, "Enable Translation")
                        .changed()
                    {
                        self.last_buffer_version = usize::MAX;
                    }
                    if self.config.enable_translation {
                        ui.horizontal(|ui| {
                            ui.label("Format:");
                            egui::ComboBox::from_id_source("translation_format_combo")
                                .selected_text(self.config.translation_format.to_string())
                                .width(100.0)
                                .show_ui(ui, |ui| {
                                    for fmt in [TranslationFormat::Hex, TranslationFormat::Binary] {
                                        if ui
                                            .selectable_value(
                                                &mut self.config.translation_format,
                                                fmt,
                                                fmt.to_string(),
                                            )
                                            .changed()
                                        {
                                            self.last_buffer_version = usize::MAX;
                                        }
                                    }
                                });
                        });
                    }
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        if ui
                            .checkbox(&mut self.config.unlimited_buffer, "Unlimited Lines")
                            .changed()
                        {
                            if self.config.unlimited_buffer {
                                self.terminal_buffer.set_max_entries(0);
                            } else {
                                self.terminal_buffer
                                    .set_max_entries(self.config.max_buffer_size);
                            }
                        }
                    });

                    if !self.config.unlimited_buffer {
                        ui.horizontal(|ui| {
                            ui.label("Max Lines:");
                            let mut max_lines = self.config.max_buffer_size;
                            if ui
                                .add(egui::DragValue::new(&mut max_lines).range(10..=100000))
                                .changed()
                            {
                                self.config.max_buffer_size = max_lines;
                                self.terminal_buffer.set_max_entries(max_lines);
                            }
                        });
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    egui::CollapsingHeader::new("🔌 Connection Settings")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.add_space(4.0);
                            egui::Grid::new("serial_config_grid")
                                .num_columns(2)
                                .spacing([10.0, 10.0])
                                .show(ui, |ui| {
                                    ui.label("Data Bits:");
                                    egui::ComboBox::from_id_source("data_bits_combo")
                                        .selected_text(self.config.serial.data_bits.to_string())
                                        .show_ui(ui, |ui| {
                                            for db in [
                                                DataBits::Five,
                                                DataBits::Six,
                                                DataBits::Seven,
                                                DataBits::Eight,
                                            ] {
                                                ui.selectable_value(
                                                    &mut self.config.serial.data_bits,
                                                    db,
                                                    db.to_string(),
                                                );
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Parity:");
                                    egui::ComboBox::from_id_source("parity_combo")
                                        .selected_text(self.config.serial.parity.to_string())
                                        .show_ui(ui, |ui| {
                                            for p in [Parity::None, Parity::Odd, Parity::Even] {
                                                ui.selectable_value(
                                                    &mut self.config.serial.parity,
                                                    p,
                                                    p.to_string(),
                                                );
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Stop Bits:");
                                    egui::ComboBox::from_id_source("stop_bits_combo")
                                        .selected_text(self.config.serial.stop_bits.to_string())
                                        .show_ui(ui, |ui| {
                                            for sb in [StopBits::One, StopBits::Two] {
                                                ui.selectable_value(
                                                    &mut self.config.serial.stop_bits,
                                                    sb,
                                                    sb.to_string(),
                                                );
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Flow Ctrl:");
                                    egui::ComboBox::from_id_source("flow_ctrl_combo")
                                        .selected_text(self.config.serial.flow_control.to_string())
                                        .show_ui(ui, |ui| {
                                            for fc in [
                                                FlowControl::None,
                                                FlowControl::Software,
                                                FlowControl::Hardware,
                                            ] {
                                                ui.selectable_value(
                                                    &mut self.config.serial.flow_control,
                                                    fc,
                                                    fc.to_string(),
                                                );
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Poll (ms):");
                                    ui.horizontal(|ui| {
                                        let mut val = self.config.serial.poll_interval_ms;
                                        if ui
                                            .add(egui::DragValue::new(&mut val).range(0..=250))
                                            .changed()
                                        {
                                            self.config.serial.poll_interval_ms = val;
                                        }
                                        if val == 0 {
                                            ui.label("🚀 Fast");
                                        }
                                    });
                                    ui.end_row();
                                });
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label("Terminal Font Settings:");
                    ui.add_space(6.0);

                    egui::Grid::new("font_settings_grid")
                        .num_columns(2)
                        .spacing([10.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("Font Size:");
                            ui.add(
                                egui::Slider::new(&mut self.config.font_size, 10.0..=24.0).text("px"),
                            );
                            ui.end_row();

                            ui.label("Font Color:");
                            let mut color = egui::Color32::from_rgba_unmultiplied(
                                self.config.font_color[0],
                                self.config.font_color[1],
                                self.config.font_color[2],
                                self.config.font_color[3],
                            );
                            if ui.color_edit_button_srgba(&mut color).changed() {
                                self.config.font_color = color.to_array();
                            }
                            ui.end_row();
                        });

                    ui.add_space(15.0);
                    ui.separator();
                    ui.add_space(15.0);

                    if ui
                        .add_sized(
                            [ui.available_width(), 32.0],
                            egui::Button::new("💾 Export Log")
                                .fill(egui::Color32::from_rgb(35, 60, 110)),
                        )
                        .clicked()
                    {
                        self.save_log_to_file();
                    }
                    ui.add_space(8.0);

                    if ui
                        .add_sized(
                            [ui.available_width(), 32.0],
                            egui::Button::new("📂 Open Export Folder")
                                .fill(egui::Color32::from_rgb(35, 60, 110)),
                        )
                        .clicked()
                    {
                        let dir_to_open = self
                            .config
                            .last_export_dir
                            .clone()
                            .unwrap_or_else(|| ".".to_string());
                        #[cfg(target_os = "windows")]
                        {
                            let _ = std::process::Command::new("explorer")
                                .arg(&dir_to_open)
                                .spawn();
                        }
                        #[cfg(target_os = "linux")]
                        {
                            let _ = std::process::Command::new("xdg-open")
                                .arg(&dir_to_open)
                                .spawn();
                        }
                        #[cfg(target_os = "macos")]
                        {
                            let _ = std::process::Command::new("open")
                                .arg(&dir_to_open)
                                .spawn();
                        }
                    }
                    ui.add_space(8.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), 32.0],
                            egui::Button::new("💾 Save Config")
                                .fill(egui::Color32::from_rgb(35, 60, 110)),
                        )
                        .clicked()
                    {
                        if let Err(e) = self.config.save_to_path(&self.config_path) {
                            self.set_error(format!("Failed to save configuration: {}", e));
                        }
                    }
                    ui.add_space(8.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), 32.0],
                            egui::Button::new("ℹ About Developer")
                                .fill(egui::Color32::from_rgb(30, 50, 75)),
                        )
                        .clicked()
                    {
                        self.show_about_dialog = true;
                    }
                });
            });

        // Live validation and preview calculation
        let mut validation_error = None;

        if !self.tx_input.is_empty() {
            match self.config.tx_mode {
                TxMode::Ascii => {}
                TxMode::Hex => {
                    if let Err(e) = parse_hex(&self.tx_input) {
                        validation_error = Some(e);
                    }
                }
                TxMode::Binary => {
                    if let Err(e) = parse_binary(&self.tx_input) {
                        validation_error = Some(e);
                    }
                }
            }
        }

        // Bottom Input Toolbar
        egui::TopBottomPanel::bottom("bottom_bar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(18, 20, 28))
                    .inner_margin(egui::Margin::symmetric(15.0, 10.0))
                    .stroke(egui::Stroke::new(0.5, egui::Color32::from_rgb(38, 42, 56))),
            )
            .show(ctx, |ui| {
                // Set consistent height for all interactive controls (comboboxes, text edits, buttons)
                ui.spacing_mut().interact_size.y = 28.0;

                ui.vertical(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        ui.label("TX Mode:");
                        egui::ComboBox::from_id_source("tx_mode_combo")
                            .selected_text(self.config.tx_mode.to_string())
                            .width(80.0)
                            .show_ui(ui, |ui| {
                                for mode in [TxMode::Ascii, TxMode::Hex, TxMode::Binary] {
                                    ui.selectable_value(
                                        &mut self.config.tx_mode,
                                        mode,
                                        mode.to_string(),
                                    );
                                }
                            });

                        if self.config.tx_mode == TxMode::Ascii {
                            ui.label("Line Ending:");
                            egui::ComboBox::from_id_source("line_ending_combo")
                                .selected_text(self.config.line_ending.to_string())
                                .width(90.0)
                                .show_ui(ui, |ui| {
                                    for le in [
                                        LineEnding::None,
                                        LineEnding::CR,
                                        LineEnding::LF,
                                        LineEnding::CRLF,
                                    ] {
                                        ui.selectable_value(
                                            &mut self.config.line_ending,
                                            le,
                                            le.to_string(),
                                        );
                                    }
                                });
                        }

                        // Calculate the remaining space for the text edit (Send button is 80.0, spacing is 8.0)
                        let text_edit_width = (ui.available_width() - 88.0).max(100.0);

                        ui.add_enabled_ui(self.is_connected, |ui| {
                            let hint_msg = if self.is_connected {
                                match self.config.tx_mode {
                                    TxMode::Ascii => "Type ASCII message and press Enter...",
                                    TxMode::Hex => "Type HEX bytes (e.g. AA 03 10 FF) and press Enter...",
                                    TxMode::Binary => {
                                        "Type Binary bytes (e.g. 10101010 00000011) and press Enter..."
                                    }
                                }
                            } else {
                                "Connect to a serial port to send messages..."
                            };

                            let response = ui.add_sized(
                                [text_edit_width, 28.0],
                                egui::TextEdit::singleline(&mut self.tx_input)
                                    .hint_text(hint_msg)
                                    .margin(egui::vec2(8.0, 4.0)),
                            );

                            if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                                if validation_error.is_none() {
                                    self.send_data();
                                }
                                response.request_focus();
                            }

                            if response.has_focus() {
                                if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp))
                                    && !self.command_history.is_empty()
                                {
                                    let idx = self
                                        .history_index
                                        .map(|i| if i > 0 { i - 1 } else { 0 })
                                        .unwrap_or(self.command_history.len() - 1);
                                    self.history_index = Some(idx);
                                    self.tx_input = self.command_history[idx].clone();
                                }
                                if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                                    if let Some(idx) = self.history_index {
                                        if idx + 1 < self.command_history.len() {
                                            let next_idx = idx + 1;
                                            self.history_index = Some(next_idx);
                                            self.tx_input = self.command_history[next_idx].clone();
                                        } else {
                                            self.history_index = None;
                                            self.tx_input.clear();
                                        }
                                    }
                                }
                            }

                            let btn_enabled = validation_error.is_none() && !self.tx_input.is_empty();
                            let btn = egui::Button::new("Send")
                                .fill(egui::Color32::from_rgb(50, 110, 190))
                                .min_size(egui::vec2(80.0, 28.0));
                            if ui.add_enabled(btn_enabled, btn).clicked() {
                                self.send_data();
                            }
                        });
                    });
                });
            });

        // Center Panel - Output console
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(14, 16, 22))
                    .inner_margin(egui::Margin::same(15.0)),
            )
            .show(ctx, |ui| {
                // Check if there is an error to display
                if let Some((ref err, timestamp)) = self.error_message {
                    let elapsed = timestamp.elapsed();
                    if elapsed.as_secs() < 6 {
                        ui.colored_label(
                            egui::Color32::from_rgb(220, 50, 50),
                            format!("⚠️ {}", err),
                        );
                        // Schedule repaint so the banner clears even when idle
                        let remaining = std::time::Duration::from_secs(6) - elapsed;
                        ctx.request_repaint_after(remaining);
                    } else {
                        self.error_message = None;
                    }
                }

                ui.horizontal(|ui| {
                    ui.heading("Terminal RX");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Clear Buffer").clicked() {
                            self.terminal_buffer.clear();
                        }
                        let pause_btn_text = if self.is_paused { "▶ Resume" } else { "⏸ Pause" };
                        if ui.button(pause_btn_text).clicked() {
                            self.is_paused = !self.is_paused;
                        }
                    });
                });

                ui.separator();
                ui.add_space(5.0);

                // Render log text area as a single read-only multiline TextEdit for smooth select and copy
                let text_color = egui::Color32::from_rgba_unmultiplied(
                    self.config.font_color[0],
                    self.config.font_color[1],
                    self.config.font_color[2],
                    self.config.font_color[3],
                );

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .stick_to_bottom(self.config.auto_scroll)
                    .show(ui, |ui| {
                        // Handle auto-scrolling when dragging mouse selection outside viewport bounds
                        if ui.input(|i| i.pointer.primary_down()) {
                            if let Some(mouse_pos) = ui.input(|i| i.pointer.latest_pos()) {
                                let clip_rect = ui.clip_rect();
                                if mouse_pos.y > clip_rect.max.y {
                                    ui.scroll_with_delta(egui::vec2(0.0, -20.0));
                                    ui.ctx().request_repaint();
                                } else if mouse_pos.y < clip_rect.min.y {
                                    ui.scroll_with_delta(egui::vec2(0.0, 20.0));
                                    ui.ctx().request_repaint();
                                }
                            }
                        }

                        let response = ui.add(
                            egui::Label::new(
                                egui::RichText::new(&self.terminal_text_cache)
                                    .font(egui::FontId::monospace(self.config.font_size))
                                    .color(text_color),
                            )
                            .selectable(true),
                        );

                        response.context_menu(|ui| {

                            if ui.button("📋 Copy All").clicked() {
                                ui.output_mut(|o| o.copied_text = self.terminal_text_cache.clone());
                                ui.close_menu();
                            }
                            if ui.button("🧹 Clear Buffer").clicked() {
                                self.terminal_buffer.clear();
                                ui.close_menu();
                            }
                            if ui.button("💾 Export Log...").clicked() {
                                self.save_log_to_file();
                                ui.close_menu();
                            }
                        });
                    });
            });

        if self.show_about_dialog {
            let mut is_open = true;
            egui::Window::new("About AA Rust Serial Monitor")
                .open(&mut is_open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("AA Rust Serial Monitor");
                        ui.label("Version 1.0.0");
                        ui.add_space(8.0);
                    });
                    ui.separator();
                    ui.add_space(8.0);

                    ui.strong("Developed by:");
                    ui.label("Ahmad Asmandar");
                    ui.label("Mechatronics Engineer & Head of Electronics Development");
                    ui.add_space(8.0);

                    ui.strong("About:");
                    ui.label("Mechatronics Engineer and Head of Electronics Development with a passion for embedded systems, Rust software engineering, and industrial automation. Specialized in STM32, FPGA, sensor systems, and high-performance desktop engineering tools. This application was built with a focus on reliability, simplicity, and productivity for engineers working with serial communication and embedded devices.");
                    ui.add_space(8.0);

                    ui.strong("Contact:");
                    ui.hyperlink_to("ahmedasmndr2@gmail.com", "mailto:ahmedasmndr2@gmail.com");

                    ui.add_space(12.0);
                    ui.vertical_centered(|ui| {
                        if ui.button("Close").clicked() {
                            self.show_about_dialog = false;
                        }
                    });
                });
            if !is_open {
                self.show_about_dialog = false;
            }
        }

        if self.show_device_info_dialog {
            if let Some(ref info) = self.device_info {
                let mut is_open = true;
                egui::Window::new(format!("Device Information - {}", info.port_name))
                    .open(&mut is_open)
                    .resizable(true)
                    .collapsible(false)
                    .default_width(450.0)
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            let mut info_text = String::new();
                            info_text.push_str(&format!("Port Name:       {}\n", info.port_name));
                            info_text.push_str(&format!("Port Type:       {}\n", info.port_type));
                            info_text.push_str(&format!("Product Name:    {}\n", info.product));
                            info_text
                                .push_str(&format!("Manufacturer:    {}\n", info.manufacturer));
                            info_text.push_str(&format!("USB VID:         {}\n", info.vid));
                            info_text.push_str(&format!("USB PID:         {}\n", info.pid));
                            info_text
                                .push_str(&format!("Serial Number:   {}\n", info.serial_number));
                            info_text.push_str(&format!("Device ID/Path:  {}\n", info.device_id));
                            info_text.push_str(&format!("Service Driver:  {}\n", info.service));
                            info_text
                                .push_str(&format!("Driver Provider: {}\n", info.driver_provider));
                            info_text
                                .push_str(&format!("Driver Version:  {}\n", info.driver_version));
                            info_text.push_str(&format!("Driver Date:     {}\n", info.driver_date));

                            ui.label("Detailed device details (selectable/copyable):");
                            ui.add_space(4.0);

                            let mut mutable_text = info_text.clone();
                            ui.add(
                                egui::TextEdit::multiline(&mut mutable_text)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(12)
                                    .desired_width(ui.available_width()),
                            );

                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                if ui.button("📋 Copy to Clipboard").clicked() {
                                    ui.output_mut(|o| o.copied_text = info_text);
                                }
                                ui.add_space(10.0);
                                if ui.button("Close").clicked() {
                                    self.show_device_info_dialog = false;
                                }
                            });
                        });
                    });
                if !is_open {
                    self.show_device_info_dialog = false;
                }
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Shutdown the background worker thread gracefully
        let _ = self.cmd_tx.send(WorkerCommand::Exit);
        let _ = self.config.save_to_path(&self.config_path);
    }
}

// TX Mode parser helper functions
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
