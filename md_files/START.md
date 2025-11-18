# How to Start the Binance Futures Orderflow Screener

## Prerequisites
- Rust installed on your system
- Internet connection (for Binance API/WebSocket data)

## Quick Start

1. **Navigate to project directory:**
   ```bash
   cd C:\Users\Asus\Desktop\screener_rust
   ```

2. **Build the application:**
   ```bash
   cargo build --release
   ```

3. **Run the application:**
   ```bash
   cargo run --release
   ```

## What to Expect

The application will:
- Connect to Binance futures WebSocket streams
- Display a GUI with multiple panels:
  - **Screener Panel**: Live orderflow alerts and big trades
  - **Imbalance Panel**: Order flow imbalances by symbol
  - **Volume Panel**: Volume profile analysis
  - **Liquidation Panel**: Detected liquidation events
  - **Footprint Panel**: Price level analysis

## Configuration

The app uses `config.toml` for settings. Key configurations:
- **Symbols**: Edit the `symbols` array to track different futures pairs
- **Thresholds**: Adjust volume and liquidation detection thresholds
- **GUI**: Modify refresh rates and display limits

## Troubleshooting

- **TLS support not compiled in**: The Cargo.toml has been updated with TLS features. Stop the app, run `cargo clean`, then `cargo build --release`
- **Only liquidations data, no trades**: Fixed! The app now uses configured symbols from config.toml instead of trying to subscribe to 495+ symbols (which exceeded Binance WebSocket limits)
- **No data showing**: Check internet connection to Binance
- **Build errors**: Run `cargo clean` then `cargo build --release`
- **Performance issues**: Reduce the number of symbols in config.toml

## Data Sources

- Live trade data from Binance Futures WebSocket
- Liquidation data from Binance force order stream
- Real-time analysis and alerts