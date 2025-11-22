use eframe::egui;
use tokio::sync::mpsc;
use std::sync::Arc;
use std::collections::HashMap;
use anyhow::Result;

use crate::data::{OrderImbalance, LiquidationEvent, VolumeProfile, GuiUpdate, DatabaseManager, BigOrderflowAlert, OrderflowEvent, BinanceSymbols, DepthSnapshot};
use crate::analysis::volume_analysis::VolumeAnalyzer;
use super::{ScreenerTheme, ScreenerPanel, ImbalancePanel, FootprintPanel, LiquidationPanel};

#[derive(Debug, PartialEq)]
enum ActivePanel {
    Screener,
    Imbalance,
    Footprint,
    Liquidation,
}

pub struct ScreenerApp {
    // Panels
    screener_panel: ScreenerPanel,
    imbalance_panel: ImbalancePanel,
    footprint_panel: FootprintPanel,
    liquidation_panel: LiquidationPanel,
    
    // State
    active_panel: ActivePanel,
    
    // Data receivers
    imbalance_receiver: Option<mpsc::Receiver<OrderImbalance>>,
    liquidation_receiver: Option<mpsc::Receiver<LiquidationEvent>>,
    volume_receiver: Option<mpsc::Receiver<VolumeProfile>>,
    gui_update_receiver: Option<mpsc::Receiver<GuiUpdate>>,
    orderflow_receiver: Option<mpsc::Receiver<OrderflowEvent>>,
    depth_snapshot_receiver: Option<mpsc::Receiver<(String, DepthSnapshot)>>,
    
    // Database
    database: Arc<DatabaseManager>,
    
    // Application state
    connection_status: ConnectionStatus,
    last_update_time: std::time::Instant,
    frame_count: u64,
    fps: f32,

    // Subscribed symbols
    subscribed_symbols: Vec<String>,

    // Volume analyzer for generating alerts
    volume_analyzer: Option<VolumeAnalyzer>,
    big_orderflow_receiver: Option<mpsc::Receiver<BigOrderflowAlert>>,

    // Demo data generation
    last_demo_generation: std::time::Instant,
}

#[derive(Debug, Clone)]
struct ConnectionStatus {
    websocket_connected: bool,
    database_connected: bool,
    last_trade_time: Option<u64>,
    total_symbols: usize,
    active_symbols: usize,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self {
            websocket_connected: false,
            database_connected: true,
            last_trade_time: None,
            total_symbols: 0,
            active_symbols: 0,
        }
    }
}

impl ScreenerApp {
    pub async fn new(
        imbalance_receiver: mpsc::Receiver<OrderImbalance>,
        liquidation_receiver: mpsc::Receiver<LiquidationEvent>,
        volume_receiver: mpsc::Receiver<VolumeProfile>,
        gui_update_receiver: mpsc::Receiver<GuiUpdate>,
        orderflow_receiver: mpsc::Receiver<OrderflowEvent>,
        depth_snapshot_receiver: mpsc::Receiver<(String, DepthSnapshot)>,
        database: Arc<DatabaseManager>,
        subscribed_symbols: Vec<String>,
    ) -> Result<Self> {
        // Create channels for volume analyzer outputs
        let (volume_sender, volume_receiver_new) = mpsc::channel(1000);
        let (alert_sender, big_orderflow_receiver) = mpsc::channel(1000);

        // Create volume analyzer with Binance API URL
        let api_base_url = "https://fapi.binance.com".to_string();
        let mut volume_analyzer = VolumeAnalyzer::new(volume_sender, api_base_url);
        volume_analyzer.set_alert_sender(alert_sender);

        let symbols = if subscribed_symbols.is_empty() {
            BinanceSymbols::get_default_symbols()
        } else {
            subscribed_symbols.clone()
        };

        Ok(Self {
            screener_panel: ScreenerPanel::new(),
            imbalance_panel: ImbalancePanel::new(),
            footprint_panel: FootprintPanel::new_with_symbols(symbols.clone()),
            liquidation_panel: LiquidationPanel::new(),
            active_panel: ActivePanel::Screener,
            imbalance_receiver: Some(imbalance_receiver),
            liquidation_receiver: Some(liquidation_receiver),
            volume_receiver: Some(volume_receiver_new), // Use new receiver from volume analyzer
            gui_update_receiver: Some(gui_update_receiver),
            orderflow_receiver: Some(orderflow_receiver),
            depth_snapshot_receiver: Some(depth_snapshot_receiver),
            database,
            connection_status: ConnectionStatus::default(),
            last_update_time: std::time::Instant::now(),
            frame_count: 0,
            fps: 0.0,
            subscribed_symbols: symbols,
            volume_analyzer: Some(volume_analyzer),
            big_orderflow_receiver: Some(big_orderflow_receiver),
            last_demo_generation: std::time::Instant::now(),
        })
    }

