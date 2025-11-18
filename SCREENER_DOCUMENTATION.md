# Binance Futures Screener Documentation

## Overview
The Binance Futures Screener is designed to monitor large orderflow events, market imbalances, and liquidations in real-time. Currently, the screener shows no data because the data pipeline components are not fully connected.

## Current Architecture

### 1. Screener Panel Structure (`src/gui/screener_panel.rs`)

#### Purpose
The screener panel displays large orderflow alerts that exceed certain thresholds as a percentage of daily volume.

#### Current Implementation
- **Data Type**: `BigOrderflowAlert` events
- **Display**: Sortable table with filtering capabilities
- **Columns**:
  - Time (timestamp of the trade)
  - Symbol (trading pair)
  - Side (BUY/SELL)
  - Size (quantity traded)
  - Price (execution price)
  - % Daily (percentage of daily volume)
  - Notional (USD value)
  - Volume Bar (visual representation)

#### Filtering Options
- Text filter (symbol/side)
- Minimum percentage threshold
- Minimum notional value
- Buy-only/Sell-only toggles

#### Current Issues
- **No data flowing**: The screener receives no `BigOrderflowAlert` events
- **Missing data pipeline**: Orderflow events are not being processed into alerts
- **No volume analysis**: Daily volume statistics are not being calculated

### 2. Data Structures

#### BigOrderflowAlert
```rust
pub struct BigOrderflowAlert {
    pub symbol: String,
    pub timestamp: u64,
    pub side: String,           // "BUY" or "SELL"
    pub price: f64,
    pub quantity: f64,
    pub percentage_of_daily: f64,  // Key metric for screening
    pub notional_value: f64,
}
```

#### OrderflowEvent (Raw Input)
```rust
pub struct OrderflowEvent {
    pub symbol: String,
    pub timestamp: u64,
    pub price: f64,
    pub quantity: f64,
    pub is_buyer_maker: bool,
    pub trade_id: u64,
}
```

### 3. Missing Components for Data Flow

#### Volume Analysis Pipeline
- **Location**: `src/analysis/volume_analysis.rs` (exists but not connected)
- **Purpose**: Process raw orderflow events and generate alerts
- **Required**: Daily volume tracking to calculate percentage thresholds

#### Data Processing Chain
1. **WebSocket Connection** → Raw market data
2. **OrderflowEvent Parser** → Structured trade data
3. **Volume Analysis** → Daily statistics + Alert generation
4. **GUI Updates** → Screen display

## Required Data Points for Futures Screener

### Core Trading Data
- [x] **Orderflow Events**: Individual trades with price, size, timestamp
- [x] **Liquidation Events**: Forced liquidations with size and direction
- [x] **Order Imbalances**: Bid/ask volume imbalances
- [x] **Volume Profiles**: Price-level volume distribution

### Missing Critical Data Points

#### 1. Open Interest Data
```rust
pub struct OpenInterestData {
    pub symbol: String,
    pub timestamp: u64,
    pub open_interest: f64,        // Total open contracts
    pub oi_change_24h: f64,        // 24h change in OI
    pub oi_change_percentage: f64, // % change in OI
}
```

#### 2. Funding Rate Data
```rust
pub struct FundingRateData {
    pub symbol: String,
    pub timestamp: u64,
    pub funding_rate: f64,         // Current funding rate
    pub predicted_rate: f64,       // Next funding rate prediction
    pub funding_interval: u64,     // Time to next funding (seconds)
}
```

#### 3. Liquidation Statistics
```rust
pub struct LiquidationSummary {
    pub symbol: String,
    pub timestamp: u64,
    pub total_liquidations_24h: f64,     // 24h liquidation volume
    pub long_liquidations: f64,          // Long position liquidations
    pub short_liquidations: f64,         // Short position liquidations
    pub largest_liquidation: f64,        // Largest single liquidation
}
```

#### 4. Enhanced Volume Metrics
```rust
pub struct VolumeMetrics {
    pub symbol: String,
    pub timestamp: u64,
    pub volume_24h: f64,              // 24h volume
    pub volume_7d_avg: f64,           // 7-day average volume
    pub volume_percentile: f64,       // Current volume vs historical
    pub buy_sell_ratio: f64,          // Buy/sell volume ratio
}
```

