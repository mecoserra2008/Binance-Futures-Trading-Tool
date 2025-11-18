use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ordered_float::OrderedFloat;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderflowEvent {
    pub symbol: String,
    pub timestamp: u64,
    pub price: f64,
    pub quantity: f64,
    pub is_buyer_maker: bool,
    pub trade_id: u64,
}

#[derive(Debug, Clone)]
pub struct VolumeAtPrice {
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub total_volume: f64,
    pub trade_count: u32,
}

#[derive(Debug, Clone)]
pub struct VolumeProfile {
    pub symbol: String,
    pub timestamp: u64,
    pub timeframe: String,
    pub price_levels: BTreeMap<OrderedFloat<f64>, VolumeAtPrice>,
    pub total_volume: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub vwap: f64,
    pub poc: f64, // Point of Control (price with highest volume)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderImbalance {
    pub symbol: String,
    pub timestamp: u64,
    pub bid_volume: f64,
    pub ask_volume: f64,
    pub imbalance_ratio: f64,
    pub window_duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationEvent {
    pub symbol: String,
    pub timestamp: u64,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub is_forced: bool,
    pub notional_value: f64,
}

#[derive(Debug, Clone)]
pub struct Candle {
    pub symbol: String,
    pub timestamp: u64,
    pub timeframe: String,
    pub open_price: f64,
    pub high_price: f64,
    pub low_price: f64,
    pub close_price: f64,
    pub volume: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub trade_count: u32,
}

#[derive(Debug, Clone)]
pub struct DailyStats {
    pub symbol: String,
    pub date: String,
    pub avg_volume: f64,
    pub total_volume: f64,
    pub avg_price: f64,
    pub high_price: f64,
    pub low_price: f64,
    pub trade_count: u64,
}

#[derive(Debug, Clone)]
pub struct BigOrderflowAlert {
    pub symbol: String,
    pub timestamp: u64,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub percentage_of_daily: f64,
    pub notional_value: f64,
}

#[derive(Debug, Clone)]
pub enum GuiUpdate {
    BigOrderflow(BigOrderflowAlert),
    Imbalance(OrderImbalance),
    Liquidation(LiquidationEvent),
    VolumeProfile(VolumeProfile),
    DailyStats(DailyStats),
}

impl VolumeProfile {
    pub fn new(symbol: String, timestamp: u64, timeframe: String) -> Self {
        Self {
            symbol,
            timestamp,
            timeframe,
            price_levels: BTreeMap::new(),
            total_volume: 0.0,
            buy_volume: 0.0,
            sell_volume: 0.0,
            vwap: 0.0,
            poc: 0.0,
        }
    }

    pub fn add_trade(&mut self, price: f64, quantity: f64, is_buy: bool) {
        let price_key = OrderedFloat(price);
        let entry = self.price_levels.entry(price_key).or_insert(VolumeAtPrice {
            buy_volume: 0.0,
            sell_volume: 0.0,
            total_volume: 0.0,
            trade_count: 0,
        });

        if is_buy {
            entry.buy_volume += quantity;
            self.buy_volume += quantity;
        } else {
            entry.sell_volume += quantity;
            self.sell_volume += quantity;
        }

        entry.total_volume += quantity;
        entry.trade_count += 1;
        self.total_volume += quantity;
    }

    pub fn calculate_metrics(&mut self) {
        if self.total_volume == 0.0 {
            return;
        }

        // Calculate VWAP
        let mut volume_price_sum = 0.0;
        for (price, volume_data) in &self.price_levels {
            volume_price_sum += price.0 * volume_data.total_volume;
        }
        self.vwap = volume_price_sum / self.total_volume;

        // Find Point of Control (highest volume price level)
        if let Some((poc_price, _)) = self.price_levels
            .iter()
            .max_by(|a, b| a.1.total_volume.partial_cmp(&b.1.total_volume).unwrap()) {
            self.poc = poc_price.0;
        }
    }
}

impl OrderImbalance {
    pub fn calculate_ratio(bid_volume: f64, ask_volume: f64) -> f64 {
        if bid_volume + ask_volume == 0.0 {
            0.0
        } else {
            (bid_volume - ask_volume) / (bid_volume + ask_volume)
        }
    }

