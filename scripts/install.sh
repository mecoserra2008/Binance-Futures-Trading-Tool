#!/bin/bash

# Binance Futures Orderflow Screener - Installation Script

set -e

echo "🚀 Installing Binance Futures Orderflow Screener..."

# Check system requirements
check_requirements() {
    echo "📋 Checking system requirements..."
    
    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        echo "❌ Rust is not installed. Please install Rust first:"
        echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    
    # Check Rust version
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    echo "✅ Rust version: $RUST_VERSION"
    
    # Check available memory
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        MEMORY_KB=$(grep MemTotal /proc/meminfo | awk '{print $2}')
        MEMORY_GB=$((MEMORY_KB / 1024 / 1024))
        if [ $MEMORY_GB -lt 4 ]; then
            echo "⚠️  Warning: Low memory ($MEMORY_GB GB). Recommended: 4GB+"
        else
            echo "✅ Memory: ${MEMORY_GB}GB"
        fi
    fi
    
    # Check disk space
    DISK_SPACE=$(df -h . | awk 'NR==2 {print $4}')
    echo "✅ Available disk space: $DISK_SPACE"
}

# Install system dependencies
install_dependencies() {
    echo "📦 Installing system dependencies..."
    
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linux
        if command -v apt-get &> /dev/null; then
            sudo apt-get update
            sudo apt-get install -y build-essential pkg-config libssl-dev sqlite3
        elif command -v yum &> /dev/null; then
            sudo yum groupinstall -y "Development Tools"
            sudo yum install -y openssl-devel sqlite-devel
        elif command -v pacman &> /dev/null; then
            sudo pacman -S --noconfirm base-devel openssl sqlite
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        if command -v brew &> /dev/null; then
            brew install openssl sqlite
        else
            echo "⚠️  Please install Homebrew first: https://brew.sh"
        fi
    fi
}

# Build application
build_application() {
    echo "🔨 Building application..."
    
    # Build optimized release
    cargo build --release
    
    echo "✅ Build completed successfully"
}

# Setup directories and files
setup_environment() {
    echo "📁 Setting up environment..."
    
    # Create necessary directories
    mkdir -p logs
    mkdir -p backups
    mkdir -p temp
    mkdir -p data
    
    # Set permissions
    chmod 755 scripts/*.sh 2>/dev/null || true
    
    # Create config file if it doesn't exist
    if [ ! -f config.toml ]; then
        echo "📝 Creating default configuration..."
        ./target/release/binance-screener --create-config 2>/dev/null || true
    fi
    
    echo "✅ Environment setup completed"
}

# Install binary
install_binary() {
    echo "📦 Installing binary..."
    
    # Install to system path
    if [ "$EUID" -eq 0 ]; then
        cp target/release/binance-screener /usr/local/bin/
        chmod +x /usr/local/bin/binance-screener
        echo "✅ Binary installed to /usr/local/bin/"
    else
        echo "💡 To install system-wide, run: sudo make install"
        echo "   Or run directly from: ./target/release/binance-screener"
    fi
}

# Setup systemd service (Linux only)
setup_service() {
    if [[ "$OSTYPE" == "linux-gnu"* ]] && [ "$EUID" -eq 0 ]; then
        echo "🔧 Setting up systemd service..."
        
        cp scripts/binance-screener.service /etc/systemd/system/
        systemctl daemon-reload
        systemctl enable binance-screener
        
        echo "✅ Systemd service installed"
        echo "   Start with: sudo systemctl start binance-screener"
        echo "   View logs:  sudo journalctl -u binance-screener -f"
    fi
}

# Create desktop entry (Linux)
create_desktop_entry() {
    if [[ "$OSTYPE" == "linux-gnu"* ]] && [ ! "$EUID" -eq 0 ]; then
        echo "🖥️  Creating desktop entry..."
        
        DESKTOP_FILE="$HOME/.local/share/applications/binance-screener.desktop"
        mkdir -p "$(dirname "$DESKTOP_FILE")"
        
        cat > "$DESKTOP_FILE" << EOF
[Desktop Entry]
Name=Binance Screener
Comment=Binance Futures Orderflow Screener
Exec=$(pwd)/target/release/binance-screener
Icon=applications-development
Terminal=false
Type=Application
Categories=Development;Finance;
EOF
        
        echo "✅ Desktop entry created"
    fi
}

# Post-installation setup
post_install() {
    echo "🎉 Installation completed successfully!"
    echo ""
    echo "📖 Next steps:"
    echo "   1. Review and edit config.toml if needed"
    echo "   2. Run the application:"
    echo "      ./target/release/binance-screener"
    echo "   3. Or use make commands:"
    echo "      make run    # Run the application"
    echo "      make health # Check system health"
    echo ""
    echo "📚 Documentation:"
    echo "   - Configuration: edit config.toml"
    echo "   - Logs: check logs/ directory"
    echo "   - Database: data.db (SQLite)"
    echo ""
    echo "🔧 Useful commands:"
    echo "   make help   # Show all available commands"
    echo "   make backup # Backup database and config"
    echo "   make clean  # Clean temporary files"
}

# Main installation flow
main() {
    echo "🎯 Binance Futures Orderflow Screener - Installation"
    echo "================================================="
    
    check_requirements
    install_dependencies
    build_application
    setup_environment
    install_binary
    setup_service
    create_desktop_entry
    post_install
}

# Handle command line arguments
case "${1:-install}" in
    "install")
        main
        ;;
    "check")
        check_requirements
        ;;
    "deps")
        install_dependencies
        ;;
    "build")
        build_application
        ;;
    *)
        echo "Usage: $0 [install|check|deps|build]"
        exit 1
        ;;
esac