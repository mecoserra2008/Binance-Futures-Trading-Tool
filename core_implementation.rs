// Cargo.toml
[package]
name = "binance-orderflow-screener"
version = "0.1.0"
edition = "2021"

[dependencies]
egui = "0.24"
eframe = "0.24"
tokio = { version = "1.0", features = ["full"] }
tokio-tungstenite = "0.20"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.11", features = ["json"] }
rusqlite = { version = "0.29", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
ordered-float = "3.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
url = "2.0"
futures-util = "0.3"
std-collections = "1.0"

// src/main.rs
use eframe::egui;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, error};

mod config;
mod data;
mod analysis;
mod gui;
mod utils;

use gui::app::ScreenerApp;
use data::websocket::WebSocketManager;
use data::market_data::MarketDataManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    info!("Starting Binance Orderflow Screener");

    // Create application channels
    let (market_data_tx, market_data_rx) = mpsc::unbounded_channel();
    let (gui_update_tx, gui_update_rx) = mpsc::unbounded_channel();

    // Initialize market data manager
    let market_data_manager = Arc::new(MarketDataManager::new().await?);
    
    // Start WebSocket manager in background
    let ws_manager = WebSocketManager::new(market_data_tx.clone());
    tokio::spawn(async move {
        if let Err(e) = ws_manager.start().await {
            error!("WebSocket manager error: {}", e);
        }
    });

    // Start market data processing
    let data_manager = market_data_manager.clone();
    tokio::spawn(async move {
        data_manager.process_market_data(market_data_rx, gui_update_tx).await;
    });

    // Launch GUI on main thread
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1400.0, 900.0)),
        min_window_size: Some(egui::vec2(800.0, 600.0)),
        dark_mode: Some(true),
        ..Default::default()
    };

    let app = ScreenerApp::new(market_data_manager, gui_update_rx);
    
    eframe::run_native(
        "Binance Orderflow Screener",
        options,
        Box::new(|_cc| Box::new(app)),
    )
    .map_err(|e| anyhow::anyhow!("GUI error: {}", e))
}

// src/data/mod.rs
pub mod websocket;
pub mod market_data;
pub mod database;

// src/data/market_data.rs
use serde::{Deserialize, Serialize};
use ordered_float::OrderedFloat;
use std::collections::{BTreeMap, HashMap};
use tokio::sync::{mpsc, RwLock};
use chrono::{DateTime, Utc};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderflowEvent {
    pub symbol: String,
    pub timestamp: u64,
    pub price: OrderedFloat<f64>,
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
    pub price_levels: BTreeMap<OrderedFloat<f64>, VolumeAtPrice>,
    pub total_volume: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub vwap: f64,
}

#[derive(Debug, Clone)]
pub struct OrderImbalance {
    pub symbol: String,
    pub timestamp: u64,
    pub bid_volume: f64,
    pub ask_volume: f64,
    pub imbalance_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct LiquidationEvent {
    pub symbol: String,
    pub timestamp: u64,
    pub side: String,
    pub price: OrderedFloat<f64>,
    pub quantity: f64,
    pub is_forced: bool,
}

#[derive(Debug, Clone)]
pub struct CandleData {
    pub symbol: String,
    pub timestamp: u64,
    pub open: OrderedFloat<f64>,
    pub high: OrderedFloat<f64>,
    pub low: OrderedFloat<f64>,
    pub close: OrderedFloat<f64>,
    pub volume: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub volume_profile: VolumeProfile,
}

#[derive(Debug, Clone)]
pub struct DailyStats {
    pub symbol: String,
    pub date: String,
    pub avg_volume: f64,
    pub total_volume: f64,
    pub price_change: f64,
}

#[derive(Debug, Clone)]
pub enum GuiUpdate {
    NewOrderflow(OrderflowEvent),
    BigOrderflow(OrderflowEvent, f64), // Event and percentage of daily volume
    ImbalanceUpdate(OrderImbalance),
    NewLiquidation(LiquidationEvent),
    CandleUpdate(CandleData),
    StatsUpdate(DailyStats),
}

pub struct MarketDataManager {
    pub symbols: RwLock<Vec<String>>,
    pub daily_stats: RwLock<HashMap<String, DailyStats>>,
    pub current_candles: RwLock<HashMap<String, CandleData>>,
    pub recent_trades: RwLock<HashMap<String, Vec<OrderflowEvent>>>,
    database: crate::data::database::Database,
}

impl MarketDataManager {
    pub async fn new() -> Result<Self> {
        let database = crate::data::database::Database::new("./data/screener.db").await?;
        
        // Get active USDT futures symbols
        let symbols = Self::fetch_active_symbols().await?;
        
        Ok(Self {
            symbols: RwLock::new(symbols),
            daily_stats: RwLock::new(HashMap::new()),
            current_candles: RwLock::new(HashMap::new()),
            recent_trades: RwLock::new(HashMap::new()),
            database,
        })
    }

