// src/data/processing_engine.rs
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;
use std::collections::HashMap;
use anyhow::Result;
use tracing::{info, warn, error};

use crate::data::market_data::{OrderflowEvent, GuiUpdate, MarketDataManager};
use crate::analysis::{
    imbalance::ImbalanceAnalyzer,
    footprint::FootprintAnalyzer,
    liquidations::LiquidationDetector,
    volume_analysis::VolumeAnalyzer,
};

pub struct ProcessingEngine {
    // Analysis engines
    imbalance_analyzer: Arc<RwLock<ImbalanceAnalyzer>>,
    footprint_analyzer: Arc<RwLock<FootprintAnalyzer>>,
    liquidation_detector: Arc<RwLock<LiquidationDetector>>,
    volume_analyzer: Arc<RwLock<VolumeAnalyzer>>,
    
    // Channels for parallel processing
    gui_update_tx: mpsc::UnboundedSender<GuiUpdate>,
    
    // Performance metrics
    processed_events: Arc<RwLock<u64>>,
    processing_times: Arc<RwLock<Vec<std::time::Duration>>>,
}

impl ProcessingEngine {
    pub fn new(gui_update_tx: mpsc::UnboundedSender<GuiUpdate>) -> Self {
        Self {
            imbalance_analyzer: Arc::new(RwLock::new(ImbalanceAnalyzer::new(60))), // 60-second window
            footprint_analyzer: Arc::new(RwLock::new(FootprintAnalyzer::new())),
            liquidation_detector: Arc::new(RwLock::new(LiquidationDetector::new())),
            volume_analyzer: Arc::new(RwLock::new(VolumeAnalyzer::new())),
            gui_update_tx,
            processed_events: Arc::new(RwLock::new(0)),
            processing_times: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn start_processing(
        &self,
        mut market_data_rx: mpsc::UnboundedReceiver<OrderflowEvent>
    ) -> Result<()> {
        info!("Starting parallel data processing engine");

        // Create processing channels for each analyzer
        let (imbalance_tx, mut imbalance_rx) = mpsc::unbounded_channel::<OrderflowEvent>();
        let (footprint_tx, mut footprint_rx) = mpsc::unbounded_channel::<OrderflowEvent>();
        let (liquidation_tx, mut liquidation_rx) = mpsc::unbounded_channel::<OrderflowEvent>();
        let (volume_tx, mut volume_rx) = mpsc::unbounded_channel::<OrderflowEvent>();

        // Spawn parallel processing tasks
        self.spawn_imbalance_processor(imbalance_rx).await;
        self.spawn_footprint_processor(footprint_rx).await;
        self.spawn_liquidation_processor(liquidation_rx).await;
        self.spawn_volume_processor(volume_rx).await;

        // Main processing loop - distributes events to all analyzers
        while let Some(event) = market_data_rx.recv().await {
            let start_time = std::time::Instant::now();

            // Send to GUI immediately for raw orderflow display
            let _ = self.gui_update_tx.send(GuiUpdate::NewOrderflow(event.clone()));

            // Distribute to all analyzers in parallel
            let _ = imbalance_tx.send(event.clone());
            let _ = footprint_tx.send(event.clone());
            let _ = liquidation_tx.send(event.clone());
            let _ = volume_tx.send(event.clone());

            // Update performance metrics
            self.update_performance_metrics(start_time).await;
        }

        Ok(())
    }

    async fn spawn_imbalance_processor(&self, mut rx: mpsc::UnboundedReceiver<OrderflowEvent>) {
        let analyzer = self.imbalance_analyzer.clone();
        let gui_tx = self.gui_update_tx.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let mut analyzer = analyzer.write().await;
                if let Some(imbalance) = analyzer.process_trade(event) {
                    let _ = gui_tx.send(GuiUpdate::ImbalanceUpdate(imbalance));
                }
            }
        });
    }

    async fn spawn_footprint_processor(&self, mut rx: mpsc::UnboundedReceiver<OrderflowEvent>) {
        let analyzer = self.footprint_analyzer.clone();
        let gui_tx = self.gui_update_tx.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let mut analyzer = analyzer.write().await;
                if let Some(candle) = analyzer.process_trade(event) {
                    let _ = gui_tx.send(GuiUpdate::CandleUpdate(candle));
                }
            }
        });
    }

    async fn spawn_liquidation_processor(&self, mut rx: mpsc::UnboundedReceiver<OrderflowEvent>) {
        let detector = self.liquidation_detector.clone();
        let gui_tx = self.gui_update_tx.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let mut detector = detector.write().await;
                if let Some(liquidation) = detector.process_trade(event) {
                    let _ = gui_tx.send(GuiUpdate::NewLiquidation(liquidation));
                }
            }
        });
    }

    async fn spawn_volume_processor(&self, mut rx: mpsc::UnboundedReceiver<OrderflowEvent>) {
        let analyzer = self.volume_analyzer.clone();
        let gui_tx = self.gui_update_tx.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let mut analyzer = analyzer.write().await;
                
                // Check for big orderflow
                if let Some(percentage) = analyzer.is_big_orderflow(&event, 0.5) {
                    let _ = gui_tx.send(GuiUpdate::BigOrderflow(event.clone(), percentage));
                }

                // Update daily stats
                if let Some(stats) = analyzer.process_trade(&event) {
                    let _ = gui_tx.send(GuiUpdate::StatsUpdate(stats));
                }
            }
        });
    }

    async fn update_performance_metrics(&self, start_time: std::time::Instant) {
        let processing_time = start_time.elapsed();

        // Update processed events counter
        {
            let mut count = self.processed_events.write().await;
            *count += 1;
        }

        // Update processing times (keep last 1000)
        {
            let mut times = self.processing_times.write().await;
            times.push(processing_time);
            if times.len() > 1000 {
                times.remove(0);
            }
        }

        // Log performance metrics every 10000 events
        let count = *self.processed_events.read().await;
        if count % 10000 == 0 {
            let times = self.processing_times.read().await;
            let avg_time = times.iter().sum::<std::time::Duration>() / times.len() as u32;
            info!("Processed {} events, avg time: {:?}", count, avg_time);
        }
    }

    pub async fn get_performance_stats(&self) -> (u64, std::time::Duration) {
        let count = *self.processed_events.read().await;
        let times = self.processing_times.read().await;
        let avg_time = if times.is_empty() {
            std::time::Duration::ZERO
        } else {
            times.iter().sum::<std::time::Duration>() / times.len() as u32
        };
        (count, avg_time)
    }
}