    fn process_incoming_data(&mut self) {
        // Process imbalance updates
        if let Some(receiver) = &mut self.imbalance_receiver {
            let mut count = 0;
            while let Ok(imbalance) = receiver.try_recv() {
                count += 1;
                self.imbalance_panel.add_imbalance(imbalance);
            }
            if count > 0 {
                tracing::debug!("GUI received {} imbalance updates", count);
            }
        }

        // Process liquidation events
        if let Some(receiver) = &mut self.liquidation_receiver {
            let mut count = 0;
            while let Ok(liquidation) = receiver.try_recv() {
                count += 1;
                self.liquidation_panel.add_liquidation(liquidation);
            }
            if count > 0 {
                tracing::debug!("GUI received {} liquidation events", count);
            }
        }

        // Process volume profiles
        if let Some(receiver) = &mut self.volume_receiver {
            let mut count = 0;
            while let Ok(volume_profile) = receiver.try_recv() {
                count += 1;
                self.footprint_panel.add_volume_profile(volume_profile);
            }
            if count > 0 {
                tracing::debug!("GUI received {} volume profiles", count);
            }
        }

        // Process big orderflow alerts for screener
        if let Some(receiver) = &mut self.big_orderflow_receiver {
            let mut count = 0;
            while let Ok(alert) = receiver.try_recv() {
                count += 1;
                self.screener_panel.add_orderflow_alert(alert);
            }
            if count > 0 {
                tracing::debug!("GUI received {} big orderflow alerts", count);
            }
        }

        // Process orderflow events for real-time footprint
        if let Some(receiver) = &mut self.orderflow_receiver {
            let mut count = 0;
            while let Ok(orderflow_event) = receiver.try_recv() {
                count += 1;
                self.footprint_panel.add_orderflow_event(&orderflow_event);
            }
            if count > 0 {
                tracing::debug!("GUI received {} orderflow events for footprint", count);
            }
        }

        // Process depth snapshots for LOB heatmap
        if let Some(receiver) = &mut self.depth_snapshot_receiver {
            let mut count = 0;
            while let Ok((symbol, snapshot)) = receiver.try_recv() {
                count += 1;
                self.footprint_panel.add_depth_snapshot(symbol, snapshot);
            }
            if count > 0 {
                tracing::debug!("GUI received {} depth snapshots", count);
            }
        }

        // Process GUI updates
        if let Some(receiver) = &mut self.gui_update_receiver {
            while let Ok(update) = receiver.try_recv() {
                match update {
                    GuiUpdate::BigOrderflow(alert) => {
                        self.screener_panel.add_orderflow_alert(alert);
                    }
                    GuiUpdate::Imbalance(imbalance) => {
                        self.imbalance_panel.add_imbalance(imbalance);
                    }
                    GuiUpdate::Liquidation(liquidation) => {
                        self.liquidation_panel.add_liquidation(liquidation);
                    }
                    GuiUpdate::VolumeProfile(profile) => {
                        self.footprint_panel.add_volume_profile(profile);
                    }
                    GuiUpdate::DailyStats(_stats) => {
                        // Update connection status or other stats
                    }
                }
            }
        }
    }

