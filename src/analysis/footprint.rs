use std::collections::{HashMap, BTreeMap};
use ordered_float::OrderedFloat;
use serde::{Serialize, Deserialize};

use crate::data::{OrderflowEvent, VolumeProfile, Candle, VolumeAtPrice};

#[derive(Debug, Clone)]
pub struct FootprintChart {
    pub symbol: String,
    pub timeframe: String,
    pub candles: Vec<FootprintCandle>,
    pub price_precision: u32,
    pub volume_precision: u32,
}

#[derive(Debug, Clone)]
pub struct FootprintCandle {
    pub candle: Candle,
    pub volume_profile: VolumeProfile,
    pub delta: f64, // Buy volume - Sell volume
    pub cvd: f64,   // Cumulative Volume Delta
    pub imbalance_levels: Vec<ImbalanceLevel>,
    pub significant_levels: Vec<SignificantLevel>,
}

#[derive(Debug, Clone)]
pub struct ImbalanceLevel {
    pub price: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub imbalance_ratio: f64,
    pub significance: ImbalanceSignificance,
}

#[derive(Debug, Clone)]
pub struct SignificantLevel {
    pub price: f64,
    pub level_type: LevelType,
    pub strength: f64,
    pub volume: f64,
}

#[derive(Debug, Clone)]
pub enum ImbalanceSignificance {
    Low,
    Medium,
    High,
    Extreme,
}

#[derive(Debug, Clone)]
pub enum LevelType {
    VolumeCluster,
    OrderImbalance,
    PointOfControl,
    VolumeGap,
    LiquidityVoid,
}

pub struct FootprintAnalyzer {
    symbol: String,
    base_timeframe_ms: u64, // Base timeframe in milliseconds (usually 1 minute)
    current_candle: Option<FootprintCandle>,
    completed_candles: Vec<FootprintCandle>,
    aggregation_timeframes: Vec<String>,
    max_candles_history: usize,
    price_tick_size: f64,
    cumulative_delta: f64,
}

impl FootprintAnalyzer {
    pub fn new(symbol: String, price_tick_size: f64) -> Self {
        Self {
            symbol,
            base_timeframe_ms: 60_000, // 1 minute
            current_candle: None,
            completed_candles: Vec::new(),
            aggregation_timeframes: vec!["1m".to_string(), "5m".to_string(), "15m".to_string(), "1h".to_string()],
            max_candles_history: 1000,
            price_tick_size,
            cumulative_delta: 0.0,
        }
    }

    pub fn process_trade(&mut self, event: &OrderflowEvent) -> Vec<FootprintUpdate> {
        let mut updates = Vec::new();
        
        // Determine which candle this trade belongs to
        let candle_timestamp = self.get_candle_timestamp(event.timestamp);
        
        // Check if we need to complete the current candle
        if let Some(current) = &self.current_candle {
            if current.candle.timestamp != candle_timestamp {
                // Complete the current candle
                let completed = self.finalize_current_candle();
                updates.push(FootprintUpdate::CandleCompleted(completed));
                
                // Check for aggregated timeframes
                updates.extend(self.check_aggregated_timeframes(candle_timestamp));
            }
        }
        
        // Ensure we have a current candle
        if self.current_candle.is_none() || 
           self.current_candle.as_ref().unwrap().candle.timestamp != candle_timestamp {
            self.start_new_candle(candle_timestamp, event.price);
        }
        
        // Add trade to current candle
        if let Some(ref mut candle) = self.current_candle {
            Self::add_trade_to_candle_static(candle, event);
            updates.push(FootprintUpdate::CandleUpdated(candle.clone()));
        }
        
        updates
    }

    fn get_candle_timestamp(&self, timestamp: u64) -> u64 {
        (timestamp / self.base_timeframe_ms) * self.base_timeframe_ms
    }

    fn start_new_candle(&mut self, timestamp: u64, price: f64) {
        let candle = Candle {
            symbol: self.symbol.clone(),
            timestamp,
            timeframe: "1m".to_string(),
            open_price: price,
            high_price: price,
            low_price: price,
            close_price: price,
            volume: 0.0,
            buy_volume: 0.0,
            sell_volume: 0.0,
            trade_count: 0,
        };

        let volume_profile = VolumeProfile::new(
            self.symbol.clone(),
            timestamp,
            "1m".to_string(),
        );

        self.current_candle = Some(FootprintCandle {
            candle,
            volume_profile,
            delta: 0.0,
            cvd: self.cumulative_delta,
            imbalance_levels: Vec::new(),
            significant_levels: Vec::new(),
        });
    }

