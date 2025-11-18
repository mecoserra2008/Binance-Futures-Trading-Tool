# Binance Futures Orderflow Screener

A professional-grade real-time trading platform for monitoring Binance USD-M futures contracts with advanced orderflow analysis capabilities. Built in Rust for maximum performance and reliability.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![SQLite](https://img.shields.io/badge/sqlite-%2307405e.svg?style=for-the-badge&logo=sqlite&logoColor=white)

## Features

### Big Orderflow Screener
- Real-time monitoring of all USD-M futures contracts
- Detects large orders exceeding 0.5% of daily average volume
- Visual alerts with color coding for buy/sell orders
- Advanced filtering and sorting capabilities

### Order Imbalance Tracker
- Real-time bid/ask imbalance monitoring across all tickers
- Visual representation of buyer/seller pressure
- Historical imbalance trends and patterns
- Multiple display modes (table, grid, chart)

### Footprint Chart Analysis
- 1-minute base candlesticks with adjustable time bins
- Volume-at-price footprint display within each candle
- Buy/sell volume segregation with visual bars
- Dynamic reaggregation when timeframe changes

### Liquidation Monitor
- Real-time tracking of forced liquidation orders
- Size and direction of liquidations
- Multi-ticker liquidation flow analysis
- Flash alerts for significant liquidations

## Architecture

- **Multi-threaded**: Tokio async runtime for optimal performance
- **Real-time GUI**: 60fps rendering with egui framework
- **Database**: SQLite for efficient data storage and retrieval
- **WebSocket**: Direct connection to Binance futures streams
- **Modular**: Clean separation of concerns across modules

## Quick Start

### Prerequisites

- Rust 1.70+ 
- 4GB+ RAM recommended
- Internet connection for Binance API access

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd binance-screener

# Run the installation script
chmod +x scripts/install.sh
./scripts/install.sh

# Or build manually
cargo build --release
```

### Running

```bash
# Using make
make run

# Or directly
./target/release/binance-screener

# Or in development mode
make dev
```

## Usage

### Configuration

Edit `config.toml` to customize settings:

```toml
[binance]
websocket_base_url = "wss://fstream.binance.com"
symbols = ["BTCUSDT", "ETHUSDT", "ADAUSDT"]

[analysis]
volume_threshold_percentage = 0.5
imbalance_window_seconds = 60

[gui]
refresh_rate_ms = 16
max_displayed_rows = 100
```

### GUI Panels

1. **Screener**: Monitor large orderflows with filtering
2. **Imbalance**: View real-time order imbalances
3. **Footprint**: Analyze volume-at-price footprint charts
4. **Liquidations**: Track forced liquidation events

### Keyboard Shortcuts

- `Ctrl+1-4`: Switch between panels
- `Ctrl+F`: Focus search/filter
- `Ctrl+R`: Refresh data
- `Ctrl+Q`: Quit application

## Development

### Build Commands

```bash
make help           # Show all available commands
make build          # Build debug version
make release        # Build optimized release
make test           # Run tests
make fmt            # Format code
make clippy         # Run linter
```

### Project Structure

```
src/
├── main.rs              # Application entry point
├── config/              # Configuration management
├── data/                # WebSocket, database, market data
├── analysis/            # Analysis engines
├── gui/                 # User interface
└── utils/               # Utilities and helpers
```

## Docker Deployment

### Docker Compose (Recommended)

```bash
# Start the application
docker-compose up -d

# With monitoring stack
docker-compose --profile monitoring up -d

# View logs
docker-compose logs -f binance-screener
```

### Manual Docker

```bash
# Build image
docker build -t binance-screener .

# Run container
docker run -d \
  --name binance-screener \
  -v $(pwd)/data:/app/data \
  -v $(pwd)/logs:/app/logs \
  binance-screener
```

## System Service (Linux)

```bash
# Install as systemd service
sudo make service-install

# Start service
sudo systemctl start binance-screener

# View logs
sudo journalctl -u binance-screener -f
```

## Monitoring

### Health Checks

```bash
# Comprehensive health check
./scripts/health_check.sh

# Specific checks
./scripts/health_check.sh process
./scripts/health_check.sh database
./scripts/health_check.sh network
```

### Performance Monitoring

- Memory usage: < 1GB under normal operation
- CPU usage: < 30% on modern systems
- Data throughput: 1000+ trades/second
- GUI rendering: 60fps

## Data Management

### Backup

```bash
# Create backup
make backup

# Automated daily backups (add to crontab)
0 2 * * * cd /path/to/screener && make backup
```

### Database Maintenance

```bash
# Check database stats
sqlite3 data.db "SELECT COUNT(*) FROM raw_trades;"

# Clean old data (older than 30 days)
sqlite3 data.db "DELETE FROM raw_trades WHERE timestamp < $(date -d '30 days ago' +%s)000;"
```

## Configuration Reference

### Binance Settings
- `websocket_base_url`: WebSocket endpoint
- `max_reconnect_attempts`: Reconnection retries
- `reconnect_delay_ms`: Delay between retries

### Analysis Settings
- `volume_threshold_percentage`: Big orderflow threshold
- `imbalance_window_seconds`: Imbalance calculation window
- `liquidation_size_threshold`: Minimum liquidation size

### GUI Settings
- `refresh_rate_ms`: GUI update interval (16ms = 60fps)
- `max_displayed_rows`: Maximum table rows
- `color_scheme`: Theme colors

## Troubleshooting

### Common Issues

**WebSocket Connection Failed**
```bash
# Check network connectivity
curl -s https://fapi.binance.com/fapi/v1/ping

# Verify DNS resolution
nslookup fstream.binance.com
```

**High Memory Usage**
```bash
# Check database size
du -h data.db

# Clean old data
make clean
```

**GUI Not Responsive**
- Check system resources
- Reduce `max_displayed_rows` in config
- Increase `refresh_rate_ms` to 33 (30fps)

### Log Files

- Application logs: `logs/app.log`
- Error logs: `logs/error.log`
- WebSocket logs: `logs/websocket.log`

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/new-feature`
3. Make changes and test: `make test`
4. Format code: `make fmt`
5. Submit pull request

### Code Style

- Use `cargo fmt` for formatting
- Follow Rust naming conventions
- Add documentation for public APIs
- Write tests for new features

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Disclaimer

This software is for educational and research purposes only. Trading cryptocurrencies involves substantial risk of loss. The authors are not responsible for any financial losses incurred through the use of this software.

## Support

- 📧 Email: mecoserra2008@gmail.com
- 📖 Linkedin: [Link](https://www.linkedin.com/in/americoserra/)

## Acknowledgments

- [Binance API](https://binance-docs.github.io/apidocs/futures/en/) for market data
- [egui](https://github.com/emilk/egui) for the GUI framework
- [tokio](https://tokio.rs/) for async runtime
- [SQLite](https://sqlite.org/) for data storage

---

**⭐ Star this repository if you find it useful!**
