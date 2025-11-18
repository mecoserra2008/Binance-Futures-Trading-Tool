use tokio::sync::mpsc;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use anyhow::Result;
use tokio::time::{sleep, Duration};
use tracing::{info, error, debug};

use crate::data::{OrderflowEvent, OrderImbalance};

pub struct ImbalanceAnalyzer {
    sender: mpsc::Sender<OrderImbalance>,
    window_duration_seconds: u64,
    calculation_interval_ms: u64,
}

struct SymbolImbalanceTracker {
    symbol: String,
    trades: VecDeque<OrderflowEvent>,
    current_bid_volume: f64,
    current_ask_volume: f64,
    last_calculation_time: u64,
    window_duration_ms: u64,
}

impl ImbalanceAnalyzer {
    pub fn new(sender: mpsc::Sender<OrderImbalance>) -> Self {
        Self {
            sender,
            window_duration_seconds: 60,
            calculation_interval_ms: 1000, // Calculate every second
        }
    }

    pub async fn start(
        &self,
        orderflow_receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<OrderflowEvent>>>,
    ) -> Result<()> {
        info!("Starting imbalance analyzer");
        
        let mut trackers: HashMap<String, SymbolImbalanceTracker> = HashMap::new();
        let mut last_cleanup = std::time::Instant::now();
        
        loop {
            // Process incoming trades with timeout
            let mut receiver = orderflow_receiver.lock().await;
            
            match tokio::time::timeout(Duration::from_millis(100), receiver.recv()).await {
                Ok(Some(event)) => {
                    drop(receiver); // Release the lock early

                    debug!("Imbalance analyzer received event for {}: price={}, qty={}", event.symbol, event.price, event.quantity);

                    // Get or create tracker for this symbol
                    let tracker = trackers
                        .entry(event.symbol.clone())
                        .or_insert_with(|| SymbolImbalanceTracker::new(
                            event.symbol.clone(),
                            self.window_duration_seconds * 1000,
                        ));

                    // Process the trade
                    if let Some(imbalance) = tracker.process_trade(event) {
                        debug!("Sending imbalance update for {}: ratio={}", imbalance.symbol, imbalance.imbalance_ratio);
                        if let Err(e) = self.sender.try_send(imbalance) {
                            debug!("Failed to send imbalance update: {}", e);
                        }
                    }
                }
                Ok(None) => {
                    drop(receiver);
                    error!("Orderflow receiver channel closed");
                    break;
                }
                Err(_) => {
                    drop(receiver);
                    // Timeout occurred, continue with periodic tasks
                }
            }

            // Periodic cleanup and calculations
            if last_cleanup.elapsed() >= Duration::from_secs(10) {
                let current_time = chrono::Utc::now().timestamp() as u64 * 1000;
                
                for tracker in trackers.values_mut() {
                    tracker.cleanup_old_trades(current_time);
                    
                    // Force calculation if enough time has passed
                    if current_time - tracker.last_calculation_time >= self.calculation_interval_ms {
                        if let Some(imbalance) = tracker.calculate_current_imbalance(current_time) {
                            if let Err(e) = self.sender.try_send(imbalance) {
                                debug!("Failed to send periodic imbalance update: {}", e);
                            }
                        }
                    }
                }
                
                last_cleanup = std::time::Instant::now();
            }

            // Small sleep to prevent busy waiting
            sleep(Duration::from_millis(1)).await;
        }

        Ok(())
    }
}

impl SymbolImbalanceTracker {
    fn new(symbol: String, window_duration_ms: u64) -> Self {
        Self {
            symbol,
            trades: VecDeque::new(),
            current_bid_volume: 0.0,
            current_ask_volume: 0.0,
            last_calculation_time: 0,
            window_duration_ms,
        }
    }

    fn process_trade(&mut self, event: OrderflowEvent) -> Option<OrderImbalance> {
        let current_time = event.timestamp;
        
        // Add new trade
        self.trades.push_back(event.clone());

        // Update volume counters
        if event.is_buyer_maker {
            // Buyer is maker (passive), so this is a sell order hitting the bid
            self.current_ask_volume += event.quantity;
        } else {
            // Buyer is taker (aggressive), so this is a buy order hitting the ask
            self.current_bid_volume += event.quantity;
        }

        // Remove old trades outside the window
        self.cleanup_old_trades(current_time);

        // Calculate imbalance if enough time has passed
        if current_time - self.last_calculation_time >= 1000 { // Every second
            self.calculate_current_imbalance(current_time)
        } else {
            None
        }
    }

