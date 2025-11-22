use std::collections::HashMap;
use crate::data::orderflow::OrderflowEvent;

/// Tracks traded volume at each price level
#[derive(Debug, Clone, Default)]
pub struct VolumeAtPrice {
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub total_volume: f64,
    pub trade_count: u64,
    pub last_trade_time: u64,
}

impl VolumeAtPrice {
    pub fn delta(&self) -> f64 {
        self.buy_volume - self.sell_volume
    }

    pub fn imbalance_ratio(&self) -> f64 {
        if self.total_volume == 0.0 {
            return 0.0;
        }
        (self.buy_volume - self.sell_volume) / self.total_volume
    }
}

/// Tracks historical traded volume for a symbol
pub struct TradedVolumeTracker {
    symbol: String,

    // price_tick -> VolumeAtPrice
    volume_at_price: HashMap<i64, VolumeAtPrice>,

    tick_size: f64,
    max_price_levels: usize,
}

impl TradedVolumeTracker {
    pub fn new(symbol: String, tick_size: f64) -> Self {
        Self {
            symbol,
            volume_at_price: HashMap::new(),
            tick_size,
            max_price_levels: 10000, // Keep last 10k price levels
        }
    }

    pub fn with_max_levels(symbol: String, tick_size: f64, max_levels: usize) -> Self {
        Self {
            symbol,
            volume_at_price: HashMap::new(),
            tick_size,
            max_price_levels: max_levels,
        }
    }

    /// Process a trade event
    pub fn process_trade(&mut self, event: &OrderflowEvent) {
        let price_tick = (event.price / self.tick_size).round() as i64;

        let entry = self.volume_at_price
            .entry(price_tick)
            .or_insert_with(VolumeAtPrice::default);

        entry.total_volume += event.quantity;
        entry.trade_count += 1;
        entry.last_trade_time = event.timestamp;

        if event.is_buyer_maker {
            // Buyer is maker = sell order hit buyer's bid = sell volume
            entry.sell_volume += event.quantity;
        } else {
            // Seller is maker = buy order hit seller's ask = buy volume
            entry.buy_volume += event.quantity;
        }

        // Manage memory if too many price levels
        if self.volume_at_price.len() > self.max_price_levels {
            self.cleanup_old_levels();
        }
    }

    /// Get volume at a specific price tick
    pub fn get_volume_at_tick(&self, price_tick: i64) -> Option<&VolumeAtPrice> {
        self.volume_at_price.get(&price_tick)
    }

    /// Get volume at a specific price (will be converted to tick)
    pub fn get_volume_at_price(&self, price: f64) -> Option<&VolumeAtPrice> {
        let price_tick = (price / self.tick_size).round() as i64;
        self.get_volume_at_tick(price_tick)
    }

    /// Get all price levels with volume
    pub fn get_all_levels(&self) -> Vec<(i64, &VolumeAtPrice)> {
        let mut levels: Vec<_> = self.volume_at_price.iter()
            .map(|(tick, vol)| (*tick, vol))
            .collect();
        levels.sort_by_key(|(tick, _)| *tick);
        levels
    }

    /// Get price levels within a range
    pub fn get_levels_in_range(&self, min_price: f64, max_price: f64) -> Vec<(i64, &VolumeAtPrice)> {
        let min_tick = (min_price / self.tick_size).round() as i64;
        let max_tick = (max_price / self.tick_size).round() as i64;

        let mut levels: Vec<_> = self.volume_at_price.iter()
            .filter(|(tick, _)| **tick >= min_tick && **tick <= max_tick)
            .map(|(tick, vol)| (*tick, vol))
            .collect();
        levels.sort_by_key(|(tick, _)| *tick);
        levels
    }

    /// Get total volume across all price levels
    pub fn get_total_volume(&self) -> f64 {
        self.volume_at_price.values()
            .map(|v| v.total_volume)
            .sum()
    }

    /// Get total buy volume
    pub fn get_total_buy_volume(&self) -> f64 {
        self.volume_at_price.values()
            .map(|v| v.buy_volume)
            .sum()
    }

