use egui::{Color32, RichText, Ui};
use std::collections::VecDeque;
use crate::data::LiquidationEvent;
use super::ScreenerTheme;

pub struct LiquidationPanel {
    liquidations: VecDeque<LiquidationEvent>,
    max_liquidations: usize,
    sort_column: SortColumn,
    sort_ascending: bool,
    filter_text: String,
    min_notional_filter: f64,
    show_long_only: bool,
    show_short_only: bool,
    auto_scroll: bool,
    flash_duration_ms: u64,
    liquidation_flash_times: std::collections::HashMap<String, u64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SortColumn {
    Timestamp,
    Symbol,
    Side,
    Size,
    Price,
    Notional,
}

impl LiquidationPanel {
    pub fn new() -> Self {
        Self {
            liquidations: VecDeque::new(),
            max_liquidations: 2000,
            sort_column: SortColumn::Timestamp,
            sort_ascending: false, // Most recent first
            filter_text: String::new(),
            min_notional_filter: 0.0,
            show_long_only: false,
            show_short_only: false,
            auto_scroll: true,
            flash_duration_ms: 3000, // 3 seconds
            liquidation_flash_times: std::collections::HashMap::new(),
        }
    }

    pub fn add_liquidation(&mut self, liquidation: LiquidationEvent) {
        // Add flash effect for new liquidation
        let flash_key = format!("{}_{}", liquidation.symbol, liquidation.timestamp);
        self.liquidation_flash_times.insert(
            flash_key,
            chrono::Utc::now().timestamp_millis() as u64,
        );

        self.liquidations.push_front(liquidation);
        
        // Maintain maximum liquidations
        while self.liquidations.len() > self.max_liquidations {
            self.liquidations.pop_back();
        }

        // Clean up old flash effects
        self.cleanup_flash_effects();
    }

