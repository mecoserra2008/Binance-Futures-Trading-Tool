# Binance Futures Orderflow Screener Platform

## Project Overview

A comprehensive real-time trading platform for monitoring Binance USD-M futures contracts with advanced orderflow analysis capabilities. The platform provides multiple specialized GUI interfaces for different aspects of market microstructure analysis.

## Intent:
"I want to build a screener tool in Rust for Binance futures contracts. The idea is to watch orderflow and track orderflow imbalances of all of the futures contracts (USD-M). Then, I want to have a GUI for screening big orderflow coming into the market (>0,5% of the daily average volume of the particular token) and popping in colors in the application interface the quantity (bought or sold) and the price.

Another GUI will track order imbalances between buyers and sellers for all tickers

Another GUI will track orderflow using candlesticks and filling it exactly like a footprint properly dimensioned for the candlestick and buys/sells in each side with a bar for the amount of volume, make it candles of 1 minute with adjustable bin for aggregation and make sure that when changed the bin, it doesnt add non-sense numbers from the previous aggregation, it reaggregates the data from the original. This will be done for all tickers.

Another GUI will be showing me big liquidations from forced orders engine for all of the tickers.

Document everything and make the plan first of the whole platform. All GUIS must be working at the sime time with parallel processing correctly inputed as Rust requests, use the Rust logic well and make the froontend of the GUI clean, with dark color background and professional, without ridiculous titles and subtitles, make it as a company would."

## Core Features

### 1. Big Orderflow Screener
- Monitors all USD-M futures contracts in real-time
- Detects large orders exceeding 0.5% of daily average volume
- Visual alerts with color coding for buy/sell orders
- Price and quantity display for significant flows

### 2. Order Imbalance Tracker
- Real-time bid/ask imbalance monitoring across all tickers
- Visual representation of buyer/seller pressure
- Historical imbalance trends and patterns

### 3. Footprint Candlestick Analysis
- 1-minute base candlesticks with adjustable time bins
- Volume-at-price footprint display within each candle
- Buy/sell volume segregation with visual bars
- Dynamic reaggregation when timeframe changes
- Available for all active tickers

### 4. Liquidation Monitor
- Real-time tracking of forced liquidation orders
- Size and direction of liquidations
- Multi-ticker liquidation flow analysis

## Technical Architecture

### Core Technologies
- **Language**: Rust (stable)
- **GUI Framework**: egui (immediate mode GUI)
- **Async Runtime**: tokio
- **WebSocket**: tokio-tungstenite
- **Database**: SQLite with rusqlite
- **JSON Processing**: serde_json
- **HTTP Client**: reqwest

### System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Main Application                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │  GUI Thread │  │Data Ingestion│  │ Processing  │         │
│  │   (egui)    │  │   Manager   │  │   Engine    │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │ WebSocket   │  │  Market     │  │ Database    │         │
│  │ Connections │  │  Data       │  │ Manager     │         │
│  │             │  │  Processor  │  │             │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow Architecture

```
Binance API → WebSocket Streams → Market Data Processor → 
→ Analysis Engines → GUI Components → Visual Output
                ↓
        Database Storage ← Historical Data ← Data Aggregator
```

## Module Structure

```
src/
├── main.rs                 # Application entry point
├── config/
│   ├── mod.rs
│   └── settings.rs         # Configuration management
├── data/
│   ├── mod.rs
│   ├── websocket.rs        # Binance WebSocket client
│   ├── market_data.rs      # Market data structures
│   ├── orderflow.rs        # Orderflow analysis logic
│   └── database.rs         # Database operations
├── analysis/
│   ├── mod.rs
│   ├── imbalance.rs        # Order imbalance calculations
│   ├── footprint.rs        # Footprint chart logic
│   ├── liquidations.rs     # Liquidation tracking
│   └── volume_analysis.rs  # Volume analysis algorithms
├── gui/
│   ├── mod.rs
│   ├── app.rs              # Main application window
│   ├── screener_panel.rs   # Big orderflow screener
│   ├── imbalance_panel.rs  # Order imbalance GUI
│   ├── footprint_panel.rs  # Footprint chart GUI
│   ├── liquidation_panel.rs # Liquidation monitor GUI
│   └── theme.rs            # Dark theme configuration
└── utils/
    ├── mod.rs
    ├── math.rs             # Mathematical utilities
    └── formatting.rs       # Data formatting helpers
```

