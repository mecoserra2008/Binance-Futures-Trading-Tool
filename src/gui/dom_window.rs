use egui::{Color32, RichText, Ui, Rect, Pos2, Vec2, ScrollArea};
use std::collections::{HashMap, VecDeque};
use crate::data::{DepthSnapshot, OrderflowEvent};
use crate::analysis::TradedVolumeTracker;
use super::ScreenerTheme;

/// Depth of Market (DOM) panel showing order book and traded volume
pub struct DOMPanel {
    symbol: String,

    // Aggregation settings
    aggregation_level: f64,
    available_aggregations: Vec<f64>,
    aggregation_index: usize,

    // Display settings
    num_levels_to_show: usize,
    show_traded_volume: bool,
    show_volume_delta: bool,
    show_imbalance_percentage: bool,

    // Data
    current_depth: Option<DepthSnapshot>,
    traded_volume_tracker: HashMap<String, TradedVolumeTracker>,

    // UI state
    scroll_to_mid: bool,
    highlight_large_orders: bool,
    large_order_threshold: f64, // Threshold for highlighting (as % of max volume)
}

impl DOMPanel {
    pub fn new(symbol: String) -> Self {
        let available_aggregations = vec![0.01, 0.1, 1.0, 10.0, 100.0];
        let aggregation_index = 0; // Default to 0.01

        Self {
            symbol: symbol.clone(),
            aggregation_level: available_aggregations[aggregation_index],
            available_aggregations,
            aggregation_index,
            num_levels_to_show: 20,
            show_traded_volume: true,
            show_volume_delta: true,
            show_imbalance_percentage: true,
            current_depth: None,
            traded_volume_tracker: {
                let mut map = HashMap::new();
                map.insert(symbol.clone(), TradedVolumeTracker::new(symbol, 0.01));
                map
            },
            scroll_to_mid: true,
            highlight_large_orders: true,
            large_order_threshold: 0.7, // Highlight orders above 70% of max
        }
    }

    pub fn set_symbol(&mut self, symbol: String) {
        if self.symbol != symbol {
            self.symbol = symbol.clone();
            if !self.traded_volume_tracker.contains_key(&symbol) {
                self.traded_volume_tracker.insert(
                    symbol.clone(),
                    TradedVolumeTracker::new(symbol, self.aggregation_level)
                );
            }
        }
    }

    pub fn update_depth(&mut self, snapshot: DepthSnapshot) {
        // Always update - symbol matching is done at the app level
        self.current_depth = Some(snapshot);
    }