// src/data/stream_manager.rs - Enhanced WebSocket with multiple streams
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;
use tracing::{info, error, warn};

use crate::data::market_data::OrderflowEvent;
use ordered_float::OrderedFloat;

pub struct StreamManager {
    market_data_tx: mpsc::UnboundedSender<OrderflowEvent>,
    active_streams: HashMap<String, String>,
}

impl StreamManager {
    pub fn new(market_data_tx: mpsc::UnboundedSender<OrderflowEvent>) -> Self {
        Self {
            market_data_tx,
            active_streams: HashMap::new(),
        }
    }

    pub async fn start_all_streams(&mut self) -> Result<()> {
        // Start multiple stream types in parallel
        let streams = vec![
            ("trade", self.start_trade_stream()),
            ("depth", self.start_depth_stream()),
            ("liquidation", self.start_liquidation_stream()),
        ];

        // Launch all streams concurrently
        let handles: Vec<_> = streams
            .into_iter()
            .map(|(name, stream_future)| {
                let name = name.to_string();
                tokio::spawn(async move {
                    if let Err(e) = stream_future.await {
                        error!("Stream {} failed: {}", name, e);
                    }
                })
            })
            .collect();

        // Wait for all streams to complete (they shouldn't in normal operation)
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    async fn start_trade_stream(&self) -> Result<()> {
        loop {
            if let Err(e) = self.connect_trade_stream().await {
                error!("Trade stream connection failed: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                info!("Reconnecting trade stream...");
            }
        }
    }

    async fn connect_trade_stream(&self) -> Result<()> {
        let url = "wss://fstream.binance.com/stream";
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Subscribe to all USDT symbol trade streams
        let symbols = self.get_usdt_symbols().await?;
        let streams: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@aggTrade", s.to_lowercase()))
            .collect();

        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": streams,
            "id": 1
        });

        write.send(Message::Text(subscribe_msg.to_string())).await?;
        info!("Subscribed to {} trade streams", symbols.len());

        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Err(e) = self.process_trade_message(&text).await {
                        warn!("Failed to process trade message: {}", e);
                    }
                }
                Ok(Message::Ping(data)) => {
                    write.send(Message::Pong(data)).await?;
                }
                Ok(Message::Close(_)) => {
                    warn!("Trade stream closed by server");
                    break;
                }
                Err(e) => {
                    error!("Trade stream error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn start_depth_stream(&self) -> Result<()> {
        // Implement order book depth stream for imbalance detection
        info!("Starting depth stream...");
        // Implementation would connect to depth stream and calculate bid/ask imbalances
        Ok(())
    }

    async fn start_liquidation_stream(&self) -> Result<()> {
        // Connect to forced liquidation stream
        let url = "wss://fstream.binance.com/ws/!forceOrder@arr";
        let (ws_stream, _) = connect_async(url).await?;
        let (_, mut read) = ws_stream.split();

        info!("Connected to liquidation stream");

        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Err(e) = self.process_liquidation_message(&text).await {
                        warn!("Failed to process liquidation message: {}", e);
                    }
                }
                Ok(Message::Close(_)) => {
                    warn!("Liquidation stream closed");
                    break;
                }
                Err(e) => {
                    error!("Liquidation stream error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn process_trade_message(&self, text: &str) -> Result<()> {
        let data: Value = serde_json::from_str(text)?;
        
        if let Some(stream_data) = data.get("data") {
            if let Some(event) = self.parse_trade_event(stream_data) {
                let _ = self.market_data_tx.send(event);
            }
        }

        Ok(())
    }

    async fn process_liquidation_message(&self, text: &str) -> Result<()> {
        let data: Value = serde_json::from_str(text)?;
        
        // Parse liquidation data and create synthetic orderflow event
        if let Some(liquidation) = data.get("o") {
            if let Some(event) = self.parse_liquidation_as_trade(liquidation) {
                let _ = self.market_data_tx.send(event);
            }
        }

        Ok(())
    }

    fn parse_trade_event(&self, data: &Value) -> Option<OrderflowEvent> {
        let symbol = data["s"].as_str()?.to_string();
        let price = data["p"].as_str()?.parse::<f64>().ok()?;
        let quantity = data["q"].as_str()?.parse::<f64>().ok()?;
        let timestamp = data["T"].as_u64()?;
        let is_buyer_maker = data["m"].as_bool()?;
        let trade_id = data["a"].as_u64()?;

        Some(OrderflowEvent {
            symbol,
            timestamp,
            price: OrderedFloat(price),
            quantity,
            is_buyer_maker,
            trade_id,
        })
    }

    fn parse_liquidation_as_trade(&self, data: &Value) -> Option<OrderflowEvent> {
        let symbol = data["s"].as_str()?.to_string();
        let price = data["p"].as_str()?.parse::<f64>().ok()?;
        let quantity = data["q"].as_str()?.parse::<f64>().ok()?;
        let timestamp = data["T"].as_u64()?;
        let side = data["S"].as_str()?;
        let trade_id = data["T"].as_u64()?;

        // Convert liquidation to synthetic trade event
        Some(OrderflowEvent {
            symbol,
            timestamp,
            price: OrderedFloat(price),
            quantity,
            is_buyer_maker: side == "SELL", // Liquidated longs appear as sells
            trade_id,
        })
    }

    async fn get_usdt_symbols(&self) -> Result<Vec<String>> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://fapi.binance.com/fapi/v1/exchangeInfo")
            .send()
            .await?;
        
        let exchange_info: Value = response.json().await?;
        let symbols = exchange_info["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|s| {
                let status = s["status"].as_str()?;
                let symbol = s["symbol"].as_str()?;
                let contract_type = s["contractType"].as_str()?;
                
                if status == "TRADING" 
                    && symbol.ends_with("USDT") 
                    && contract_type == "PERPETUAL" {
                    Some(symbol.to_string())
                } else {
                    None
                }
            })
            .collect();
        
        Ok(symbols)
    }
}

