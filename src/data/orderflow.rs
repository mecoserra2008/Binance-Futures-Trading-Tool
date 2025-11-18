use std::collections::{HashMap, VecDeque};
use chrono::{DateTime, Utc};
use super::{OrderflowEvent, VolumeProfile, OrderImbalance, BigOrderflowAlert, DailyStats};
use anyhow::Result;

pub struct OrderflowProcessor {
    trade_buffer: HashMap<String, VecDeque<OrderflowEvent>>,
    volume_profiles: HashMap<String, VolumeProfile>,
    imbalance_windows: HashMap<String, ImbalanceWindow>,
    daily_stats: HashMap<String, DailyStats>,
    buffer_size_limit: usize,
}

struct ImbalanceWindow {
    symbol: String,
    window_duration_seconds: u64,
    trades: VecDeque<OrderflowEvent>,
    current_bid_volume: f64,
    current_ask_volume: f64,
    last_calculation_time: u64,
}

impl OrderflowProcessor {
    pub fn new() -> Self {
        Self {
            trade_buffer: HashMap::new(),
            volume_profiles: HashMap::new(),
            imbalance_windows: HashMap::new(),
            daily_stats: HashMap::new(),
            buffer_size_limit: 10000,
        }
    }

    pub fn process_trade(&mut self, event: OrderflowEvent) -> Result<Vec<ProcessorOutput>> {
        let mut outputs = Vec::new();
        let symbol = event.symbol.clone();

        // Add to trade buffer
        self.add_to_buffer(&event);

        // Update daily statistics
        if let Some(stats_update) = self.update_daily_stats(&event)? {
            outputs.push(ProcessorOutput::DailyStatsUpdate(stats_update));
        }

        // Check for big orderflow
        if let Some(alert) = self.check_big_orderflow(&event)? {
            outputs.push(ProcessorOutput::BigOrderflowAlert(alert));
        }

        // Update imbalance calculations
        if let Some(imbalance) = self.update_imbalance_window(&event)? {
            outputs.push(ProcessorOutput::ImbalanceUpdate(imbalance));
        }

        // Update volume profile
        self.update_volume_profile(&event);

        Ok(outputs)
    }

    fn add_to_buffer(&mut self, event: &OrderflowEvent) {
        let buffer = self.trade_buffer
            .entry(event.symbol.clone())
            .or_insert_with(VecDeque::new);

        buffer.push_back(event.clone());

        // Maintain buffer size
        while buffer.len() > self.buffer_size_limit {
            buffer.pop_front();
        }
    }

    fn update_daily_stats(&mut self, event: &OrderflowEvent) -> Result<Option<DailyStats>> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let symbol = &event.symbol;

        let stats = self.daily_stats
            .entry(symbol.clone())
            .or_insert_with(|| DailyStats {
                symbol: symbol.clone(),
                date: today.clone(),
                avg_volume: 0.0,
                total_volume: 0.0,
                avg_price: 0.0,
                high_price: event.price,
                low_price: event.price,
                trade_count: 0,
            });

        // Update statistics
        stats.total_volume += event.quantity;
        stats.trade_count += 1;
        stats.avg_volume = stats.total_volume / stats.trade_count as f64;
        
        // Update price statistics
        let total_notional = stats.avg_price * (stats.trade_count - 1) as f64 + event.price;
        stats.avg_price = total_notional / stats.trade_count as f64;
        
        if event.price > stats.high_price {
            stats.high_price = event.price;
        }
        if event.price < stats.low_price {
            stats.low_price = event.price;
        }