    pub fn process_trade(&mut self, event: &OrderflowEvent) {
        if let Some(tracker) = self.traded_volume_tracker.get_mut(&event.symbol) {
            tracker.process_trade(event);
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Header controls
            self.draw_controls(ui);

            ui.separator();

            // DOM ladder
            self.draw_dom_ladder(ui);
        });
    }

    fn draw_controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Symbol:");
            ui.label(RichText::new(&self.symbol).strong().color(Color32::WHITE));

            ui.separator();

            ui.label("Aggregation:");
            let mut agg_changed = false;
            egui::ComboBox::from_id_source("dom_aggregation_selector")
                .selected_text(format!("{}", self.aggregation_level))
                .width(80.0)
                .show_ui(ui, |ui| {
                    for (i, &agg) in self.available_aggregations.iter().enumerate() {
                        if ui.selectable_value(&mut self.aggregation_index, i, format!("{}", agg)).clicked() {
                            agg_changed = true;
                        }
                    }
                });

            if agg_changed {
                self.aggregation_level = self.available_aggregations[self.aggregation_index];
            }

            ui.separator();

            ui.label("Levels:");
            egui::ComboBox::from_id_source("dom_levels_selector")
                .selected_text(format!("{}", self.num_levels_to_show))
                .width(50.0)
                .show_ui(ui, |ui| {
                    for &num in &[10, 20, 30, 50, 100] {
                        ui.selectable_value(&mut self.num_levels_to_show, num, format!("{}", num));
                    }
                });

            ui.separator();

            ui.checkbox(&mut self.show_traded_volume, "Traded Vol");
            ui.checkbox(&mut self.show_volume_delta, "Delta");
            ui.checkbox(&mut self.highlight_large_orders, "Highlight Large");
        });
    }

    fn draw_dom_ladder(&mut self, ui: &mut Ui) {
        if let Some(ref depth) = self.current_depth {
            let available_rect = ui.available_rect_before_wrap();

            // Aggregate order book to specified level
            let (aggregated_bids, aggregated_asks) = self.aggregate_depth(depth);

            // Find max volumes for bar sizing
            let max_bid_vol = aggregated_bids.iter().map(|(_, vol)| *vol).fold(0.0f64, f64::max);
            let max_ask_vol = aggregated_asks.iter().map(|(_, vol)| *vol).fold(0.0f64, f64::max);
            let max_vol = max_bid_vol.max(max_ask_vol);

            // Get traded volumes
            let traded_volumes = if let Some(tracker) = self.traded_volume_tracker.get(&self.symbol) {
                self.get_aggregated_traded_volumes(tracker)
            } else {
                HashMap::new()
            };

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Column headers
                    self.draw_header_row(ui, available_rect.width());

                    ui.separator();

                    // Draw asks (in reverse order - highest to lowest)
                    let asks_to_show: Vec<_> = aggregated_asks.iter()
                        .rev()
                        .take(self.num_levels_to_show)
                        .collect();

                    for (price, volume) in asks_to_show {
                        let traded_vol = traded_volumes.get(&price_to_tick(*price, self.aggregation_level));
                        self.draw_price_level(
                            ui,
                            *price,
                            *volume,
                            traded_vol,
                            max_vol,
                            true, // is_ask
                            available_rect.width()
                        );
                    }

                    // Current price marker
                    self.draw_current_price_marker(ui, depth, available_rect.width());

                    // Draw bids (highest to lowest)
                    let bids_to_show: Vec<_> = aggregated_bids.iter()
                        .take(self.num_levels_to_show)
                        .collect();

                    for (price, volume) in bids_to_show {
                        let traded_vol = traded_volumes.get(&price_to_tick(*price, self.aggregation_level));
                        self.draw_price_level(
                            ui,
                            *price,
                            *volume,
                            traded_vol,
                            max_vol,
                            false, // is_bid
                            available_rect.width()
                        );
                    }
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No market depth data available");
            });
        }
    }

    fn draw_header_row(&self, ui: &mut Ui, width: f32) {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(width, 25.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    // Left side (bids)
                    ui.label(RichText::new("Bid Size").size(11.0).color(Color32::GRAY));
                    ui.add_space(5.0);

                    if self.show_traded_volume {
                        ui.label(RichText::new("Traded").size(11.0).color(Color32::GRAY));
                        ui.add_space(5.0);
                    }

                    // Center (price)
                    ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                        ui.label(RichText::new("Price").size(12.0).strong().color(Color32::WHITE));
                    });

                    // Right side (asks)
                    if self.show_traded_volume {
                        ui.label(RichText::new("Traded").size(11.0).color(Color32::GRAY));
                        ui.add_space(5.0);
                    }

                    ui.label(RichText::new("Ask Size").size(11.0).color(Color32::GRAY));
                },
            );
        });
    }

    fn draw_current_price_marker(&self, ui: &mut Ui, depth: &DepthSnapshot, width: f32) {
        // Calculate current mid price
        let best_bid = depth.bids.first().map(|(p, _)| *p).unwrap_or(0.0);
        let best_ask = depth.asks.first().map(|(p, _)| *p).unwrap_or(0.0);
        let mid_price = (best_bid + best_ask) / 2.0;
        let spread = best_ask - best_bid;
        let spread_pct = if best_bid > 0.0 { (spread / best_bid) * 100.0 } else { 0.0 };

        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(width, 30.0),
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(40, 40, 50));

                    ui.label(
                        RichText::new(format!(
                            "MID: {:.2}  │  SPREAD: {:.2} ({:.3}%)",
                            mid_price, spread, spread_pct
                        ))
                        .size(11.0)
                        .strong()
                        .color(Color32::YELLOW)
                    );
                },
            );
        });

        ui.add_space(2.0);
    }

    fn draw_price_level(
        &self,
        ui: &mut Ui,
        price: f64,
        volume: f64,
        traded_vol: Option<&(f64, f64, f64)>, // (buy, sell, total)
        max_volume: f64,
        is_ask: bool,
        width: f32,
    ) {
        ui.horizontal(|ui| {
            let row_height = 20.0;
            let rect = Rect::from_min_size(
                ui.cursor().min,
                Vec2::new(width, row_height)
            );

            // Background color based on side
            let bg_color = if is_ask {
                Color32::from_rgb(40, 25, 25) // Dark red tint
            } else {
                Color32::from_rgb(25, 40, 25) // Dark green tint
            };
            ui.painter().rect_filled(rect, 0.0, bg_color);

            // Volume bar background
            let volume_pct = if max_volume > 0.0 { volume / max_volume } else { 0.0 };
            let bar_width = (width * 0.4 * volume_pct as f32).max(2.0);

            let bar_color = if is_ask {
                if self.highlight_large_orders && volume_pct > self.large_order_threshold {
                    Color32::from_rgb(180, 50, 50) // Bright red for large ask
                } else {
                    Color32::from_rgb(120, 40, 40)
                }
            } else {
                if self.highlight_large_orders && volume_pct > self.large_order_threshold {
                    Color32::from_rgb(50, 180, 50) // Bright green for large bid
                } else {
                    Color32::from_rgb(40, 120, 40)
                }
            };

            if is_ask {
                // Ask bar on the right
                let bar_rect = Rect::from_min_size(
                    Pos2::new(rect.max.x - bar_width, rect.min.y),
                    Vec2::new(bar_width, row_height)
                );
                ui.painter().rect_filled(bar_rect, 0.0, bar_color);
            } else {
                // Bid bar on the left
                let bar_rect = Rect::from_min_size(
                    rect.min,
                    Vec2::new(bar_width, row_height)
                );
                ui.painter().rect_filled(bar_rect, 0.0, bar_color);
            }

            ui.allocate_ui_with_layout(
                Vec2::new(width, row_height),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    let text_color = Color32::WHITE;

                    // Bid side
                    if !is_ask {
                        ui.label(RichText::new(format!("{:.2}", volume))
                            .size(10.0)
                            .color(ScreenerTheme::BUY_COLOR));

                        if self.show_traded_volume {
                            if let Some((buy, sell, _total)) = traded_vol {
                                let delta = buy - sell;
                                let delta_color = if delta > 0.0 { Color32::GREEN } else { Color32::RED };
                                ui.label(RichText::new(format!("{:.1}", delta.abs()))
                                    .size(9.0)
                                    .color(delta_color));
                            } else {
                                ui.label(RichText::new("-").size(9.0).color(Color32::GRAY));
                            }
                        }
                    } else {
                        ui.add_space(width * 0.25);
                    }

                    // Price (centered)
                    ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                        let price_color = if is_ask {
                            ScreenerTheme::SELL_COLOR
                        } else {
                            ScreenerTheme::BUY_COLOR
                        };
                        ui.label(RichText::new(format!("{:.2}", price))
                            .size(11.0)
                            .strong()
                            .color(price_color));
                    });

                    // Ask side
                    if is_ask {
                        if self.show_traded_volume {
                            if let Some((buy, sell, _total)) = traded_vol {
                                let delta = buy - sell;
                                let delta_color = if delta > 0.0 { Color32::GREEN } else { Color32::RED };
                                ui.label(RichText::new(format!("{:.1}", delta.abs()))
                                    .size(9.0)
                                    .color(delta_color));
                            } else {
                                ui.label(RichText::new("-").size(9.0).color(Color32::GRAY));
                            }
                        }

                        ui.label(RichText::new(format!("{:.2}", volume))
                            .size(10.0)
                            .color(ScreenerTheme::SELL_COLOR));
                    }
                },
            );
        });
    }

    fn aggregate_depth(&self, depth: &DepthSnapshot) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
        let mut aggregated_bids: HashMap<i64, f64> = HashMap::new();
        let mut aggregated_asks: HashMap<i64, f64> = HashMap::new();

        // Aggregate bids
        for (price, quantity) in &depth.bids {
            let price_tick = price_to_tick(*price, self.aggregation_level);
            *aggregated_bids.entry(price_tick).or_insert(0.0) += quantity;
        }

        // Aggregate asks
        for (price, quantity) in &depth.asks {
            let price_tick = price_to_tick(*price, self.aggregation_level);
            *aggregated_asks.entry(price_tick).or_insert(0.0) += quantity;
        }

        // Convert to sorted vectors
        let mut bids: Vec<(f64, f64)> = aggregated_bids
            .into_iter()
            .map(|(tick, vol)| (tick_to_price(tick, self.aggregation_level), vol))
            .collect();
        bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap()); // Descending by price

        let mut asks: Vec<(f64, f64)> = aggregated_asks
            .into_iter()
            .map(|(tick, vol)| (tick_to_price(tick, self.aggregation_level), vol))
            .collect();
        asks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap()); // Ascending by price

        (bids, asks)
    }

    fn get_aggregated_traded_volumes(&self, tracker: &TradedVolumeTracker) -> HashMap<i64, (f64, f64, f64)> {
        // Return map of price_tick -> (buy_volume, sell_volume, total_volume)
        tracker.volume_at_price.iter()
            .map(|(tick, vol)| (*tick, (vol.buy_volume, vol.sell_volume, vol.total_volume)))
            .collect()
    }
}

// Helper functions
fn price_to_tick(price: f64, aggregation: f64) -> i64 {
    (price / aggregation).round() as i64
}

fn tick_to_price(tick: i64, aggregation: f64) -> f64 {
    tick as f64 * aggregation
}