    async fn fetch_active_symbols() -> Result<Vec<String>> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://fapi.binance.com/fapi/v1/exchangeInfo")
            .send()
            .await?;
        
        let exchange_info: serde_json::Value = response.json().await?;
        let symbols = exchange_info["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|s| {
                let status = s["status"].as_str()?;
                let symbol = s["symbol"].as_str()?;
                if status == "TRADING" && symbol.ends_with("USDT") {
                    Some(symbol.to_string())
                } else {
                    None
                }
            })
            .collect();
        
        Ok(symbols)
    }

    pub async fn process_market_data(
        &self,
        mut market_data_rx: mpsc::UnboundedReceiver<OrderflowEvent>,
        gui_update_tx: mpsc::UnboundedSender<GuiUpdate>,
    ) {
        while let Some(event) = market_data_rx.recv().await {
            self.process_orderflow_event(event.clone(), &gui_update_tx).await;
        }
    }

    async fn process_orderflow_event(
        &self,
        event: OrderflowEvent,
        gui_update_tx: &mpsc::UnboundedSender<GuiUpdate>,
    ) {
        // Send to GUI
        let _ = gui_update_tx.send(GuiUpdate::NewOrderflow(event.clone()));

        // Check if it's a big orderflow event
        if let Some(daily_stats) = self.daily_stats.read().await.get(&event.symbol) {
            let volume_threshold = daily_stats.avg_volume * 0.005; // 0.5%
            if event.quantity * event.price.into_inner() > volume_threshold {
                let percentage = (event.quantity * event.price.into_inner()) / daily_stats.avg_volume * 100.0;
                let _ = gui_update_tx.send(GuiUpdate::BigOrderflow(event.clone(), percentage));
            }
        }

        // Update recent trades
        let mut recent_trades = self.recent_trades.write().await;
        let trades = recent_trades.entry(event.symbol.clone()).or_insert_with(Vec::new);
        trades.push(event.clone());
        
        // Keep only last 1000 trades per symbol
        if trades.len() > 1000 {
            trades.drain(0..trades.len() - 1000);
        }

        // Update current candle
        self.update_current_candle(event).await;
    }

    async fn update_current_candle(&self, event: OrderflowEvent) {
        let mut candles = self.current_candles.write().await;
        let candle = candles.entry(event.symbol.clone()).or_insert_with(|| {
            CandleData {
                symbol: event.symbol.clone(),
                timestamp: (event.timestamp / 60000) * 60000, // Round to minute
                open: event.price,
                high: event.price,
                low: event.price,
                close: event.price,
                volume: 0.0,
                buy_volume: 0.0,
                sell_volume: 0.0,
                volume_profile: VolumeProfile {
                    price_levels: BTreeMap::new(),
                    total_volume: 0.0,
                    buy_volume: 0.0,
                    sell_volume: 0.0,
                    vwap: 0.0,
                },
            }
        });

        // Update OHLC
        if event.price > candle.high {
            candle.high = event.price;
        }
        if event.price < candle.low {
            candle.low = event.price;
        }
        candle.close = event.price;

        // Update volume
        let volume = event.quantity * event.price.into_inner();
        candle.volume += volume;
        
        if event.is_buyer_maker {
            candle.sell_volume += volume;
        } else {
            candle.buy_volume += volume;
        }

        // Update volume profile
        let price_level = candle.volume_profile.price_levels
            .entry(event.price)
            .or_insert_with(|| VolumeAtPrice {
                buy_volume: 0.0,
                sell_volume: 0.0,
                total_volume: 0.0,
                trade_count: 0,
            });

        price_level.total_volume += volume;
        price_level.trade_count += 1;
        
        if event.is_buyer_maker {
            price_level.sell_volume += volume;
        } else {
            price_level.buy_volume += volume;
        }

        // Update profile totals
        candle.volume_profile.total_volume += volume;
        if event.is_buyer_maker {
            candle.volume_profile.sell_volume += volume;
        } else {
            candle.volume_profile.buy_volume += volume;
        }

        // Calculate VWAP
        let mut total_volume_weighted = 0.0;
        let mut total_volume = 0.0;
        
        for (price, vol_at_price) in &candle.volume_profile.price_levels {
            total_volume_weighted += price.into_inner() * vol_at_price.total_volume;
            total_volume += vol_at_price.total_volume;
        }
        
        if total_volume > 0.0 {
            candle.volume_profile.vwap = total_volume_weighted / total_volume;
        }
    }

    pub async fn get_symbols(&self) -> Vec<String> {
        self.symbols.read().await.clone()
    }

    pub async fn get_daily_stats(&self, symbol: &str) -> Option<DailyStats> {
        self.daily_stats.read().await.get(symbol).cloned()
    }

    pub async fn get_current_candle(&self, symbol: &str) -> Option<CandleData> {
        self.current_candles.read().await.get(symbol).cloned()
    }
}