    /// Generate demo data for testing the screener
    fn generate_demo_data(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Generate demo alerts every 3 seconds and keep max 10 alerts
        if self.last_demo_generation.elapsed().as_secs() >= 3 && self.screener_panel.get_alert_count() < 10 {

            let symbols = BinanceSymbols::get_high_volume_symbols();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            // Generate a random number for variety
            let random_offset = (timestamp % 1000) as f64 / 100.0;

            // Create some demo alerts
            for (i, symbol) in symbols.iter().take(4).enumerate() {
                let side = if (timestamp + i as u64) % 2 == 0 { "BUY" } else { "SELL" };
                let price = match symbol.as_str() {
                    "BTCUSDT" => 65000.0 + random_offset + (i as f64 * 50.0),
                    "ETHUSDT" => 3500.0 + random_offset + (i as f64 * 25.0),
                    "BNBUSDT" => 600.0 + random_offset + (i as f64 * 10.0),
                    "SOLUSDT" => 150.0 + random_offset + (i as f64 * 5.0),
                    _ => 100.0 + random_offset + (i as f64 * 5.0),
                };
                let quantity = 25.0 + random_offset + (i as f64 * 15.0);
                let notional_value = price * quantity;

                let demo_alert = BigOrderflowAlert {
                    symbol: symbol.clone(),
                    timestamp: timestamp + (i as u64 * 1000), // Slight time offset
                    side: side.to_string(),
                    price,
                    quantity,
                    percentage_of_daily: 1.5 + random_offset + (i as f64 * 0.3), // 1.5-3.5% of daily volume
                    notional_value,
                };

                self.screener_panel.add_orderflow_alert(demo_alert);
                tracing::debug!("Generated demo alert for {}: {} {} @ {:.2}", symbol, side, quantity, price);
            }

            // Update the last generation time
            self.last_demo_generation = std::time::Instant::now();
            tracing::info!("Generated {} demo alerts. Total alerts: {}", 4, self.screener_panel.get_alert_count());
        }
    }

    /// Force generate initial demo data for immediate testing
    fn force_generate_initial_demo_data(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        tracing::info!("Force generating initial demo data for screener testing");

        // Generate immediate demo alerts
        let demo_alerts = vec![
            BigOrderflowAlert {
                symbol: "BTCUSDT".to_string(),
                timestamp,
                side: "BUY".to_string(),
                price: 67500.0,
                quantity: 150.0,
                percentage_of_daily: 5.2,
                notional_value: 67500.0 * 150.0,
            },
            BigOrderflowAlert {
                symbol: "ETHUSDT".to_string(),
                timestamp: timestamp + 1000,
                side: "SELL".to_string(),
                price: 3650.0,
                quantity: 200.0,
                percentage_of_daily: 3.8,
                notional_value: 3650.0 * 200.0,
            },
            BigOrderflowAlert {
                symbol: "BNBUSDT".to_string(),
                timestamp: timestamp + 2000,
                side: "BUY".to_string(),
                price: 620.0,
                quantity: 80.0,
                percentage_of_daily: 2.1,
                notional_value: 620.0 * 80.0,
            },
        ];

        for alert in demo_alerts {
            tracing::debug!("Adding demo alert: {} {} {} @ {}", alert.symbol, alert.side, alert.quantity, alert.price);
            self.screener_panel.add_orderflow_alert(alert);
        }

        tracing::info!("Added {} initial demo alerts. Total: {}", 3, self.screener_panel.get_alert_count());
    }

    fn update_fps(&mut self) {
        self.frame_count += 1;
        let elapsed = self.last_update_time.elapsed();
        
        if elapsed.as_secs() >= 1 {
            self.fps = self.frame_count as f32 / elapsed.as_secs_f32();
            self.frame_count = 0;
            self.last_update_time = std::time::Instant::now();
        }
    }

    fn draw_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Connection status
            let ws_color = if self.connection_status.websocket_connected {
                ScreenerTheme::BUY_COLOR
            } else {
                ScreenerTheme::SELL_COLOR
            };

            let db_color = if self.connection_status.database_connected {
                ScreenerTheme::BUY_COLOR
            } else {
                ScreenerTheme::SELL_COLOR
            };

            ui.colored_label(ws_color, "●");
            ui.label("WebSocket");
            
            ui.separator();
            
            ui.colored_label(db_color, "●");
            ui.label("Database");
            
            ui.separator();
            
