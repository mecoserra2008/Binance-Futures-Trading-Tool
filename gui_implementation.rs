// src/gui/mod.rs
pub mod app;
pub mod theme;
pub mod screener_panel;
pub mod imbalance_panel;
pub mod footprint_panel;
pub mod liquidation_panel;

// src/gui/theme.rs
use egui::{Color32, Visuals, Style, Rounding, Stroke};

pub struct TradingTheme;

impl TradingTheme {
    pub const BACKGROUND: Color32 = Color32::from_rgb(30, 30, 30);
    pub const SURFACE: Color32 = Color32::from_rgb(40, 40, 40);
    pub const SURFACE_LIGHT: Color32 = Color32::from_rgb(50, 50, 50);
    
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(224, 224, 224);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(180, 180, 180);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(140, 140, 140);
    
    pub const BUY_COLOR: Color32 = Color32::from_rgb(0, 255, 136);
    pub const SELL_COLOR: Color32 = Color32::from_rgb(255, 68, 68);
    pub const NEUTRAL_COLOR: Color32 = Color32::from_rgb(255, 170, 0);
    
    pub const ACCENT: Color32 = Color32::from_rgb(0, 150, 255);
    pub const SUCCESS: Color32 = Self::BUY_COLOR;
    pub const WARNING: Color32 = Self::NEUTRAL_COLOR;
    pub const ERROR: Color32 = Self::SELL_COLOR;

    pub fn apply(ctx: &egui::Context) {
        let mut style = Style::default();
        let mut visuals = Visuals::dark();

        // Background colors
        visuals.panel_fill = Self::BACKGROUND;
        visuals.window_fill = Self::SURFACE;
        visuals.extreme_bg_color = Self::SURFACE_LIGHT;
        
        // Text colors
        visuals.text_color = Self::TEXT_PRIMARY;
        visuals.weak_text_color = Self::TEXT_SECONDARY;
        
        // Widget colors
        visuals.widgets.inactive.bg_fill = Self::SURFACE;
        visuals.widgets.inactive.weak_bg_fill = Self::SURFACE;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Self::SURFACE_LIGHT);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Self::TEXT_SECONDARY);
        
        visuals.widgets.hovered.bg_fill = Self::SURFACE_LIGHT;
        visuals.widgets.hovered.weak_bg_fill = Self::SURFACE_LIGHT;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Self::ACCENT);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Self::TEXT_PRIMARY);
        
        visuals.widgets.active.bg_fill = Self::ACCENT;
        visuals.widgets.active.weak_bg_fill = Self::ACCENT;
        
        // Hyperlinks
        visuals.hyperlink_color = Self::ACCENT;
        
        // Selection
        visuals.selection.bg_fill = Self::ACCENT.linear_multiply(0.3);
        visuals.selection.stroke = Stroke::new(1.0, Self::ACCENT);
        
        // Rounding
        style.visuals.widgets.noninteractive.rounding = Rounding::same(4.0);
        style.visuals.widgets.inactive.rounding = Rounding::same(4.0);
        style.visuals.widgets.hovered.rounding = Rounding::same(4.0);
        style.visuals.widgets.active.rounding = Rounding::same(4.0);
        
        style.visuals = visuals;
        ctx.set_style(style);
    }
}

// src/gui/app.rs
use egui::{CentralPanel, TopBottomPanel, SidePanel, Context, Ui};
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;
use std::collections::HashMap;

use crate::data::market_data::{MarketDataManager, GuiUpdate, OrderflowEvent, OrderImbalance, LiquidationEvent, CandleData};
use crate::gui::theme::TradingTheme;
use crate::gui::screener_panel::ScreenerPanel;
use crate::gui::imbalance_panel::ImbalancePanel;
use crate::gui::footprint_panel::FootprintPanel;
use crate::gui::liquidation_panel::LiquidationPanel;

#[derive(PartialEq)]
enum ActivePanel {
    Screener,
    Imbalance,
    Footprint,
    Liquidations,
}

pub struct ScreenerApp {
    market_data_manager: Arc<MarketDataManager>,
    gui_update_rx: Arc<RwLock<mpsc::UnboundedReceiver<GuiUpdate>>>,
    
