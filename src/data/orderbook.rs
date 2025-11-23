use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ordered_float::OrderedFloat;
use anyhow::Result;

/// Represents a single price level in the order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthLevel {
    pub price: f64,
    pub quantity: f64,
    pub timestamp: u64,
}

/// Complete order book snapshot for a symbol
#[derive(Debug, Clone)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: BTreeMap<OrderedFloat<f64>, f64>,  // price -> quantity
    pub asks: BTreeMap<OrderedFloat<f64>, f64>,
    pub last_update_id: u64,
    pub timestamp: u64,
}

impl OrderBook {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_update_id: 0,
            timestamp: 0,
        }
    }

    /// Apply depth update from WebSocket
    pub fn apply_update(&mut self, update: DepthUpdate) {
        // Update bids
        for bid in update.bids {
            let price = OrderedFloat(bid.0);
            if bid.1 == 0.0 {
                self.bids.remove(&price);
            } else {
                self.bids.insert(price, bid.1);
            }
        }

        // Update asks
        for ask in update.asks {
            let price = OrderedFloat(ask.0);
            if ask.1 == 0.0 {
                self.asks.remove(&price);
            } else {
                self.asks.insert(price, ask.1);
            }
        }

        self.last_update_id = update.last_update_id;
        self.timestamp = update.event_time;
    }

    /// Get top N levels for heatmap visualization
    pub fn get_depth_snapshot(&self, num_levels: usize) -> DepthSnapshot {
        let bids: Vec<(f64, f64)> = self.bids.iter()
            .rev()
            .take(num_levels)
            .map(|(p, q)| (p.0, *q))
            .collect();

        let asks: Vec<(f64, f64)> = self.asks.iter()
            .take(num_levels)
            .map(|(p, q)| (p.0, *q))
            .collect();

        DepthSnapshot {
            bids,
            asks,
            timestamp: self.timestamp,
        }
    }

    /// Calculate cumulative depth for heatmap intensity
    pub fn get_cumulative_depth(&self, price_levels: &[i64], tick_size: f64) -> Vec<(i64, f64, f64)> {
        let mut result = Vec::new();

        for &price_tick in price_levels {
            let price = price_tick as f64 * tick_size;
            let bid_depth = self.get_bid_depth_at_price(price);
            let ask_depth = self.get_ask_depth_at_price(price);
            result.push((price_tick, bid_depth, ask_depth));
        }

        result
    }

    fn get_bid_depth_at_price(&self, price: f64) -> f64 {
        self.bids.iter()
            .filter(|(p, _)| p.0 <= price)
            .map(|(_, q)| q)
            .sum()
    }

    fn get_ask_depth_at_price(&self, price: f64) -> f64 {
        self.asks.iter()
            .filter(|(p, _)| p.0 >= price)
            .map(|(_, q)| q)
            .sum()
    }

    /// Get aggregated depth at specific price tick
    pub fn get_aggregated_depth_at_tick(&self, price_tick: i64, tick_size: f64, levels: usize) -> (f64, f64) {
        let center_price = price_tick as f64 * tick_size;
        let tick_range = tick_size / 2.0;

        let bid_qty: f64 = self.bids.iter()
            .filter(|(p, _)| (p.0 - center_price).abs() <= tick_range)
            .map(|(_, q)| q)
            .sum();

        let ask_qty: f64 = self.asks.iter()
            .filter(|(p, _)| (p.0 - center_price).abs() <= tick_range)
            .map(|(_, q)| q)
            .sum();

        (bid_qty, ask_qty)
    }
}

/// Incremental depth update from WebSocket
#[derive(Debug, Clone, Deserialize)]
pub struct DepthUpdate {
    #[serde(rename = "e")]
    pub event_type: String,  // "depthUpdate"

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "U")]
    pub first_update_id: u64,

    #[serde(rename = "u")]
    pub last_update_id: u64,

    #[serde(rename = "b")]
    #[serde(deserialize_with = "deserialize_levels")]
    pub bids: Vec<(f64, f64)>,

    #[serde(rename = "a")]
    #[serde(deserialize_with = "deserialize_levels")]
    pub asks: Vec<(f64, f64)>,
}

/// Deserialize price levels from string arrays to f64 tuples
fn deserialize_levels<'de, D>(deserializer: D) -> Result<Vec<(f64, f64)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let levels: Vec<Vec<String>> = Vec::deserialize(deserializer)?;

    levels.iter()
        .map(|level| {
            if level.len() < 2 {
                return Err(D::Error::custom("Invalid level format"));
            }
            let price = level[0].parse::<f64>()
                .map_err(|e| D::Error::custom(format!("Failed to parse price: {}", e)))?;
            let quantity = level[1].parse::<f64>()
                .map_err(|e| D::Error::custom(format!("Failed to parse quantity: {}", e)))?;
            Ok((price, quantity))
        })
        .collect()
}

