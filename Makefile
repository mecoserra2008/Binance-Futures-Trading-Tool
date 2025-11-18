# Binance Futures Orderflow Screener - Makefile

.PHONY: help build run test clean release install dev check lint fmt clippy deps update backup restore health

# Default target
help:
	@echo "Binance Futures Orderflow Screener - Available Commands:"
	@echo ""
	@echo "Development:"
	@echo "  dev       - Run in development mode with hot reload"
	@echo "  build     - Build debug version"
	@echo "  run       - Run the application"
	@echo "  test      - Run all tests"
	@echo "  check     - Check code without building"
	@echo ""
	@echo "Code Quality:"
	@echo "  lint      - Run all linting tools"
	@echo "  fmt       - Format code"
	@echo "  clippy    - Run Clippy linter"
	@echo ""
	@echo "Dependencies:"
	@echo "  deps      - Install dependencies"
	@echo "  update    - Update dependencies"
	@echo ""
	@echo "Production:"
	@echo "  release   - Build optimized release version"
	@echo "  install   - Install release binary"
	@echo ""
	@echo "Data Management:"
	@echo "  backup    - Backup database and config"
	@echo "  restore   - Restore from backup"
	@echo "  clean     - Clean build artifacts and temporary files"
	@echo ""
	@echo "Monitoring:"
	@echo "  health    - Check application health"
	@echo ""

# Development targets
dev:
	@echo "Starting development server..."
	cargo run

build:
	@echo "Building debug version..."
	cargo build

run: build
	@echo "Running application..."
	./target/debug/binance-screener

test:
	@echo "Running tests..."
	cargo test

check:
	@echo "Checking code..."
	cargo check

# Code quality targets
lint: fmt clippy

fmt:
	@echo "Formatting code..."
	cargo fmt

clippy:
	@echo "Running Clippy..."
	cargo clippy -- -D warnings

# Dependencies
deps:
	@echo "Installing dependencies..."
	cargo fetch

update:
	@echo "Updating dependencies..."
	cargo update

# Production targets
release:
	@echo "Building optimized release..."
	cargo build --release

install: release
	@echo "Installing release binary..."
	sudo cp target/release/binance-screener /usr/local/bin/

# Data management
backup:
	@echo "Creating backup..."
	@mkdir -p backups
	@cp data.db backups/data_$(shell date +%Y%m%d_%H%M%S).db 2>/dev/null || true
	@cp config.toml backups/config_$(shell date +%Y%m%d_%H%M%S).toml 2>/dev/null || true
	@echo "Backup created in backups/ directory"

restore:
	@echo "Available backups:"
	@ls -la backups/ 2>/dev/null || echo "No backups found"
	@echo "To restore, manually copy the desired backup file"

clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	@echo "Cleaning temporary files..."
	@rm -rf logs/*.log 2>/dev/null || true
	@rm -rf temp/* 2>/dev/null || true

# Health check
health:
	@echo "Checking application health..."
	@if pgrep -f binance-screener > /dev/null; then \
		echo "✓ Application is running"; \
	else \
		echo "✗ Application is not running"; \
	fi
	@if [ -f data.db ]; then \
		echo "✓ Database file exists"; \
		echo "Database size: $$(du -h data.db | cut -f1)"; \
	else \
		echo "✗ Database file not found"; \
	fi
	@if [ -f config.toml ]; then \
		echo "✓ Config file exists"; \
	else \
		echo "✗ Config file not found"; \
	fi

# Docker targets (if using Docker)
docker-build:
	@echo "Building Docker image..."
	docker build -t binance-screener .

docker-run:
	@echo "Running Docker container..."
	docker run -p 8080:8080 -v $(PWD)/data:/app/data binance-screener

# Systemd service management (Linux)
service-install:
	@echo "Installing systemd service..."
	sudo cp scripts/binance-screener.service /etc/systemd/system/
	sudo systemctl daemon-reload
	sudo systemctl enable binance-screener

service-start:
	sudo systemctl start binance-screener

service-stop:
	sudo systemctl stop binance-screener

service-status:
	sudo systemctl status binance-screener

service-logs:
	sudo journalctl -u binance-screener -f

# Performance profiling
profile:
	@echo "Running with profiling..."
	cargo build --release
	perf record --call-graph=dwarf ./target/release/binance-screener
	perf report

benchmark:
	@echo "Running benchmarks..."
	cargo bench