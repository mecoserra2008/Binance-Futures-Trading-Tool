use tokio::sync::mpsc;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, error, debug};
use tokio::time::{sleep, Duration};

use crate::data::{OrderflowEvent, LiquidationEvent};

pub struct LiquidationDetector {
    sender: mpsc::Sender<LiquidationEvent>,
    volume_spike_threshold: f64,
    price_movement_threshold: f64,
    detection_window_ms: u64,
}

struct LiquidationCandidate {
    symbol: String,
    timestamp: u64,
    price: f64,
    volume_spike: f64,
    price_movement: f64,
    confidence_score: f64,
}

struct SymbolLiquidationTracker {
    symbol: String,
    recent_trades: VecDeque<OrderflowEvent>,
    recent_liquidations: VecDeque<LiquidationEvent>,
    baseline_volume: f64,
    baseline_update_count: u32,
    last_price: f64,
    volume_window_ms: u64,
    max_trades_history: usize,
}

impl LiquidationDetector {
    pub fn new(sender: mpsc::Sender<LiquidationEvent>) -> Self {
        Self {
            sender,
            volume_spike_threshold: 3.0, // 3x normal volume
            price_movement_threshold: 0.02, // 2% price movement
            detection_window_ms: 5000, // 5 second detection window
        }
    }

