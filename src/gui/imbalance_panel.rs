use egui::{Color32, RichText, Ui};
use std::collections::HashMap;
use crate::data::{OrderImbalance, BinanceSymbols};
use super::{ScreenerTheme, VolumeBar, ImbalanceIndicator};

pub struct ImbalancePanel {
    imbalances: HashMap<String, OrderImbalance>,
    display_mode: DisplayMode,
    sort_by: SortBy,
    sort_ascending: bool,
    filter_text: String,
    min_imbalance_threshold: f64,
    show_significant_only: bool,
    grid_columns: usize,

    // Symbol management
    symbol_category: String,
    watched_symbols: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DisplayMode {
    Table,
    Grid,
    Chart,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SortBy {
    Symbol,
    ImbalanceRatio,
    BidVolume,
    AskVolume,
    TotalVolume,
    Timestamp,
}

impl ImbalancePanel {
    pub fn new() -> Self {
        let watched_symbols = BinanceSymbols::get_high_volume_symbols();

        Self {
            imbalances: HashMap::new(),
            display_mode: DisplayMode::Grid,
            sort_by: SortBy::ImbalanceRatio,
            sort_ascending: false, // Highest imbalance first
            filter_text: String::new(),
            min_imbalance_threshold: 0.1, // 10% minimum imbalance to show
            show_significant_only: false,
            grid_columns: 6,

            // Symbol management
            symbol_category: "High Volume".to_string(),
            watched_symbols,
        }
    }

    pub fn add_imbalance(&mut self, imbalance: OrderImbalance) {
        self.imbalances.insert(imbalance.symbol.clone(), imbalance);
    }

    pub fn get_symbol_count(&self) -> usize {
        self.imbalances.len()
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Controls
            self.show_controls(ui);
            
            ui.separator();
            
            // Content based on display mode
            match self.display_mode {
                DisplayMode::Table => self.show_table_view(ui),
                DisplayMode::Grid => self.show_grid_view(ui),
                DisplayMode::Chart => self.show_chart_view(ui),
            }
        });
    }

    fn show_controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Display:");
            ui.selectable_value(&mut self.display_mode, DisplayMode::Table, "Table");
            ui.selectable_value(&mut self.display_mode, DisplayMode::Grid, "Grid");
            ui.selectable_value(&mut self.display_mode, DisplayMode::Chart, "Chart");

            ui.separator();

            // Category selector
            ui.label("Category:");
            egui::ComboBox::from_id_source("imbalance_category_selector")
                .selected_text(&self.symbol_category)
                .width(100.0)
                .show_ui(ui, |ui| {
                    let categories = vec![
                        "High Volume", "Major", "DeFi", "Layer2", "Gaming",
                        "AI", "Meme", "Infrastructure", "New", "All"
                    ];
                    for category in categories {
                        if ui.selectable_value(&mut self.symbol_category, category.to_string(), category).clicked() {
                            self.update_watched_symbols();
                        }
                    }
                });

