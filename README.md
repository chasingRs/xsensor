# XSensor

XSensor is a desktop application built with Rust and Dioxus for monitoring and configuring BLE-based sensor devices.

## Features

- **BLE Connectivity**:
  - Automatic scanning for compatible devices.
  - **Auto-Connect**: Automatically connects to devices broadcasting the target service UUID (`0000ffe0...`).
  - Real-time connection status and signal strength (RSSI) monitoring.

- **Real-time Status**:
  - Visualizes sensor data streams.
  - **Smart Counting**: Implements logic to filter signal bounce and release events, ensuring accurate event counting.

- **Parameter Configuration**:
  - Read and write device parameters wirelessly.
  - Configurable thresholds:
    - Low Pressure Threshold
    - High Pressure Threshold
    - Acceleration Threshold

## Tech Stack

- **Language**: [Rust](https://www.rust-lang.org/)
- **UI Framework**: [Dioxus](https://dioxuslabs.com/) (v0.7)
- **Styling**: [Tailwind CSS](https://tailwindcss.com/)
- **BLE Library**: [btleplug](https://github.com/deviceplug/btleplug)

## Project Structure

```
xsensor/
├── assets/          # Static assets and compiled CSS
├── src/
│   ├── api/         # BLE implementation and data models
│   ├── components/  # Reusable UI components
│   ├── views/       # Application pages (Connection, Status, Parameters)
│   ├── context.rs   # Global application state management
│   └── main.rs      # Application entry point
├── Cargo.toml       # Rust dependencies
└── Dioxus.toml      # Dioxus configuration
```

## Getting Started

### Prerequisites

- Rust and Cargo installed.
- Dioxus CLI installed (`cargo install dioxus-cli`).

### Running the App

```bash
dx serve --platform desktop
```

## Development Notes

- **Auto-Connect**: The application is configured to automatically connect to the first discovered device that advertises the service UUID `0000ffe0-0000-1000-8000-00805f9b34fb`.
- **Counter Logic**: The status view ignores `0` values from the notification stream to prevent double-counting on button release.