    /// Get total sell volume
    pub fn get_total_sell_volume(&self) -> f64 {
        self.volume_at_price.values()
            .map(|v| v.sell_volume)
            .sum()
    }

    /// Get total delta
    pub fn get_total_delta(&self) -> f64 {
        self.get_total_buy_volume() - self.get_total_sell_volume()
    }

    /// Get price level with highest volume (POC - Point of Control)
    pub fn get_poc(&self) -> Option<(i64, &VolumeAtPrice)> {
        self.volume_at_price.iter()
            .max_by(|a, b| a.1.total_volume.partial_cmp(&b.1.total_volume).unwrap())
            .map(|(tick, vol)| (*tick, vol))
    }

    /// Get top N price levels by volume
    pub fn get_top_levels(&self, n: usize) -> Vec<(i64, &VolumeAtPrice)> {
        let mut levels: Vec<_> = self.volume_at_price.iter()
            .map(|(tick, vol)| (*tick, vol))
            .collect();
        levels.sort_by(|a, b| b.1.total_volume.partial_cmp(&a.1.total_volume).unwrap());
        levels.truncate(n);
        levels
    }

    /// Get levels with significant imbalance (above threshold)
    pub fn get_imbalanced_levels(&self, threshold: f64) -> Vec<(i64, &VolumeAtPrice)> {
        let mut levels: Vec<_> = self.volume_at_price.iter()
            .filter(|(_, vol)| vol.imbalance_ratio().abs() > threshold)
            .map(|(tick, vol)| (*tick, vol))
            .collect();
        levels.sort_by_key(|(tick, _)| *tick);
        levels
    }

    /// Clear old price levels to manage memory
    fn cleanup_old_levels(&mut self) {
        if self.volume_at_price.len() <= self.max_price_levels {
            return;
        }

        // Keep only the most recent or most voluminous levels
        let mut levels: Vec<_> = self.volume_at_price.iter()
            .map(|(tick, vol)| (*tick, vol.clone()))
            .collect();

        // Sort by last trade time (descending)
        levels.sort_by(|a, b| b.1.last_trade_time.cmp(&a.1.last_trade_time));

        // Keep only max_price_levels
        levels.truncate(self.max_price_levels);

        // Rebuild HashMap
        self.volume_at_price = levels.into_iter().collect();
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.volume_at_price.clear();
    }