#### 5. Price Movement Correlation
```rust
pub struct PriceMovementData {
    pub symbol: String,
    pub timestamp: u64,
    pub price_change_1h: f64,         // 1h price change %
    pub price_change_4h: f64,         // 4h price change %
    pub price_change_24h: f64,        // 24h price change %
    pub volatility_index: f64,        // Price volatility measure
}
```

## Recommended Screener Enhancements

### 1. Multi-Tab Screener Interface
- **Large Orders Tab**: Current BigOrderflowAlert functionality
- **Liquidations Tab**: Real-time liquidation events
- **Open Interest Tab**: OI changes and correlations
- **Funding Tab**: Funding rate extremes and predictions

### 2. Additional Screening Criteria

#### Large Order Screening
- Minimum order size (absolute and % of daily volume)
- Price impact threshold
- Unusual volume patterns
- Cross-exchange arbitrage opportunities

#### Liquidation Screening
- Liquidation cascades (multiple liquidations in sequence)
- Large liquidation events (>$1M, >$10M thresholds)
- Liquidation clusters by price level
- Long vs short liquidation ratios

#### Open Interest Screening
- Rapid OI increases (potential setup for big moves)
- OI divergence from price (warning signals)
- High OI at specific price levels (support/resistance)

#### Funding Rate Screening
- Extreme funding rates (>0.1%, <-0.1%)
- Funding rate divergence from other exchanges
- Predicted funding rate changes

### 3. Alert System
```rust
pub enum ScreenerAlert {
    LargeOrder(BigOrderflowAlert),
    MassLiquidation {
        symbol: String,
        total_size: f64,
        duration_seconds: u64,
        price_impact: f64,
    },
    OpenInterestSpike {
        symbol: String,
        oi_change: f64,
        timeframe: String,
    },
    FundingExtreme {
        symbol: String,
        funding_rate: f64,
        deviation_from_norm: f64,
    },
}
```

## Implementation Priority

### Phase 1: Fix Current Screener
1. **Connect Volume Analysis**: Wire up `VolumeAnalyzer` to process orderflow events
2. **Daily Volume Tracking**: Implement rolling 24h volume statistics
3. **Alert Generation**: Generate `BigOrderflowAlert` events from large trades
4. **GUI Integration**: Ensure alerts flow to the screener panel

### Phase 2: Add Missing Data Sources
1. **Open Interest API**: Integrate Binance open interest endpoints
2. **Funding Rate API**: Add funding rate data collection
3. **Enhanced Liquidation Tracking**: Improve liquidation event processing

### Phase 3: Enhanced Screening
1. **Multi-tab Interface**: Expand screener to multiple data types
2. **Advanced Filters**: Add more sophisticated screening criteria
3. **Alert Notifications**: System notifications for extreme events
4. **Historical Analysis**: Track patterns and generate insights

## API Endpoints Needed

### Binance Futures API Endpoints
```
# Current Price & Volume
GET /fapi/v1/ticker/24hr

# Open Interest
GET /fapi/v1/openInterest
GET /fapi/v1/openInterestHist

# Funding Rate
GET /fapi/v1/fundingRate
GET /fapi/v1/premiumIndex

# Force Orders (Liquidations)
WebSocket: liquidationOrder@symbol

# Aggregate Trade (Current)
WebSocket: aggTrade@symbol

# Market Data
GET /fapi/v1/klines
```

## Configuration Options

### Screening Thresholds
```toml
[screener]
min_order_percentage = 0.5      # Minimum % of daily volume
min_notional_value = 100000     # Minimum $100k orders
liquidation_cascade_threshold = 5 # 5+ liquidations in 60 seconds

[open_interest]
oi_spike_threshold = 10.0       # 10% OI increase threshold
oi_timeframe = "1h"             # OI change timeframe

[funding]
extreme_funding_threshold = 0.1  # 0.1% funding rate threshold
```

This documentation provides a comprehensive overview of the current state and required improvements for the Binance Futures Screener.