    pub fn is_significant(&self, threshold: f64) -> bool {
        self.imbalance_ratio.abs() > threshold
    }
}

impl LiquidationEvent {
    pub fn from_trade(
        symbol: String,
        timestamp: u64,
        price: f64,
        quantity: f64,
        is_buyer_maker: bool,
        is_liquidation: bool,
    ) -> Option<Self> {
        if !is_liquidation {
            return None;
        }

        Some(Self {
            symbol,
            timestamp,
            side: if is_buyer_maker { "SELL".to_string() } else { "BUY".to_string() },
            price,
            quantity,
            is_forced: true,
            notional_value: price * quantity,
        })
    }
}

// Additional futures market data structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInterestData {
    pub symbol: String,
    pub timestamp: u64,
    pub open_interest: f64,        // Total open contracts
    pub oi_change_24h: f64,        // 24h change in OI
    pub oi_change_percentage: f64, // % change in OI
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRateData {
    pub symbol: String,
    pub timestamp: u64,
    pub funding_rate: f64,         // Current funding rate
    pub predicted_rate: f64,       // Next funding rate prediction
    pub funding_interval: u64,     // Time to next funding (seconds)
    pub mark_price: f64,           // Mark price used for funding
    pub index_price: f64,          // Index price
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationSummary {
    pub symbol: String,
    pub timestamp: u64,
    pub total_liquidations_24h: f64,     // 24h liquidation volume
    pub long_liquidations: f64,          // Long position liquidations
    pub short_liquidations: f64,         // Short position liquidations
    pub largest_liquidation: f64,        // Largest single liquidation
    pub liquidation_count: u32,          // Number of liquidations
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMetrics {
    pub symbol: String,
    pub timestamp: u64,
    pub volume_24h: f64,              // 24h volume
    pub volume_7d_avg: f64,           // 7-day average volume
    pub volume_percentile: f64,       // Current volume vs historical (0-100)
    pub buy_sell_ratio: f64,          // Buy/sell volume ratio
    pub volume_spike_factor: f64,     // Current volume / average volume
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceMovementData {
    pub symbol: String,
    pub timestamp: u64,
    pub price_change_1h: f64,         // 1h price change %
    pub price_change_4h: f64,         // 4h price change %
    pub price_change_24h: f64,        // 24h price change %
    pub volatility_index: f64,        // Price volatility measure
    pub price_impact_score: f64,      // Recent price impact from large orders
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDepthData {
    pub symbol: String,
    pub timestamp: u64,
    pub bid_levels: Vec<PriceLevel>,  // Top bid levels
    pub ask_levels: Vec<PriceLevel>,  // Top ask levels
    pub spread: f64,                  // Bid-ask spread
    pub depth_imbalance: f64,         // Bid/ask volume imbalance
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone)]
pub enum ScreenerAlert {
    LargeOrder(BigOrderflowAlert),
    MassLiquidation {
        symbol: String,
        total_size: f64,
        duration_seconds: u64,
        price_impact: f64,
        liquidation_count: u32,
    },
    OpenInterestSpike {
        symbol: String,
        oi_change: f64,
        oi_change_percentage: f64,
        timeframe: String,
    },
    FundingExtreme {
        symbol: String,
        funding_rate: f64,
        deviation_from_norm: f64,
        predicted_rate: f64,
    },
    VolumeSpike {
        symbol: String,
        volume_spike_factor: f64,
        volume_24h: f64,
        timeframe: String,
    },
    PriceImpact {
        symbol: String,
        price_change: f64,
        volume_causing_impact: f64,
        timeframe_seconds: u64,
    },
}

// Enhanced GuiUpdate enum to include new data types
#[derive(Debug, Clone)]
pub enum EnhancedGuiUpdate {
    // Original updates
    BigOrderflow(BigOrderflowAlert),
    Imbalance(OrderImbalance),
    Liquidation(LiquidationEvent),
    VolumeProfile(VolumeProfile),
    DailyStats(DailyStats),

    // New futures-specific updates
    OpenInterest(OpenInterestData),
    FundingRate(FundingRateData),
    LiquidationSummary(LiquidationSummary),
    VolumeMetrics(VolumeMetrics),
    PriceMovement(PriceMovementData),
    MarketDepth(MarketDepthData),
    ScreenerAlert(ScreenerAlert),
}

impl OpenInterestData {
    pub fn new(symbol: String, timestamp: u64, open_interest: f64) -> Self {
        Self {
            symbol,
            timestamp,
            open_interest,
            oi_change_24h: 0.0,
            oi_change_percentage: 0.0,
        }
    }

    pub fn calculate_change(&mut self, previous_oi: f64) {
        self.oi_change_24h = self.open_interest - previous_oi;
        if previous_oi > 0.0 {
            self.oi_change_percentage = (self.oi_change_24h / previous_oi) * 100.0;
        }
    }

    pub fn is_significant_change(&self, threshold_percentage: f64) -> bool {
        self.oi_change_percentage.abs() > threshold_percentage
    }
}

impl FundingRateData {
    pub fn new(symbol: String, timestamp: u64, funding_rate: f64) -> Self {
        Self {
            symbol,
            timestamp,
            funding_rate,
            predicted_rate: funding_rate, // Default to current rate
            funding_interval: 8 * 3600,   // 8 hours default
            mark_price: 0.0,
            index_price: 0.0,
        }
    }

    pub fn is_extreme(&self, threshold: f64) -> bool {
        self.funding_rate.abs() > threshold
    }

    pub fn funding_cost_annual(&self) -> f64 {
        // Approximate annual funding cost as percentage
        self.funding_rate * 365.0 * 3.0 * 100.0 // 3 times per day
    }
}

impl LiquidationSummary {
    pub fn new(symbol: String, timestamp: u64) -> Self {
        Self {
            symbol,
            timestamp,
            total_liquidations_24h: 0.0,
            long_liquidations: 0.0,
            short_liquidations: 0.0,
            largest_liquidation: 0.0,
            liquidation_count: 0,
        }
    }

    pub fn add_liquidation(&mut self, liquidation: &LiquidationEvent) {
        self.total_liquidations_24h += liquidation.notional_value;
        self.liquidation_count += 1;

        if liquidation.side == "BUY" {
            // Liquidated short position
            self.short_liquidations += liquidation.notional_value;
        } else {
            // Liquidated long position
            self.long_liquidations += liquidation.notional_value;
        }

        if liquidation.notional_value > self.largest_liquidation {
            self.largest_liquidation = liquidation.notional_value;
        }
    }

    pub fn liquidation_ratio(&self) -> f64 {
        if self.short_liquidations + self.long_liquidations == 0.0 {
            0.0
        } else {
            self.long_liquidations / (self.long_liquidations + self.short_liquidations)
        }
    }

    pub fn is_liquidation_cascade(&self, threshold_count: u32, threshold_volume: f64) -> bool {
        self.liquidation_count >= threshold_count && self.total_liquidations_24h >= threshold_volume
    }
}

impl VolumeMetrics {
    pub fn new(symbol: String, timestamp: u64, volume_24h: f64) -> Self {
        Self {
            symbol,
            timestamp,
            volume_24h,
            volume_7d_avg: volume_24h, // Default to current volume
            volume_percentile: 50.0,   // Default to median
            buy_sell_ratio: 1.0,       // Default to balanced
            volume_spike_factor: 1.0,  // Default to normal
        }
    }

    pub fn calculate_spike_factor(&mut self) {
        if self.volume_7d_avg > 0.0 {
            self.volume_spike_factor = self.volume_24h / self.volume_7d_avg;
        }
    }

    pub fn is_volume_spike(&self, spike_threshold: f64) -> bool {
        self.volume_spike_factor > spike_threshold
    }

    pub fn volume_category(&self) -> String {
        match self.volume_percentile {
            p if p >= 95.0 => "Extreme High".to_string(),
            p if p >= 80.0 => "High".to_string(),
            p if p >= 60.0 => "Above Average".to_string(),
            p if p >= 40.0 => "Average".to_string(),
            p if p >= 20.0 => "Below Average".to_string(),
            _ => "Low".to_string(),
        }
    }
}

impl MarketDepthData {
    pub fn new(symbol: String, timestamp: u64) -> Self {
        Self {
            symbol,
            timestamp,
            bid_levels: Vec::new(),
            ask_levels: Vec::new(),
            spread: 0.0,
            depth_imbalance: 0.0,
        }
    }

    pub fn calculate_spread(&mut self) {
        if let (Some(best_bid), Some(best_ask)) = (self.bid_levels.first(), self.ask_levels.first()) {
            self.spread = best_ask.price - best_bid.price;
        }
    }

    pub fn calculate_depth_imbalance(&mut self, depth_levels: usize) {
        let bid_volume: f64 = self.bid_levels.iter().take(depth_levels).map(|level| level.quantity).sum();
        let ask_volume: f64 = self.ask_levels.iter().take(depth_levels).map(|level| level.quantity).sum();

        if bid_volume + ask_volume > 0.0 {
            self.depth_imbalance = (bid_volume - ask_volume) / (bid_volume + ask_volume);
        }
    }

    pub fn total_bid_volume(&self, levels: usize) -> f64 {
        self.bid_levels.iter().take(levels).map(|level| level.quantity).sum()
    }

    pub fn total_ask_volume(&self, levels: usize) -> f64 {
        self.ask_levels.iter().take(levels).map(|level| level.quantity).sum()
    }
}