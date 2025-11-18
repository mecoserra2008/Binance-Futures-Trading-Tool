use egui::{Color32, RichText};
use std::collections::VecDeque;
use crate::data::BigOrderflowAlert;
use super::{ScreenerTheme, VolumeBar};

pub struct ScreenerPanel {
    alerts: VecDeque<BigOrderflowAlert>,
    max_alerts: usize,
    sort_column: SortColumn,
    sort_ascending: bool,
    filter_text: String,
    min_percentage_filter: f64,
    min_notional_filter: f64,
    show_buy_only: bool,
    show_sell_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SortColumn {
    Timestamp,
    Symbol,
    Side,
    Size,
    Price,
    Percentage,
    Notional,
}

impl ScreenerPanel {
    pub fn new() -> Self {
        Self {
            alerts: VecDeque::new(),
            max_alerts: 1000,
            sort_column: SortColumn::Timestamp,
            sort_ascending: false, // Most recent first by default
            filter_text: String::new(),
            min_percentage_filter: 0.0,
            min_notional_filter: 0.0,
            show_buy_only: false,
            show_sell_only: false,
        }
    }

    pub fn add_orderflow_alert(&mut self, alert: BigOrderflowAlert) {
        self.alerts.push_front(alert);
        
        // Maintain maximum alerts
        while self.alerts.len() > self.max_alerts {
            self.alerts.pop_back();
        }
    }

    pub fn get_alert_count(&self) -> usize {
        self.alerts.len()
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Filters and controls
            self.show_controls(ui);
            
            ui.separator();
            
            // Table header and content
            self.show_table(ui);
        });
    }

    fn show_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter_text);
            
            ui.separator();
            
            ui.label("Min %:");
            ui.add(egui::DragValue::new(&mut self.min_percentage_filter)
                .speed(0.1)
                .suffix("%"));
            
            ui.separator();
            
            ui.label("Min Notional:");
            ui.add(egui::DragValue::new(&mut self.min_notional_filter)
                .speed(1000.0)
                .prefix("$"));
            
            ui.separator();
            
            ui.checkbox(&mut self.show_buy_only, "Buy Only");
            ui.checkbox(&mut self.show_sell_only, "Sell Only");
            
            ui.separator();
            
            if ui.button("Clear").clicked() {
                self.alerts.clear();
            }
        });
    }

    fn show_table(&mut self, ui: &mut egui::Ui) {
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
            .column(Column::auto().resizable(true)) // % of Daily
            .column(Column::auto().resizable(true)) // Notional
            .column(Column::remainder())            // Volume Bar
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
                    if ui.button("% Daily").clicked() {
                        self.toggle_sort(SortColumn::Percentage);
                    }
                });
                header.col(|ui| {
                    if ui.button("Notional").clicked() {
                        self.toggle_sort(SortColumn::Notional);
                    }
                });
                header.col(|ui| {
                    ui.label("Volume");
                });
            })
            .body(|body| {
                let filtered_alerts = self.get_filtered_and_sorted_alerts();
                
                body.rows(25.0, filtered_alerts.len(), |row_index, mut row| {
                    if let Some(alert) = filtered_alerts.get(row_index) {
                        self.show_alert_row(&mut row, alert);
                    }
                });
            });
    }

    fn show_alert_row(&self, row: &mut egui_extras::TableRow, alert: &BigOrderflowAlert) {
        let side_color = if alert.side == "BUY" {
            ScreenerTheme::BUY_COLOR
        } else {
            ScreenerTheme::SELL_COLOR
        };

        row.col(|ui| {
            ui.label(ScreenerTheme::format_timestamp(alert.timestamp));
        });

        row.col(|ui| {
            ui.label(&alert.symbol);
        });

        row.col(|ui| {
            ui.colored_label(side_color, &alert.side);
        });

        row.col(|ui| {
            ui.label(ScreenerTheme::format_volume(alert.quantity));
        });

        row.col(|ui| {
            ui.label(ScreenerTheme::format_price(alert.price, 2));
        });

        row.col(|ui| {
            let percentage_color = if alert.percentage_of_daily >= 1.0 {
                ScreenerTheme::ERROR
            } else if alert.percentage_of_daily >= 0.75 {
                ScreenerTheme::WARNING
            } else {
                ScreenerTheme::NEUTRAL_COLOR
            };
            ui.colored_label(
                percentage_color,
                ScreenerTheme::format_percentage(alert.percentage_of_daily)
            );
        });

        row.col(|ui| {
            ui.label(ScreenerTheme::format_currency(alert.notional_value));
        });

        row.col(|ui| {
            // Volume intensity bar
            let intensity = (alert.percentage_of_daily / 2.0).min(1.0); // Scale to 0-1
            let bar_width = ui.available_width() - 10.0;
            
            VolumeBar::show(
                ui,
                if alert.side == "BUY" { alert.quantity } else { 0.0 },
                if alert.side == "SELL" { alert.quantity } else { 0.0 },
                bar_width,
                20.0,
            );
        });
    }

    fn toggle_sort(&mut self, column: SortColumn) {
        if self.sort_column == column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column;
            self.sort_ascending = match column {
                SortColumn::Timestamp => false, // Most recent first
                SortColumn::Size | SortColumn::Percentage | SortColumn::Notional => false, // Largest first
                _ => true, // Alphabetical for text fields
            };
        }
    }

    fn get_filtered_and_sorted_alerts(&self) -> Vec<&BigOrderflowAlert> {
        let mut filtered: Vec<&BigOrderflowAlert> = self.alerts
            .iter()
            .filter(|alert| self.passes_filters(alert))
            .collect();

        // Sort the filtered results
        filtered.sort_by(|a, b| {
            let ordering = match self.sort_column {
                SortColumn::Timestamp => a.timestamp.cmp(&b.timestamp),
                SortColumn::Symbol => a.symbol.cmp(&b.symbol),
                SortColumn::Side => a.side.cmp(&b.side),
                SortColumn::Size => a.quantity.partial_cmp(&b.quantity).unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::Price => a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::Percentage => a.percentage_of_daily.partial_cmp(&b.percentage_of_daily).unwrap_or(std::cmp::Ordering::Equal),
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

    fn passes_filters(&self, alert: &BigOrderflowAlert) -> bool {
        // Text filter
        if !self.filter_text.is_empty() {
            let filter_lower = self.filter_text.to_lowercase();
            if !alert.symbol.to_lowercase().contains(&filter_lower) &&
               !alert.side.to_lowercase().contains(&filter_lower) {
                return false;
            }
        }

        // Percentage filter
        if alert.percentage_of_daily < self.min_percentage_filter {
            return false;
        }

        // Notional filter
        if alert.notional_value < self.min_notional_filter {
            return false;
        }

        // Side filters
        if self.show_buy_only && alert.side != "BUY" {
            return false;
        }
        if self.show_sell_only && alert.side != "SELL" {
            return false;
        }

        true
    }
}