            ui.separator();

            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter_text);

            ui.separator();
            
            ui.label("Min Imbalance:");
            ui.add(egui::DragValue::new(&mut self.min_imbalance_threshold)
                .speed(0.01)
                .suffix(""));
            
            ui.separator();
            
            ui.checkbox(&mut self.show_significant_only, "Significant Only");
            
            if self.display_mode == DisplayMode::Grid {
                ui.separator();
                ui.label("Columns:");
                ui.add(egui::DragValue::new(&mut self.grid_columns)
                    .speed(1));
            }
        });
    }

    fn show_table_view(&mut self, ui: &mut Ui) {
        use egui_extras::{TableBuilder, Column};

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().resizable(true)) // Symbol
            .column(Column::auto().resizable(true)) // Imbalance Ratio
            .column(Column::auto().resizable(true)) // Bid Volume
            .column(Column::auto().resizable(true)) // Ask Volume
            .column(Column::auto().resizable(true)) // Total Volume
            .column(Column::auto().resizable(true)) // Last Update
            .column(Column::remainder())            // Visual Indicator
            .header(25.0, |mut header| {
                header.col(|ui| {
                    if ui.button("Symbol").clicked() {
                        self.toggle_sort(SortBy::Symbol);
                    }
                });
                header.col(|ui| {
                    if ui.button("Imbalance").clicked() {
                        self.toggle_sort(SortBy::ImbalanceRatio);
                    }
                });
                header.col(|ui| {
                    if ui.button("Bid Volume").clicked() {
                        self.toggle_sort(SortBy::BidVolume);
                    }
                });
                header.col(|ui| {
                    if ui.button("Ask Volume").clicked() {
                        self.toggle_sort(SortBy::AskVolume);
                    }
                });
                header.col(|ui| {
                    if ui.button("Total Volume").clicked() {
                        self.toggle_sort(SortBy::TotalVolume);
                    }
                });
                header.col(|ui| {
                    if ui.button("Updated").clicked() {
                        self.toggle_sort(SortBy::Timestamp);
                    }
                });
                header.col(|ui| {
                    ui.label("Indicator");
                });
            })
            .body(|body| {
                let filtered_imbalances = self.get_filtered_and_sorted_imbalances();
                
                body.rows(30.0, filtered_imbalances.len(), |row_index, mut row| {
                    if let Some(imbalance) = filtered_imbalances.get(row_index) {
                        self.show_imbalance_table_row(&mut row, imbalance);
                    }
                });
            });
    }

    fn show_imbalance_table_row(&self, row: &mut egui_extras::TableRow, imbalance: &OrderImbalance) {
        let imbalance_color = ScreenerTheme::get_imbalance_color(imbalance.imbalance_ratio);
        
        row.col(|ui| {
            ui.strong(&imbalance.symbol);
        });

        row.col(|ui| {
            ui.colored_label(
                imbalance_color,
                ScreenerTheme::format_imbalance_ratio(imbalance.imbalance_ratio)
            );
        });

        row.col(|ui| {
            ui.colored_label(
                ScreenerTheme::BUY_COLOR,
                ScreenerTheme::format_volume(imbalance.bid_volume)
            );
        });

        row.col(|ui| {
            ui.colored_label(
                ScreenerTheme::SELL_COLOR,
                ScreenerTheme::format_volume(imbalance.ask_volume)
            );
        });

        row.col(|ui| {
            ui.label(ScreenerTheme::format_volume(imbalance.bid_volume + imbalance.ask_volume));
        });

        row.col(|ui| {
            ui.label(ScreenerTheme::format_timestamp(imbalance.timestamp));
        });

        row.col(|ui| {
            let bar_width = ui.available_width() - 10.0;
            
            VolumeBar::show(
                ui,
                imbalance.bid_volume,
                imbalance.ask_volume,
                bar_width,
                20.0,
            );
        });
    }

    fn show_grid_view(&mut self, ui: &mut Ui) {
        let filtered_imbalances = self.get_filtered_and_sorted_imbalances();
        let available_width = ui.available_width();
        let cell_width = (available_width / self.grid_columns as f32) - 10.0;
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.columns(self.grid_columns, |columns| {
                for (index, imbalance) in filtered_imbalances.iter().enumerate() {
                    let col_index = index % self.grid_columns;
                    self.show_imbalance_card(&mut columns[col_index], imbalance, cell_width);
                }
            });
        });
    }

    fn show_imbalance_card(&self, ui: &mut Ui, imbalance: &OrderImbalance, width: f32) {
        let imbalance_color = ScreenerTheme::get_imbalance_color(imbalance.imbalance_ratio);
        
        egui::Frame::none()
            .fill(ScreenerTheme::SURFACE)
            .stroke(egui::Stroke::new(1.0, ScreenerTheme::BORDER_COLOR))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                ui.set_width(width);
                
                ui.vertical_centered(|ui| {
                    // Symbol
                    ui.heading(&imbalance.symbol);
                    
                    ui.add_space(4.0);
                    
                    // Imbalance ratio with large text and color
                    ui.colored_label(
                        imbalance_color,
                        RichText::new(ScreenerTheme::format_imbalance_ratio(imbalance.imbalance_ratio))
                            .size(18.0)
                            .strong()
                    );
                    
                    ui.add_space(4.0);
                    
                    // Volume bar
                    VolumeBar::show(
                        ui,
                        imbalance.bid_volume,
                        imbalance.ask_volume,
                        width - 16.0,
                        20.0,
                    );
                    
                    ui.add_space(4.0);
                    
                    // Volume details
                    ui.horizontal(|ui| {
                        ui.colored_label(ScreenerTheme::BUY_COLOR, "B:");
                        ui.label(ScreenerTheme::format_volume(imbalance.bid_volume));
                        ui.separator();
                        ui.colored_label(ScreenerTheme::SELL_COLOR, "S:");
                        ui.label(ScreenerTheme::format_volume(imbalance.ask_volume));
                    });
                    
                    // Timestamp
                    ui.small(ScreenerTheme::format_timestamp(imbalance.timestamp));
                });
            });
        
        ui.add_space(8.0);
    }

    fn show_chart_view(&mut self, ui: &mut Ui) {
        let filtered_imbalances = self.get_filtered_and_sorted_imbalances();
        let available_height = ui.available_height();
        let row_height = 30.0;
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            for imbalance in filtered_imbalances {
                self.show_imbalance_chart_row(ui, imbalance, row_height);
            }
        });
    }

    fn show_imbalance_chart_row(&self, ui: &mut Ui, imbalance: &OrderImbalance, height: f32) {
        ui.horizontal(|ui| {
            // Symbol (fixed width)
            ui.allocate_ui_with_layout(
                egui::vec2(80.0, height),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.strong(&imbalance.symbol);
                }
            );
            
            ui.separator();
            
            // Imbalance indicator
            let indicator_width = ui.available_width() - 150.0;
            ImbalanceIndicator::show(ui, imbalance.imbalance_ratio, indicator_width, height - 4.0);
            
            ui.separator();
            
            // Value display
            ui.allocate_ui_with_layout(
                egui::vec2(140.0, height),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    let imbalance_color = ScreenerTheme::get_imbalance_color(imbalance.imbalance_ratio);
                    ui.colored_label(
                        imbalance_color,
                        ScreenerTheme::format_imbalance_ratio(imbalance.imbalance_ratio)
                    );
                }
            );
        });
        
        ui.add_space(2.0);
    }

    fn toggle_sort(&mut self, sort_by: SortBy) {
        if self.sort_by == sort_by {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_by = sort_by;
            self.sort_ascending = match sort_by {
                SortBy::Symbol => true,
                SortBy::Timestamp => false, // Most recent first
                _ => false, // Highest values first
            };
        }
    }

    fn get_filtered_and_sorted_imbalances(&self) -> Vec<&OrderImbalance> {
        let mut filtered: Vec<&OrderImbalance> = self.imbalances
            .values()
            .filter(|imbalance| self.passes_filters(imbalance))
            .collect();

        // Sort the filtered results
        filtered.sort_by(|a, b| {
            let ordering = match self.sort_by {
                SortBy::Symbol => a.symbol.cmp(&b.symbol),
                SortBy::ImbalanceRatio => a.imbalance_ratio.abs().partial_cmp(&b.imbalance_ratio.abs()).unwrap_or(std::cmp::Ordering::Equal),
                SortBy::BidVolume => a.bid_volume.partial_cmp(&b.bid_volume).unwrap_or(std::cmp::Ordering::Equal),
                SortBy::AskVolume => a.ask_volume.partial_cmp(&b.ask_volume).unwrap_or(std::cmp::Ordering::Equal),
                SortBy::TotalVolume => (a.bid_volume + a.ask_volume).partial_cmp(&(b.bid_volume + b.ask_volume)).unwrap_or(std::cmp::Ordering::Equal),
                SortBy::Timestamp => a.timestamp.cmp(&b.timestamp),
            };

            if self.sort_ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });

        filtered
    }

    fn passes_filters(&self, imbalance: &OrderImbalance) -> bool {
        // Text filter
        if !self.filter_text.is_empty() {
            let filter_lower = self.filter_text.to_lowercase();
            if !imbalance.symbol.to_lowercase().contains(&filter_lower) {
                return false;
            }
        }

        // Minimum imbalance threshold
        if imbalance.imbalance_ratio.abs() < self.min_imbalance_threshold {
            return false;
        }

        // Significant only filter
        if self.show_significant_only && imbalance.imbalance_ratio.abs() < 0.3 {
            return false;
        }

        true
    }

    fn update_watched_symbols(&mut self) {
        let symbols_by_category = BinanceSymbols::get_symbols_by_category();

        self.watched_symbols = match self.symbol_category.as_str() {
            "High Volume" => BinanceSymbols::get_high_volume_symbols(),
            "Major" => symbols_by_category.get("Major").unwrap_or(&vec![]).clone(),
            "DeFi" => symbols_by_category.get("DeFi").unwrap_or(&vec![]).clone(),
            "Layer2" => symbols_by_category.get("Layer2").unwrap_or(&vec![]).clone(),
            "Gaming" => symbols_by_category.get("Gaming").unwrap_or(&vec![]).clone(),
            "AI" => symbols_by_category.get("AI").unwrap_or(&vec![]).clone(),
            "Meme" => symbols_by_category.get("Meme").unwrap_or(&vec![]).clone(),
            "Infrastructure" => symbols_by_category.get("Infrastructure").unwrap_or(&vec![]).clone(),
            "New" => symbols_by_category.get("New").unwrap_or(&vec![]).clone(),
            "All" => BinanceSymbols::get_all_symbols(),
            _ => BinanceSymbols::get_default_symbols(),
        };

        // Filter existing imbalances to only show watched symbols
        self.imbalances.retain(|symbol, _| self.watched_symbols.contains(symbol));
    }

    pub fn get_watched_symbols(&self) -> &Vec<String> {
        &self.watched_symbols
    }
}