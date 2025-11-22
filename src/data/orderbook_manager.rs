use tokio::sync::mpsc;
use std::collections::HashMap;
use anyhow::Result;
use tracing::{info, debug, warn};

use crate::data::orderbook::{OrderBook, DepthUpdate, DepthSnapshot, DepthHistory, TimedDepthSnapshot};

pub struct OrderBookManager {
    orderbooks: HashMap<String, OrderBook>,
    depth_histories: HashMap<String, DepthHistory>,
    depth_receiver: mpsc::Receiver<DepthUpdate>,

    // Channels to send processed data to GUI
    snapshot_sender: mpsc::Sender<(String, DepthSnapshot)>,

    max_levels: usize,  // Max depth levels to maintain
    snapshot_interval_ms: u64,  // How often to snapshot for history
    tick_size: f64,  // Default tick size for aggregation
}

impl OrderBookManager {
    pub fn new(
        depth_receiver: mpsc::Receiver<DepthUpdate>,
        snapshot_sender: mpsc::Sender<(String, DepthSnapshot)>,
    ) -> Self {
        Self {
            orderbooks: HashMap::new(),
            depth_histories: HashMap::new(),
            depth_receiver,
            snapshot_sender,
            max_levels: 100,
            snapshot_interval_ms: 100,
            tick_size: 0.01,
        }
    }

    pub fn with_config(
        depth_receiver: mpsc::Receiver<DepthUpdate>,
        snapshot_sender: mpsc::Sender<(String, DepthSnapshot)>,
        max_levels: usize,
        snapshot_interval_ms: u64,
        tick_size: f64,
    ) -> Self {
        Self {
            orderbooks: HashMap::new(),
            depth_histories: HashMap::new(),
            depth_receiver,
            snapshot_sender,
            max_levels,
            snapshot_interval_ms,
            tick_size,
        }
    }

    pub async fn start(mut self) {
        info!("OrderBookManager started");

        let mut snapshot_timer = tokio::time::interval(
            std::time::Duration::from_millis(self.snapshot_interval_ms)
        );

        loop {
            tokio::select! {
                Some(update) = self.depth_receiver.recv() => {
                    self.process_depth_update(update);
                }
                _ = snapshot_timer.tick() => {
                    self.capture_snapshots();
                }
            }
        }
    }

    fn process_depth_update(&mut self, update: DepthUpdate) {
        let symbol = update.symbol.clone();

        let orderbook = self.orderbooks
            .entry(symbol.clone())
            .or_insert_with(|| {
                debug!("Creating new orderbook for {}", symbol);
                OrderBook::new(symbol.clone())
            });

        orderbook.apply_update(update);
    }

    fn capture_snapshots(&mut self) {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;

        for (symbol, orderbook) in &self.orderbooks {
            let snapshot = orderbook.get_depth_snapshot(self.max_levels);

            // Store in history
            let history = self.depth_histories
                .entry(symbol.clone())
                .or_insert_with(|| {
                    DepthHistory::new(symbol.clone(), 500)  // 50 seconds at 100ms
                });

            // Create timed snapshot with aggregation
            let timed_snapshot = TimedDepthSnapshot::new(timestamp, snapshot.clone(), self.tick_size);
            history.add_snapshot(timed_snapshot);

            // Send to GUI
            if let Err(e) = self.snapshot_sender.try_send((symbol.clone(), snapshot)) {
                debug!("Failed to send depth snapshot for {}: {}", symbol, e);
            }
        }
    }

    /// Get current orderbook for a symbol
    pub fn get_orderbook(&self, symbol: &str) -> Option<&OrderBook> {
        self.orderbooks.get(symbol)
    }

    /// Get depth history for a symbol
    pub fn get_depth_history(&self, symbol: &str) -> Option<&DepthHistory> {
        self.depth_histories.get(symbol)
    }

    /// Get number of tracked symbols
    pub fn symbol_count(&self) -> usize {
        self.orderbooks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orderbook_manager_creation() {
        let (depth_tx, depth_rx) = mpsc::channel(100);
        let (snapshot_tx, _snapshot_rx) = mpsc::channel(100);

        let manager = OrderBookManager::new(depth_rx, snapshot_tx);
        assert_eq!(manager.symbol_count(), 0);
    }
}