## Data Structures

### Core Market Data Types

```rust
#[derive(Debug, Clone)]
pub struct OrderflowEvent {
    pub symbol: String,
    pub timestamp: u64,
    pub price: f64,
    pub quantity: f64,
    pub is_buyer_maker: bool,
    pub trade_id: u64,
}

#[derive(Debug, Clone)]
pub struct VolumeProfile {
    pub price_levels: BTreeMap<OrderedFloat<f64>, VolumeAtPrice>,
    pub total_volume: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
}

#[derive(Debug, Clone)]
pub struct OrderImbalance {
    pub symbol: String,
    pub timestamp: u64,
    pub bid_volume: f64,
    pub ask_volume: f64,
    pub imbalance_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct LiquidationEvent {
    pub symbol: String,
    pub timestamp: u64,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub is_forced: bool,
}
```

## Database Schema

### Tables Structure

```sql
-- Market data aggregation
CREATE TABLE candles (
    id INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    open_price REAL NOT NULL,
    high_price REAL NOT NULL,
    low_price REAL NOT NULL,
    close_price REAL NOT NULL,
    volume REAL NOT NULL,
    buy_volume REAL NOT NULL,
    sell_volume REAL NOT NULL,
    timeframe TEXT NOT NULL,
    UNIQUE(symbol, timestamp, timeframe)
);

-- Volume profile data
CREATE TABLE volume_profile (
    id INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    price_level REAL NOT NULL,
    buy_volume REAL NOT NULL,
    sell_volume REAL NOT NULL,
    total_volume REAL NOT NULL,
    timeframe TEXT NOT NULL
);

-- Order imbalance tracking
CREATE TABLE order_imbalances (
    id INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    bid_volume REAL NOT NULL,
    ask_volume REAL NOT NULL,
    imbalance_ratio REAL NOT NULL
);

-- Liquidation events
CREATE TABLE liquidations (
    id INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    side TEXT NOT NULL,
    price REAL NOT NULL,
    quantity REAL NOT NULL,
    is_forced INTEGER NOT NULL
);

-- Daily volume statistics
CREATE TABLE daily_stats (
    id INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL,
    date TEXT NOT NULL,
    avg_volume REAL NOT NULL,
    total_volume REAL NOT NULL,
    UNIQUE(symbol, date)
);
```

## Concurrent Processing Strategy

### Thread Architecture

1. **Main GUI Thread**: egui rendering and user interaction
2. **WebSocket Manager Thread**: Manages all Binance connections
3. **Data Processing Pool**: tokio thread pool for parallel analysis
4. **Database Writer Thread**: Handles all database operations
5. **Market Data Distributor**: Routes data to appropriate analyzers

### Channel Communication

```rust
// Data flow channels
mpsc::Sender<OrderflowEvent>     // Raw market data
mpsc::Sender<VolumeProfile>      // Processed volume data  
mpsc::Sender<OrderImbalance>     // Imbalance calculations
mpsc::Sender<LiquidationEvent>   // Liquidation events
mpsc::Sender<GuiUpdate>          // GUI state updates
```

## GUI Design Specifications