    // Panel instances
    screener_panel: ScreenerPanel,
    imbalance_panel: ImbalancePanel,
    footprint_panel: FootprintPanel,
    liquidation_panel: LiquidationPanel,
    
    // UI state
    active_panel: ActivePanel,
    symbols: Vec<String>,
    
    // Performance metrics
    frame_count: u64,
    last_fps_update: std::time::Instant,
    fps: f32,
}

impl ScreenerApp {
    pub fn new(
        market_data_manager: Arc<MarketDataManager>,
        gui_update_rx: mpsc::UnboundedReceiver<GuiUpdate>,
    ) -> Self {
        Self {
            market_data_manager,
            gui_update_rx: Arc::new(RwLock::new(gui_update_rx)),
            screener_panel: ScreenerPanel::new(),
            imbalance_panel: ImbalancePanel::new(),
            footprint_panel: FootprintPanel::new(),
            liquidation_panel: LiquidationPanel::new(),
            active_panel: ActivePanel::Screener,
            symbols: Vec::new(),
            frame_count: 0,
            last_fps_update: std::time::Instant::now(),
            fps: 0.0,
        }
    }

    fn process_gui_updates(&mut self) {
        if let Ok(mut rx) = self.gui_update_rx.try_write() {
            while let Ok(update) = rx.try_recv() {
                match update {
                    GuiUpdate::NewOrderflow(event) => {
                        // Update panels with new orderflow data
                    }
                    GuiUpdate::BigOrderflow(event, percentage) => {
                        self.screener_panel.add_big_orderflow(event, percentage);
                    }
                    GuiUpdate::ImbalanceUpdate(imbalance) => {
                        self.imbalance_panel.update_imbalance(imbalance);
                    }
                    GuiUpdate::NewLiquidation(liquidation) => {
                        self.liquidation_panel.add_liquidation(liquidation);
                    }
                    GuiUpdate::CandleUpdate(candle) => {
                        self.footprint_panel.update_candle(candle);
                    }
                    GuiUpdate::StatsUpdate(stats) => {
                        // Update daily stats
                    }
                }
            }
        }
    }

    fn update_fps(&mut self) {
        self.frame_count += 1;
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_fps_update).as_secs_f32();
        
        if elapsed >= 1.0 {
            self.fps = self.frame_count as f32 / elapsed;
            self.frame_count = 0;
            self.last_fps_update = now;
        }
    }
}

impl eframe::App for ScreenerApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        TradingTheme::apply(ctx);
        self.process_gui_updates();
        self.update_fps();

        // Top panel with navigation tabs
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_panel, ActivePanel::Screener, "Orderflow Screener");
                ui.separator();
                ui.selectable_value(&mut self.active_panel, ActivePanel::Imbalance, "Order Imbalances");
                ui.separator();
                ui.selectable_value(&mut self.active_panel, ActivePanel::Footprint, "Footprint Charts");
                ui.separator();
                ui.selectable_value(&mut self.active_panel, ActivePanel::Liquidations, "Liquidations");
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("FPS: {:.0}", self.fps));
                    ui.separator();
                    ui.label(format!("Symbols: {}", self.symbols.len()));
                });
            });
        });

        // Bottom status panel
        TopBottomPanel::bottom("status_panel").min_height(25.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Status: Connected");
                ui.separator();
                ui.label("Data Feed: Binance Futures");
                ui.separator();
                ui.label("Active Streams: All USDT Pairs");
            });
        });

        // Main content panel
        CentralPanel::default().show(ctx, |ui| {
            match self.active_panel {
                ActivePanel::Screener => self.screener_panel.show(ui),
                ActivePanel::Imbalance => self.imbalance_panel.show(ui),
                ActivePanel::Footprint => self.footprint_panel.show(ui),
                ActivePanel::Liquidations => self.liquidation_panel.show(ui),
            }
        });

        // Request repaint for smooth updates
        ctx.request_repaint();
    }
}

// src/gui/screener_panel.rs
use egui::{Ui, ScrollArea, Color32, RichText};
use std::collections::VecDeque;
use crate::data::market_data::OrderflowEvent;
use crate::gui::theme::TradingTheme;