    fn cleanup_old_trades(&mut self, current_time: u64) {
        let cutoff_time = current_time.saturating_sub(self.window_duration_ms);
        
        while let Some(front_trade) = self.trades.front() {
            if front_trade.timestamp < cutoff_time {
                let old_trade = self.trades.pop_front().unwrap();
                
                // Subtract from volume counters
                if old_trade.is_buyer_maker {
                    self.current_ask_volume = (self.current_ask_volume - old_trade.quantity).max(0.0);
                } else {
                    self.current_bid_volume = (self.current_bid_volume - old_trade.quantity).max(0.0);
                }
            } else {
                break;
            }
        }
    }

    fn calculate_current_imbalance(&mut self, current_time: u64) -> Option<OrderImbalance> {
        self.last_calculation_time = current_time;
        
        let imbalance_ratio = if self.current_bid_volume + self.current_ask_volume > 0.0 {
            (self.current_bid_volume - self.current_ask_volume) / 
            (self.current_bid_volume + self.current_ask_volume)
        } else {
            0.0
        };

        Some(OrderImbalance {
            symbol: self.symbol.clone(),
            timestamp: current_time,
            bid_volume: self.current_bid_volume,
            ask_volume: self.current_ask_volume,
            imbalance_ratio,
            window_duration_seconds: self.window_duration_ms / 1000,
        })
    }

    pub fn get_statistics(&self) -> ImbalanceStatistics {
        let total_volume = self.current_bid_volume + self.current_ask_volume;
        let bid_percentage = if total_volume > 0.0 {
            self.current_bid_volume / total_volume * 100.0
        } else {
            50.0
        };
        let ask_percentage = if total_volume > 0.0 {
            self.current_ask_volume / total_volume * 100.0
        } else {
            50.0
        };

        ImbalanceStatistics {
            symbol: self.symbol.clone(),
            total_trades: self.trades.len(),
            bid_volume: self.current_bid_volume,
            ask_volume: self.current_ask_volume,
            total_volume,
            bid_percentage,
            ask_percentage,
            imbalance_ratio: (self.current_bid_volume - self.current_ask_volume) / total_volume.max(1.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImbalanceStatistics {
    pub symbol: String,
    pub total_trades: usize,
    pub bid_volume: f64,
    pub ask_volume: f64,
    pub total_volume: f64,
    pub bid_percentage: f64,
    pub ask_percentage: f64,
    pub imbalance_ratio: f64,
}

pub struct ImbalanceAggregator {
    current_imbalances: HashMap<String, OrderImbalance>,
    historical_data: HashMap<String, VecDeque<OrderImbalance>>,
    max_history_length: usize,
}

impl ImbalanceAggregator {
    pub fn new() -> Self {
        Self {
            current_imbalances: HashMap::new(),
            historical_data: HashMap::new(),
            max_history_length: 1000,
        }
    }

    pub fn update_imbalance(&mut self, imbalance: OrderImbalance) {
        let symbol = imbalance.symbol.clone();
        
        // Update current imbalance
        self.current_imbalances.insert(symbol.clone(), imbalance.clone());
        
        // Add to historical data
        let history = self.historical_data
            .entry(symbol)
            .or_insert_with(VecDeque::new);
        
        history.push_back(imbalance);
        
        // Maintain history limit
        while history.len() > self.max_history_length {
            history.pop_front();
        }
    }

    pub fn get_current_imbalance(&self, symbol: &str) -> Option<&OrderImbalance> {
        self.current_imbalances.get(symbol)
    }

    pub fn get_all_current_imbalances(&self) -> &HashMap<String, OrderImbalance> {
        &self.current_imbalances
    }

    pub fn get_historical_imbalances(&self, symbol: &str, count: usize) -> Vec<OrderImbalance> {
        if let Some(history) = self.historical_data.get(symbol) {
            history.iter()
                .rev()
                .take(count)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_average_imbalance(&self, symbol: &str, duration_minutes: u32) -> Option<f64> {
        if let Some(history) = self.historical_data.get(symbol) {
            let now = chrono::Utc::now().timestamp() as u64 * 1000;
            let cutoff = now - (duration_minutes as u64 * 60 * 1000);
            
            let recent_imbalances: Vec<f64> = history.iter()
                .filter(|imb| imb.timestamp >= cutoff)
                .map(|imb| imb.imbalance_ratio)
                .collect();
            
            if !recent_imbalances.is_empty() {
                Some(recent_imbalances.iter().sum::<f64>() / recent_imbalances.len() as f64)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn cleanup_old_data(&mut self, max_age_hours: u32) {
        let cutoff = chrono::Utc::now().timestamp() as u64 * 1000 - (max_age_hours as u64 * 60 * 60 * 1000);
        
        for history in self.historical_data.values_mut() {
            while let Some(front) = history.front() {
                if front.timestamp < cutoff {
                    history.pop_front();
                } else {
                    break;
                }
            }
        }
    }
}