            // Symbol count
            ui.label(format!(
                "Symbols: {}/{}",
                self.connection_status.active_symbols,
                self.connection_status.total_symbols
            ));
            
            ui.separator();
            
            // Last trade time
            if let Some(last_trade) = self.connection_status.last_trade_time {
                let elapsed = chrono::Utc::now().timestamp() as u64 * 1000 - last_trade;
                ui.label(format!("Last trade: {}ms ago", elapsed));
            } else {
                ui.label("Last trade: N/A");
            }
            
            // Right-aligned content
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // FPS counter
                ui.label(format!("FPS: {:.1}", self.fps));
                
                ui.separator();
                
                // Current time
                let now = chrono::Local::now();
                ui.label(now.format("%H:%M:%S").to_string());
            });
        });
    }

    fn draw_panel_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_panel, ActivePanel::Screener, "📊 Screener");
            ui.selectable_value(&mut self.active_panel, ActivePanel::Imbalance, "⚖️ Imbalance");
            ui.selectable_value(&mut self.active_panel, ActivePanel::Footprint, "📈 Footprint");
            ui.selectable_value(&mut self.active_panel, ActivePanel::Liquidation, "💥 Liquidations");
        });
    }

    fn draw_active_panel(&mut self, ui: &mut egui::Ui) {
        match self.active_panel {
            ActivePanel::Screener => {
                self.screener_panel.show(ui);
            }
            ActivePanel::Imbalance => {
                self.imbalance_panel.show(ui);
            }
            ActivePanel::Footprint => {
                self.footprint_panel.show(ui);
            }
            ActivePanel::Liquidation => {
                self.liquidation_panel.show(ui);
            }
        }
    }
}

impl eframe::App for ScreenerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply dark theme
        ScreenerTheme::apply_dark_theme(ctx);
        
        // Process incoming data
        self.process_incoming_data();

        // Demo data generation DISABLED - using real Binance data from VolumeAnalyzer
        // self.generate_demo_data();

        // Force generate some initial data for testing - DISABLED
        // if self.screener_panel.get_alert_count() == 0 && self.frame_count < 10 {
        //     self.force_generate_initial_demo_data();
        // }

        // Update performance metrics
        self.update_fps();
        
        // Request repaint for smooth updates
        ctx.request_repaint();
        
        // Main panel
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Status bar at the top
                ui.horizontal(|ui| {
                    ui.set_height(25.0);
                    self.draw_status_bar(ui);
                });
                
                ui.separator();
                
                // Panel tabs
                ui.horizontal(|ui| {
                    ui.set_height(30.0);
                    self.draw_panel_tabs(ui);
                });
                
                ui.separator();
                
                // Active panel content
                ui.vertical(|ui| {
                    self.draw_active_panel(ui);
                });
            });
        });
        
        // Debug window (optional, for development)
        #[cfg(debug_assertions)]
        {
            egui::Window::new("Debug Info")
                .default_size([300.0, 200.0])
                .show(ctx, |ui| {
                    ui.label(format!("FPS: {:.1}", self.fps));
                    ui.label(format!("Frame: {}", self.frame_count));
                    ui.label(format!("Active Panel: {:?}", self.active_panel));
                    
                    ui.separator();
                    
                    ui.label("Connection Status:");
                    ui.label(format!("  WebSocket: {}", self.connection_status.websocket_connected));
                    ui.label(format!("  Database: {}", self.connection_status.database_connected));
                    ui.label(format!("  Symbols: {}/{}", 
                        self.connection_status.active_symbols,
                        self.connection_status.total_symbols
                    ));
                    
                    ui.separator();
                    
                    ui.label("Panel Data:");
                    ui.label(format!("  Orderflow alerts: {}", self.screener_panel.get_alert_count()));
                    ui.label(format!("  Imbalances: {}", self.imbalance_panel.get_symbol_count()));
                    ui.label(format!("  Liquidations: {}", self.liquidation_panel.get_liquidation_count()));
                    ui.label(format!("  Volume profiles: {}", self.footprint_panel.get_profile_count()));
                });
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Cleanup resources if needed
        tracing::info!("Application shutting down");
    }
}