use std::collections::{HashMap, VecDeque, BTreeMap};
use serde::{Deserialize, Serialize};
use crate::analysis::footprint::FootprintCandle;
use crate::data::market_data::Candle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Timeframe {
    Seconds(u64),   // Sub-minute: 15s, 30s
    Minutes(u64),   // 1m, 5m, 15m, 30m
    Hours(u64),     // 1h, 4h, 12h
    Days(u64),      // 1d
}

impl Timeframe {
    pub fn to_millis(&self) -> u64 {
        match self {
            Timeframe::Seconds(s) => s * 1000,
            Timeframe::Minutes(m) => m * 60 * 1000,
            Timeframe::Hours(h) => h * 60 * 60 * 1000,
            Timeframe::Days(d) => d * 24 * 60 * 60 * 1000,
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "15s" => Some(Timeframe::Seconds(15)),
            "30s" => Some(Timeframe::Seconds(30)),
            "1m" => Some(Timeframe::Minutes(1)),
            "5m" => Some(Timeframe::Minutes(5)),
            "15m" => Some(Timeframe::Minutes(15)),
            "30m" => Some(Timeframe::Minutes(30)),
            "1h" => Some(Timeframe::Hours(1)),
            "4h" => Some(Timeframe::Hours(4)),
            "12h" => Some(Timeframe::Hours(12)),
            "1d" => Some(Timeframe::Days(1)),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Timeframe::Seconds(s) => format!("{}s", s),
            Timeframe::Minutes(m) => format!("{}m", m),
            Timeframe::Hours(h) => format!("{}h", h),
            Timeframe::Days(d) => format!("{}d", d),
        }
    }

    pub fn all_timeframes() -> Vec<Timeframe> {
        vec![
            Timeframe::Seconds(15),
            Timeframe::Seconds(30),
            Timeframe::Minutes(1),
            Timeframe::Minutes(5),
            Timeframe::Minutes(15),
            Timeframe::Minutes(30),
            Timeframe::Hours(1),
            Timeframe::Hours(4),
            Timeframe::Hours(12),
            Timeframe::Days(1),
        ]
    }
}

impl std::fmt::Display for Timeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

pub struct TimeframeManager {
    // Base data storage (finest granularity)
    base_timeframe: Timeframe,
    base_candles: HashMap<String, VecDeque<FootprintCandle>>,

    // Cached aggregated candles per timeframe
    cached_candles: HashMap<Timeframe, HashMap<String, VecDeque<FootprintCandle>>>,

    // Maximum candles to keep per symbol per timeframe
    max_candles: usize,
}

impl TimeframeManager {
    pub fn new(base_timeframe: Timeframe, max_candles: usize) -> Self {
        Self {
            base_timeframe,
            base_candles: HashMap::new(),
            cached_candles: HashMap::new(),
            max_candles,
        }
    }

    /// Add a new base candle
    pub fn add_base_candle(&mut self, symbol: &str, candle: FootprintCandle) {
        let candles = self.base_candles
            .entry(symbol.to_string())
            .or_insert_with(VecDeque::new);

        candles.push_back(candle);

        // Maintain max size
        while candles.len() > self.max_candles {
            candles.pop_front();
        }

        // Invalidate cached aggregations for this symbol
        self.invalidate_cache_for_symbol(symbol);
    }

    /// Get candles for a specific timeframe (with caching)
    pub fn get_candles(&mut self, symbol: &str, timeframe: Timeframe) -> Option<&VecDeque<FootprintCandle>> {
        // If base timeframe requested, return directly
        if timeframe == self.base_timeframe {
            return self.base_candles.get(symbol);
        }

        // Check cache first
        if let Some(cached) = self.cached_candles.get(&timeframe) {
            if let Some(candles) = cached.get(symbol) {
                return Some(candles);
            }
        }

        // Need to aggregate
        self.aggregate_and_cache(symbol, timeframe);

        // Return cached result
        self.cached_candles
            .get(&timeframe)
            .and_then(|cache| cache.get(symbol))
    }