    pub fn get_liquidation_count(&self) -> usize {
        self.liquidations.len()
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Controls and statistics
            self.show_controls_and_stats(ui);
            
            ui.separator();
            
            // Liquidation feed
            self.show_liquidation_feed(ui);
        });
    }

    fn show_controls_and_stats(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Filters
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter_text);
            
            ui.separator();
            
            ui.label("Min Notional:");
            ui.add(egui::DragValue::new(&mut self.min_notional_filter)
                .speed(10000.0)
                .prefix("$"));
            
            ui.separator();
            
            ui.checkbox(&mut self.show_long_only, "Long Only");
            ui.checkbox(&mut self.show_short_only, "Short Only");
            ui.checkbox(&mut self.auto_scroll, "Auto Scroll");
            
            ui.separator();
            
            if ui.button("Clear").clicked() {
                self.liquidations.clear();
                self.liquidation_flash_times.clear();
            }
        });

        // Statistics row
        ui.horizontal(|ui| {
            let stats = self.calculate_statistics();
            
            ui.label(format!("Total: {}", stats.total_count));
            ui.separator();
            
            ui.colored_label(
                ScreenerTheme::SELL_COLOR,
                format!("Long Liq: {}", stats.long_liquidations)
            );
            ui.separator();
            
            ui.colored_label(
                ScreenerTheme::BUY_COLOR,
                format!("Short Liq: {}", stats.short_liquidations)
            );
            ui.separator();
            
            ui.label(format!("Volume: {}", ScreenerTheme::format_currency(stats.total_volume)));
            ui.separator();
            
            ui.label(format!("Avg Size: {}", ScreenerTheme::format_currency(stats.average_size)));
        });
    }

    fn show_liquidation_feed(&mut self, ui: &mut Ui) {
        use egui_extras::{TableBuilder, Column};

        let available_height = ui.available_height();
        
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().resizable(true)) // Time
            .column(Column::auto().resizable(true)) // Symbol
            .column(Column::auto().resizable(true)) // Side
            .column(Column::auto().resizable(true)) // Size
            .column(Column::auto().resizable(true)) // Price
            .column(Column::auto().resizable(true)) // Notional
            .column(Column::remainder())            // Status
            .header(25.0, |mut header| {
                header.col(|ui| {
                    if ui.button("Time").clicked() {
                        self.toggle_sort(SortColumn::Timestamp);
                    }
                });
                header.col(|ui| {
                    if ui.button("Symbol").clicked() {
                        self.toggle_sort(SortColumn::Symbol);
                    }
                });
                header.col(|ui| {
                    if ui.button("Side").clicked() {
                        self.toggle_sort(SortColumn::Side);
                    }
                });
                header.col(|ui| {
                    if ui.button("Size").clicked() {
                        self.toggle_sort(SortColumn::Size);
                    }
                });
                header.col(|ui| {
                    if ui.button("Price").clicked() {
                        self.toggle_sort(SortColumn::Price);
                    }
                });
                header.col(|ui| {
                    if ui.button("Notional").clicked() {
                        self.toggle_sort(SortColumn::Notional);
                    }
                });
                header.col(|ui| {
                    ui.label("Status");
                });
            })
            .body(|body| {
                let filtered_liquidations = self.get_filtered_and_sorted_liquidations();
                
                body.rows(28.0, filtered_liquidations.len(), |row_index, mut row| {
                    if let Some(liquidation) = filtered_liquidations.get(row_index) {
                        self.show_liquidation_row(&mut row, liquidation);
                    }
                });
            });
    }

    fn show_liquidation_row(&self, row: &mut egui_extras::TableRow, liquidation: &LiquidationEvent) {
        let side_color = ScreenerTheme::get_liquidation_color(&liquidation.side);
        
        // Check if this liquidation should be flashing
        let flash_key = format!("{}_{}", liquidation.symbol, liquidation.timestamp);
        let is_flashing = self.liquidation_flash_times.contains_key(&flash_key);
        
        let background_color = if is_flashing {
            Some(side_color.gamma_multiply(0.3))
        } else {
            None
        };

        row.col(|ui| {
            if let Some(bg) = background_color {
                ui.painter().rect_filled(
                    ui.available_rect_before_wrap(),
                    egui::Rounding::same(2.0),
                    bg,
                );
            }
            ui.label(ScreenerTheme::format_timestamp(liquidation.timestamp));
        });

        row.col(|ui| {
            if let Some(bg) = background_color {
                ui.painter().rect_filled(
                    ui.available_rect_before_wrap(),
                    egui::Rounding::same(2.0),
                    bg,
                );
            }
            ui.strong(&liquidation.symbol);
        });

        row.col(|ui| {
            if let Some(bg) = background_color {
                ui.painter().rect_filled(
                    ui.available_rect_before_wrap(),
                    egui::Rounding::same(2.0),
                    bg,
                );
            }
            ui.colored_label(side_color, &liquidation.side);
        });

        row.col(|ui| {
            if let Some(bg) = background_color {
                ui.painter().rect_filled(
                    ui.available_rect_before_wrap(),
                    egui::Rounding::same(2.0),
                    bg,
                );
            }
            
            // Size with emphasis for large liquidations
            let size_text = ScreenerTheme::format_volume(liquidation.quantity);
            if liquidation.notional_value > 1_000_000.0 {
                ui.colored_label(ScreenerTheme::ERROR, RichText::new(size_text).strong());
            } else if liquidation.notional_value > 100_000.0 {
                ui.colored_label(ScreenerTheme::WARNING, size_text);
            } else {
                ui.label(size_text);
            }
        });

        row.col(|ui| {
            if let Some(bg) = background_color {
                ui.painter().rect_filled(
                    ui.available_rect_before_wrap(),
                    egui::Rounding::same(2.0),
                    bg,
                );
            }
            ui.label(ScreenerTheme::format_price(liquidation.price, 2));
        });

        row.col(|ui| {
            if let Some(bg) = background_color {
                ui.painter().rect_filled(
                    ui.available_rect_before_wrap(),
                    egui::Rounding::same(2.0),
                    bg,
                );
            }
            
            // Notional value with color coding
            let notional_text = ScreenerTheme::format_currency(liquidation.notional_value);
            let notional_color = if liquidation.notional_value >= 1_000_000.0 {
                ScreenerTheme::ERROR
            } else if liquidation.notional_value >= 100_000.0 {
                ScreenerTheme::WARNING
            } else if liquidation.notional_value >= 10_000.0 {
                ScreenerTheme::NEUTRAL_COLOR
            } else {
                ScreenerTheme::TEXT_PRIMARY
            };
            
            ui.colored_label(notional_color, notional_text);
        });

        row.col(|ui| {
            if let Some(bg) = background_color {
                ui.painter().rect_filled(
                    ui.available_rect_before_wrap(),
                    egui::Rounding::same(2.0),
                    bg,
                );
            }
            
            // Status indicators
            ui.horizontal(|ui| {
                if liquidation.is_forced {
                    ui.colored_label(ScreenerTheme::ERROR, "🔥");
                }
                
                // Size indicator
                if liquidation.notional_value >= 1_000_000.0 {
                    ui.colored_label(ScreenerTheme::ERROR, "💥");
                } else if liquidation.notional_value >= 100_000.0 {
                    ui.colored_label(ScreenerTheme::WARNING, "⚡");
                }
            });
        });
    }

    fn toggle_sort(&mut self, column: SortColumn) {
        if self.sort_column == column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column;
            self.sort_ascending = match column {
                SortColumn::Timestamp => false, // Most recent first
                SortColumn::Size | SortColumn::Notional => false, // Largest first
                _ => true, // Alphabetical for text fields
            };
        }
    }

    fn get_filtered_and_sorted_liquidations(&self) -> Vec<&LiquidationEvent> {
        let mut filtered: Vec<&LiquidationEvent> = self.liquidations
            .iter()
            .filter(|liquidation| self.passes_filters(liquidation))
            .collect();

        // Sort the filtered results
        filtered.sort_by(|a, b| {
            let ordering = match self.sort_column {
                SortColumn::Timestamp => a.timestamp.cmp(&b.timestamp),
                SortColumn::Symbol => a.symbol.cmp(&b.symbol),
                SortColumn::Side => a.side.cmp(&b.side),
                SortColumn::Size => a.quantity.partial_cmp(&b.quantity).unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::Price => a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::Notional => a.notional_value.partial_cmp(&b.notional_value).unwrap_or(std::cmp::Ordering::Equal),
            };

            if self.sort_ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });

        filtered
    }

    fn passes_filters(&self, liquidation: &LiquidationEvent) -> bool {
        // Text filter
        if !self.filter_text.is_empty() {
            let filter_lower = self.filter_text.to_lowercase();
            if !liquidation.symbol.to_lowercase().contains(&filter_lower) &&
               !liquidation.side.to_lowercase().contains(&filter_lower) {
                return false;
            }
        }

        // Notional filter
        if liquidation.notional_value < self.min_notional_filter {
            return false;
        }

        // Side filters
        if self.show_long_only && liquidation.side != "LONG" {
            return false;
        }
        if self.show_short_only && liquidation.side != "SHORT" {
            return false;
        }

        true
    }

    fn calculate_statistics(&self) -> LiquidationStatistics {
        let filtered_liquidations = self.get_filtered_and_sorted_liquidations();
        
        let mut stats = LiquidationStatistics {
            total_count: filtered_liquidations.len(),
            long_liquidations: 0,
            short_liquidations: 0,
            total_volume: 0.0,
            average_size: 0.0,
        };

        for liquidation in &filtered_liquidations {
            match liquidation.side.as_str() {
                "LONG" => stats.long_liquidations += 1,
                "SHORT" => stats.short_liquidations += 1,
                _ => {}
            }
            stats.total_volume += liquidation.notional_value;
        }

        if stats.total_count > 0 {
            stats.average_size = stats.total_volume / stats.total_count as f64;
        }

        stats
    }

    fn cleanup_flash_effects(&mut self) {
        let current_time = chrono::Utc::now().timestamp_millis() as u64;
        self.liquidation_flash_times.retain(|_, &mut flash_time| {
            current_time - flash_time < self.flash_duration_ms
        });
    }
}

#[derive(Debug, Clone)]
struct LiquidationStatistics {
    total_count: usize,
    long_liquidations: usize,
    short_liquidations: usize,
    total_volume: f64,
    average_size: f64,
}