        Ok(Some(stats.clone()))
    }

    fn check_big_orderflow(&self, event: &OrderflowEvent) -> Result<Option<BigOrderflowAlert>> {
        if let Some(daily_stats) = self.daily_stats.get(&event.symbol) {
            let volume_percentage = (event.quantity / daily_stats.avg_volume) * 100.0;
            
            if volume_percentage > 0.5 { // 0.5% threshold
                return Ok(Some(BigOrderflowAlert {
                    symbol: event.symbol.clone(),
                    timestamp: event.timestamp,
                    side: if event.is_buyer_maker { "SELL".to_string() } else { "BUY".to_string() },
                    price: event.price,
                    quantity: event.quantity,
                    percentage_of_daily: volume_percentage,
                    notional_value: event.price * event.quantity,
                }));
            }
        }

        Ok(None)
    }

    fn update_imbalance_window(&mut self, event: &OrderflowEvent) -> Result<Option<OrderImbalance>> {
        let window_duration = 60; // 60 seconds
        let current_time = event.timestamp;

        let window = self.imbalance_windows
            .entry(event.symbol.clone())
            .or_insert_with(|| ImbalanceWindow {
                symbol: event.symbol.clone(),
                window_duration_seconds: window_duration,
                trades: VecDeque::new(),
                current_bid_volume: 0.0,
                current_ask_volume: 0.0,
                last_calculation_time: current_time,
            });

        // Add new trade
        window.trades.push_back(event.clone());

        // Update volumes
        if event.is_buyer_maker {
            window.current_ask_volume += event.quantity; // Buyer is taking ask
        } else {
            window.current_bid_volume += event.quantity; // Buyer is taking bid
        }

        // Remove old trades outside the window
        let cutoff_time = current_time - (window_duration * 1000); // Convert to milliseconds
        while let Some(front_trade) = window.trades.front() {
            if front_trade.timestamp < cutoff_time {
                let old_trade = window.trades.pop_front().unwrap();
                if old_trade.is_buyer_maker {
                    window.current_ask_volume -= old_trade.quantity;
                } else {
                    window.current_bid_volume -= old_trade.quantity;
                }
            } else {
                break;
            }
        }

        // Calculate imbalance every second
        if current_time - window.last_calculation_time >= 1000 {
            window.last_calculation_time = current_time;
            
            let imbalance_ratio = OrderImbalance::calculate_ratio(
                window.current_bid_volume,
                window.current_ask_volume,
            );

            return Ok(Some(OrderImbalance {
                symbol: event.symbol.clone(),
                timestamp: current_time,
                bid_volume: window.current_bid_volume,
                ask_volume: window.current_ask_volume,
                imbalance_ratio,
                window_duration_seconds: window_duration,
            }));
        }

        Ok(None)
    }

    fn update_volume_profile(&mut self, event: &OrderflowEvent) {
        // Create or update 1-minute volume profile
        let minute_timestamp = (event.timestamp / 60000) * 60000; // Round to minute
        let key = format!("{}_{}", event.symbol, minute_timestamp);

        let profile = self.volume_profiles
            .entry(key)
            .or_insert_with(|| VolumeProfile::new(
                event.symbol.clone(),
                minute_timestamp,
                "1m".to_string(),
            ));

        profile.add_trade(event.price, event.quantity, !event.is_buyer_maker);
    }

    pub fn get_volume_profile(&mut self, symbol: &str, timestamp: u64, timeframe: &str) -> Option<VolumeProfile> {
        let key = format!("{}_{}", symbol, timestamp);
        if let Some(mut profile) = self.volume_profiles.remove(&key) {
            profile.calculate_metrics();
            Some(profile)
        } else {
            None
        }
    }

    pub fn get_recent_trades(&self, symbol: &str, count: usize) -> Vec<OrderflowEvent> {
        if let Some(buffer) = self.trade_buffer.get(symbol) {
            buffer.iter()
                .rev()
                .take(count)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_symbol_statistics(&self) -> HashMap<String, SymbolStatistics> {
        let mut stats = HashMap::new();

        for (symbol, buffer) in &self.trade_buffer {
            let trade_count = buffer.len();
            let total_volume: f64 = buffer.iter().map(|t| t.quantity).sum();
            let avg_price: f64 = if trade_count > 0 {
                buffer.iter().map(|t| t.price).sum::<f64>() / trade_count as f64
            } else {
                0.0
            };

            let buy_volume: f64 = buffer.iter()
                .filter(|t| !t.is_buyer_maker)
                .map(|t| t.quantity)
                .sum();

            let sell_volume: f64 = buffer.iter()
                .filter(|t| t.is_buyer_maker)
                .map(|t| t.quantity)
                .sum();

            stats.insert(symbol.clone(), SymbolStatistics {
                symbol: symbol.clone(),
                trade_count,
                total_volume,
                buy_volume,
                sell_volume,
                avg_price,
                last_price: buffer.back().map(|t| t.price).unwrap_or(0.0),
                last_timestamp: buffer.back().map(|t| t.timestamp).unwrap_or(0),
            });
        }

        stats
    }

    pub fn cleanup_old_data(&mut self, max_age_seconds: u64) {
        let current_time = chrono::Utc::now().timestamp() as u64 * 1000;
        let cutoff_time = current_time - (max_age_seconds * 1000);

        // Cleanup trade buffers
        for buffer in self.trade_buffer.values_mut() {
            while let Some(front) = buffer.front() {
                if front.timestamp < cutoff_time {
                    buffer.pop_front();
                } else {
                    break;
                }
            }
        }

        // Cleanup volume profiles
        self.volume_profiles.retain(|_, profile| {
            profile.timestamp >= cutoff_time
        });

        // Cleanup imbalance windows
        for window in self.imbalance_windows.values_mut() {
            while let Some(front) = window.trades.front() {
                if front.timestamp < cutoff_time {
                    let old_trade = window.trades.pop_front().unwrap();
                    if old_trade.is_buyer_maker {
                        window.current_ask_volume -= old_trade.quantity;
                    } else {
                        window.current_bid_volume -= old_trade.quantity;
                    }
                } else {
                    break;
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SymbolStatistics {
    pub symbol: String,
    pub trade_count: usize,
    pub total_volume: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub avg_price: f64,
    pub last_price: f64,
    pub last_timestamp: u64,
}

#[derive(Debug)]
pub enum ProcessorOutput {
    BigOrderflowAlert(BigOrderflowAlert),
    ImbalanceUpdate(OrderImbalance),
    DailyStatsUpdate(DailyStats),
    VolumeProfileUpdate(VolumeProfile),
}

impl ImbalanceWindow {
    pub fn get_current_imbalance(&self) -> f64 {
        OrderImbalance::calculate_ratio(self.current_bid_volume, self.current_ask_volume)
    }

    pub fn get_volume_ratio(&self) -> (f64, f64) {
        let total = self.current_bid_volume + self.current_ask_volume;
        if total > 0.0 {
            (self.current_bid_volume / total, self.current_ask_volume / total)
        } else {
            (0.5, 0.5)
        }
    }
}