    fn aggregate_and_cache(&mut self, symbol: &str, target_timeframe: Timeframe) {
        let base_candles = match self.base_candles.get(symbol) {
            Some(candles) => candles,
            None => return,
        };

        let timeframe_ms = target_timeframe.to_millis();
        let mut aggregated_candles = VecDeque::new();

        // Group base candles by target timeframe
        let mut current_group: Vec<FootprintCandle> = Vec::new();
        let mut current_period_start: Option<u64> = None;

        for candle in base_candles {
            let period_start = (candle.candle.timestamp / timeframe_ms) * timeframe_ms;

            if let Some(current_start) = current_period_start {
                if period_start != current_start {
                    // New period - aggregate current group
                    if !current_group.is_empty() {
                        let aggregated = Self::aggregate_candles(&current_group);
                        aggregated_candles.push_back(aggregated);
                        current_group.clear();
                    }
                }
            }

            current_period_start = Some(period_start);
            current_group.push(candle.clone());
        }

        // Aggregate remaining group
        if !current_group.is_empty() {
            let aggregated = Self::aggregate_candles(&current_group);
            aggregated_candles.push_back(aggregated);
        }

        // Store in cache
        self.cached_candles
            .entry(target_timeframe)
            .or_insert_with(HashMap::new)
            .insert(symbol.to_string(), aggregated_candles);
    }

    fn aggregate_candles(candles: &[FootprintCandle]) -> FootprintCandle {
        use crate::data::orderflow::VolumeAtPrice;

        let first = &candles[0];
        let last = &candles[candles.len() - 1];

        let mut aggregated = FootprintCandle {
            candle: first.candle.clone(),
            volume_profile: first.volume_profile.clone(),
            delta: 0.0,
            cvd: last.cvd,  // Preserve CVD from last candle
            imbalance_levels: Vec::new(),
            significant_levels: Vec::new(),
        };

        // Merge OHLC
        aggregated.candle.open = first.candle.open;
        aggregated.candle.close = last.candle.close;
        aggregated.candle.high = candles.iter()
            .map(|c| c.candle.high)
            .fold(f64::NEG_INFINITY, f64::max);
        aggregated.candle.low = candles.iter()
            .map(|c| c.candle.low)
            .fold(f64::INFINITY, f64::min);
        aggregated.candle.volume = candles.iter()
            .map(|c| c.candle.volume)
            .sum();

        // Merge volume profiles
        let mut merged_price_levels = BTreeMap::new();
        for candle in candles {
            for (price_tick, vol) in &candle.volume_profile.price_levels {
                let entry = merged_price_levels
                    .entry(*price_tick)
                    .or_insert_with(|| VolumeAtPrice {
                        buy_volume: 0.0,
                        sell_volume: 0.0,
                        total_volume: 0.0,
                        trade_count: 0,
                    });
                entry.buy_volume += vol.buy_volume;
                entry.sell_volume += vol.sell_volume;
                entry.total_volume += vol.total_volume;
                entry.trade_count += vol.trade_count;
            }
        }

        aggregated.volume_profile.price_levels = merged_price_levels;

        // Recalculate total volumes
        aggregated.volume_profile.total_volume = aggregated.volume_profile.price_levels.values()
            .map(|v| v.total_volume)
            .sum();

        aggregated.volume_profile.buy_volume = aggregated.volume_profile.price_levels.values()
            .map(|v| v.buy_volume)
            .sum();

        aggregated.volume_profile.sell_volume = aggregated.volume_profile.price_levels.values()
            .map(|v| v.sell_volume)
            .sum();

        // Recalculate delta
        aggregated.delta = aggregated.volume_profile.buy_volume - aggregated.volume_profile.sell_volume;

        // Recalculate VWAP
        let mut vwap_sum = 0.0;
        let mut volume_sum = 0.0;

        for (price_tick, vol) in &aggregated.volume_profile.price_levels {
            let price = *price_tick as f64 * first.volume_profile.price_levels.keys().next()
                .map(|k| first.candle.close / *k as f64)
                .unwrap_or(1.0);
            vwap_sum += price * vol.total_volume;
            volume_sum += vol.total_volume;
        }

        aggregated.volume_profile.vwap = if volume_sum > 0.0 {
            vwap_sum / volume_sum
        } else {
            aggregated.candle.close
        };

        // Recalculate POC (Point of Control - highest volume price level)
        aggregated.volume_profile.poc = aggregated.volume_profile.price_levels.iter()
            .max_by(|a, b| a.1.total_volume.partial_cmp(&b.1.total_volume).unwrap())
            .map(|(tick, _)| *tick)
            .unwrap_or(0);

        aggregated
    }