    fn add_trade_to_candle_static(footprint_candle: &mut FootprintCandle, event: &OrderflowEvent) {
        let is_buy = !event.is_buyer_maker;
        
        // Update basic candle data
        footprint_candle.candle.high_price = footprint_candle.candle.high_price.max(event.price);
        footprint_candle.candle.low_price = footprint_candle.candle.low_price.min(event.price);
        footprint_candle.candle.close_price = event.price;
        footprint_candle.candle.volume += event.quantity;
        footprint_candle.candle.trade_count += 1;

        if is_buy {
            footprint_candle.candle.buy_volume += event.quantity;
            footprint_candle.delta += event.quantity;
        } else {
            footprint_candle.candle.sell_volume += event.quantity;
            footprint_candle.delta -= event.quantity;
        }

        // Update volume profile
        footprint_candle.volume_profile.add_trade(event.price, event.quantity, is_buy);

        // Note: cumulative delta update moved to finalize_current_candle
    }

    fn finalize_current_candle(&mut self) -> FootprintCandle {
        if let Some(mut candle) = self.current_candle.take() {
            // Calculate final metrics
            candle.volume_profile.calculate_metrics();
            candle.cvd = self.cumulative_delta;
            
            // Analyze imbalances and significant levels
            candle.imbalance_levels = self.calculate_imbalance_levels(&candle.volume_profile);
            candle.significant_levels = self.identify_significant_levels(&candle.volume_profile);
            
            // Add to history
            self.completed_candles.push(candle.clone());
            
            // Maintain history limit
            while self.completed_candles.len() > self.max_candles_history {
                self.completed_candles.remove(0);
            }
            
            candle
        } else {
            panic!("No current candle to finalize");
        }
    }

    fn calculate_imbalance_levels(&self, profile: &VolumeProfile) -> Vec<ImbalanceLevel> {
        let mut imbalances = Vec::new();
        
        for (price, volume_data) in &profile.price_levels {
            if volume_data.total_volume > 0.0 {
                let buy_ratio = volume_data.buy_volume / volume_data.total_volume;
                let sell_ratio = volume_data.sell_volume / volume_data.total_volume;
                let imbalance_ratio = buy_ratio - sell_ratio;
                
                // Only include significant imbalances
                if imbalance_ratio.abs() > 0.3 {
                    let significance = match imbalance_ratio.abs() {
                        x if x > 0.8 => ImbalanceSignificance::Extreme,
                        x if x > 0.6 => ImbalanceSignificance::High,
                        x if x > 0.4 => ImbalanceSignificance::Medium,
                        _ => ImbalanceSignificance::Low,
                    };
                    
                    imbalances.push(ImbalanceLevel {
                        price: price.0,
                        buy_volume: volume_data.buy_volume,
                        sell_volume: volume_data.sell_volume,
                        imbalance_ratio,
                        significance,
                    });
                }
            }
        }
        
        imbalances
    }

    fn identify_significant_levels(&self, profile: &VolumeProfile) -> Vec<SignificantLevel> {
        let mut levels = Vec::new();
        
        // Find Point of Control
        if let Some((poc_price, poc_volume)) = profile.price_levels
            .iter()
            .max_by(|a, b| a.1.total_volume.partial_cmp(&b.1.total_volume).unwrap()) {
            levels.push(SignificantLevel {
                price: poc_price.0,
                level_type: LevelType::PointOfControl,
                strength: 1.0,
                volume: poc_volume.total_volume,
            });
        }
        
        // Find volume clusters (high volume areas)
        let avg_volume = profile.total_volume / profile.price_levels.len() as f64;
        let cluster_threshold = avg_volume * 2.0;
        
        for (price, volume_data) in &profile.price_levels {
            if volume_data.total_volume > cluster_threshold {
                levels.push(SignificantLevel {
                    price: price.0,
                    level_type: LevelType::VolumeCluster,
                    strength: volume_data.total_volume / cluster_threshold,
                    volume: volume_data.total_volume,
                });
            }
        }
        
        // Find volume gaps (areas with very low volume)
        self.find_volume_gaps(profile, &mut levels);
        
        levels
    }

