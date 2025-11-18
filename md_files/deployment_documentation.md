# Binance Orderflow Screener - Production Deployment Guide

## Quick Start

### Prerequisites

1. **Rust Toolchain** (Latest Stable)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup update stable
```

2. **System Dependencies**
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install build-essential pkg-config libssl-dev libsqlite3-dev

# MacOS
brew install openssl sqlite3

# Windows
# Install Visual Studio Build Tools 2019 or later
# Install vcpkg and sqlite3
```

3. **Hardware Requirements**
- **Minimum**: 4GB RAM, 2-core CPU, 1GB storage
- **Recommended**: 8GB RAM, 4-core CPU, 5GB storage
- **High-Frequency**: 16GB RAM, 8-core CPU, 10GB storage

### Installation Steps

1. **Clone and Build**
```bash
git clone <repository-url> binance-screener
cd binance-screener
cargo build --release
```

2. **Create Data Directory**
```bash
mkdir -p data
chmod 755 data
```

3. **Configure Settings**
```bash
cp config.example.toml config.toml
# Edit config.toml with your preferences
```

4. **Run Application**
```bash
./target/release/binance-orderflow-screener
```

## Configuration Files

### config.toml (Main Configuration)

```toml
[binance]
# Optional: API credentials for enhanced features
api_key = ""
secret_key = ""
testnet = false
reconnect_delay_seconds = 5
max_reconnect_attempts = 10

[analysis]
# Threshold for big orderflow detection (0.5 = 0.5% of daily volume)
big_orderflow_threshold_percent = 0.5

# Time window for imbalance analysis (seconds)
imbalance_window_seconds = 60

# Liquidation detection sensitivity (0.0-1.0, higher = more sensitive)
liquidation_detection_sensitivity = 0.7

# Number of days to keep volume history
volume_history_days = 30

# Custom price precision for specific symbols
[analysis.price_precision_levels]
"BTCUSDT" = 2
"ETHUSDT" = 2
"ADAUSDT" = 4

[gui]
# Target FPS for GUI updates
update_rate_fps = 60

# Maximum events to display in each panel
max_displayed_events = 1000

# Default symbol to show on startup
default_symbol = "BTCUSDT"

# Default timeframe for footprint charts
default_timeframe = "1m"

# Theme selection
theme = "dark"

[database]
# Database file location
path = "./data/screener.db"

# Backup interval (hours)
backup_interval_hours = 24

# Maximum backup files to keep
max_backup_files = 7

# Database maintenance interval (days)
vacuum_interval_days = 7
```

### logging.toml (Logging Configuration)

```toml
[appenders.stdout]
kind = "console"
[appenders.stdout.encoder]
pattern = "{d(%Y-%m-%d %H:%M:%S)} | {({l}):5.5} | {f}:{L} — {m}{n}"

[appenders.file]
kind = "file"
path = "logs/screener.log"
[appenders.file.encoder]
pattern = "{d(%Y-%m-%d %H:%M:%S)} | {({l}):5.5} | {t} | {f}:{L} — {m}{n}"

[root]
level = "info"
appenders = ["stdout", "file"]

[loggers."binance_orderflow_screener::data"]
level = "debug"
additive = false
appenders = ["file"]

[loggers."binance_orderflow_screener::analysis"]
level = "info"
additive = false
appenders = ["file"]
```

## Production Deployment

### Systemd Service (Linux)

Create `/etc/systemd/system/binance-screener.service`:

```ini
[Unit]
Description=Binance Orderflow Screener
After=network.target

[Service]
Type=simple
User=screener
Group=screener
WorkingDirectory=/opt/binance-screener
ExecStart=/opt/binance-screener/target/release/binance-orderflow-screener
Restart=always
RestartSec=10
StandardOutput=syslog
StandardError=syslog
SyslogIdentifier=binance-screener

# Resource limits
MemoryLimit=2G
CPUQuota=200%

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/binance-screener/data /opt/binance-screener/logs

[Install]
WantedBy=multi-user.target
```

**Enable and start service:**
```bash
sudo systemctl daemon-reload
sudo systemctl enable binance-screener
sudo systemctl start binance-screener
sudo systemctl status binance-screener
```

### Docker Deployment

**Dockerfile:**
```dockerfile
FROM rust:1.75-slim as builder

WORKDIR /usr/src/app
COPY . .

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

# Build application
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create user
RUN useradd -r -s /bin/false screener

# Copy binary and set permissions
COPY --from=builder /usr/src/app/target/release/binance-orderflow-screener /usr/local/bin/
RUN chmod +x /usr/local/bin/binance-orderflow-screener

# Create directories
RUN mkdir -p /app/data /app/logs && \
    chown -R screener:screener /app

WORKDIR /app
USER screener

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD pgrep -f binance-orderflow-screener || exit 1

EXPOSE 8080
CMD ["binance-orderflow-screener"]
```

**docker-compose.yml:**
```yaml
version: '3.8'

services:
  screener:
    build: .
    container_name: binance-screener
    restart: unless-stopped
    volumes:
      - ./data:/app/data
      - ./logs:/app/logs
      - ./config.toml:/app/config.toml:ro
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
      - RUST_BACKTRACE=1
    healthcheck:
      test: ["CMD", "pgrep", "-f", "binance-orderflow-screener"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    mem_limit: 2g
    cpus: 2
```