// src/data/websocket.rs
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use serde_json::Value;
use crate::data::market_data::OrderflowEvent;
use ordered_float::OrderedFloat;
use anyhow::Result;
use tracing::{info, error, warn};

pub struct WebSocketManager {
    market_data_tx: mpsc::UnboundedSender<OrderflowEvent>,
}

impl WebSocketManager {
    pub fn new(market_data_tx: mpsc::UnboundedSender<OrderflowEvent>) -> Self {
        Self { market_data_tx }
    }

    pub async fn start(&self) -> Result<()> {
        loop {
            if let Err(e) = self.connect_and_stream().await {
                error!("WebSocket connection failed: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                info!("Attempting to reconnect...");
            }
        }
    }

    async fn connect_and_stream(&self) -> Result<()> {
        let url = "wss://fstream.binance.com/ws/!ticker@arr";
        info!("Connecting to Binance WebSocket: {}", url);
        
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Subscribe to all symbol streams
        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": ["!miniTicker@arr"],
            "id": 1
        });
        
        write.send(Message::Text(subscribe_msg.to_string())).await?;
        info!("Subscribed to all symbol mini ticker stream");

        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Err(e) = self.process_message(&text).await {
                        warn!("Failed to process message: {}", e);
                    }
                }
                Ok(Message::Ping(data)) => {
                    write.send(Message::Pong(data)).await?;
                }
                Ok(Message::Close(_)) => {
                    warn!("WebSocket connection closed by server");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn process_message(&self, text: &str) -> Result<()> {
        let data: Value = serde_json::from_str(text)?;
        
        // Handle array of mini ticker data
        if let Some(tickers) = data.as_array() {
            for ticker in tickers {
                if let Some(event) = self.parse_ticker_event(ticker) {
                    let _ = self.market_data_tx.send(event);
                }
            }
        } else if let Some(event) = self.parse_ticker_event(&data) {
            let _ = self.market_data_tx.send(event);
        }

        Ok(())
    }

    fn parse_ticker_event(&self, data: &Value) -> Option<OrderflowEvent> {
        let symbol = data["s"].as_str()?.to_string();
        let price = data["c"].as_str()?.parse::<f64>().ok()?;
        let volume = data["v"].as_str()?.parse::<f64>().ok()?;
        let timestamp = data["E"].as_u64()?;

        // Simulate buy/sell based on price change
        let price_change = data["P"].as_str()?.parse::<f64>().ok()?;
        let is_buyer_maker = price_change < 0.0;

        Some(OrderflowEvent {
            symbol,
            timestamp,
            price: OrderedFloat(price),
            quantity: volume,
            is_buyer_maker,
            trade_id: timestamp, // Using timestamp as trade_id for simplicity
        })
    }
}

