use crate::serial_types::{DeviceInfo, SerialSettings};
use crossbeam_channel::{Receiver, Sender};
use serialport::SerialPort;
use std::time::Duration;
use tracing::{error, info, warn};

#[derive(Debug)]
pub enum WorkerCommand {
    Connect(SerialSettings),
    Disconnect,
    WriteData(Vec<u8>),
    GetDeviceInfo(String),
    Exit,
}

#[derive(Debug, Clone)]
pub enum WorkerEvent {
    Connected(String),
    Disconnected,
    DataReceived(Vec<u8>),
    ErrorOccurred(String),
    DeviceInfo(Box<DeviceInfo>),
}

pub struct SerialWorker {
    cmd_rx: Receiver<WorkerCommand>,
    event_tx: Sender<WorkerEvent>,
    ctx: eframe::egui::Context,
    port: Option<Box<dyn SerialPort>>,
}

impl SerialWorker {
    pub fn spawn(
        cmd_rx: Receiver<WorkerCommand>,
        event_tx: Sender<WorkerEvent>,
        ctx: eframe::egui::Context,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let mut worker = Self {
                cmd_rx,
                event_tx,
                ctx,
                port: None,
            };
            worker.run();
        })
    }

    fn run(&mut self) {
        info!("Serial worker thread started.");
        let mut read_buffer = vec![0u8; 4096];
        let mut batch_buffer = Vec::with_capacity(8192);
        let mut last_send = std::time::Instant::now();
        let mut poll_interval = Duration::from_millis(10);

        loop {
            if self.port.is_some() {
                // Poll command channel
                if poll_interval.is_zero() {
                    // Non-blocking command check
                    while let Ok(cmd) = self.cmd_rx.try_recv() {
                        if let WorkerCommand::Connect(ref settings) = cmd {
                            poll_interval = Duration::from_millis(settings.poll_interval_ms);
                        }
                        if !self.handle_command(cmd) {
                            return;
                        }
                    }
                } else {
                    let cmd_timeout = Duration::from_millis(2).min(poll_interval);
                    match self.cmd_rx.recv_timeout(cmd_timeout) {
                        Ok(cmd) => {
                            if let WorkerCommand::Connect(ref settings) = cmd {
                                poll_interval = Duration::from_millis(settings.poll_interval_ms);
                            }
                            if !self.handle_command(cmd) {
                                break;
                            }
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                            info!("Command channel disconnected. Exiting worker thread.");
                            break;
                        }
                    }
                }

                // Read from serial port
                let mut read_occurred = false;
                if let Some(ref mut port) = self.port {
                    match port.read(&mut read_buffer) {
                        Ok(count) => {
                            if count > 0 {
                                batch_buffer.extend_from_slice(&read_buffer[..count]);
                                read_occurred = true;
                            }
                        }
                        Err(ref e)
                            if e.kind() == std::io::ErrorKind::TimedOut
                                || e.kind() == std::io::ErrorKind::WouldBlock =>
                        {
                            // Timeout is expected on non-blocking serial reads
                        }
                        Err(e) => {
                            let err_msg = format!("Serial read error: {}", e);
                            error!("{}", err_msg);
                            let _ = self.event_tx.send(WorkerEvent::ErrorOccurred(err_msg));
                            self.ctx.request_repaint();
                            self.close_port();
                        }
                    }
                }

                // Send batched data if size threshold (2KB) or time threshold (20ms, or 2ms if poll_interval is 0) is reached
                let time_limit = if poll_interval.is_zero() {
                    Duration::from_millis(2)
                } else {
                    Duration::from_millis(20)
                };

                if !batch_buffer.is_empty()
                    && (batch_buffer.len() >= 2048 || last_send.elapsed() >= time_limit)
                {
                    let data = std::mem::take(&mut batch_buffer);
                    let _ = self.event_tx.send(WorkerEvent::DataReceived(data));
                    self.ctx.request_repaint();
                    last_send = std::time::Instant::now();
                }

                // If poll_interval is 0 and no read occurred, yield thread to prevent 100% CPU busy-wait
                if poll_interval.is_zero() && !read_occurred {
                    std::thread::yield_now();
                }
            } else {
                // If there was any residual batched data when we disconnected, send it
                if !batch_buffer.is_empty() {
                    let data = std::mem::take(&mut batch_buffer);
                    let _ = self.event_tx.send(WorkerEvent::DataReceived(data));
                    self.ctx.request_repaint();
                }

                // No active connection: block on commands to save CPU
                match self.cmd_rx.recv() {
                    Ok(cmd) => {
                        if let WorkerCommand::Connect(ref settings) = cmd {
                            poll_interval = Duration::from_millis(settings.poll_interval_ms);
                        }
                        if !self.handle_command(cmd) {
                            break;
                        }
                    }
                    Err(_) => {
                        info!("Command channel disconnected. Exiting worker thread.");
                        break;
                    }
                }
            }
        }

        self.close_port();
        info!("Serial worker thread finished.");
    }

    /// Returns `true` to keep running, `false` to exit the thread loop.
    fn handle_command(&mut self, cmd: WorkerCommand) -> bool {
        match cmd {
            WorkerCommand::Connect(settings) => {
                self.close_port();
                info!(
                    "Connecting to port {} at {} baud...",
                    settings.port_name, settings.baud_rate
                );

                let port_timeout = if settings.poll_interval_ms == 0 {
                    Duration::from_millis(1)
                } else {
                    Duration::from_millis(settings.poll_interval_ms)
                };

                let builder = serialport::new(&settings.port_name, settings.baud_rate)
                    .data_bits(settings.data_bits.to_serialport())
                    .parity(settings.parity.to_serialport())
                    .stop_bits(settings.stop_bits.to_serialport())
                    .flow_control(settings.flow_control.to_serialport())
                    .timeout(port_timeout);

                match builder.open() {
                    Ok(p) => {
                        info!("Connected successfully to {}", settings.port_name);
                        // Discard any stale bytes buffered by the OS while the port was closed
                        if let Err(e) = p.clear(serialport::ClearBuffer::Input) {
                            warn!("Failed to clear serial input buffer: {}", e);
                        }
                        self.port = Some(p);
                        let _ = self
                            .event_tx
                            .send(WorkerEvent::Connected(settings.port_name));
                    }
                    Err(e) => {
                        let err_msg = format!("Failed to open port {}: {}", settings.port_name, e);
                        error!("{}", err_msg);
                        let _ = self.event_tx.send(WorkerEvent::ErrorOccurred(err_msg));
                        let _ = self.event_tx.send(WorkerEvent::Disconnected);
                    }
                }
                self.ctx.request_repaint();
            }
            WorkerCommand::Disconnect => {
                self.close_port();
            }
            WorkerCommand::WriteData(data) => {
                if let Some(ref mut port) = self.port {
                    match port.write_all(&data) {
                        Ok(_) => {
                            if let Err(e) = port.flush() {
                                warn!("Failed to flush port: {}", e);
                            }
                        }
                        Err(e) => {
                            let err_msg = format!("Write failed: {}", e);
                            error!("{}", err_msg);
                            let _ = self.event_tx.send(WorkerEvent::ErrorOccurred(err_msg));
                            self.ctx.request_repaint();
                            self.close_port();
                        }
                    }
                } else {
                    let _ = self.event_tx.send(WorkerEvent::ErrorOccurred(
                        "Cannot write: No open port".to_string(),
                    ));
                    self.ctx.request_repaint();
                }
            }
            WorkerCommand::GetDeviceInfo(port_name) => {
                let mut basic_info = None;
                if let Ok(ports) = serialport::available_ports() {
                    for p in ports {
                        if p.port_name == port_name {
                            basic_info = Some(p);
                            break;
                        }
                    }
                }

                let mut manufacturer = "Not available".to_string();
                let mut device_id = "Not available".to_string();
                let mut service = "Not available".to_string();
                let mut driver_provider = "Not available".to_string();
                let mut driver_version = "Not available".to_string();
                let mut driver_date = "Not available".to_string();

                #[cfg(target_os = "windows")]
                {
                    // 1. Query Win32_PnPEntity using powershell
                    let cmd = format!(
                        "Get-CimInstance Win32_PnPEntity | Where-Object {{ $_.Name -like '*({})*' }} | Select-Object Manufacturer, DeviceID, Service | ConvertTo-Json",
                        port_name
                    );
                    if let Ok(output) = std::process::Command::new("powershell")
                        .args(["-Command", &cmd])
                        .output()
                    {
                        if output.status.success() {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                                if let Some(m) = json["Manufacturer"].as_str() {
                                    manufacturer = m.to_string();
                                }
                                if let Some(id) = json["DeviceID"].as_str() {
                                    device_id = id.to_string();
                                }
                                if let Some(s) = json["Service"].as_str() {
                                    service = s.to_string();
                                }
                            }
                        }
                    }

                    // 2. Query Win32_PnPSignedDriver if we got a DeviceID
                    if device_id != "Not available" {
                        let escaped_id = device_id.replace("\\", "\\\\");
                        let cmd_drv = format!(
                            "Get-CimInstance Win32_PnPSignedDriver | Where-Object {{ $_.DeviceID -eq '{}' }} | Select-Object DriverProviderName, DriverVersion, DriverDate | ConvertTo-Json",
                            escaped_id
                        );
                        if let Ok(output) = std::process::Command::new("powershell")
                            .args(["-Command", &cmd_drv])
                            .output()
                        {
                            if output.status.success() {
                                let stdout = String::from_utf8_lossy(&output.stdout);
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout)
                                {
                                    if let Some(dp) = json["DriverProviderName"].as_str() {
                                        driver_provider = dp.to_string();
                                    }
                                    if let Some(dv) = json["DriverVersion"].as_str() {
                                        driver_version = dv.to_string();
                                    }
                                    if let Some(dd) = json["DriverDate"].as_str() {
                                        let clean_date = dd.replace("/Date(", "").replace(")/", "");
                                        if let Ok(ms) = clean_date.parse::<i64>() {
                                            if let Some(dt) =
                                                chrono::DateTime::from_timestamp_millis(ms)
                                            {
                                                driver_date = dt.format("%Y-%m-%d").to_string();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let mut port_type = "Unknown".to_string();
                let mut vid = "Not available".to_string();
                let mut pid = "Not available".to_string();
                let mut serial_number = "Not available".to_string();
                let mut product = "Not available".to_string();

                if let Some(info) = basic_info {
                    match info.port_type {
                        serialport::SerialPortType::UsbPort(usb) => {
                            port_type = "USB".to_string();
                            vid = format!("0x{:04X}", usb.vid);
                            pid = format!("0x{:04X}", usb.pid);
                            if let Some(sn) = usb.serial_number {
                                serial_number = sn;
                            }
                            if let Some(prod) = usb.product {
                                product = prod;
                            }
                            if manufacturer == "Not available" {
                                if let Some(m) = usb.manufacturer {
                                    manufacturer = m;
                                }
                            }
                        }
                        serialport::SerialPortType::PciPort => {
                            port_type = "PCI".to_string();
                        }
                        serialport::SerialPortType::BluetoothPort => {
                            port_type = "Bluetooth".to_string();
                        }
                        serialport::SerialPortType::Unknown => {
                            port_type = "Unknown".to_string();
                        }
                    }
                }

                let info_payload = DeviceInfo {
                    port_name,
                    manufacturer,
                    device_id,
                    service,
                    driver_provider,
                    driver_version,
                    driver_date,
                    port_type,
                    vid,
                    pid,
                    serial_number,
                    product,
                };

                let _ = self
                    .event_tx
                    .send(WorkerEvent::DeviceInfo(Box::new(info_payload)));
                self.ctx.request_repaint();
            }
            WorkerCommand::Exit => {
                return false;
            }
        }
        true
    }

    fn close_port(&mut self) {
        if self.port.is_some() {
            self.port = None;
            info!("Serial port closed.");
            let _ = self.event_tx.send(WorkerEvent::Disconnected);
            self.ctx.request_repaint();
        }
    }
}