pub struct BigOrderflowEvent {
    pub event: OrderflowEvent,
    pub percentage: f64,
    pub timestamp: std::time::Instant,
}

pub struct ScreenerPanel {
    big_orders: VecDeque<BigOrderflowEvent>,
    volume_threshold: f64,
}

impl ScreenerPanel {
    pub fn new() -> Self {
        Self {
            big_orders: VecDeque::new(),
            volume_threshold: 0.5,
        }
    }

    pub fn add_big_orderflow(&mut self, event: OrderflowEvent, percentage: f64) {
        self.big_orders.push_back(BigOrderflowEvent {
            event,
            percentage,
            timestamp: std::time::Instant::now(),
        });

        // Keep only last 1000 events
        if self.big_orders.len() > 1000 {
            self.big_orders.pop_front();
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Header with controls
            ui.horizontal(|ui| {
                ui.heading("Large Orderflow Detection");
                ui.separator();
                ui.label("Min Volume %:");
                ui.add(egui::DragValue::new(&mut self.volume_threshold)
                    .speed(0.1)
                    .range(0.1..=10.0)
                    .suffix("%"));
            });

            ui.separator();

            // Table headers
            ui.horizontal(|ui| {
                ui.set_min_height(30.0);
                ui.label(RichText::new("Symbol").color(TradingTheme::TEXT_SECONDARY).size(12.0));
                ui.separator();
                ui.label(RichText::new("Side").color(TradingTheme::TEXT_SECONDARY).size(12.0));
                ui.separator();
                ui.label(RichText::new("Size").color(TradingTheme::TEXT_SECONDARY).size(12.0));
                ui.separator();
                ui.label(RichText::new("Price").color(TradingTheme::TEXT_SECONDARY).size(12.0));
                ui.separator();
                ui.label(RichText::new("% Daily Vol").color(TradingTheme::TEXT_SECONDARY).size(12.0));
                ui.separator();
                ui.label(RichText::new("Time").color(TradingTheme::TEXT_SECONDARY).size(12.0));
            });

            ui.separator();

            // Scrollable list of big orders
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for big_order in self.big_orders.iter().rev() {
                        let side_color = if big_order.event.is_buyer_maker {
                            TradingTheme::SELL_COLOR
                        } else {
                            TradingTheme::BUY_COLOR
                        };

                        let side_text = if big_order.event.is_buyer_maker {
                            "SELL"
                        } else {
                            "BUY"
                        };

                        let age = big_order.timestamp.elapsed().as_secs();
                        let alpha = if age > 60 { 0.5 } else { 1.0 };

                        ui.horizontal(|ui| {
                            ui.set_min_height(25.0);
                            
                            // Symbol
                            ui.label(RichText::new(&big_order.event.symbol)
                                .color(TradingTheme::TEXT_PRIMARY.gamma_multiply(alpha))
                                .size(13.0));
                            ui.separator();

                            // Side
                            ui.label(RichText::new(side_text)
                                .color(side_color.gamma_multiply(alpha))
                                .size(13.0)
                                .strong());
                            ui.separator();

                            // Size (USD value)
                            let size_usd = big_order.event.quantity * big_order.event.price.into_inner();
                            ui.label(RichText::new(format!("${:.0}", size_usd))
                                .color(TradingTheme::TEXT_PRIMARY.gamma_multiply(alpha))
                                .size(13.0));
                            ui.separator();

                            // Price
                            ui.label(RichText::new(format!("{:.4}", big_order.event.price))
                                .color(TradingTheme::TEXT_PRIMARY.gamma_multiply(alpha))
                                .size(13.0));
                            ui.separator();

                            // Percentage
                            let percentage_color = if big_order.percentage > 2.0 {
                                TradingTheme::ERROR
                            } else if big_order.percentage > 1.0 {
                                TradingTheme::WARNING
                            } else {
                                TradingTheme::TEXT_PRIMARY
                            };
                            
                            ui.label(RichText::new(format!("{:.2}%", big_order.percentage))
                                .color(percentage_color.gamma_multiply(alpha))
                                .size(13.0)
                                .strong());
                            ui.separator();

                            // Time ago
                            ui.label(RichText::new(format!("{}s", age))
                                .color(TradingTheme::TEXT_MUTED.gamma_multiply(alpha))
                                .size(11.0));
                        });
                        
                        ui.separator();
                    }
                });
        });
    }
}