/// Snapshot for heatmap rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthSnapshot {
    pub bids: Vec<(f64, f64)>,  // (price, quantity)
    pub asks: Vec<(f64, f64)>,
    pub timestamp: u64,
}

/// Historical depth data for heatmap background
#[derive(Debug, Clone)]
pub struct DepthHistory {
    pub symbol: String,
    pub snapshots: Vec<TimedDepthSnapshot>,
    pub max_history: usize,  // Number of snapshots to keep
}

#[derive(Debug, Clone)]
pub struct TimedDepthSnapshot {
    pub timestamp: u64,
    pub depth: DepthSnapshot,
    pub aggregated_bids: BTreeMap<i64, f64>,  // price_tick -> cumulative quantity
    pub aggregated_asks: BTreeMap<i64, f64>,
}

impl DepthHistory {
    pub fn new(symbol: String, max_history: usize) -> Self {
        Self {
            symbol,
            snapshots: Vec::with_capacity(max_history),
            max_history,
        }
    }

    pub fn add_snapshot(&mut self, snapshot: TimedDepthSnapshot) {
        self.snapshots.push(snapshot);
        if self.snapshots.len() > self.max_history {
            self.snapshots.remove(0);
        }
    }

    /// Get depth intensity at specific time and price for heatmap rendering
    pub fn get_intensity_at(&self, timestamp: u64, price_tick: i64) -> Option<(f64, f64)> {
        // Find closest snapshot
        let snapshot = self.snapshots.iter()
            .min_by_key(|s| (s.timestamp as i64 - timestamp as i64).abs())?;

        let bid_qty = snapshot.aggregated_bids.get(&price_tick).copied().unwrap_or(0.0);
        let ask_qty = snapshot.aggregated_asks.get(&price_tick).copied().unwrap_or(0.0);

        Some((bid_qty, ask_qty))
    }

    /// Get all snapshots within a time range
    pub fn get_snapshots_in_range(&self, start_time: u64, end_time: u64) -> Vec<&TimedDepthSnapshot> {
        self.snapshots.iter()
            .filter(|s| s.timestamp >= start_time && s.timestamp <= end_time)
            .collect()
    }
}

impl TimedDepthSnapshot {
    pub fn new(timestamp: u64, depth: DepthSnapshot, tick_size: f64) -> Self {
        let mut aggregated_bids = BTreeMap::new();
        let mut aggregated_asks = BTreeMap::new();

        // Aggregate bids to price ticks
        for (price, qty) in &depth.bids {
            let price_tick = (*price / tick_size).round() as i64;
            *aggregated_bids.entry(price_tick).or_insert(0.0) += qty;
        }

        // Aggregate asks to price ticks
        for (price, qty) in &depth.asks {
            let price_tick = (*price / tick_size).round() as i64;
            *aggregated_asks.entry(price_tick).or_insert(0.0) += qty;
        }

        Self {
            timestamp,
            depth,
            aggregated_bids,
            aggregated_asks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_creation() {
        let ob = OrderBook::new("BTCUSDT".to_string());
        assert_eq!(ob.symbol, "BTCUSDT");
        assert_eq!(ob.bids.len(), 0);
        assert_eq!(ob.asks.len(), 0);
    }

    #[test]
    fn test_depth_snapshot() {
        let mut ob = OrderBook::new("BTCUSDT".to_string());
        ob.bids.insert(OrderedFloat(50000.0), 1.0);
        ob.bids.insert(OrderedFloat(49999.0), 2.0);
        ob.asks.insert(OrderedFloat(50001.0), 1.5);
        ob.asks.insert(OrderedFloat(50002.0), 2.5);

        let snapshot = ob.get_depth_snapshot(10);
        assert_eq!(snapshot.bids.len(), 2);
        assert_eq!(snapshot.asks.len(), 2);
        assert_eq!(snapshot.bids[0].0, 50000.0);  // Highest bid first
    }

    #[test]
    fn test_cumulative_depth() {
        let mut ob = OrderBook::new("BTCUSDT".to_string());
        ob.bids.insert(OrderedFloat(50000.0), 1.0);
        ob.bids.insert(OrderedFloat(49999.0), 2.0);
        ob.bids.insert(OrderedFloat(49998.0), 3.0);

        let cumulative = ob.get_bid_depth_at_price(50000.0);
        assert_eq!(cumulative, 6.0);  // Sum of all bids <= 50000
    }
}
