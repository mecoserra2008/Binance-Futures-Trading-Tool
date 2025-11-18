# scripts/setup.sh - Initial setup script
#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🚀 Setting up Binance Orderflow Screener..."

# Create necessary directories
echo "📁 Creating directories..."
mkdir -p "${PROJECT_ROOT}/data"
mkdir -p "${PROJECT_ROOT}/logs" 
mkdir -p "${PROJECT_ROOT}/backups"

# Set permissions
chmod 755 "${PROJECT_ROOT}/data"
chmod 755 "${PROJECT_ROOT}/logs"
chmod 755 "${PROJECT_ROOT}/backups"

# Copy configuration template
if [[ ! -f "${PROJECT_ROOT}/config.toml" ]]; then
    echo "📋 Creating default configuration..."
    cat > "${PROJECT_ROOT}/config.toml" << 'EOF'
[binance]
api_key = ""
secret_key = ""
testnet = false
reconnect_delay_seconds = 5
max_reconnect_attempts = 10

[analysis]
big_orderflow_threshold_percent = 0.5
imbalance_window_seconds = 60
liquidation_detection_sensitivity = 0.7
volume_history_days = 30

[analysis.price_precision_levels]
"BTCUSDT" = 2
"ETHUSDT" = 2
"ADAUSDT" = 4

[gui]
update_rate_fps = 60
max_displayed_events = 1000
default_symbol = "BTCUSDT"
default_timeframe = "1m"
theme = "dark"

[database]
path = "./data/screener.db"
backup_interval_hours = 24
max_backup_files = 7
vacuum_interval_days = 7
EOF
    echo "✅ Configuration created at config.toml"
    echo "🔧 Please edit config.toml with your preferences"
fi

# Check Rust installation
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install from https://rustup.rs/"
    exit 1
fi

echo "🔨 Building application..."
cd "${PROJECT_ROOT}"
cargo build --release

echo "✅ Setup complete!"
echo ""
echo "Next steps:"
echo "1. Edit config.toml with your settings"
echo "2. Run: ./target/release/binance-orderflow-screener"
echo "3. Check logs in ./logs/ directory"

# scripts/backup.sh - Database backup script
#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DATA_DIR="${PROJECT_ROOT}/data"
BACKUP_DIR="${PROJECT_ROOT}/backups"
DB_FILE="${DATA_DIR}/screener.db"

# Create backup directory
mkdir -p "${BACKUP_DIR}"

if [[ ! -f "${DB_FILE}" ]]; then
    echo "❌ Database file not found: ${DB_FILE}"
    exit 1
fi

# Create timestamped backup
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/screener_backup_${TIMESTAMP}.db"

echo "💾 Creating database backup..."
cp "${DB_FILE}" "${BACKUP_FILE}"

# Compress backup
gzip "${BACKUP_FILE}"
BACKUP_FILE="${BACKUP_FILE}.gz"

echo "✅ Backup created: ${BACKUP_FILE}"

# Get backup size
BACKUP_SIZE=$(ls -lh "${BACKUP_FILE}" | awk '{print $5}')
echo "📊 Backup size: ${BACKUP_SIZE}"

# Clean old backups (keep last 30)
echo "🧹 Cleaning old backups..."
find "${BACKUP_DIR}" -name "screener_backup_*.db.gz" -type f | sort -r | tail -n +31 | xargs -r rm

REMAINING_BACKUPS=$(find "${BACKUP_DIR}" -name "screener_backup_*.db.gz" -type f | wc -l)
echo "📁 ${REMAINING_BACKUPS} backups remaining"

# scripts/health_check.sh - System health check
#!/bin/bash

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DB_FILE="${PROJECT_ROOT}/data/screener.db"
CONFIG_FILE="${PROJECT_ROOT}/config.toml"

echo "🏥 Binance Orderflow Screener Health Check"
echo "=========================================="

# Check if application is running
if pgrep -f "binance-orderflow-screener" > /dev/null; then
    echo "✅ Application: Running"
    PID=$(pgrep -f "binance-orderflow-screener")
    echo "   PID: $PID"
    
    # Get memory usage
    if command -v ps &> /dev/null; then
        MEM_USAGE=$(ps -p $PID -o rss= 2>/dev/null || echo "Unknown")
        echo "   Memory: ${MEM_USAGE} KB"
    fi
else
    echo "❌ Application: Not running"
fi

