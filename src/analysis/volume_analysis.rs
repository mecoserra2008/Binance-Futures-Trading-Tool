use tokio::sync::mpsc;
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, error, debug, warn};
use tokio::time::{sleep, Duration};

use crate::data::{OrderflowEvent, VolumeProfile, BigOrderflowAlert, DailyStats};

pub struct VolumeAnalyzer {
    sender: mpsc::Sender<VolumeProfile>,
    alert_sender: Option<mpsc::Sender<BigOrderflowAlert>>,
    volume_threshold_percentage: f64,
    api_base_url: String,
}

struct SymbolVolumeTracker {
    symbol: String,
    daily_stats: DailyStats,
    current_volume_profile: VolumeProfile,
    last_profile_update: u64,
    profile_interval_ms: u64,
}

impl VolumeAnalyzer {
    pub fn new(sender: mpsc::Sender<VolumeProfile>, api_base_url: String) -> Self {
        Self {
            sender,
            alert_sender: None,
            volume_threshold_percentage: 0.5,
            api_base_url,
        }
    }

    pub fn set_alert_sender(&mut self, sender: mpsc::Sender<BigOrderflowAlert>) {
        self.alert_sender = Some(sender);
    }

    pub async fn start_with_receiver(&mut self, orderflow_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<OrderflowEvent>>>) -> Result<()> {
        info!("Starting volume analyzer with orderflow receiver");

        let mut trackers: HashMap<String, SymbolVolumeTracker> = HashMap::new();
        let mut pending_init_symbols: Vec<String> = Vec::new();
        let mut last_cleanup = std::time::Instant::now();

        loop {
            tokio::select! {
                // Process incoming orderflow events
                event = async {
                    let mut rx = orderflow_rx.lock().await;
                    rx.recv().await
                } => {
                    if let Some(event) = event {
                        // Check if tracker exists, if not add symbol to pending init list
                        if !trackers.contains_key(&event.symbol) {
                            if !pending_init_symbols.contains(&event.symbol) {
                                pending_init_symbols.push(event.symbol.clone());
                                info!("Symbol {} needs initialization, total pending: {}", event.symbol, pending_init_symbols.len());

                                // If we have accumulated some symbols, initialize them in batch
                                if pending_init_symbols.len() >= 5 || trackers.is_empty() {
                                    let symbols_to_init = pending_init_symbols.clone();
                                    pending_init_symbols.clear();

                                    info!("Initializing {} symbols with 24h Binance data...", symbols_to_init.len());
                                    for symbol in symbols_to_init {
                                        let mut tracker = SymbolVolumeTracker::new(symbol.clone());
                                        if let Err(e) = tracker.initialize_from_binance(&self.api_base_url).await {
                                            error!("Failed to initialize {}: {}", symbol, e);
                                        }
                                        trackers.insert(symbol, tracker);
                                    }
                                }
                            }
                        }

                        debug!("Volume analyzer received event for {}: price={}, qty={}", event.symbol, event.price, event.quantity);
                        let results = self.process_orderflow_event(&event, &mut trackers);

                        // Send volume profile updates
                        for result in results {
                            match result {
                                VolumeAnalysisResult::VolumeProfile(profile) => {
                                    if let Err(e) = self.sender.try_send(profile) {
                                        debug!("Failed to send volume profile: {}", e);
                                    }
                                }
                                VolumeAnalysisResult::BigOrderflowAlert(alert) => {
                                    if let Some(alert_sender) = &self.alert_sender {
                                        if let Err(e) = alert_sender.try_send(alert) {
                                            debug!("Failed to send big orderflow alert: {}", e);
                                        }
                                    }
                                }
                                _ => {} // Handle other variants if needed
                            }
                        }
                    } else {
                        debug!("Volume analyzer orderflow channel closed");
                        break;
                    }
                }

                // Periodic cleanup and maintenance
                _ = sleep(Duration::from_secs(60)) => {
                    if last_cleanup.elapsed() >= Duration::from_secs(60) {
                        let current_time = chrono::Utc::now().timestamp() as u64 * 1000;

                        // Update volume profiles for all symbols
                        for tracker in trackers.values_mut() {
                            if current_time - tracker.last_profile_update >= tracker.profile_interval_ms {
                                let mut profile = tracker.current_volume_profile.clone();
                                profile.calculate_metrics();

                                if let Err(e) = self.sender.try_send(profile) {
                                    debug!("Failed to send volume profile: {}", e);
                                }

                                // Reset the current profile for next interval
                                tracker.reset_current_profile(current_time);
                            }
                        }

                        last_cleanup = std::time::Instant::now();
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting volume analyzer");

        let mut trackers: HashMap<String, SymbolVolumeTracker> = HashMap::new();
        let mut last_cleanup = std::time::Instant::now();

        // This is a placeholder implementation - in a real system, this would
        // receive orderflow events and process them
        loop {
            // Periodic cleanup and maintenance
            if last_cleanup.elapsed() >= Duration::from_secs(60) {
                let current_time = chrono::Utc::now().timestamp() as u64 * 1000;

                // Update volume profiles for all symbols
                for tracker in trackers.values_mut() {
                    if current_time - tracker.last_profile_update >= tracker.profile_interval_ms {
                        let mut profile = tracker.current_volume_profile.clone();
                        profile.calculate_metrics();

                        if let Err(e) = self.sender.try_send(profile) {
                            debug!("Failed to send volume profile: {}", e);
                        }

                        // Reset the current profile for next interval
                        tracker.reset_current_profile(current_time);
                    }
                }

                last_cleanup = std::time::Instant::now();
            }

            sleep(Duration::from_millis(100)).await;
        }
    }

    pub fn process_orderflow_event(&mut self, event: &OrderflowEvent, trackers: &mut HashMap<String, SymbolVolumeTracker>) -> Vec<VolumeAnalysisResult> {
        let mut results = Vec::new();

        // Get or create tracker for this symbol
        let tracker = trackers
            .entry(event.symbol.clone())
            .or_insert_with(|| {
                info!("New symbol tracker created for {}", event.symbol);
                SymbolVolumeTracker::new(event.symbol.clone())
            });

        // Update daily statistics
        tracker.update_daily_stats(event);
        
        // Check for big orderflow alert
        if let Some(alert) = self.check_big_orderflow(event, &tracker.daily_stats) {
            results.push(VolumeAnalysisResult::BigOrderflowAlert(alert));
        }
        
        // Update volume profile
        tracker.update_volume_profile(event);
        
        // Check if it's time to emit volume profile
        if event.timestamp - tracker.last_profile_update >= tracker.profile_interval_ms {
            let mut profile = tracker.current_volume_profile.clone();
            profile.calculate_metrics();
            results.push(VolumeAnalysisResult::VolumeProfile(profile));
            tracker.reset_current_profile(event.timestamp);
        }
        
        results
    }

    fn check_big_orderflow(&self, event: &OrderflowEvent, daily_stats: &DailyStats) -> Option<BigOrderflowAlert> {
        if daily_stats.avg_volume > 0.0 {
            let volume_percentage = (event.quantity / daily_stats.avg_volume) * 100.0;
            
            if volume_percentage >= self.volume_threshold_percentage {
                return Some(BigOrderflowAlert {
                    symbol: event.symbol.clone(),
                    timestamp: event.timestamp,
                    side: if event.is_buyer_maker { "SELL".to_string() } else { "BUY".to_string() },
                    price: event.price,
                    quantity: event.quantity,
                    percentage_of_daily: volume_percentage,
                    notional_value: event.price * event.quantity,
                });
            }
        }
        
        None
    }
}

impl SymbolVolumeTracker {
    fn new(symbol: String) -> Self {
        let current_time = chrono::Utc::now().timestamp() as u64 * 1000;
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        Self {
            symbol: symbol.clone(),
            daily_stats: DailyStats {
                symbol: symbol.clone(),
                date: today,
                avg_volume: 1.0, // Initialize to 1.0 to avoid division by zero, will be replaced by real data
                total_volume: 0.0,
                avg_price: 0.0,
                high_price: 0.0,
                low_price: f64::MAX,
                trade_count: 0,
            },
            current_volume_profile: VolumeProfile::new(symbol.clone(), current_time, "1m".to_string()),
            last_profile_update: current_time,
            profile_interval_ms: 60_000, // 1 minute
        }
    }

    async fn initialize_from_binance(&mut self, api_base_url: &str) -> Result<()> {
        // Fetch 24h ticker statistics from Binance
        let client = reqwest::Client::new();
        let url = format!("{}/fapi/v1/ticker/24hr?symbol={}", api_base_url, self.symbol);

        match client.get(&url).send().await {
            Ok(response) => {
                if let Ok(data) = response.json::<serde_json::Value>().await {
                    // Extract 24h volume and calculate average per trade
                    if let Some(volume_str) = data["volume"].as_str() {
                        if let Ok(total_24h_volume) = volume_str.parse::<f64>() {
                            // Extract number of trades
                            if let Some(trade_count) = data["count"].as_u64() {
                                if trade_count > 0 {
                                    self.daily_stats.total_volume = total_24h_volume;
                                    self.daily_stats.trade_count = trade_count;
                                    self.daily_stats.avg_volume = total_24h_volume / trade_count as f64;

                                    info!("Initialized {} with 24h volume: {}, trades: {}, avg: {}",
                                          self.symbol, total_24h_volume, trade_count, self.daily_stats.avg_volume);
                                } else {
                                    warn!("{}: No trades in 24h period, using default", self.symbol);
                                }
                            }

                            // Also get price info
                            if let (Some(high), Some(low), Some(last)) = (
                                data["highPrice"].as_str().and_then(|s| s.parse::<f64>().ok()),
                                data["lowPrice"].as_str().and_then(|s| s.parse::<f64>().ok()),
                                data["lastPrice"].as_str().and_then(|s| s.parse::<f64>().ok()),
                            ) {
                                self.daily_stats.high_price = high;
                                self.daily_stats.low_price = low;
                                self.daily_stats.avg_price = last;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to fetch 24h stats for {}: {}. Using defaults.", self.symbol, e);
            }
        }

        Ok(())
    }

    fn update_daily_stats(&mut self, event: &OrderflowEvent) {
        self.daily_stats.trade_count += 1;
        self.daily_stats.total_volume += event.quantity;
        self.daily_stats.avg_volume = self.daily_stats.total_volume / self.daily_stats.trade_count as f64;
        
        // Update price statistics
        let price_sum = self.daily_stats.avg_price * (self.daily_stats.trade_count - 1) as f64 + event.price;
        self.daily_stats.avg_price = price_sum / self.daily_stats.trade_count as f64;
        
        if event.price > self.daily_stats.high_price {
            self.daily_stats.high_price = event.price;
        }
        if event.price < self.daily_stats.low_price {
            self.daily_stats.low_price = event.price;
        }
    }

    fn update_volume_profile(&mut self, event: &OrderflowEvent) {
        let is_buy = !event.is_buyer_maker; // Buyer is taker when not maker
        self.current_volume_profile.add_trade(event.price, event.quantity, is_buy);
    }

    fn reset_current_profile(&mut self, timestamp: u64) {
        self.current_volume_profile = VolumeProfile::new(
            self.symbol.clone(),
            timestamp,
            "1m".to_string(),
        );
        self.last_profile_update = timestamp;
    }

    pub fn get_statistics(&self) -> VolumeStatistics {
        VolumeStatistics {
            symbol: self.symbol.clone(),
            daily_stats: self.daily_stats.clone(),
            current_profile_volume: self.current_volume_profile.total_volume,
            current_profile_buy_volume: self.current_volume_profile.buy_volume,
            current_profile_sell_volume: self.current_volume_profile.sell_volume,
            price_levels_count: self.current_volume_profile.price_levels.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VolumeStatistics {
    pub symbol: String,
    pub daily_stats: DailyStats,
    pub current_profile_volume: f64,
    pub current_profile_buy_volume: f64,
    pub current_profile_sell_volume: f64,
    pub price_levels_count: usize,
}

#[derive(Debug)]
pub enum VolumeAnalysisResult {
    BigOrderflowAlert(BigOrderflowAlert),
    VolumeProfile(VolumeProfile),
    DailyStatsUpdate(DailyStats),
}

pub struct VolumeProfileAggregator {
    profiles: HashMap<String, Vec<VolumeProfile>>,
    max_profiles_per_symbol: usize,
}

impl VolumeProfileAggregator {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            max_profiles_per_symbol: 1440, // Store up to 24 hours of 1-minute profiles
        }
    }

    pub fn add_profile(&mut self, profile: VolumeProfile) {
        let profiles = self.profiles
            .entry(profile.symbol.clone())
            .or_insert_with(Vec::new);
        
        profiles.push(profile);
        
        // Maintain size limit
        while profiles.len() > self.max_profiles_per_symbol {
            profiles.remove(0);
        }
        
        // Keep profiles sorted by timestamp
        profiles.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    }

    pub fn get_recent_profiles(&self, symbol: &str, count: usize) -> Vec<&VolumeProfile> {
        if let Some(profiles) = self.profiles.get(symbol) {
            profiles.iter()
                .rev()
                .take(count)
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn aggregate_profiles(&self, symbol: &str, timeframe_minutes: u32) -> Option<VolumeProfile> {
        if let Some(profiles) = self.profiles.get(symbol) {
            if profiles.is_empty() {
                return None;
            }

            let latest_timestamp = profiles.last()?.timestamp;
            let aggregation_start = latest_timestamp - (timeframe_minutes as u64 * 60 * 1000);
            
            let relevant_profiles: Vec<&VolumeProfile> = profiles.iter()
                .filter(|p| p.timestamp >= aggregation_start)
                .collect();

            if relevant_profiles.is_empty() {
                return None;
            }

            let mut aggregated = VolumeProfile::new(
                symbol.to_string(),
                latest_timestamp,
                format!("{}m", timeframe_minutes),
            );

            for profile in relevant_profiles {
                // Merge price levels
                for (price, volume_data) in &profile.price_levels {
                    let entry = aggregated.price_levels.entry(*price).or_insert_with(|| {
                        crate::data::VolumeAtPrice {
                            buy_volume: 0.0,
                            sell_volume: 0.0,
                            total_volume: 0.0,
                            trade_count: 0,
                        }
                    });

                    entry.buy_volume += volume_data.buy_volume;
                    entry.sell_volume += volume_data.sell_volume;
                    entry.total_volume += volume_data.total_volume;
                    entry.trade_count += volume_data.trade_count;
                }

                aggregated.total_volume += profile.total_volume;
                aggregated.buy_volume += profile.buy_volume;
                aggregated.sell_volume += profile.sell_volume;
            }

            aggregated.calculate_metrics();
            Some(aggregated)
        } else {
            None
        }
    }

    pub fn get_volume_at_price(&self, symbol: &str, price: f64, tolerance: f64) -> Option<f64> {
        if let Some(profiles) = self.profiles.get(symbol) {
            let mut total_volume = 0.0;
            
            for profile in profiles {
                for (profile_price, volume_data) in &profile.price_levels {
                    if (profile_price.0 - price).abs() <= tolerance {
                        total_volume += volume_data.total_volume;
                    }
                }
            }
            
            if total_volume > 0.0 {
                Some(total_volume)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn cleanup_old_profiles(&mut self, max_age_hours: u32) {
        let cutoff = chrono::Utc::now().timestamp() as u64 * 1000 - (max_age_hours as u64 * 60 * 60 * 1000);
        
        for profiles in self.profiles.values_mut() {
            profiles.retain(|profile| profile.timestamp >= cutoff);
        }
    }
}