### Design Principles
- **Professional Dark Theme**: Dark background (#1e1e1e) with accent colors
- **Clean Interface**: Minimal text, focus on visual data representation
- **Multi-Panel Layout**: Tabbed interface for different analysis views
- **Real-time Updates**: Smooth 60fps rendering with efficient data streaming
- **Color Coding**: 
  - Buy orders: Green (#00ff88)
  - Sell orders: Red (#ff4444)
  - Neutral/Imbalance: Orange (#ffaa00)
  - Background: Dark gray (#1e1e1e)
  - Text: Light gray (#e0e0e0)

### Panel Layout Specifications

#### 1. Big Orderflow Screener Panel
```
┌─────────────────────────────────────────────────┐
│ [Symbol] [Side] [Size] [Price] [% of Daily Vol] │
│ ┌─────────────────────────────────────────────┐ │
│ │ BTCUSDT  BUY   450.2  45,234   0.67%       │ │
│ │ ETHUSDT  SELL  234.1  3,456    0.89%       │ │
│ │ ...                                         │ │
│ └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

#### 2. Order Imbalance Tracker Panel
```
┌─────────────────────────────────────────────────┐
│ [Symbol Grid with Imbalance Heatmap]            │
│ ┌─────────────────────────────────────────────┐ │
│ │ BTCUSDT  [████████░░] +0.23                │ │
│ │ ETHUSDT  [░░████████] -0.45                │ │
│ │ ADAUSDT  [██████░░░░] +0.12                │ │
│ └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

#### 3. Footprint Chart Panel
```
┌─────────────────────────────────────────────────┐
│ [Symbol Selector] [Timeframe Controls]          │
│ ┌─────────────────────────────────────────────┐ │
│ │     [Candlestick with Volume Footprint]    │ │
│ │  Price │ Sells │ Buys │ Price              │ │  
│ │ 45,250 │ ███   │ █    │ 45,250             │ │
│ │ 45,240 │ █     │ ███  │ 45,240             │ │
│ │ 45,230 │ ██    │ ██   │ 45,230             │ │
│ └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

#### 4. Liquidation Monitor Panel
```
┌─────────────────────────────────────────────────┐
│ [Real-time Liquidation Feed]                    │
│ ┌─────────────────────────────────────────────┐ │
│ │ BTCUSDT  LONG  $2.4M @ 45,234  [FORCED]    │ │
│ │ ETHUSDT  SHORT $890K @ 3,456   [FORCED]    │ │
│ │ ...                                         │ │
│ └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Core Infrastructure (Week 1-2)
- [ ] Project setup and dependency configuration
- [ ] Binance WebSocket client implementation
- [ ] Basic market data structures
- [ ] Database schema and operations
- [ ] Configuration management

### Phase 2: Data Processing Engine (Week 2-3)
- [ ] Orderflow event processing
- [ ] Volume analysis algorithms
- [ ] Order imbalance calculations
- [ ] Data aggregation and storage
- [ ] Historical data management

### Phase 3: GUI Framework (Week 3-4)
- [ ] egui application setup
- [ ] Dark theme implementation
- [ ] Basic panel structure
- [ ] Real-time data binding
- [ ] Multi-threading integration

### Phase 4: Analysis Modules (Week 4-5)
- [ ] Big orderflow detection logic
- [ ] Imbalance tracking algorithms
- [ ] Footprint chart calculations
- [ ] Liquidation event processing
- [ ] Performance optimization

### Phase 5: GUI Implementation (Week 5-6)
- [ ] Orderflow screener panel
- [ ] Imbalance tracker panel
- [ ] Footprint chart visualization
- [ ] Liquidation monitor panel
- [ ] Professional styling and polish

### Phase 6: Testing and Optimization (Week 6-7)
- [ ] Performance testing and optimization
- [ ] Memory usage optimization
- [ ] Stress testing with high-frequency data
- [ ] Error handling and recovery
- [ ] Documentation completion

## Performance Requirements

### System Specifications
- **Memory Usage**: < 1GB RAM under normal operation
- **CPU Usage**: < 30% on modern multi-core systems
- **Latency**: < 100ms data processing pipeline
- **GUI Responsiveness**: 60fps rendering with smooth updates
- **Data Throughput**: Handle 1000+ trades/second across all symbols

### Optimization Strategies
- Efficient data structures (BTreeMap, Vec with pre-allocation)
- Lock-free data structures where possible
- Memory pools for frequent allocations
- Async processing with proper back-pressure handling
- Database connection pooling and batch operations

## Dependencies

```toml
[dependencies]
egui = "0.24"
eframe = "0.24"
tokio = { version = "1.0", features = ["full"] }
tokio-tungstenite = "0.20"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.11", features = ["json"] }
rusqlite = { version = "0.29", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
ordered-float = "3.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Risk Management and Error Handling

### Connection Management
- Automatic WebSocket reconnection with exponential backoff
- Graceful handling of API rate limits
- Connection health monitoring and alerting

### Data Integrity
- Validation of incoming market data
- Duplicate detection and handling
- Data consistency checks across modules

### System Resilience
- Graceful degradation during high load periods
- Memory leak prevention and monitoring
- Crash recovery and state restoration

## Security Considerations

### API Security
- Secure storage of API credentials
- IP address whitelisting where possible
- Rate limiting compliance

### Data Protection
- Local data encryption for sensitive information
- Secure memory handling for credentials
- Regular security audit practices

---

This comprehensive plan provides the foundation for building a professional-grade Binance futures orderflow screener. The modular architecture ensures maintainability and scalability while the concurrent processing design handles real-time market data efficiently.