// src/gui/imbalance_panel.rs
use egui::{Ui, ScrollArea, Color32, RichText};
use std::collections::HashMap;
use crate::data::market_data::OrderImbalance;
use crate::gui::theme::TradingTheme;

pub struct ImbalancePanel {
    imbalances: HashMap<String, OrderImbalance>,
}

impl ImbalancePanel {
    pub fn new() -> Self {
        Self {
            imbalances: HashMap::new(),
        }
    }

    pub fn update_imbalance(&mut self, imbalance: OrderImbalance) {
        self.imbalances.insert(imbalance.symbol.clone(), imbalance);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.heading("Order Flow Imbalances");
            ui.separator();

            // Sort by imbalance ratio for display
            let mut sorted_imbalances: Vec<_> = self.imbalances.values().collect();
            sorted_imbalances.sort_by(|a, b| {
                b.imbalance_ratio.abs().partial_cmp(&a.imbalance_ratio.abs()).unwrap()
            });

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for imbalance in sorted_imbalances.iter().take(50) {
                        self.draw_imbalance_row(ui, imbalance);
                    }
                });
        });
    }

    fn draw_imbalance_row(&self, ui: &mut Ui, imbalance: &OrderImbalance) {
        ui.horizontal(|ui| {
            ui.set_min_height(35.0);
            
            // Symbol
            ui.label(RichText::new(&imbalance.symbol)
                .color(TradingTheme::TEXT_PRIMARY)
                .size(14.0)
                .strong());

            ui.separator();

            // Imbalance visualization bar
            let bar_width = 200.0;
            let bar_height = 20.0;
            
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(bar_width, bar_height),
                egui::Sense::hover()
            );

            // Draw background
            ui.painter().rect_filled(rect, 2.0, TradingTheme::SURFACE_LIGHT);

            // Calculate bar positions
            let center = rect.center().x;
            let ratio = imbalance.imbalance_ratio.clamp(-1.0, 1.0);
            
            if ratio > 0.0 {
                // More buying pressure - green bar to the right
                let bar_width = (ratio * bar_width / 2.0) as f32;
                let bar_rect = egui::Rect::from_min_size(
                    egui::pos2(center, rect.min.y),
                    egui::vec2(bar_width, bar_height)
                );
                ui.painter().rect_filled(bar_rect, 2.0, TradingTheme::BUY_COLOR);
            } else if ratio < 0.0 {
                // More selling pressure - red bar to the left
                let bar_width = (-ratio * bar_width / 2.0) as f32;
                let bar_rect = egui::Rect::from_min_size(
                    egui::pos2(center - bar_width, rect.min.y),
                    egui::vec2(bar_width, bar_height)
                );
                ui.painter().rect_filled(bar_rect, 2.0, TradingTheme::SELL_COLOR);
            }

            // Center line
            ui.painter().vline(center, rect.y_range(), egui::Stroke::new(1.0, TradingTheme::TEXT_MUTED));

            // Imbalance ratio text
            let ratio_color = if ratio > 0.1 {
                TradingTheme::BUY_COLOR
            } else if ratio < -0.1 {
                TradingTheme::SELL_COLOR
            } else {
                TradingTheme::TEXT_SECONDARY
            };

            ui.label(RichText::new(format!("{:+.3}", imbalance.imbalance_ratio))
                .color(ratio_color)
                .size(13.0));
        });

        ui.separator();
    }
}

// src/gui/footprint_panel.rs
use egui::{Ui, ScrollArea, Color32, RichText};
use std::collections::{HashMap, BTreeMap};
use crate::data::market_data::{CandleData, VolumeAtPrice};
use crate::gui::theme::TradingTheme;
use ordered_float::OrderedFloat;

pub struct FootprintPanel {
    candles: HashMap<String, CandleData>,
    selected_symbol: String,
    timeframe: String,
}