// src/gui/enhanced_panels.rs - Enhanced GUI with better performance
use egui::{Ui, ScrollArea, Color32, RichText, Vec2, Rect, Stroke};
use std::collections::{VecDeque, HashMap};
use plotters::prelude::*;
use plotters_egui::EguiBackend;

use crate::data::market_data::*;
use crate::gui::theme::TradingTheme;

pub struct EnhancedScreenerPanel {
    big_orders: VecDeque<BigOrderflowEvent>,
    volume_threshold: f64,
    selected_symbols: HashMap<String, bool>,
    sort_column: SortColumn,
    sort_ascending: bool,
    filter_text: String,
}

#[derive(PartialEq)]
enum SortColumn {
    Symbol,
    Size,
    Percentage,
    Time,
}

pub struct BigOrderflowEvent {
    pub event: OrderflowEvent,
    pub percentage: f64,
    pub timestamp: std::time::Instant,
    pub usd_value: f64,
}

impl EnhancedScreenerPanel {
    pub fn new() -> Self {
        Self {
            big_orders: VecDeque::new(),
            volume_threshold: 0.5,
            selected_symbols: HashMap::new(),
            sort_column: SortColumn::Time,
            sort_ascending: false,
            filter_text: String::new(),
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Enhanced header with controls
            self.show_header(ui);
            ui.separator();

            // Filters and sorting
            self.show_filters(ui);
            ui.separator();

            // Enhanced table with sorting
            self.show_orderflow_table(ui);
        });
    }

    fn show_header(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Large Orderflow Detection");
            
            ui.separator();
            
            ui.label("Min Volume %:");
            ui.add(egui::DragValue::new(&mut self.volume_threshold)
                .speed(0.1)
                .range(0.1..=10.0)
                .suffix("%"));

            ui.separator();
            
            ui.label(format!("Active Alerts: {}", self.big_orders.len()));
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear All").clicked() {
                    self.big_orders.clear();
                }
                
                if ui.button("Export CSV").clicked() {
                    self.export_to_csv();
                }
            });
        });
    }

    fn show_filters(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter_text);
            
            ui.separator();
            
            egui::ComboBox::from_label("Sort by")
                .selected_text(match self.sort_column {
                    SortColumn::Symbol => "Symbol",
                    SortColumn::Size => "Size",
                    SortColumn::Percentage => "Percentage",
                    SortColumn::Time => "Time",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.sort_column, SortColumn::Symbol, "Symbol");
                    ui.selectable_value(&mut self.sort_column, SortColumn::Size, "Size");
                    ui.selectable_value(&mut self.sort_column, SortColumn::Percentage, "Percentage");
                    ui.selectable_value(&mut self.sort_column, SortColumn::Time, "Time");
                });

            ui.checkbox(&mut self.sort_ascending, "Ascending");
        });
    }

    fn show_orderflow_table(&mut self, ui: &mut Ui) {
        // Filter and sort orders
        let filtered_orders: Vec<_> = self.big_orders
            .iter()
            .filter(|order| {
                if self.filter_text.is_empty() {
                    true
                } else {
                    order.event.symbol.to_lowercase().contains(&self.filter_text.to_lowercase())
                }
            })
            .collect();

        let mut sorted_orders = filtered_orders;
        sorted_orders.sort_by(|a, b| {
            let comparison = match self.sort_column {
                SortColumn::Symbol => a.event.symbol.cmp(&b.event.symbol),
                SortColumn::Size => a.usd_value.partial_cmp(&b.usd_value).unwrap(),
                SortColumn::Percentage => a.percentage.partial_cmp(&b.percentage).unwrap(),
                SortColumn::Time => a.timestamp.cmp(&b.timestamp),
            };

            if self.sort_ascending {
                comparison
            } else {
                comparison.reverse()
            }
        });

        // Table headers
        ui.horizontal(|ui| {
            self.header_button(ui, "Symbol", SortColumn::Symbol);
            ui.separator();
            self.header_button(ui, "Side", SortColumn::Size); // Size used for side sorting
            ui.separator();
            self.header_button(ui, "Size", SortColumn::Size);
            ui.separator();
            ui.label("Price");
            ui.separator();
            self.header_button(ui, "% Daily Vol", SortColumn::Percentage);
            ui.separator();
            self.header_button(ui, "Time", SortColumn::Time);
        });

        ui.separator();

        // Scrollable table
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .max_height(ui.available_height() - 50.0)
            .show(ui, |ui| {
                for order in sorted_orders {
                    self.draw_orderflow_row(ui, order);
                }
            });
    }

    fn header_button(&mut self, ui: &mut Ui, text: &str, column: SortColumn) {
        let is_active = self.sort_column == column;
        let mut button_text = text.to_string();
        
        if is_active {
            button_text.push_str(if self.sort_ascending { " ↑" } else { " ↓" });
        }

        if ui.button(RichText::new(button_text)
            .color(if is_active { TradingTheme::ACCENT } else { TradingTheme::TEXT_SECONDARY })
            .size(12.0))
            .clicked() 
        {
            if self.sort_column == column {
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = column;
                self.sort_ascending = false;
            }
        }
    }

    fn draw_orderflow_row(&self, ui: &mut Ui, order: &BigOrderflowEvent) {
        let side_color = if order.event.is_buyer_maker {
            TradingTheme::SELL_COLOR
        } else {
            TradingTheme::BUY_COLOR
        };

        let side_text = if order.event.is_buyer_maker { "SELL" } else { "BUY" };
        let age_seconds = order.timestamp.elapsed().as_secs();
        let alpha = (1.0 - (age_seconds as f32 / 300.0).min(0.8)).max(0.2);

        ui.horizontal(|ui| {
            ui.set_min_height(28.0);
            
            // Add background highlight for recent orders
            if age_seconds < 10 {
                let rect = ui.max_rect();
                ui.painter().rect_filled(
                    rect,
                    2.0,
                    TradingTheme::ACCENT.gamma_multiply(0.1)
                );
            }

            ui.label(RichText::new(&order.event.symbol)
                .color(TradingTheme::TEXT_PRIMARY.gamma_multiply(alpha))
                .size(13.0)
                .strong());
            ui.separator();

            ui.label(RichText::new(side_text)
                .color(side_color.gamma_multiply(alpha))
                .size(13.0)
                .strong());
            ui.separator();

            ui.label(RichText::new(self.format_usd_amount(order.usd_value))
                .color(TradingTheme::TEXT_PRIMARY.gamma_multiply(alpha))
                .size(13.0));
            ui.separator();

            ui.label(RichText::new(format!("{:.4}", order.event.price))
                .color(TradingTheme::TEXT_PRIMARY.gamma_multiply(alpha))
                .size(13.0));
            ui.separator();

            let percentage_color = if order.percentage > 2.0 {
                TradingTheme::ERROR
            } else if order.percentage > 1.0 {
                TradingTheme::WARNING
            } else {
                TradingTheme::TEXT_PRIMARY
            };
            
            ui.label(RichText::new(format!("{:.2}%", order.percentage))
                .color(percentage_color.gamma_multiply(alpha))
                .size(13.0)
                .strong());
            ui.separator();

            ui.label(RichText::new(format!("{}s", age_seconds))
                .color(TradingTheme::TEXT_MUTED.gamma_multiply(alpha))
                .size(11.0));
        });
        
        ui.separator();
    }

    fn format_usd_amount(&self, amount: f64) -> String {
        if amount >= 1_000_000.0 {
            format!("${:.1}M", amount / 1_000_000.0)
        } else if amount >= 1_000.0 {
            format!("${:.1}K", amount / 1_000.0)
        } else {
            format!("${:.0}", amount)
        }
    }

    fn export_to_csv(&self) {
        // Implementation for CSV export
        info!("Exporting orderflow data to CSV...");
    }

    pub fn add_big_orderflow(&mut self, event: OrderflowEvent, percentage: f64) {
        let usd_value = event.quantity * event.price.into_inner();
        
        self.big_orders.push_back(BigOrderflowEvent {
            event,
            percentage,
            usd_value,
            timestamp: std::time::Instant::now(),
        });

        // Keep only last 2000 events for performance
        if self.big_orders.len() > 2000 {
            self.big_orders.pop_front();
        }
    }
}

