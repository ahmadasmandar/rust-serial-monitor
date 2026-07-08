# Serial Port Filtering and Custom Naming Guide

This guide summarizes the serial port listing, filtering, and custom naming logic implemented in the Rust monitor and demonstrates how to achieve the identical behavior in a **Python** project.

---

## 1. Logic Summary

The core logic handles three main tasks:

1. **Extract Metadata**: Scans all available serial ports on the system and attempts to read detailed USB descriptors (Manufacturer, Product Name, Vendor ID, Product ID, and Serial Number).
2. **Filter Out Low-Info and Microsoft Devices**: Flags devices that are either legacy/virtual ports or default system interfaces (like Microsoft Bluetooth or Dial-up ports) that usually clutter the port selection menu.
3. **Construct Clean Display Names**: Generates readable display names matching the format `PORT_DeviceName` (e.g., `COM15_USB Serial Port`).

### Filtering Criteria (`is_low_info_or_microsoft`)
A port is flagged to be filtered if **any** of the following conditions are met:
* It is **not** a USB device (e.g., legacy serial ports, PCI cards, virtual loopback ports).
* The manufacturer name contains the word **"microsoft"** (case-insensitive).
* The manufacturer name is missing/empty.
* The device is missing essential descriptors:
  * Has no Vendor ID (`vid`) or Product ID (`pid`) (both are `0`).
  * Has no Serial Number (or it is set to `"Not available"`).
  * Has no Product Name (or it is set to `"Not available"`).

---

## 2. Python Implementation

In Python, the standard library for serial ports is `pyserial`. The helper module `serial.tools.list_ports` provides native access to port descriptors on Windows, Linux, and macOS.

### Dependencies
Install `pyserial` via pip:
```bash
pip install pyserial
```

### Python Code Implementation

Here is the Python implementation of the filtering and naming logic:

```python
import sys
import serial.tools.list_ports

class PortDetails:
    def __init__(self, port_name: str, display_name: str, is_low_info_or_microsoft: bool):
        self.port_name = port_name
        self.display_name = display_name
        self.is_low_info_or_microsoft = is_low_info_or_microsoft

    def __repr__(self):
        status = "Low-Info/Filtered" if self.is_low_info_or_microsoft else "Valid"
        return f"{self.display_name} ({status})"

def get_filtered_ports(exclude_low_info: bool = True) -> list[PortDetails]:
    available_ports = []
    
    # Retrieve all serial ports on the system
    ports = serial.tools.list_ports.comports()
    
    for p in ports:
        port_name = p.device
        product_name = p.product
        manufacturer = p.manufacturer
        is_low_info_or_microsoft = False
        
        # 1. Check if the port is a USB port
        # pyserial sets vid/pid to None for non-USB serial devices
        if p.vid is not None or p.pid is not None:
            # Check manufacturer
            if manufacturer:
                mtf_lower = manufacturer.lower()
                if "microsoft" in mtf_lower:
                    is_low_info_or_microsoft = True
            else:
                is_low_info_or_microsoft = True
            
            # Check presence of vital USB details
            has_vid_pid = (p.vid and p.vid != 0) or (p.pid and p.pid != 0)
            has_serial = p.serial_number is not None and p.serial_number.lower() != "not available"
            has_product = product_name is not None and product_name.lower() != "not available"
            
            if not has_vid_pid or not has_serial or not has_product:
                is_low_info_or_microsoft = True
        else:
            # Non-USB ports (PCI, virtual, loopbacks, legacy DB9) are flagged as low-info
            is_low_info_or_microsoft = True
            
        # 2. Build a clean, formatted display name
        name_part = None
        if product_name and product_name.lower() != "not available":
            name_part = product_name.strip()
        elif manufacturer and manufacturer.lower() != "not available":
            name_part = manufacturer.strip()
            
        if name_part and name_part != "":
            display_name = f"{port_name}_{name_part}"
        else:
            display_name = port_name

        # Create details object
        details = PortDetails(
            port_name=port_name,
            display_name=display_name,
            is_low_info_or_microsoft=is_low_info_or_microsoft
        )
        
        # 3. Add to output list (respect filter flag)
        if not exclude_low_info or not is_low_info_or_microsoft:
            available_ports.append(details)
            
    return available_ports

# Example Usage
if __name__ == "__main__":
    print("--- Listing All Detected Serial Ports (Unfiltered) ---")
    all_ports = get_filtered_ports(exclude_low_info=False)
    for p in all_ports:
        print(f"Name: {p.port_name:<8} | Display: {p.display_name:<30} | Filtered: {p.is_low_info_or_microsoft}")

    print("\n--- Listing Active Ports (Filtered / Clean Selection) ---")
    clean_ports = get_filtered_ports(exclude_low_info=True)
    for p in clean_ports:
        print(f"Device Path: {p.port_name:<8} -> Display: {p.display_name}")
```