impl FootprintPanel {
    pub fn new() -> Self {
        Self {
            candles: HashMap::new(),
            selected_symbol: "BTCUSDT".to_string(),
            timeframe: "1m".to_string(),
        }
    }

    pub fn update_candle(&mut self, candle: CandleData) {
        self.candles.insert(candle.symbol.clone(), candle);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Controls
            ui.horizontal(|ui| {
                ui.heading("Footprint Analysis");
                ui.separator();
                
                egui::ComboBox::from_label("Symbol")
                    .selected_text(&self.selected_symbol)
                    .show_ui(ui, |ui| {
                        for symbol in ["BTCUSDT", "ETHUSDT", "ADAUSDT", "SOLUSDT"] {
                            ui.selectable_value(&mut self.selected_symbol, symbol.to_string(), symbol);
                        }
                    });

                ui.separator();

                egui::ComboBox::from_label("Timeframe")
                    .selected_text(&self.timeframe)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.timeframe, "1m".to_string(), "1m");
                        ui.selectable_value(&mut self.timeframe, "5m".to_string(), "5m");
                        ui.selectable_value(&mut self.timeframe, "15m".to_string(), "15m");
                    });
            });

            ui.separator();

            if let Some(candle) = self.candles.get(&self.selected_symbol) {
                self.draw_footprint_chart(ui, candle);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Waiting for data...");
                });
            }
        });
    }

    fn draw_footprint_chart(&self, ui: &mut Ui, candle: &CandleData) {
        let available_size = ui.available_size();
        let chart_height = available_size.y - 100.0;

        ui.horizontal(|ui| {
            // Price column
            ui.vertical(|ui| {
                ui.set_width(80.0);
                ui.label(RichText::new("Price").color(TradingTheme::TEXT_SECONDARY).size(12.0));
                
                ScrollArea::vertical()
                    .max_height(chart_height)
                    .show(ui, |ui| {
                        for (price, _) in candle.volume_profile.price_levels.iter().rev() {
                            ui.label(RichText::new(format!("{:.2}", price))
                                .color(TradingTheme::TEXT_PRIMARY)
                                .size(11.0));
                        }
                    });
            });

            ui.separator();

            // Sell volume column
            ui.vertical(|ui| {
                ui.set_width(100.0);
                ui.label(RichText::new("Sells").color(TradingTheme::SELL_COLOR).size(12.0));
                
                ScrollArea::vertical()
                    .max_height(chart_height)
                    .show(ui, |ui| {
                        let max_volume = candle.volume_profile.price_levels
                            .values()
                            .map(|v| v.total_volume)
                            .max_by(|a, b| a.partial_cmp(b).unwrap())
                            .unwrap_or(1.0);

                        for (_, volume_data) in candle.volume_profile.price_levels.iter().rev() {
                            self.draw_volume_bar(ui, volume_data.sell_volume, max_volume, TradingTheme::SELL_COLOR, true);
                        }
                    });
            });

            ui.separator();

            // Buy volume column  
            ui.vertical(|ui| {
                ui.set_width(100.0);
                ui.label(RichText::new("Buys").color(TradingTheme::BUY_COLOR).size(12.0));
                
                ScrollArea::vertical()
                    .max_height(chart_height)
                    .show(ui, |ui| {
                        let max_volume = candle.volume_profile.price_levels
                            .values()
                            .map(|v| v.total_volume)
                            .max_by(|a, b| a.partial_cmp(b).unwrap())
                            .unwrap_or(1.0);

                        for (_, volume_data) in candle.volume_profile.price_levels.iter().rev() {
                            self.draw_volume_bar(ui, volume_data.buy_volume, max_volume, TradingTheme::BUY_COLOR, false);
                        }
                    });
            });
        });
    }

    fn draw_volume_bar(&self, ui: &mut Ui, volume: f64, max_volume: f64, color: Color32, right_align: bool) {
        let bar_height = 20.0;
        let max_bar_width = 80.0;
        let bar_width = (volume / max_volume * max_bar_width) as f32;
        
        ui.horizontal(|ui| {
            ui.set_min_height(bar_height);
            
            if right_align {
                ui.add_space(max_bar_width - bar_width);
            }
            
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(bar_width, bar_height - 2.0),
                egui::Sense::hover()
            );
            
            ui.painter().rect_filled(rect, 1.0, color);
            
            if !right_align {
                ui.add_space(max_bar_width - bar_width);
            }
            
            ui.label(RichText::new(format!("{:.0}", volume))
                .color(TradingTheme::TEXT_PRIMARY)
                .size(10.0));
        });
    }
}