    fn find_volume_gaps(&self, profile: &VolumeProfile, levels: &mut Vec<SignificantLevel>) {
        if profile.price_levels.len() < 3 {
            return;
        }
        
        let prices: Vec<OrderedFloat<f64>> = profile.price_levels.keys().cloned().collect();
        let avg_volume = profile.total_volume / profile.price_levels.len() as f64;
        let gap_threshold = avg_volume * 0.1; // 10% of average volume
        
        for i in 1..prices.len()-1 {
            let current_price = prices[i];
            if let Some(volume_data) = profile.price_levels.get(&current_price) {
                if volume_data.total_volume < gap_threshold {
                    levels.push(SignificantLevel {
                        price: current_price.0,
                        level_type: LevelType::VolumeGap,
                        strength: 1.0 - (volume_data.total_volume / gap_threshold),
                        volume: volume_data.total_volume,
                    });
                }
            }
        }
    }

    fn check_aggregated_timeframes(&self, current_timestamp: u64) -> Vec<FootprintUpdate> {
        let mut updates = Vec::new();
        
        for timeframe in &self.aggregation_timeframes {
            if timeframe == "1m" {
                continue; // Skip base timeframe
            }
            
            if let Some(aggregated) = self.aggregate_candles_for_timeframe(timeframe, current_timestamp) {
                updates.push(FootprintUpdate::AggregatedCandle {
                    timeframe: timeframe.clone(),
                    candle: aggregated,
                });
            }
        }
        
        updates
    }

    fn aggregate_candles_for_timeframe(&self, timeframe: &str, current_timestamp: u64) -> Option<FootprintCandle> {
        let timeframe_ms = match timeframe {
            "5m" => 5 * 60 * 1000,
            "15m" => 15 * 60 * 1000,
            "1h" => 60 * 60 * 1000,
            "4h" => 4 * 60 * 60 * 1000,
            "1d" => 24 * 60 * 60 * 1000,
            _ => return None,
        };

        let aggregation_start = (current_timestamp / timeframe_ms) * timeframe_ms;
        let relevant_candles: Vec<&FootprintCandle> = self.completed_candles
            .iter()
            .filter(|c| c.candle.timestamp >= aggregation_start && c.candle.timestamp < current_timestamp)
            .collect();

        if relevant_candles.is_empty() {
            return None;
        }

        // Aggregate basic candle data
        let first_candle = relevant_candles.first().unwrap();
        let last_candle = relevant_candles.last().unwrap();

        let mut aggregated_candle = Candle {
            symbol: self.symbol.clone(),
            timestamp: aggregation_start,
            timeframe: timeframe.to_string(),
            open_price: first_candle.candle.open_price,
            high_price: relevant_candles.iter().map(|c| c.candle.high_price).fold(0.0, f64::max),
            low_price: relevant_candles.iter().map(|c| c.candle.low_price).fold(f64::INFINITY, f64::min),
            close_price: last_candle.candle.close_price,
            volume: relevant_candles.iter().map(|c| c.candle.volume).sum(),
            buy_volume: relevant_candles.iter().map(|c| c.candle.buy_volume).sum(),
            sell_volume: relevant_candles.iter().map(|c| c.candle.sell_volume).sum(),
            trade_count: relevant_candles.iter().map(|c| c.candle.trade_count).sum(),
        };

        // Aggregate volume profile
        let mut aggregated_profile = VolumeProfile::new(
            self.symbol.clone(),
            aggregation_start,
            timeframe.to_string(),
        );

        for candle in &relevant_candles {
            for (price, volume_data) in &candle.volume_profile.price_levels {
                let entry = aggregated_profile.price_levels.entry(*price).or_insert(VolumeAtPrice {
                    buy_volume: 0.0,
                    sell_volume: 0.0,
                    total_volume: 0.0,
                    trade_count: 0,
                });

                entry.buy_volume += volume_data.buy_volume;
                entry.sell_volume += volume_data.sell_volume;
                entry.total_volume += volume_data.total_volume;
                entry.trade_count += volume_data.trade_count;
            }
        }

        aggregated_profile.total_volume = aggregated_candle.volume;
        aggregated_profile.buy_volume = aggregated_candle.buy_volume;
        aggregated_profile.sell_volume = aggregated_candle.sell_volume;
        aggregated_profile.calculate_metrics();

        // Calculate aggregated delta and CVD
        let delta = aggregated_candle.buy_volume - aggregated_candle.sell_volume;
        let cvd = relevant_candles.last().unwrap().cvd;

        let cloned_profile = aggregated_profile.clone();

        Some(FootprintCandle {
            candle: aggregated_candle,
            volume_profile: aggregated_profile,
            delta,
            cvd,
            imbalance_levels: self.calculate_imbalance_levels(&cloned_profile),
            significant_levels: self.identify_significant_levels(&cloned_profile),
        })
    }