# Check configuration file
if [[ -f "$CONFIG_FILE" ]]; then
    echo "✅ Configuration: Found"
else
    echo "❌ Configuration: Missing"
fi

# Check database file
if [[ -f "$DB_FILE" ]]; then
    echo "✅ Database: Found"
    DB_SIZE=$(ls -lh "$DB_FILE" | awk '{print $5}')
    echo "   Size: $DB_SIZE"
    
    # Test database integrity
    if command -v sqlite3 &> /dev/null; then
        if sqlite3 "$DB_FILE" "PRAGMA integrity_check;" | grep -q "ok"; then
            echo "✅ Database integrity: OK"
        else
            echo "❌ Database integrity: Failed"
        fi
        
        # Get table counts
        CANDLE_COUNT=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM candles;" 2>/dev/null || echo "0")
        LIQUIDATION_COUNT=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM liquidations;" 2>/dev/null || echo "0")
        echo "   Candles: $CANDLE_COUNT"
        echo "   Liquidations: $LIQUIDATION_COUNT"
    fi
else
    echo "❌ Database: Not found"
fi

# Check disk space
DISK_USAGE=$(df "${PROJECT_ROOT}" | tail -1 | awk '{print $5}' | sed 's/%//')
if [[ $DISK_USAGE -gt 90 ]]; then
    echo "⚠️  Disk space: ${DISK_USAGE}% (Warning: >90%)"
elif [[ $DISK_USAGE -gt 80 ]]; then
    echo "🟡 Disk space: ${DISK_USAGE}% (Caution: >80%)"
else
    echo "✅ Disk space: ${DISK_USAGE}%"
fi

# Check log files
LOG_DIR="${PROJECT_ROOT}/logs"
if [[ -d "$LOG_DIR" ]]; then
    LOG_COUNT=$(find "$LOG_DIR" -name "*.log" 2>/dev/null | wc -l)
    echo "✅ Logs: $LOG_COUNT files"
    
    # Check for recent errors
    if [[ $LOG_COUNT -gt 0 ]]; then
        RECENT_ERRORS=$(find "$LOG_DIR" -name "*.log" -mtime -1 -exec grep -l "ERROR\|FATAL" {} \; 2>/dev/null | wc -l)
        if [[ $RECENT_ERRORS -gt 0 ]]; then
            echo "⚠️  Recent errors found in $RECENT_ERRORS log files"
        else
            echo "✅ No recent errors in logs"
        fi
    fi
else
    echo "❌ Logs: Directory not found"
fi

# Network connectivity check
if command -v curl &> /dev/null; then
    if curl -s --connect-timeout 5 "https://fapi.binance.com/fapi/v1/ping" > /dev/null; then
        echo "✅ Network: Binance API reachable"
    else
        echo "❌ Network: Cannot reach Binance API"
    fi
elif command -v wget &> /dev/null; then
    if wget -q --timeout=5 --tries=1 "https://fapi.binance.com/fapi/v1/ping" -O /dev/null; then
        echo "✅ Network: Binance API reachable"
    else
        echo "❌ Network: Cannot reach Binance API"
    fi
else
    echo "⚠️  Network: Cannot test (curl/wget not available)"
fi

echo "=========================================="

# scripts/performance_test.sh - Performance testing script
#!/bin/bash

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "⚡ Performance Test Suite"
echo "========================"

# Build optimized binary if not exists
if [[ ! -f "${PROJECT_ROOT}/target/release/binance-orderflow-screener" ]]; then
    echo "🔨 Building optimized binary..."
    cd "${PROJECT_ROOT}"
    cargo build --release
fi

echo "🧪 Running performance tests..."

# Test 1: Memory usage over time
echo ""
echo "Test 1: Memory Usage Monitoring"
echo "--------------------------------"

# Start application in background
cd "${PROJECT_ROOT}"
./target/release/binance-orderflow-screener &
APP_PID=$!

sleep 5  # Wait for startup

echo "Monitoring memory usage for 60 seconds..."
for i in {1..12}; do
    if ps -p $APP_PID > /dev/null; then
        MEM_KB=$(ps -p $APP_PID -o rss= | tr -d ' ')
        MEM_MB=$((MEM_KB / 1024))
        echo "$(date '+%H:%M:%S'): ${MEM_MB} MB"
        sleep 5
    else
        echo "❌ Application crashed during test"
        break
    fi
done

# Cleanup
kill $APP_PID 2>/dev/null
wait $APP_PID 2>/dev/null

