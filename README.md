# XDP-TOP

A throughput visualizer and diagnostic tool for eXpress Data Path (XDP) capable network cards.

## Why?
Understanding if XDP is working correctly utilizing the hardware's full capabilities is hard. Visualizing the network queues helps understand performance and throughput.

## How?
XDP-TOP is a terminal-based user interface built in Rust that provides real-time insights into your network interfaces. It tracks hardware capabilities, XDP attachment states, global throughput, and fine-grained per-queue traffic distribution to help you evaluate RSS (Receive Side Scaling) hashing efficiency and overall data flow.

<img width="2859" height="1722" alt="image" src="https://github.com/user-attachments/assets/5a7f4a57-7366-49a3-8fde-0e0fb0709fbd" />


## Features

* **Per-Queue Load Distribution:** Dense bar charts visualizing packet distribution across individual RX and TX hardware queues.
* **Throughput Monitoring:** Live chart displaying RX/TX throughput in Bits Per Second (bps) and Packets Per Second (pps).
* **XDP Capability Detection:** Automatically probes interfaces for native Zero-Copy (drv) capability and displays current XDP attachment states (Native, Generic, Offload).
* **Hardware & Driver Insights:** Displays active driver, firmware version, PCI bus info, and MAC address.
* **Flow Efficiency Assessment:** Compares active hardware queues against physical CPU cores to detect suboptimal context switching or inefficient RSS traffic distribution.
* **Interactive UI:** Fully supports both keyboard navigation and mouse interactions. 

## Prerequisites

XDP-TOP relies on Linux-specific networking utilities to gather hardware-level statistics:

* **OS:** Linux
* **Dependencies:** `ethtool` and `iproute2` (specifically the `ip` command) must be installed and accessible in the system path.
* **Permissions:** Some `ethtool` hardware queries and queue statistics may require elevated privileges. Running with `sudo` is recommended for full visibility.

## Installation

Ensure you have the Rust toolchain installed. If not, install it via [rustup](https://rustup.rs/).

1. Clone the repository:
   ```bash
   git clone https://github.com/cwittenberg/xdp-top.git
   cd xdp-top
   ```
2. Build the project:
   ```bash
   cargo build --release
   sudo cp target/release/xdp-top /usr/local/bin/
   ```
3. Usage:
   ```bash
   sudo xdp-top
   ```
   
### Controls

* **`Tab` / `Shift+Tab`**: Cycle focus between top menu buttons.
* **`Enter`**: Activate the focused button.
* **`Left Arrow` / `Right Arrow`**: Quickly cycle through available network interfaces.
* **`m`**: Open the Network Interface Selection Menu.
* **`Up Arrow` / `Down Arrow`**: Navigate the Interface Selection Menu.
* **`q` / `Esc`**: Quit the application (or close the current popup/menu).
* **Mouse**: Click to select interfaces, toggle throughput charts, open the About menu, or Quit.

## Roadmap
* Windows and Mac version
* More comprehensive efficiency diagnostics/advice


## License

This project is licensed under the MIT License.
