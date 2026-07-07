# AA Rust Serial Monitor

A high-performance, premium, and reliable desktop engineering utility built with Rust for serial port communication, embedded systems diagnostics, and industrial automation. Designed with focus on productivity, aesthetic clarity, and visual precision.

Developed by **Ahmad Asmandar** (Mechatronics Engineer & Head of Electronics Development).

---

## 🚀 Key Features

### 📡 Asynchronous Multithreaded Architecture
- **Dedicated Worker Thread**: All serial communication operations (reads, writes, scanning) run on a dedicated worker thread using buffered reads to ensure the graphical interface stays **100% responsive and never freezes**, even under high baud rates (e.g., `921600` or `2000000`).
- **Bounded Channels**: Thread synchronization is handled via bounded crossbeam channels (`bounded(500)`) preventing runaway memory consumption.
- **CPU Yielding**: Configured with a `0ms` (Fastest) poll rate option. Uses standard yielding (`std::thread::yield_now()`) under 0ms timeout loops to prevent CPU core pegging.

### 🔌 Asynchronous Device Information Inspector
- Scan and inspect detailed hardware metadata of the selected COM port via the `ℹ Info` button before opening a connection.
- Gathers hardware VID, PID, Product Name, Serial Number, and Manufacturer.
- On Windows systems, it queries deeper driver information using PowerShell/WMI queries (Service Driver Name, Device ID / Path, Driver Provider Name, Driver Version, and Release Date).
- Details are displayed in a clean, copyable monospace text popup format.

### 🛠️ Advanced Serial Settings Panel
- **Collapsible Connection Settings**: Advanced serial parameters (Data Bits, Parity, Stop Bits, and Flow Control) are neatly hidden under an expandable layout to keep the workspace clean.
- **Buffer & View customizer**:
  - Toggle timestamps.
  - Toggle Auto-scroll.
  - Toggle Unlimited line logs or specify a strict maximum entry size to preserve memory.
  - Customize terminal font color (via color picker) and font size.
  - One-click *Open Export Folder* button to open Windows Explorer targeting the location of your exported logs.

### 📝 Multi-Mode Transmit (TX) Control
- **TX Mode Selection**:
  - **ASCII Mode**: Enter printable text. Live hex preview is rendered underneath.
  - **HEX Mode**: Input hex words (e.g. `AA 03 10 FF 55`). Real-time syntax validation disables the send button and alerts in red if inputs are malformed.
  - **Binary Mode**: Input space-separated 8-bit bytes (e.g. `10101010 00000011`). Fully validated live.
- Command history tracking allows you to reuse sent lines using the Up and Down arrow keys.

### 🖥️ Rich Monospaced Output Console
- **Stacked Layout**: Displays the printable ASCII characters on the main line (with `\r` and `\n` characters stripped to avoid trailing dot artifacts).
- **Optional Translation**: Under each line, the formatted representation is printed (`↳ [HEX] ...` or `↳ [BIN] ...`) matching your target translation setting.
- **Interactive Drag-Selection**: Built with a single read-only text frame allowing seamless text selection, copy-pasting, and scrolling.
- **Right-Click Context Menu**:
  - `📋 Copy Selection` (copies highlighted text).
  - `📋 Copy All` (copies the entire buffer).
  - `🧹 Clear Buffer` (wipes the screen).
  - `💾 Export Log...` (saves history to a `.txt` file).

---

## 🛠️ Build and Installation

### Prerequisites
Make sure you have Rust and Cargo installed.
- **Rust**: [rustup.rs](https://rustup.rs/)

### Setup and Compilation
To configure cargo targets correctly on Nextcloud or file-locking directory environments, it is recommended to redirect the target directory to your temporary filesystem path.

1. Clone or navigate to the project directory:
   ```powershell
   cd c:/Users/asmandar/Nextcloud4/Rust/hello_world
   ```

2. Build & Run:
   ```powershell
   $env:CARGO_TARGET_DIR="C:\Users\asmandar\AppData\Local\Temp\cargo-target"
   cargo run --release
   ```

3. Run Tests:
   ```powershell
   $env:CARGO_TARGET_DIR="C:\Users\asmandar\AppData\Local\Temp\cargo-target"
   cargo test
   ```

---

## 🏛️ Project Structure

- `src/main.rs`: Application entry point, channel allocation, and native options.
- `src/app.rs`: Egui graphical user interface, event listener, state handling, and layout rendering.
- `src/config.rs`: Configuration persistence helper serializing settings to a local JSON file.
- `src/serial_worker.rs`: Non-blocking serial port connection worker. Handles WMI driver calls, reads, writes, and connection status reporting.
- `src/serial_types.rs`: Standard structures, enums, display formats, and type wrappers for serial settings.
- `src/terminal_buffer.rs`: Smart bounded queue handling line splits, timestamps, and multi-mode hex/binary conversions.
- `tests/serial_tests.rs`: Integration tests suite covering parser verification, hex rendering, and buffer limits.

---

## ✉️ Contact & Bio
Developed by **Ahmad Asmandar**  
Mechatronics Engineer and Head of Electronics Development with a passion for embedded systems, Rust software engineering, and industrial automation. Specialized in STM32, FPGA, sensor systems, and high-performance desktop engineering tools.

- **Email**: [ahmedasmndr2@gmail.com](mailto:ahmedasmndr2@gmail.com)