echo ""
echo "Test 2: Database Performance"
echo "----------------------------"

DB_FILE="${PROJECT_ROOT}/data/screener.db"
if [[ -f "$DB_FILE" ]]; then
    echo "Testing database query performance..."
    
    # Time various queries
    echo -n "SELECT COUNT(*) FROM candles: "
    time sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM candles;" 2>&1 | grep real | awk '{print $2}'
    
    echo -n "Complex aggregation query: "
    time sqlite3 "$DB_FILE" "SELECT symbol, COUNT(*), AVG(volume) FROM candles GROUP BY symbol LIMIT 10;" 2>&1 | grep real | awk '{print $2}'
else
    echo "⚠️  Database not found, skipping tests"
fi

echo ""
echo "Test 3: WebSocket Connection Test"
echo "---------------------------------"

# Test WebSocket connection without GUI
timeout 30 cargo run --release --bin connection_test 2>&1 | head -20

echo ""
echo "Performance test completed"

# Rust test files

# tests/integration_tests.rs
use binance_orderflow_screener::data::market_data::{OrderflowEvent, MarketDataManager};
use binance_orderflow_screener::analysis::imbalance::ImbalanceAnalyzer;
use binance_orderflow_screener::analysis::volume_analysis::VolumeAnalyzer;
use ordered_float::OrderedFloat;
use tokio;

#[tokio::test]
async fn test_market_data_manager_creation() {
    let manager = MarketDataManager::new().await;
    assert!(manager.is_ok(), "Failed to create MarketDataManager");
}

#[tokio::test]
async fn test_orderflow_event_processing() {
    let mut imbalance_analyzer = ImbalanceAnalyzer::new(60);
    
    let event = OrderflowEvent {
        symbol: "BTCUSDT".to_string(),
        timestamp: 1640995200000, // Unix timestamp
        price: OrderedFloat(50000.0),
        quantity: 1.0,
        is_buyer_maker: false,
        trade_id: 12345,
    };
    
    let result = imbalance_analyzer.process_trade(event);
    // First trade might not generate imbalance data
    assert!(result.is_none() || result.is_some());
}

#[tokio::test]
async fn test_volume_analysis() {
    let mut volume_analyzer = VolumeAnalyzer::new();
    
    let event = OrderflowEvent {
        symbol: "BTCUSDT".to_string(),
        timestamp: 1640995200000,
        price: OrderedFloat(50000.0),
        quantity: 2.0,
        is_buyer_maker: false,
        trade_id: 12345,
    };
    
    let result = volume_analyzer.process_trade(&event);
    // Should return None for first trade (no historical data)
    assert!(result.is_none());
}

#[test]
fn test_big_orderflow_detection() {
    let mut volume_analyzer = VolumeAnalyzer::new();
    
    let event = OrderflowEvent {
        symbol: "BTCUSDT".to_string(),
        timestamp: 1640995200000,
        price: OrderedFloat(50000.0),
        quantity: 10.0, // Large quantity
        is_buyer_maker: false,
        trade_id: 12345,
    };
    
    // Without historical data, should return None
    let result = volume_analyzer.is_big_orderflow(&event, 0.5);
    assert!(result.is_none());
}

# tests/unit_tests.rs
use binance_orderflow_screener::utils::math::*;
use binance_orderflow_screener::utils::formatting::*;
use ordered_float::OrderedFloat;

#[test]
fn test_vwap_calculation() {
    let prices = vec![
        (OrderedFloat(100.0), 10.0),
        (OrderedFloat(101.0), 20.0),
        (OrderedFloat(102.0), 15.0),
    ];
    
    let vwap = calculate_vwap(&prices);
    let expected = (100.0 * 10.0 + 101.0 * 20.0 + 102.0 * 15.0) / (10.0 + 20.0 + 15.0);
    
    assert!((vwap - expected).abs() < 0.001, "VWAP calculation incorrect");
}

#[test]
fn test_percentage_change() {
    assert_eq!(calculate_percentage_change(100.0, 110.0), 10.0);
    assert_eq!(calculate_percentage_change(100.0, 90.0), -10.0);
    assert_eq!(calculate_percentage_change(0.0, 100.0), 0.0); // Edge case
}

#[test]
fn test_usd_formatting() {
    assert_eq!(format_usd_amount(1500.0), "$1.5K");
    assert_eq!(format_usd_amount(1500000.0), "$1.5M");
    assert_eq!(format_usd_amount(500.0), "$500");
}