// Enhanced performance monitoring
pub struct PerformanceMonitor {
    fps_history: VecDeque<f32>,
    memory_usage: f64,
    last_memory_check: std::time::Instant,
    event_throughput: VecDeque<(std::time::Instant, u64)>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            fps_history: VecDeque::new(),
            memory_usage: 0.0,
            last_memory_check: std::time::Instant::now(),
            event_throughput: VecDeque::new(),
        }
    }

    pub fn update_fps(&mut self, fps: f32) {
        self.fps_history.push_back(fps);
        if self.fps_history.len() > 60 {
            self.fps_history.pop_front();
        }
    }

    pub fn record_events(&mut self, count: u64) {
        let now = std::time::Instant::now();
        self.event_throughput.push_back((now, count));
        
        // Keep last minute of data
        let cutoff = now - std::time::Duration::from_secs(60);
        while let Some((time, _)) = self.event_throughput.front() {
            if *time < cutoff {
                self.event_throughput.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn get_average_fps(&self) -> f32 {
        if self.fps_history.is_empty() {
            return 0.0;
        }
        self.fps_history.iter().sum::<f32>() / self.fps_history.len() as f32
    }

    pub fn get_events_per_second(&self) -> f64 {
        if self.event_throughput.len() < 2 {
            return 0.0;
        }

        let total_events: u64 = self.event_throughput.iter().map(|(_, count)| count).sum();
        let time_span = self.event_throughput.back().unwrap().0
            .duration_since(self.event_throughput.front().unwrap().0)
            .as_secs_f64();

        if time_span > 0.0 {
            total_events as f64 / time_span
        } else {
            0.0
        }
    }

    pub fn show_performance_panel(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.heading("Performance Monitor");
            
            ui.horizontal(|ui| {
                ui.label(format!("Average FPS: {:.1}", self.get_average_fps()));
                ui.separator();
                ui.label(format!("Events/sec: {:.0}", self.get_events_per_second()));
                ui.separator();
                ui.label(format!("Memory: {:.1} MB", self.memory_usage / 1024.0 / 1024.0));
            });

            // Simple FPS graph
            if !self.fps_history.is_empty() {
                let plot_points: Vec<_> = self.fps_history
                    .iter()
                    .enumerate()
                    .map(|(i, &fps)| [i as f64, fps as f64])
                    .collect();

                egui::plot::Plot::new("fps_plot")
                    .height(100.0)
                    .show_axes([false, true])
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            egui::plot::Line::new(plot_points)
                                .color(TradingTheme::ACCENT)
                                .width(2.0)
                        );
                    });
            }
        });
    }
}
