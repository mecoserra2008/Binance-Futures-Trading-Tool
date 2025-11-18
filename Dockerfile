# Multi-stage Docker build for Binance Futures Orderflow Screener

# Build stage
FROM rust:1.70-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (cached layer)
RUN cargo build --release
RUN rm src/main.rs

# Copy source code
COPY src ./src

# Build application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN groupadd -r screener && useradd -r -g screener screener

# Create app directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/binance-screener /usr/local/bin/

# Copy configuration files
COPY config.toml ./config.toml

# Create directories
RUN mkdir -p logs backups data temp && \
    chown -R screener:screener /app

# Switch to app user
USER screener

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD pgrep binance-screener || exit 1

# Expose ports (if needed for monitoring)
EXPOSE 8080

# Environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Default command
CMD ["binance-screener"]