// src/gui/liquidation_panel.rs
use egui::{Ui, ScrollArea, Color32, RichText};
use std::collections::VecDeque;
use crate::data::market_data::LiquidationEvent;
use crate::gui::theme::TradingTheme;

pub struct LiquidationPanel {
    liquidations: VecDeque<LiquidationEvent>,
}

impl LiquidationPanel {
    pub fn new() -> Self {
        Self {
            liquidations: VecDeque::new(),
        }
    }

    pub fn add_liquidation(&mut self, liquidation: LiquidationEvent) {
        self.liquidations.push_back(liquidation);
        
        // Keep only last 500 liquidations
        if self.liquidations.len() > 500 {
            self.liquidations.pop_front();
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.heading("Forced Liquidations");
            ui.separator();

            // Headers
            ui.horizontal(|ui| {
                ui.set_min_height(25.0);
                ui.label(RichText::new("Symbol").color(TradingTheme::TEXT_SECONDARY).size(12.0));
                ui.separator();
                ui.label(RichText::new("Side").color(TradingTheme::TEXT_SECONDARY).size(12.0));
                ui.separator();
                ui.label(RichText::new("Size").color(TradingTheme::TEXT_SECONDARY).size(12.0));
                ui.separator();
                ui.label(RichText::new("Price").color(TradingTheme::TEXT_SECONDARY).size(12.0));
                ui.separator();
                ui.label(RichText::new("USD Value").color(TradingTheme::TEXT_SECONDARY).size(12.0));
                ui.separator();
                ui.label(RichText::new("Type").color(TradingTheme::TEXT_SECONDARY).size(12.0));
            });

            ui.separator();

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for liquidation in self.liquidations.iter().rev() {
                        self.draw_liquidation_row(ui, liquidation);
                    }
                });
        });
    }

    fn draw_liquidation_row(&self, ui: &mut Ui, liquidation: &LiquidationEvent) {
        let side_color = if liquidation.side == "LONG" {
            TradingTheme::BUY_COLOR
        } else {
            TradingTheme::SELL_COLOR
        };

        let usd_value = liquidation.quantity * liquidation.price.into_inner();

        ui.horizontal(|ui| {
            ui.set_min_height(25.0);
            
            // Symbol
            ui.label(RichText::new(&liquidation.symbol)
                .color(TradingTheme::TEXT_PRIMARY)
                .size(13.0));
            ui.separator();

            // Side
            ui.label(RichText::new(&liquidation.side)
                .color(side_color)
                .size(13.0)
                .strong());
            ui.separator();

            // Size
            ui.label(RichText::new(format!("{:.4}", liquidation.quantity))
                .color(TradingTheme::TEXT_PRIMARY)
                .size(13.0));
            ui.separator();

            // Price
            ui.label(RichText::new(format!("{:.4}", liquidation.price))
                .color(TradingTheme::TEXT_PRIMARY)
                .size(13.0));
            ui.separator();

            // USD Value
            let value_color = if usd_value > 1_000_000.0 {
                TradingTheme::ERROR
            } else if usd_value > 100_000.0 {
                TradingTheme::WARNING
            } else {
                TradingTheme::TEXT_PRIMARY
            };
            
            ui.label(RichText::new(format!("${:.0}", usd_value))
                .color(value_color)
                .size(13.0)
                .strong());
            ui.separator();

            // Type
            let type_text = if liquidation.is_forced { "FORCED" } else { "REGULAR" };
            let type_color = if liquidation.is_forced { 
                TradingTheme::ERROR 
            } else { 
                TradingTheme::TEXT_SECONDARY 
            };
            
            ui.label(RichText::new(type_text)
                .color(type_color)
                .size(12.0));
        });

        ui.separator();
    }
}