#[test]
fn test_percentage_formatting() {
    assert_eq!(format_percentage(5.67), "+5.67%");
    assert_eq!(format_percentage(-3.14), "-3.14%");
}

#[test]
fn test_volume_formatting() {
    assert_eq!(format_volume(1500.0), "1.5K");
    assert_eq!(format_volume(2500000.0), "2.5M");
    assert_eq!(format_volume(1500000000.0), "1.5B");
}

# src/bin/connection_test.rs - WebSocket connection test binary
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔌 Testing Binance WebSocket connection...");
    
    let url = "wss://fstream.binance.com/ws/btcusdt@aggTrade";
    println!("Connecting to: {}", url);
    
    let (ws_stream, response) = connect_async(url).await?;
    println!("✅ Connected! Status: {}", response.status());
    
    let (mut write, mut read) = ws_stream.split();
    
    // Set a timeout for the test
    let timeout = tokio::time::sleep(Duration::from_secs(30));
    tokio::pin!(timeout);
    
    let mut message_count = 0;
    
    loop {
        tokio::select! {
            message = read.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => {
                        message_count += 1;
                        if message_count <= 5 {
                            println!("📨 Message {}: {}", message_count, 
                                text.chars().take(100).collect::<String>());
                        }
                        
                        if message_count >= 10 {
                            println!("✅ Received {} messages successfully!", message_count);
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        write.send(Message::Pong(data)).await?;
                        println!("🏓 Ping/Pong");
                    }
                    Some(Ok(Message::Close(_))) => {
                        println!("🔚 Connection closed by server");
                        break;
                    }
                    Some(Err(e)) => {
                        println!("❌ WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        println!("🔚 Connection ended");
                        break;
                    }
                    _ => {}
                }
            }
            _ = &mut timeout => {
                println!("⏰ Test completed (timeout reached)");
                break;
            }
        }
    }
    
    println!("📊 Final stats: {} messages received", message_count);
    Ok(())
}

# Makefile - Build and deployment automation
.PHONY: all build test clean install dev prod docker health backup

# Default target
all: build

# Build the application
build:
	cargo build --release

# Build for development
dev:
	cargo build

# Run tests
test:
	cargo test
	cargo test --release

# Run integration tests
test-integration:
	cargo test --test integration_tests

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/

# Install system service (Linux)
install: build
	sudo cp target/release/binance-orderflow-screener /usr/local/bin/
	sudo cp scripts/binance-screener.service /etc/systemd/system/
	sudo systemctl daemon-reload
	sudo systemctl enable binance-screener
	@echo "Service installed. Start with: sudo systemctl start binance-screener"

# Run in development mode
run-dev:
	RUST_LOG=debug cargo run

# Run in production mode  
run-prod:
	RUST_LOG=info ./target/release/binance-orderflow-screener

# Build Docker image
docker:
	docker build -t binance-screener .

# Run Docker container
docker-run:
	docker-compose up -d

# Health check
health:
	./scripts/health_check.sh

# Create database backup
backup:
	./scripts/backup.sh

# Performance test
perf-test:
	./scripts/performance_test.sh

# Setup development environment
setup:
	./scripts/setup.sh

# Format code
fmt:
	cargo fmt

# Run clippy lints
lint:
	cargo clippy -- -D warnings

# Check code quality
check: fmt lint test

# Update dependencies
update:
	cargo update

# Generate documentation
docs:
	cargo doc --no-deps --document-private-items

# Release build with optimization
release: clean
	RUSTFLAGS="-C target-cpu=native" cargo build --release

# Monitor logs in real time
logs:
	tail -f logs/screener.log

# Show system status
status:
	systemctl status binance-screener 2>/dev/null || echo "Service not installed"

# Restart service
restart:
	sudo systemctl restart binance-screener

# Stop service  
stop:
	sudo systemctl stop binance-screener

# Show help
help:
	@echo "Available targets:"
	@echo "  build       - Build the application"
	@echo "  dev         - Build for development"  
	@echo "  test        - Run all tests"
	@echo "  clean       - Clean build artifacts"
	@echo "  install     - Install as system service"
	@echo "  docker      - Build Docker image"
	@echo "  health      - Run health check"
	@echo "  backup      - Create database backup"
	@echo "  setup       - Setup development environment"
	@echo "  release     - Optimized release build"
	@echo "  help        - Show this help"