// src/data/database.rs
use rusqlite::{Connection, Result};
use tokio::task;
use std::sync::{Arc, Mutex};
use crate::data::market_data::{CandleData, OrderImbalance, LiquidationEvent, DailyStats};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub async fn new(db_path: &str) -> anyhow::Result<Self> {
        let conn = task::spawn_blocking({
            let db_path = db_path.to_string();
            move || -> Result<Connection> {
                let conn = Connection::open(db_path)?;
                
                // Create tables
                conn.execute_batch("
                    CREATE TABLE IF NOT EXISTS candles (
                        id INTEGER PRIMARY KEY,
                        symbol TEXT NOT NULL,
                        timestamp INTEGER NOT NULL,
                        open_price REAL NOT NULL,
                        high_price REAL NOT NULL,
                        low_price REAL NOT NULL,
                        close_price REAL NOT NULL,
                        volume REAL NOT NULL,
                        buy_volume REAL NOT NULL,
                        sell_volume REAL NOT NULL,
                        timeframe TEXT NOT NULL,
                        UNIQUE(symbol, timestamp, timeframe)
                    );

                    CREATE TABLE IF NOT EXISTS volume_profile (
                        id INTEGER PRIMARY KEY,
                        symbol TEXT NOT NULL,
                        timestamp INTEGER NOT NULL,
                        price_level REAL NOT NULL,
                        buy_volume REAL NOT NULL,
                        sell_volume REAL NOT NULL,
                        total_volume REAL NOT NULL,
                        timeframe TEXT NOT NULL
                    );

                    CREATE TABLE IF NOT EXISTS order_imbalances (
                        id INTEGER PRIMARY KEY,
                        symbol TEXT NOT NULL,
                        timestamp INTEGER NOT NULL,
                        bid_volume REAL NOT NULL,
                        ask_volume REAL NOT NULL,
                        imbalance_ratio REAL NOT NULL
                    );

                    CREATE TABLE IF NOT EXISTS liquidations (
                        id INTEGER PRIMARY KEY,
                        symbol TEXT NOT NULL,
                        timestamp INTEGER NOT NULL,
                        side TEXT NOT NULL,
                        price REAL NOT NULL,
                        quantity REAL NOT NULL,
                        is_forced INTEGER NOT NULL
                    );

                    CREATE TABLE IF NOT EXISTS daily_stats (
                        id INTEGER PRIMARY KEY,
                        symbol TEXT NOT NULL,
                        date TEXT NOT NULL,
                        avg_volume REAL NOT NULL,
                        total_volume REAL NOT NULL,
                        UNIQUE(symbol, date)
                    );
                ")?;
                
                Ok(conn)
            }
        }).await??;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn save_candle(&self, candle: &CandleData) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        let candle = candle.clone();
        
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = conn.lock().unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO candles 
                (symbol, timestamp, open_price, high_price, low_price, close_price, 
                 volume, buy_volume, sell_volume, timeframe) 
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                [
                    candle.symbol.as_str(),
                    &candle.timestamp.to_string(),
                    &candle.open.to_string(),
                    &candle.high.to_string(),
                    &candle.low.to_string(),
                    &candle.close.to_string(),
                    &candle.volume.to_string(),
                    &candle.buy_volume.to_string(),
                    &candle.sell_volume.to_string(),
                    "1m",
                ],
            )?;
            Ok(())
        }).await??;

        Ok(())
    }
}

// Continue with GUI implementation in next artifact...