    pub fn get_recent_candles(&self, timeframe: &str, count: usize) -> Vec<FootprintCandle> {
        if timeframe == "1m" {
            self.completed_candles
                .iter()
                .rev()
                .take(count)
                .cloned()
                .collect()
        } else {
            // Generate aggregated candles on demand
            self.generate_aggregated_candles(timeframe, count)
        }
    }

    fn generate_aggregated_candles(&self, timeframe: &str, count: usize) -> Vec<FootprintCandle> {
        let timeframe_ms = match timeframe {
            "5m" => 5 * 60 * 1000,
            "15m" => 15 * 60 * 1000,
            "1h" => 60 * 60 * 1000,
            "4h" => 4 * 60 * 60 * 1000,
            "1d" => 24 * 60 * 60 * 1000,
            _ => return Vec::new(),
        };

        let mut aggregated_candles = Vec::new();
        
        if self.completed_candles.is_empty() {
            return aggregated_candles;
        }

        let latest_timestamp = self.completed_candles.last().unwrap().candle.timestamp;
        
        for i in 0..count {
            let end_timestamp = latest_timestamp - (i as u64 * timeframe_ms);
            let start_timestamp = end_timestamp - timeframe_ms;
            
            if let Some(candle) = self.aggregate_candles_for_timeframe(timeframe, end_timestamp) {
                aggregated_candles.push(candle);
            }
        }
        
        aggregated_candles.reverse();
        aggregated_candles
    }
}

#[derive(Debug, Clone)]
pub enum FootprintUpdate {
    CandleUpdated(FootprintCandle),
    CandleCompleted(FootprintCandle),
    AggregatedCandle {
        timeframe: String,
        candle: FootprintCandle,
    },
}

pub struct FootprintRenderer {
    pub width: f32,
    pub height: f32,
    pub price_precision: u32,
    pub volume_precision: u32,
}

impl FootprintRenderer {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            price_precision: 2,
            volume_precision: 1,
        }
    }

    pub fn render_footprint_data(&self, candle: &FootprintCandle) -> FootprintRenderData {
        let mut price_levels = Vec::new();
        
        for (price, volume_data) in &candle.volume_profile.price_levels {
            price_levels.push(FootprintPriceLevel {
                price: price.0,
                buy_volume: volume_data.buy_volume,
                sell_volume: volume_data.sell_volume,
                total_volume: volume_data.total_volume,
                trade_count: volume_data.trade_count,
                imbalance_ratio: if volume_data.total_volume > 0.0 {
                    (volume_data.buy_volume - volume_data.sell_volume) / volume_data.total_volume
                } else {
                    0.0
                },
            });
        }

        // Sort by price (highest first for display)
        price_levels.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());

        FootprintRenderData {
            candle_data: CandleRenderData {
                open: candle.candle.open_price,
                high: candle.candle.high_price,
                low: candle.candle.low_price,
                close: candle.candle.close_price,
                volume: candle.candle.volume,
                delta: candle.delta,
                cvd: candle.cvd,
            },
            price_levels,
            significant_levels: candle.significant_levels.clone(),
            imbalance_levels: candle.imbalance_levels.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FootprintRenderData {
    pub candle_data: CandleRenderData,
    pub price_levels: Vec<FootprintPriceLevel>,
    pub significant_levels: Vec<SignificantLevel>,
    pub imbalance_levels: Vec<ImbalanceLevel>,
}

#[derive(Debug, Clone)]
pub struct CandleRenderData {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub delta: f64,
    pub cvd: f64,
}

#[derive(Debug, Clone)]
pub struct FootprintPriceLevel {
    pub price: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub total_volume: f64,
    pub trade_count: u32,
    pub imbalance_ratio: f64,
}