    /// Get statistics
    pub fn get_stats(&self) -> VolumeStats {
        VolumeStats {
            symbol: self.symbol.clone(),
            total_volume: self.get_total_volume(),
            buy_volume: self.get_total_buy_volume(),
            sell_volume: self.get_total_sell_volume(),
            delta: self.get_total_delta(),
            price_levels: self.volume_at_price.len(),
            poc: self.get_poc().map(|(tick, _)| tick as f64 * self.tick_size),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VolumeStats {
    pub symbol: String,
    pub total_volume: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub delta: f64,
    pub price_levels: usize,
    pub poc: Option<f64>,
}

/// Manages traded volume tracking for multiple symbols
pub struct MultiSymbolVolumeTracker {
    trackers: HashMap<String, TradedVolumeTracker>,
    default_tick_size: f64,
}

impl MultiSymbolVolumeTracker {
    pub fn new(default_tick_size: f64) -> Self {
        Self {
            trackers: HashMap::new(),
            default_tick_size,
        }
    }

    /// Get or create tracker for a symbol
    pub fn get_or_create_tracker(&mut self, symbol: &str) -> &mut TradedVolumeTracker {
        self.trackers.entry(symbol.to_string())
            .or_insert_with(|| TradedVolumeTracker::new(symbol.to_string(), self.default_tick_size))
    }

    /// Process trade for a symbol
    pub fn process_trade(&mut self, event: &OrderflowEvent) {
        let tracker = self.get_or_create_tracker(&event.symbol);
        tracker.process_trade(event);
    }

    /// Get tracker for a symbol
    pub fn get_tracker(&self, symbol: &str) -> Option<&TradedVolumeTracker> {
        self.trackers.get(symbol)
    }

    /// Get mutable tracker for a symbol
    pub fn get_tracker_mut(&mut self, symbol: &str) -> Option<&mut TradedVolumeTracker> {
        self.trackers.get_mut(symbol)
    }

    /// Get stats for all symbols
    pub fn get_all_stats(&self) -> Vec<VolumeStats> {
        self.trackers.values()
            .map(|tracker| tracker.get_stats())
            .collect()
    }

    /// Clear all trackers
    pub fn clear_all(&mut self) {
        self.trackers.clear();
    }

    /// Get symbol count
    pub fn symbol_count(&self) -> usize {
        self.trackers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_event(symbol: &str, price: f64, quantity: f64, is_buyer_maker: bool, timestamp: u64) -> OrderflowEvent {
        OrderflowEvent {
            symbol: symbol.to_string(),
            timestamp,
            price,
            quantity,
            is_buyer_maker,
            trade_id: 0,
        }
    }

    #[test]
    fn test_volume_tracker_basic() {
        let mut tracker = TradedVolumeTracker::new("BTCUSDT".to_string(), 1.0);

        let event1 = create_test_event("BTCUSDT", 50000.0, 1.0, false, 1000);
        tracker.process_trade(&event1);

        assert_eq!(tracker.get_total_volume(), 1.0);
        assert_eq!(tracker.get_total_buy_volume(), 1.0);
        assert_eq!(tracker.get_total_sell_volume(), 0.0);
    }

    #[test]
    fn test_volume_at_price() {
        let mut tracker = TradedVolumeTracker::new("BTCUSDT".to_string(), 1.0);

        let event1 = create_test_event("BTCUSDT", 50000.0, 1.0, false, 1000);
        let event2 = create_test_event("BTCUSDT", 50000.0, 2.0, true, 1001);

        tracker.process_trade(&event1);
        tracker.process_trade(&event2);

        let vol = tracker.get_volume_at_price(50000.0).unwrap();
        assert_eq!(vol.total_volume, 3.0);
        assert_eq!(vol.buy_volume, 1.0);
        assert_eq!(vol.sell_volume, 2.0);
        assert_eq!(vol.delta(), -1.0);
    }

    #[test]
    fn test_poc() {
        let mut tracker = TradedVolumeTracker::new("BTCUSDT".to_string(), 1.0);

        tracker.process_trade(&create_test_event("BTCUSDT", 50000.0, 1.0, false, 1000));
        tracker.process_trade(&create_test_event("BTCUSDT", 50001.0, 5.0, false, 1001));
        tracker.process_trade(&create_test_event("BTCUSDT", 50002.0, 2.0, false, 1002));

        let (poc_tick, poc_vol) = tracker.get_poc().unwrap();
        assert_eq!(poc_tick, 50001); // Price with highest volume
        assert_eq!(poc_vol.total_volume, 5.0);
    }

    #[test]
    fn test_imbalanced_levels() {
        let mut tracker = TradedVolumeTracker::new("BTCUSDT".to_string(), 1.0);

        // Create imbalanced level (all buy)
        tracker.process_trade(&create_test_event("BTCUSDT", 50000.0, 10.0, false, 1000));

        // Create balanced level
        tracker.process_trade(&create_test_event("BTCUSDT", 50001.0, 5.0, false, 1001));
        tracker.process_trade(&create_test_event("BTCUSDT", 50001.0, 5.0, true, 1002));

        let imbalanced = tracker.get_imbalanced_levels(0.5);
        assert_eq!(imbalanced.len(), 1); // Only one imbalanced level
    }

    #[test]
    fn test_multi_symbol_tracker() {
        let mut multi_tracker = MultiSymbolVolumeTracker::new(1.0);

        multi_tracker.process_trade(&create_test_event("BTCUSDT", 50000.0, 1.0, false, 1000));
        multi_tracker.process_trade(&create_test_event("ETHUSDT", 3000.0, 2.0, true, 1001));

        assert_eq!(multi_tracker.symbol_count(), 2);

        let btc_stats = multi_tracker.get_tracker("BTCUSDT").unwrap().get_stats();
        assert_eq!(btc_stats.total_volume, 1.0);

        let eth_stats = multi_tracker.get_tracker("ETHUSDT").unwrap().get_stats();
        assert_eq!(eth_stats.total_volume, 2.0);
    }
}