    pub async fn start_with_receiver(&mut self, orderflow_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<OrderflowEvent>>>) -> Result<()> {
        info!("Starting liquidation detector with orderflow receiver");

        let mut trackers: HashMap<String, SymbolLiquidationTracker> = HashMap::new();
        let mut last_analysis = std::time::Instant::now();

        loop {
            tokio::select! {
                // Process incoming orderflow events
                event = async {
                    let mut rx = orderflow_rx.lock().await;
                    rx.recv().await
                } => {
                    if let Some(event) = event {
                        if let Some(liquidation_event) = self.process_orderflow_event(&event, &mut trackers) {
                            if let Err(e) = self.sender.try_send(liquidation_event) {
                                debug!("Failed to send liquidation event: {}", e);
                            }
                        }
                    } else {
                        debug!("Liquidation detector orderflow channel closed");
                        break;
                    }
                }

                // Periodic analysis
                _ = sleep(Duration::from_secs(1)) => {
                    if last_analysis.elapsed() >= Duration::from_secs(1) {
                        // Cleanup old data and perform maintenance
                        let current_time = chrono::Utc::now().timestamp() as u64 * 1000;

                        for tracker in trackers.values_mut() {
                            tracker.cleanup_old_data(current_time);
                        }

                        last_analysis = std::time::Instant::now();
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting liquidation detector");

        let mut trackers: HashMap<String, SymbolLiquidationTracker> = HashMap::new();
        let mut last_analysis = std::time::Instant::now();
        
        // This is a placeholder implementation - in a real system, this would
        // receive orderflow events and analyze them for liquidation patterns
        loop {
            // Periodic analysis
            if last_analysis.elapsed() >= Duration::from_secs(1) {
                let current_time = chrono::Utc::now().timestamp() as u64 * 1000;
                
                // Analyze all symbols for liquidation patterns
                for tracker in trackers.values_mut() {
                    if let Some(liquidation) = self.analyze_liquidation_pattern(tracker, current_time) {
                        if let Err(e) = self.sender.try_send(liquidation) {
                            debug!("Failed to send liquidation event: {}", e);
                        }
                    }
                }
                
                last_analysis = std::time::Instant::now();
            }

            sleep(Duration::from_millis(100)).await;
        }
    }

    pub fn process_orderflow_event(&mut self, event: &OrderflowEvent, trackers: &mut HashMap<String, SymbolLiquidationTracker>) -> Option<LiquidationEvent> {
        // Get or create tracker for this symbol
        let tracker = trackers
            .entry(event.symbol.clone())
            .or_insert_with(|| SymbolLiquidationTracker::new(event.symbol.clone()));
        
        // Add trade to tracker
        tracker.add_trade(event.clone());
        
        // Analyze for liquidation patterns
        self.analyze_liquidation_pattern(tracker, event.timestamp)
    }

    fn analyze_liquidation_pattern(&self, tracker: &mut SymbolLiquidationTracker, current_time: u64) -> Option<LiquidationEvent> {
        if tracker.recent_trades.len() < 10 {
            return None; // Need sufficient data
        }

        // Check for volume spike
        let recent_volume = self.calculate_recent_volume(tracker, current_time);
        let volume_ratio = if tracker.baseline_volume > 0.0 {
            recent_volume / tracker.baseline_volume
        } else {
            1.0
        };

        // Check for rapid price movement
        let price_movement = self.calculate_price_movement(tracker, current_time);
        
        // Calculate confidence score
        let confidence = self.calculate_liquidation_confidence(volume_ratio, price_movement);
        
        // Detect liquidation if confidence is high enough
        if confidence > 0.7 && volume_ratio > self.volume_spike_threshold {
            // Determine liquidation details
            let side = if price_movement > 0.0 { "SHORT" } else { "LONG" };
            let estimated_quantity = recent_volume * 0.8; // Estimate liquidated amount
            
            let liquidation = LiquidationEvent {
                symbol: tracker.symbol.clone(),
                timestamp: current_time,
                side: side.to_string(),
                price: tracker.last_price,
                quantity: estimated_quantity,
                is_forced: true,
                notional_value: tracker.last_price * estimated_quantity,
            };

            // Add to recent liquidations to avoid duplicate detection
            tracker.recent_liquidations.push_back(liquidation.clone());
            
            return Some(liquidation);
        }

        None
    }

    fn calculate_recent_volume(&self, tracker: &SymbolLiquidationTracker, current_time: u64) -> f64 {
        let window_start = current_time.saturating_sub(self.detection_window_ms);
        
        tracker.recent_trades.iter()
            .filter(|trade| trade.timestamp >= window_start)
            .map(|trade| trade.quantity)
            .sum()
    }

    fn calculate_price_movement(&self, tracker: &SymbolLiquidationTracker, current_time: u64) -> f64 {
        let window_start = current_time.saturating_sub(self.detection_window_ms);
        
        let relevant_trades: Vec<&OrderflowEvent> = tracker.recent_trades.iter()
            .filter(|trade| trade.timestamp >= window_start)
            .collect();

        if relevant_trades.len() < 2 {
            return 0.0;
        }

        let start_price = relevant_trades.first().unwrap().price;
        let end_price = relevant_trades.last().unwrap().price;
        
        (end_price - start_price) / start_price
    }

    fn calculate_liquidation_confidence(&self, volume_ratio: f64, price_movement: f64) -> f64 {
        let volume_score = if volume_ratio > self.volume_spike_threshold {
            ((volume_ratio - self.volume_spike_threshold) / self.volume_spike_threshold).min(1.0)
        } else {
            0.0
        };

        let price_score = if price_movement.abs() > self.price_movement_threshold {
            ((price_movement.abs() - self.price_movement_threshold) / self.price_movement_threshold).min(1.0)
        } else {
            0.0
        };

        // Combined confidence score
        (volume_score * 0.6 + price_score * 0.4).min(1.0)
    }

    pub fn process_binance_liquidation(&self, liquidation: LiquidationEvent) {
        // Process confirmed liquidation from Binance's force order stream
        if let Err(e) = self.sender.try_send(liquidation) {
            debug!("Failed to send Binance liquidation: {}", e);
        }
    }
}

impl SymbolLiquidationTracker {
    fn new(symbol: String) -> Self {
        Self {
            symbol,
            recent_trades: VecDeque::new(),
            recent_liquidations: VecDeque::new(),
            baseline_volume: 0.0,
            baseline_update_count: 0,
            last_price: 0.0,
            volume_window_ms: 60_000, // 1 minute baseline window
            max_trades_history: 1000,
        }
    }

    fn add_trade(&mut self, trade: OrderflowEvent) {
        self.recent_trades.push_back(trade.clone());
        self.last_price = trade.price;
        
        // Maintain trade history limit
        while self.recent_trades.len() > self.max_trades_history {
            self.recent_trades.pop_front();
        }

        // Update baseline volume (moving average)
        self.update_baseline_volume();
        
        // Cleanup old liquidations
        let cutoff_time = trade.timestamp.saturating_sub(300_000); // 5 minutes
        self.recent_liquidations.retain(|liq| liq.timestamp >= cutoff_time);
    }

    fn update_baseline_volume(&mut self) {
        if self.recent_trades.len() < 60 {
            return; // Need more data for baseline
        }

        // Calculate average volume over the baseline window
        let current_time = self.recent_trades.back().unwrap().timestamp;
        let window_start = current_time.saturating_sub(self.volume_window_ms);
        
        let window_trades: Vec<&OrderflowEvent> = self.recent_trades.iter()
            .filter(|trade| trade.timestamp >= window_start)
            .collect();

        if !window_trades.is_empty() {
            let total_volume: f64 = window_trades.iter().map(|t| t.quantity).sum();
            let new_baseline = total_volume / window_trades.len() as f64;
            
            // Exponential moving average
            if self.baseline_update_count == 0 {
                self.baseline_volume = new_baseline;
            } else {
                let alpha = 0.1; // Smoothing factor
                self.baseline_volume = alpha * new_baseline + (1.0 - alpha) * self.baseline_volume;
            }
            
            self.baseline_update_count += 1;
        }
    }

    pub fn get_statistics(&self) -> LiquidationStatistics {
        let recent_liquidation_count = self.recent_liquidations.len();
        let total_liquidated_volume: f64 = self.recent_liquidations.iter()
            .map(|liq| liq.quantity)
            .sum();

        LiquidationStatistics {
            symbol: self.symbol.clone(),
            baseline_volume: self.baseline_volume,
            recent_trades_count: self.recent_trades.len(),
            recent_liquidation_count,
            total_liquidated_volume,
            last_price: self.last_price,
        }
    }

    pub fn cleanup_old_data(&mut self, current_time: u64) {
        // Remove trades older than 5 minutes
        let cutoff_time = current_time.saturating_sub(300_000);
        self.recent_trades.retain(|trade| trade.timestamp >= cutoff_time);

        // Remove liquidations older than 10 minutes
        let liquidation_cutoff = current_time.saturating_sub(600_000);
        self.recent_liquidations.retain(|liq| liq.timestamp >= liquidation_cutoff);
    }
}

#[derive(Debug, Clone)]
pub struct LiquidationStatistics {
    pub symbol: String,
    pub baseline_volume: f64,
    pub recent_trades_count: usize,
    pub recent_liquidation_count: usize,
    pub total_liquidated_volume: f64,
    pub last_price: f64,
}

pub struct LiquidationAggregator {
    liquidations: HashMap<String, VecDeque<LiquidationEvent>>,
    max_liquidations_per_symbol: usize,
}

impl LiquidationAggregator {
    pub fn new() -> Self {
        Self {
            liquidations: HashMap::new(),
            max_liquidations_per_symbol: 1000,
        }
    }

    pub fn add_liquidation(&mut self, liquidation: LiquidationEvent) {
        let liquidations = self.liquidations
            .entry(liquidation.symbol.clone())
            .or_insert_with(VecDeque::new);
        
        liquidations.push_back(liquidation);
        
        // Maintain size limit
        while liquidations.len() > self.max_liquidations_per_symbol {
            liquidations.pop_front();
        }
    }

    pub fn get_recent_liquidations(&self, symbol: &str, count: usize) -> Vec<LiquidationEvent> {
        if let Some(liquidations) = self.liquidations.get(symbol) {
            liquidations.iter()
                .rev()
                .take(count)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_all_recent_liquidations(&self, count: usize) -> Vec<LiquidationEvent> {
        let mut all_liquidations = Vec::new();
        
        for liquidations in self.liquidations.values() {
            all_liquidations.extend(liquidations.iter().cloned());
        }
        
        // Sort by timestamp (most recent first)
        all_liquidations.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_liquidations.truncate(count);
        
        all_liquidations
    }

    pub fn get_liquidation_summary(&self, symbol: &str, duration_minutes: u32) -> LiquidationSummary {
        let now = chrono::Utc::now().timestamp() as u64 * 1000;
        let cutoff = now - (duration_minutes as u64 * 60 * 1000);
        
        let mut long_liquidations = 0;
        let mut short_liquidations = 0;
        let mut total_volume = 0.0;
        let mut total_notional = 0.0;
        
        if let Some(liquidations) = self.liquidations.get(symbol) {
            for liquidation in liquidations.iter().filter(|liq| liq.timestamp >= cutoff) {
                if liquidation.side == "LONG" {
                    long_liquidations += 1;
                } else {
                    short_liquidations += 1;
                }
                
                total_volume += liquidation.quantity;
                total_notional += liquidation.notional_value;
            }
        }
        
        LiquidationSummary {
            symbol: symbol.to_string(),
            duration_minutes,
            long_liquidations,
            short_liquidations,
            total_liquidations: long_liquidations + short_liquidations,
            total_volume,
            total_notional,
        }
    }

    pub fn cleanup_old_liquidations(&mut self, max_age_hours: u32) {
        let cutoff = chrono::Utc::now().timestamp() as u64 * 1000 - (max_age_hours as u64 * 60 * 60 * 1000);
        
        for liquidations in self.liquidations.values_mut() {
            liquidations.retain(|liq| liq.timestamp >= cutoff);
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiquidationSummary {
    pub symbol: String,
    pub duration_minutes: u32,
    pub long_liquidations: u32,
    pub short_liquidations: u32,
    pub total_liquidations: u32,
    pub total_volume: f64,
    pub total_notional: f64,
}