    fn invalidate_cache_for_symbol(&mut self, symbol: &str) {
        for (_, cache) in &mut self.cached_candles {
            cache.remove(symbol);
        }
    }

    /// Get all candles for a symbol at base timeframe
    pub fn get_base_candles(&self, symbol: &str) -> Option<&VecDeque<FootprintCandle>> {
        self.base_candles.get(symbol)
    }

    /// Clear all cached data
    pub fn clear_cache(&mut self) {
        self.cached_candles.clear();
    }

    /// Get symbol count
    pub fn symbol_count(&self) -> usize {
        self.base_candles.len()
    }

    /// Get total candle count across all symbols
    pub fn total_candle_count(&self) -> usize {
        self.base_candles.values()
            .map(|candles| candles.len())
            .sum()
    }

    /// Check if symbol has data
    pub fn has_symbol(&self, symbol: &str) -> bool {
        self.base_candles.contains_key(symbol)
    }

    /// Remove all data for a symbol
    pub fn remove_symbol(&mut self, symbol: &str) {
        self.base_candles.remove(symbol);
        self.invalidate_cache_for_symbol(symbol);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::market_data::Candle;
    use crate::analysis::footprint::VolumeProfile;

    fn create_test_candle(timestamp: u64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> FootprintCandle {
        FootprintCandle {
            candle: Candle {
                timestamp,
                open,
                high,
                low,
                close,
                volume,
            },
            volume_profile: VolumeProfile {
                price_levels: BTreeMap::new(),
                total_volume: volume,
                buy_volume: volume / 2.0,
                sell_volume: volume / 2.0,
                vwap: close,
                poc: 0,
            },
            delta: 0.0,
            cvd: 0.0,
            imbalance_levels: Vec::new(),
            significant_levels: Vec::new(),
        }
    }

    #[test]
    fn test_timeframe_conversion() {
        assert_eq!(Timeframe::Seconds(15).to_millis(), 15000);
        assert_eq!(Timeframe::Minutes(1).to_millis(), 60000);
        assert_eq!(Timeframe::Hours(1).to_millis(), 3600000);
        assert_eq!(Timeframe::Days(1).to_millis(), 86400000);
    }

    #[test]
    fn test_timeframe_from_string() {
        assert_eq!(Timeframe::from_string("1m"), Some(Timeframe::Minutes(1)));
        assert_eq!(Timeframe::from_string("5m"), Some(Timeframe::Minutes(5)));
        assert_eq!(Timeframe::from_string("1h"), Some(Timeframe::Hours(1)));
        assert_eq!(Timeframe::from_string("invalid"), None);
    }

    #[test]
    fn test_timeframe_manager_add_candle() {
        let mut manager = TimeframeManager::new(Timeframe::Minutes(1), 1000);

        let candle = create_test_candle(60000, 50000.0, 50100.0, 49900.0, 50050.0, 100.0);
        manager.add_base_candle("BTCUSDT", candle);

        assert_eq!(manager.symbol_count(), 1);
        assert_eq!(manager.total_candle_count(), 1);
        assert!(manager.has_symbol("BTCUSDT"));
    }

    #[test]
    fn test_timeframe_aggregation() {
        let mut manager = TimeframeManager::new(Timeframe::Minutes(1), 1000);

        // Add 5 one-minute candles
        for i in 0..5 {
            let timestamp = (i + 1) * 60000; // 1m, 2m, 3m, 4m, 5m
            let candle = create_test_candle(timestamp, 50000.0, 50100.0, 49900.0, 50050.0, 100.0);
            manager.add_base_candle("BTCUSDT", candle);
        }

        // Get 5m aggregated candles
        let aggregated = manager.get_candles("BTCUSDT", Timeframe::Minutes(5));
        assert!(aggregated.is_some());

        let candles = aggregated.unwrap();
        assert_eq!(candles.len(), 1); // Should be one 5m candle from 5 1m candles
    }
}