### Performance Optimization

#### System Tuning

**Linux sysctl optimizations** (`/etc/sysctl.conf`):
```ini
# Network optimizations
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216

# Increase file descriptor limits
fs.file-max = 65536

# Memory management
vm.swappiness = 10
vm.dirty_ratio = 15
```

**File descriptor limits** (`/etc/security/limits.conf`):
```
screener soft nofile 65536
screener hard nofile 65536
```

#### Application Tuning

**Environment variables for production:**
```bash
export RUST_LOG=info
export RUST_BACKTRACE=0  # Disable in production
export MALLOC_CONF="background_thread:true,metadata_thp:auto"
```

**Cargo build optimizations** (`Cargo.toml`):
```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true

[profile.release.package.binance-orderflow-screener]
opt-level = 3
```

### Monitoring and Maintenance

#### Log Management

**logrotate configuration** (`/etc/logrotate.d/binance-screener`):
```
/opt/binance-screener/logs/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 screener screener
    postrotate
        systemctl reload binance-screener
    endscript
}
```

#### Database Maintenance

**Weekly maintenance script** (`scripts/maintenance.sh`):
```bash
#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_DIR="${SCRIPT_DIR}/../data"
DB_FILE="${DATA_DIR}/screener.db"
BACKUP_DIR="${DATA_DIR}/backups"

# Create backup directory
mkdir -p "${BACKUP_DIR}"

# Create backup
BACKUP_FILE="${BACKUP_DIR}/screener_$(date +%Y%m%d_%H%M%S).db"
cp "${DB_FILE}" "${BACKUP_FILE}"
gzip "${BACKUP_FILE}"

# Remove old backups (keep 30 days)
find "${BACKUP_DIR}" -name "*.db.gz" -mtime +30 -delete

# Vacuum database
sqlite3 "${DB_FILE}" "VACUUM;"

# Update statistics
sqlite3 "${DB_FILE}" "ANALYZE;"

echo "Database maintenance completed at $(date)"
```

**Crontab entry:**
```cron
0 2 * * 0 /opt/binance-screener/scripts/maintenance.sh >> /opt/binance-screener/logs/maintenance.log 2>&1
```

### Security Considerations

#### Network Security
- Use firewall to restrict incoming connections
- Consider VPN or SSH tunneling for remote access
- Implement rate limiting if exposing API endpoints

#### File System Security
```bash
# Set appropriate permissions
chmod 750 /opt/binance-screener
chmod 640 /opt/binance-screener/config.toml
chmod 755 /opt/binance-screener/data
chmod -R 644 /opt/binance-screener/data/*.db
```

#### API Key Security (if using Binance API)
- Store keys in environment variables or secure key management
- Use read-only API keys when possible
- Enable IP restrictions on Binance API settings
- Rotate keys regularly

### Troubleshooting

#### Common Issues

1. **Connection Failures**
   - Check internet connectivity
   - Verify Binance API status
   - Check firewall settings
   - Review WebSocket connection logs

2. **High Memory Usage**
   - Reduce `max_displayed_events` in config
   - Decrease `volume_history_days`
   - Monitor for memory leaks in logs

3. **Poor Performance**
   - Lower `update_rate_fps`
   - Reduce number of analyzed symbols
   - Check system resources (CPU, Memory)
   - Optimize database queries

4. **Database Issues**
   - Run VACUUM command on database
   - Check disk space
   - Verify file permissions
   - Restore from backup if corrupted

#### Log Analysis

**Useful log queries:**
```bash
# Connection errors
grep "WebSocket connection failed" logs/screener.log

# Performance metrics
grep "Processed.*events" logs/screener.log | tail -10

# Memory usage
grep -i "memory" logs/screener.log

# Database errors
grep -i "database\|sqlite" logs/screener.log
```

### Scaling and High Availability

#### Multiple Instance Deployment
For high-frequency trading environments, consider:

1. **Load Balancer** with multiple application instances
2. **Shared Database** with read replicas
3. **Message Queue** for event distribution
4. **Monitoring Stack** (Prometheus + Grafana)

#### Resource Planning

**Traffic Estimates:**
- Binance Futures: ~50,000 trades/minute across all symbols
- Database writes: ~500 MB/day for full orderflow data
- Memory usage: ~1GB base + ~500MB per 1M cached events

**Scaling Thresholds:**
- CPU > 70%: Add more instances or upgrade hardware
- Memory > 80%: Reduce cache sizes or add RAM
- Disk I/O > 80%: Use SSD or optimize database queries

### Support and Maintenance

#### Regular Tasks
- [ ] Weekly database maintenance
- [ ] Monthly log rotation cleanup  
- [ ] Quarterly performance review
- [ ] Annual security audit

#### Health Checks
```bash
# Application health
curl -f http://localhost:8080/health || exit 1

# Database integrity
sqlite3 data/screener.db "PRAGMA integrity_check;"

# Disk space
df -h | grep -E "(data|logs)" | awk '{if($5 > 90) exit 1}'
```

This deployment guide provides a production-ready setup for the Binance Orderflow Screener with proper monitoring, security, and maintenance procedures.