use egui::{Color32, RichText, Ui, Rect, Pos2, Vec2};
use std::collections::{HashMap, VecDeque, BTreeMap};
use crate::data::{VolumeProfile, OrderflowEvent, BinanceSymbols, DepthSnapshot};
use super::{ScreenerTheme, HeatmapColorScheme};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct FootprintCell {
    pub price: f64,
    pub bid_volume: u64,  // market sells at bid
    pub ask_volume: u64,  // market buys at ask
}

impl FootprintCell {
    pub fn new(price: f64) -> Self {
        Self {
            price,
            bid_volume: 0,
            ask_volume: 0,
        }
    }

    pub fn total_volume(&self) -> u64 {
        self.bid_volume + self.ask_volume
    }

    pub fn delta(&self) -> i64 {
        self.ask_volume as i64 - self.bid_volume as i64
    }
}

#[derive(Debug, Clone)]
pub struct FootprintCandle {
    pub timestamp: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub cells: BTreeMap<i64, FootprintCell>, // price_tick -> FootprintCell
    pub tick_size: f64,
}

impl FootprintCandle {
    pub fn new(timestamp: u64, tick_size: f64) -> Self {
        Self {
            timestamp,
            open: 0.0,
            high: 0.0,
            low: f64::MAX,
            close: 0.0,
            cells: BTreeMap::new(),
            tick_size,
        }
    }

    pub fn add_trade(&mut self, event: &OrderflowEvent) {
        let price_tick = (event.price / self.tick_size).round() as i64;

        // Update OHLC
        if self.open == 0.0 {
            self.open = event.price;
        }
        self.close = event.price;
        self.high = self.high.max(event.price);
        self.low = self.low.min(event.price);

        // Update volume at price level
        let cell = self.cells.entry(price_tick).or_insert(FootprintCell::new(event.price));

        if event.is_buyer_maker {
            // Buyer is maker = sell order hit buyer's bid = bid volume
            cell.bid_volume += event.quantity as u64;
        } else {
            // Seller is maker = buy order hit seller's ask = ask volume
            cell.ask_volume += event.quantity as u64;
        }
    }

    pub fn get_price_range(&self) -> (f64, f64) {
        if self.cells.is_empty() {
            return (0.0, 0.0);
        }
        let min_tick = *self.cells.keys().min().unwrap();
        let max_tick = *self.cells.keys().max().unwrap();
        (min_tick as f64 * self.tick_size, max_tick as f64 * self.tick_size)
    }

    pub fn max_volume(&self) -> u64 {
        self.cells.values().map(|cell| cell.total_volume()).max().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DragAxis {
    Horizontal,  // X-axis (time)
    Vertical,    // Y-axis (price)
}

pub struct FootprintPanel {
    symbols: Vec<String>,
    selected_symbol: String,
    volume_profiles: HashMap<String, VecDeque<VolumeProfile>>,

    // Footprint data - base timeframe storage (1m)
    base_current_candles: HashMap<String, FootprintCandle>, // symbol -> current 1m candle
    base_completed_candles: HashMap<String, VecDeque<FootprintCandle>>, // symbol -> historical 1m candles

    // Timeframe management
    available_timeframes: Vec<(String, u64)>, // (label, milliseconds)
    selected_timeframe_index: usize,
    timeframe_ms: u64, // Current timeframe in ms

    // Cached aggregated candles for non-base timeframes
    cached_aggregated_candles: HashMap<String, VecDeque<FootprintCandle>>,
    cache_valid: bool,

    // Display settings
    max_candles_display: usize,
    show_volume: bool,
    show_delta: bool,
    show_imbalance: bool,

    // Scale and zoom settings
    price_scale: f64, // Aggregation scale for price bins
    available_scales: Vec<f64>,
    scale_index: usize,

    // Zoom and pan settings
    zoom_level: f32,
    pan_x: f32,
    pan_y: f32,
    min_zoom: f32,
    max_zoom: f32,

    // Independent axis scaling (for right-click drag)
    x_scale: f32,  // Time axis scale
    y_scale: f32,  // Price axis scale
    x_scale_sensitivity: f32,
    y_scale_sensitivity: f32,

    // Mouse interaction state
    dragging: bool,
    right_dragging: bool,
    drag_start_pos: Option<egui::Pos2>,
    last_mouse_pos: Option<egui::Pos2>,
    drag_axis: Option<DragAxis>,  // Which axis is being scaled

    // Symbol management
    symbol_category: String,
    show_symbol_selector: bool,

    // Cumulative Volume Delta tracking per symbol
    cumulative_cvd: HashMap<String, i64>,

    // LOB Heatmap data
    depth_snapshots: HashMap<String, VecDeque<DepthSnapshot>>,
    max_depth_snapshots: usize,

    // LOB Heatmap rendering settings
    enable_heatmap: bool,
    heatmap_color_scheme: HeatmapColorScheme,
    heatmap_opacity: f32,
}

impl FootprintPanel {
    pub fn new() -> Self {
        let available_scales = vec![0.0001, 0.001, 0.01, 0.1, 1.0, 10.0, 100.0];
        let scale_index = 2; // Default to 0.01
        let symbols = BinanceSymbols::get_high_volume_symbols(); // Use high-volume symbols by default

        // Define available timeframes
        let available_timeframes = vec![
            ("15s".to_string(), 15_000u64),
            ("30s".to_string(), 30_000u64),
            ("1m".to_string(), 60_000u64),
            ("5m".to_string(), 300_000u64),
            ("15m".to_string(), 900_000u64),
            ("30m".to_string(), 1_800_000u64),
            ("1h".to_string(), 3_600_000u64),
            ("4h".to_string(), 14_400_000u64),
            ("12h".to_string(), 43_200_000u64),
            ("1d".to_string(), 86_400_000u64),
        ];
        let selected_timeframe_index = 2; // Default to 1m (index 2)

        Self {
            symbols: symbols.clone(),
            selected_symbol: symbols.first().unwrap_or(&"BTCUSDT".to_string()).clone(),
            volume_profiles: HashMap::new(),
            base_current_candles: HashMap::new(),
            base_completed_candles: HashMap::new(),

            // Timeframe management
            timeframe_ms: available_timeframes[selected_timeframe_index].1,
            available_timeframes,
            selected_timeframe_index,
            cached_aggregated_candles: HashMap::new(),
            cache_valid: false,

            max_candles_display: 50,
            show_volume: true,
            show_delta: true,
            show_imbalance: false,

            // Scale settings
            price_scale: available_scales[scale_index],
            available_scales,
            scale_index,

            // Zoom and pan settings
            zoom_level: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            min_zoom: 0.1,
            max_zoom: 10.0,

            // Independent axis scaling
            x_scale: 1.0,
            y_scale: 1.0,
            x_scale_sensitivity: 0.01,
            y_scale_sensitivity: 0.01,

            // Mouse interaction state
            dragging: false,
            right_dragging: false,
            drag_start_pos: None,
            last_mouse_pos: None,
            drag_axis: None,

            // Symbol management
            symbol_category: "High Volume".to_string(),
            show_symbol_selector: false,

            // Cumulative CVD tracking
            cumulative_cvd: HashMap::new(),

            // LOB Heatmap data
            depth_snapshots: HashMap::new(),
            max_depth_snapshots: 100,  // Keep last 100 snapshots per symbol

            // LOB Heatmap rendering settings
            enable_heatmap: true,
            heatmap_color_scheme: HeatmapColorScheme::default(),
            heatmap_opacity: 0.6,
        }
    }

    pub fn new_with_symbols(symbols: Vec<String>) -> Self {
        let default_symbols = if symbols.is_empty() {
            BinanceSymbols::get_default_symbols()
        } else {
            symbols
        };

        let selected = default_symbols.first().unwrap_or(&"BTCUSDT".to_string()).clone();

        let available_scales = vec![0.0001, 0.001, 0.01, 0.1, 1.0, 10.0, 100.0];
        let scale_index = 2; // Default to 0.01

        // Define available timeframes
        let available_timeframes = vec![
            ("15s".to_string(), 15_000u64),
            ("30s".to_string(), 30_000u64),
            ("1m".to_string(), 60_000u64),
            ("5m".to_string(), 300_000u64),
            ("15m".to_string(), 900_000u64),
            ("30m".to_string(), 1_800_000u64),
            ("1h".to_string(), 3_600_000u64),
            ("4h".to_string(), 14_400_000u64),
            ("12h".to_string(), 43_200_000u64),
            ("1d".to_string(), 86_400_000u64),
        ];
        let selected_timeframe_index = 2; // Default to 1m (index 2)

        Self {
            symbols: default_symbols,
            selected_symbol: selected,
            volume_profiles: HashMap::new(),
            base_current_candles: HashMap::new(),
            base_completed_candles: HashMap::new(),

            // Timeframe management
            timeframe_ms: available_timeframes[selected_timeframe_index].1,
            available_timeframes,
            selected_timeframe_index,
            cached_aggregated_candles: HashMap::new(),
            cache_valid: false,

            max_candles_display: 50,
            show_volume: true,
            show_delta: true,
            show_imbalance: false,

            // Scale settings
            price_scale: available_scales[scale_index],
            available_scales,
            scale_index,

            // Zoom and pan settings
            zoom_level: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            min_zoom: 0.1,
            max_zoom: 10.0,

            // Independent axis scaling
            x_scale: 1.0,
            y_scale: 1.0,
            x_scale_sensitivity: 0.01,
            y_scale_sensitivity: 0.01,

            // Mouse interaction state
            dragging: false,
            right_dragging: false,
            drag_start_pos: None,
            last_mouse_pos: None,
            drag_axis: None,

            // Symbol management
            symbol_category: "Default".to_string(),
            show_symbol_selector: false,

            // Cumulative CVD tracking
            cumulative_cvd: HashMap::new(),

            // LOB Heatmap data
            depth_snapshots: HashMap::new(),
            max_depth_snapshots: 100,  // Keep last 100 snapshots per symbol

            // LOB Heatmap rendering settings
            enable_heatmap: true,
            heatmap_color_scheme: HeatmapColorScheme::default(),
            heatmap_opacity: 0.6,
        }
    }

    pub fn add_volume_profile(&mut self, profile: VolumeProfile) {
        let profiles = self.volume_profiles.entry(profile.symbol.clone()).or_insert_with(VecDeque::new);
        profiles.push_back(profile);

        while profiles.len() > 100 {
            profiles.pop_front();
        }
    }

    pub fn add_orderflow_event(&mut self, event: &OrderflowEvent) {
        // Always use base timeframe (1m = 60000ms) for incoming events
        const BASE_TIMEFRAME_MS: u64 = 60_000;
        let current_time = chrono::Utc::now().timestamp_millis() as u64;
        let candle_start = (current_time / BASE_TIMEFRAME_MS) * BASE_TIMEFRAME_MS;

        // Get or create current base candle for this symbol
        let candle = self.base_current_candles.entry(event.symbol.clone()).or_insert_with(|| {
            FootprintCandle::new(candle_start, self.price_scale)
        });

        // If this trade belongs to a new candle, complete the old one
        if event.timestamp < candle.timestamp || event.timestamp >= candle.timestamp + BASE_TIMEFRAME_MS {
            // Complete the old candle
            if candle.open != 0.0 { // Only if it has data
                let completed_candle = candle.clone();
                let completed_candles = self.base_completed_candles.entry(event.symbol.clone()).or_insert_with(VecDeque::new);
                completed_candles.push_back(completed_candle);

                // Keep up to 1000 base candles (more than display to allow aggregation)
                while completed_candles.len() > 1000 {
                    completed_candles.pop_front();
                }

                // Invalidate cache when new base candle completes
                self.cache_valid = false;
            }

            // Start new candle
            let new_candle_start = (event.timestamp / BASE_TIMEFRAME_MS) * BASE_TIMEFRAME_MS;
            *candle = FootprintCandle::new(new_candle_start, self.price_scale);
        }

        // Add trade to current candle
        candle.add_trade(event);

        // Invalidate cache on new trades (current candle updated)
        self.cache_valid = false;
    }

    pub fn add_depth_snapshot(&mut self, symbol: String, snapshot: DepthSnapshot) {
        let snapshots = self.depth_snapshots.entry(symbol).or_insert_with(VecDeque::new);
        snapshots.push_back(snapshot);

        // Maintain max size
        while snapshots.len() > self.max_depth_snapshots {
            snapshots.pop_front();
        }
    }

    pub fn get_profile_count(&self) -> usize {
        self.volume_profiles.values().map(|v| v.len()).sum()
    }

    /// Get candles for the selected timeframe, aggregating from base candles if needed
    fn get_candles_for_timeframe(&mut self, symbol: &str) -> (Vec<FootprintCandle>, Option<FootprintCandle>) {
        const BASE_TIMEFRAME_MS: u64 = 60_000;

        // If base timeframe (1m) is selected, return base candles directly
        if self.timeframe_ms == BASE_TIMEFRAME_MS {
            let completed = self.base_completed_candles.get(symbol).cloned().unwrap_or_default();
            let current = self.base_current_candles.get(symbol).cloned();
            return (completed.into_iter().collect(), current);
        }

        // For larger timeframes, check cache or aggregate
        if !self.cache_valid {
            self.aggregate_candles_for_symbol(symbol);
        }

        let completed = self.cached_aggregated_candles.get(symbol).cloned().unwrap_or_default();
        (completed.into_iter().collect(), None) // Current candle is included in aggregation
    }

    /// Aggregate base candles into the selected timeframe
    fn aggregate_candles_for_symbol(&mut self, symbol: &str) {
        let base_candles = self.base_completed_candles.get(symbol);
        let current_candle = self.base_current_candles.get(symbol);

        if base_candles.is_none() && current_candle.is_none() {
            return;
        }

        // Combine completed and current base candles
        let mut all_base_candles: Vec<FootprintCandle> = base_candles
            .map(|c| c.iter().cloned().collect())
            .unwrap_or_else(Vec::new);

        if let Some(current) = current_candle {
            if current.open != 0.0 {
                all_base_candles.push(current.clone());
            }
        }

        if all_base_candles.is_empty() {
            return;
        }

        // Group base candles by target timeframe
        let mut aggregated = VecDeque::new();
        let mut current_group: Vec<FootprintCandle> = Vec::new();
        let mut current_period_start: Option<u64> = None;

        for candle in all_base_candles {
            let period_start = (candle.timestamp / self.timeframe_ms) * self.timeframe_ms;

            if let Some(current_start) = current_period_start {
                if period_start != current_start {
                    // New period - aggregate current group
                    if !current_group.is_empty() {
                        aggregated.push_back(Self::aggregate_candle_group(&current_group));
                        current_group.clear();
                    }
                }
            }

            current_period_start = Some(period_start);
            current_group.push(candle);
        }

        // Aggregate remaining group
        if !current_group.is_empty() {
            aggregated.push_back(Self::aggregate_candle_group(&current_group));
        }

        // Store in cache
        self.cached_aggregated_candles.insert(symbol.to_string(), aggregated);
        self.cache_valid = true;
    }

    /// Aggregate a group of candles into one
    fn aggregate_candle_group(candles: &[FootprintCandle]) -> FootprintCandle {
        if candles.is_empty() {
            return FootprintCandle::new(0, 0.01);
        }

        let first = &candles[0];
        let last = &candles[candles.len() - 1];

        let mut aggregated = FootprintCandle {
            timestamp: first.timestamp,
            open: first.open,
            high: candles.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max),
            low: candles.iter().map(|c| c.low).fold(f64::INFINITY, f64::min),
            close: last.close,
            cells: BTreeMap::new(),
            tick_size: first.tick_size,
        };

        // Merge all cells from all candles
        for candle in candles {
            for (price_tick, cell) in &candle.cells {
                let entry = aggregated.cells.entry(*price_tick).or_insert_with(|| {
                    FootprintCell::new(cell.price)
                });
                entry.bid_volume += cell.bid_volume;
                entry.ask_volume += cell.ask_volume;
            }
        }

        aggregated
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Header with symbol selection
            ui.horizontal(|ui| {
                ui.label("Symbol:");
                egui::ComboBox::from_id_source("footprint_symbol_selector")
                    .selected_text(&self.selected_symbol)
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for symbol in &self.symbols.clone() {
                            ui.selectable_value(&mut self.selected_symbol, symbol.clone(), symbol);
                        }
                    });

                // Category selector
                ui.label("Category:");
                egui::ComboBox::from_id_source("footprint_category_selector")
                    .selected_text(&self.symbol_category)
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        let categories = vec![
                            "High Volume", "Major", "DeFi", "Layer2", "Gaming",
                            "AI", "Meme", "Infrastructure", "New", "All"
                        ];
                        for category in categories {
                            if ui.selectable_value(&mut self.symbol_category, category.to_string(), category).clicked() {
                                self.update_symbols_for_category();
                            }
                        }
                    });

                ui.separator();

                // Timeframe selector
                ui.label("Timeframe:");
                let mut timeframe_changed = false;
                egui::ComboBox::from_id_source("footprint_timeframe_selector")
                    .selected_text(&self.available_timeframes[self.selected_timeframe_index].0)
                    .width(60.0)
                    .show_ui(ui, |ui| {
                        for (i, (label, _ms)) in self.available_timeframes.iter().enumerate() {
                            if ui.selectable_value(&mut self.selected_timeframe_index, i, label).clicked() {
                                timeframe_changed = true;
                            }
                        }
                    });

                if timeframe_changed {
                    self.timeframe_ms = self.available_timeframes[self.selected_timeframe_index].1;
                    self.cache_valid = false; // Invalidate cache when timeframe changes
                }

                ui.separator();

                // Scale controls
                ui.label("Scale:");
                let mut scale_changed = false;
                egui::ComboBox::from_id_source("footprint_scale_selector")
                    .selected_text(format!("{}", self.price_scale))
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        for (i, &scale) in self.available_scales.iter().enumerate() {
                            if ui.selectable_value(&mut self.scale_index, i, format!("{}", scale)).clicked() {
                                scale_changed = true;
                            }
                        }
                    });

                if scale_changed {
                    self.price_scale = self.available_scales[self.scale_index];
                    // Clear current candles to force regeneration with new scale
                    self.base_current_candles.clear();
                    self.cache_valid = false;
                }

                ui.separator();

                // Zoom controls
                ui.label("Zoom:");
                if ui.button("−").clicked() {
                    self.zoom_level = (self.zoom_level / 1.2).max(self.min_zoom);
                }
                ui.label(format!("{:.1}x", self.zoom_level));
                if ui.button("+").clicked() {
                    self.zoom_level = (self.zoom_level * 1.2).min(self.max_zoom);
                }
                if ui.button("Reset").clicked() {
                    self.zoom_level = 1.0;
                    self.pan_x = 0.0;
                    self.pan_y = 0.0;
                }

                ui.separator();

                // Display mode toggles
                ui.checkbox(&mut self.show_volume, "Volume");
                ui.checkbox(&mut self.show_delta, "Delta");
                ui.checkbox(&mut self.show_imbalance, "Imbalance");

                ui.separator();

                // LOB Heatmap controls
                ui.checkbox(&mut self.enable_heatmap, "Heatmap");
                if self.enable_heatmap {
                    ui.label("Opacity:");
                    ui.add(egui::Slider::new(&mut self.heatmap_opacity, 0.0..=1.0)
                        .show_value(false)
                        .max_decimals(2));
                }
            });

            ui.separator();

            // Footprint chart
            self.draw_footprint_chart(ui);
        });
    }

    fn draw_footprint_chart(&mut self, ui: &mut Ui) {
        let available_rect = ui.available_rect_before_wrap();

        // Reserve space for axes
        let axis_width = 80.0;
        let axis_height = 30.0;
        let stats_height = 60.0; // Space for statistics above chart

        let chart_rect = Rect::from_min_size(
            available_rect.min + Vec2::new(axis_width, stats_height),
            Vec2::new(available_rect.width() - axis_width, available_rect.height() - axis_height - stats_height - 20.0)
        );

        // Handle mouse interactions for pan and zoom
        self.handle_mouse_interactions(ui, chart_rect);

        // Get candles for selected symbol and timeframe
        let selected_symbol = self.selected_symbol.clone();
        let (completed_candles, current_candle) = self.get_candles_for_timeframe(&selected_symbol);

        if completed_candles.is_empty() && current_candle.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label(format!("No footprint data for {}", self.selected_symbol));
            });
            return;
        }

        // Combine completed and current candles
        let mut all_candles: Vec<FootprintCandle> = completed_candles;
        if let Some(current) = current_candle {
            if current.open != 0.0 {
                all_candles.push(current);
            }
        }

        if all_candles.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(format!("No data for {}", self.selected_symbol));
            });
            return;
        }

        // Calculate candle width with both zoom and X-axis scale
        let base_candle_width = chart_rect.width() / all_candles.len().max(1) as f32;
        let candle_width = base_candle_width * self.zoom_level * self.x_scale;

        // Draw statistics header above chart
        self.draw_candle_statistics(ui, &all_candles, available_rect.min, available_rect.width(), stats_height, self.pan_x, candle_width);

        // Calculate price range across all candles
        let (base_min_price, base_max_price) = self.calculate_overall_price_range(&all_candles);
        let base_price_range = base_max_price - base_min_price;

        if base_price_range <= 0.0 {
            return;
        }

        // Apply zoom, Y-axis scale, and pan to price range
        let zoomed_price_range = base_price_range / (self.zoom_level as f64 * self.y_scale as f64);
        let pan_price_offset = self.pan_y as f64 * base_price_range / chart_rect.height() as f64;

        let overall_min_price = base_min_price + pan_price_offset;
        let overall_max_price = overall_min_price + zoomed_price_range;

        // Get max volume for rendering
        let max_volume = all_candles.iter().map(|c| c.max_volume()).max().unwrap_or(1);

        // Draw axes
        self.draw_price_axis(ui, chart_rect, overall_min_price, overall_max_price, available_rect.min.x, axis_width);
        self.draw_time_axis(ui, &all_candles, chart_rect, available_rect.min.y + available_rect.height() - axis_height, axis_height, candle_width);

        // Calculate which candles are visible based on pan_x
        let visible_start_index = (-self.pan_x / candle_width).max(0.0) as usize;
        let visible_end_index = ((chart_rect.width() - self.pan_x) / candle_width).min(all_candles.len() as f32) as usize;

        // Draw visible candles
        for (i, candle) in all_candles.iter().enumerate().skip(visible_start_index).take(visible_end_index - visible_start_index) {
            let x = chart_rect.min.x + i as f32 * candle_width + self.pan_x;

            // Only draw if candle is within chart bounds
            if x + candle_width >= chart_rect.min.x && x <= chart_rect.max.x {
                self.draw_footprint_candle(ui, candle, x, candle_width, chart_rect, overall_min_price, overall_max_price, max_volume);
            }
        }
    }

    fn calculate_overall_price_range(&self, candles: &[FootprintCandle]) -> (f64, f64) {
        let mut min_price = f64::MAX;
        let mut max_price = f64::MIN;

        for candle in candles {
            let (candle_min, candle_max) = candle.get_price_range();
            if candle_min > 0.0 {
                min_price = min_price.min(candle_min);
                max_price = max_price.max(candle_max);
            }
        }

        if min_price == f64::MAX {
            return (0.0, 1.0);
        }

        (min_price, max_price)
    }

    fn draw_footprint_candle(&self, ui: &mut Ui, candle: &FootprintCandle, x: f32, width: f32, chart_rect: Rect, min_price: f64, max_price: f64, max_volume: u64) {
        let price_range = max_price - min_price;

        // Draw LOB Heatmap background FIRST (if enabled)
        if self.enable_heatmap {
            self.draw_heatmap_for_candle(ui, candle, x, width, chart_rect, min_price, max_price);
        }

        // Draw OHLC outline
        if candle.open != 0.0 {
            let open_y = chart_rect.max.y - ((candle.open - min_price) / price_range) as f32 * chart_rect.height();
            let close_y = chart_rect.max.y - ((candle.close - min_price) / price_range) as f32 * chart_rect.height();
            let high_y = chart_rect.max.y - ((candle.high - min_price) / price_range) as f32 * chart_rect.height();
            let low_y = chart_rect.max.y - ((candle.low - min_price) / price_range) as f32 * chart_rect.height();

            // Draw wick
            ui.painter().line_segment(
                [Pos2::new(x + width / 2.0, high_y), Pos2::new(x + width / 2.0, low_y)],
                egui::Stroke::new(1.0, Color32::GRAY)
            );

            // Draw body
            let body_color = if candle.close >= candle.open {
                ScreenerTheme::BUY_COLOR
            } else {
                ScreenerTheme::SELL_COLOR
            };

            let body_rect = Rect::from_min_max(
                Pos2::new(x + 1.0, open_y.min(close_y)),
                Pos2::new(x + width - 1.0, open_y.max(close_y))
            );
            ui.painter().rect_stroke(body_rect, 0.0, egui::Stroke::new(1.0, body_color));
        }

        // Draw footprint cells
        for (price_tick, cell) in &candle.cells {
            let cell_price = *price_tick as f64 * candle.tick_size;
            let cell_y = chart_rect.max.y - ((cell_price - min_price) / price_range) as f32 * chart_rect.height();

            // Calculate cell height (price tick height in pixels)
            let tick_height = (candle.tick_size / price_range) as f32 * chart_rect.height();
            let cell_height = tick_height.max(12.0); // Minimum readable height

            let cell_rect = Rect::from_min_size(
                Pos2::new(x + 2.0, cell_y - cell_height / 2.0),
                Vec2::new(width - 4.0, cell_height)
            );

            // Color based on volume and delta
            let color = self.get_cell_color(cell, max_volume);
            ui.painter().rect_filled(cell_rect, 0.0, color);

            // Draw text if cell is large enough
            if cell_height > 10.0 && width > 40.0 {
                let text = if self.show_delta {
                    format!("{}", cell.delta())
                } else if self.show_volume {
                    format!("{}", cell.total_volume())
                } else {
                    format!("{}|{}", cell.ask_volume, cell.bid_volume)
                };

                ui.painter().text(
                    cell_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::monospace(8.0),
                    Color32::WHITE
                );
            }
        }
    }

    fn get_cell_color(&self, cell: &FootprintCell, max_volume: u64) -> Color32 {
        if max_volume == 0 {
            return Color32::TRANSPARENT;
        }

        let volume_intensity = (cell.total_volume() as f32 / max_volume as f32).clamp(0.0, 1.0);
        let delta = cell.delta();

        if self.show_delta {
            // Delta-based coloring
            let max_delta = (cell.ask_volume + cell.bid_volume) as f32;
            if max_delta > 0.0 {
                let delta_ratio = (delta as f32 / max_delta).clamp(-1.0, 1.0);
                if delta_ratio > 0.0 {
                    // More buying (green)
                    let intensity = (delta_ratio * volume_intensity * 255.0) as u8;
                    Color32::from_rgb(0, intensity, 0)
                } else {
                    // More selling (red)
                    let intensity = (-delta_ratio * volume_intensity * 255.0) as u8;
                    Color32::from_rgb(intensity, 0, 0)
                }
            } else {
                Color32::GRAY
            }
        } else {
            // Volume-based coloring
            let intensity = (volume_intensity * 255.0) as u8;
            Color32::from_rgb(intensity, intensity, intensity / 2)
        }
    }

    fn draw_heatmap_for_candle(&self, ui: &mut Ui, candle: &FootprintCandle, x: f32, width: f32, chart_rect: Rect, min_price: f64, max_price: f64) {
        let price_range = max_price - min_price;

        // Find the nearest depth snapshot for this candle's timestamp
        // We'll use the most recent snapshot available (depth snapshots are per-symbol)
        let symbol = &self.selected_symbol;
        if let Some(snapshots) = self.depth_snapshots.get(symbol) {
            if let Some(snapshot) = snapshots.back() {
                // Find max volume for normalization
                let max_bid_volume = snapshot.bids.iter().map(|(_, qty)| *qty).fold(0.0f64, f64::max);
                let max_ask_volume = snapshot.asks.iter().map(|(_, qty)| *qty).fold(0.0f64, f64::max);
                let max_volume = max_bid_volume.max(max_ask_volume);

                if max_volume > 0.0 {
                    // Draw bid levels (green) - only within visible price range
                    for (price, quantity) in &snapshot.bids {
                        if *price >= min_price && *price <= max_price {
                            let cell_y = chart_rect.max.y - ((*price - min_price) / price_range) as f32 * chart_rect.height();

                            // Calculate height for this price level (proportional to tick size)
                            let level_height = (candle.tick_size / price_range) as f32 * chart_rect.height();
                            let level_height = level_height.max(2.0); // Minimum 2px

                            let rect = Rect::from_min_size(
                                Pos2::new(x, cell_y - level_height / 2.0),
                                Vec2::new(width, level_height)
                            );

                            // Calculate color intensity based on volume
                            let volume_pct = (*quantity / max_volume) as f32;
                            let color = self.heatmap_color_scheme.get_bid_color(volume_pct);

                            // Apply user-controlled opacity
                            let color_with_opacity = Color32::from_rgba_premultiplied(
                                color.r(),
                                color.g(),
                                color.b(),
                                (color.a() as f32 * self.heatmap_opacity) as u8
                            );

                            ui.painter().rect_filled(rect, 0.0, color_with_opacity);
                        }
                    }

                    // Draw ask levels (red) - only within visible price range
                    for (price, quantity) in &snapshot.asks {
                        if *price >= min_price && *price <= max_price {
                            let cell_y = chart_rect.max.y - ((*price - min_price) / price_range) as f32 * chart_rect.height();

                            let level_height = (candle.tick_size / price_range) as f32 * chart_rect.height();
                            let level_height = level_height.max(2.0);

                            let rect = Rect::from_min_size(
                                Pos2::new(x, cell_y - level_height / 2.0),
                                Vec2::new(width, level_height)
                            );

                            let volume_pct = (*quantity / max_volume) as f32;
                            let color = self.heatmap_color_scheme.get_ask_color(volume_pct);

                            let color_with_opacity = Color32::from_rgba_premultiplied(
                                color.r(),
                                color.g(),
                                color.b(),
                                (color.a() as f32 * self.heatmap_opacity) as u8
                            );

                            ui.painter().rect_filled(rect, 0.0, color_with_opacity);
                        }
                    }
                }
            }
        }
    }

    fn handle_mouse_interactions(&mut self, ui: &mut Ui, chart_rect: Rect) {
        let response = ui.allocate_rect(chart_rect, egui::Sense::click_and_drag());

        // Check which mouse button is being used
        let is_primary_down = ui.input(|i| i.pointer.primary_down());
        let is_secondary_down = ui.input(|i| i.pointer.secondary_down());

        // Handle scroll wheel for uniform zooming
        if response.hovered() {
            ui.input(|i| {
                let scroll_delta = i.scroll_delta.y;
                if scroll_delta != 0.0 {
                    let zoom_factor = 1.0 + scroll_delta * 0.001;
                    let old_zoom = self.zoom_level;
                    self.zoom_level = (self.zoom_level * zoom_factor).clamp(self.min_zoom, self.max_zoom);

                    // Adjust pan to zoom towards mouse position
                    if let Some(mouse_pos) = i.pointer.hover_pos() {
                        let relative_x = (mouse_pos.x - chart_rect.min.x) / chart_rect.width();
                        let relative_y = (mouse_pos.y - chart_rect.min.y) / chart_rect.height();

                        let zoom_ratio = self.zoom_level / old_zoom;
                        self.pan_x = self.pan_x * zoom_ratio + chart_rect.width() * relative_x * (1.0 - zoom_ratio);
                        self.pan_y = self.pan_y * zoom_ratio + chart_rect.height() * relative_y * (1.0 - zoom_ratio);
                    }
                }
            });
        }

        // Handle RIGHT-CLICK drag for axis-specific scaling
        if is_secondary_down && response.dragged() {
            if let Some(current_pos) = response.interact_pointer_pos() {
                // Initialize drag if starting
                if !self.right_dragging {
                    self.right_dragging = true;
                    self.drag_start_pos = Some(current_pos);
                    self.last_mouse_pos = Some(current_pos);
                    self.drag_axis = None;
                }

                if let Some(last_pos) = self.last_mouse_pos {
                    let delta = current_pos - last_pos;
                    let abs_delta_x = delta.x.abs();
                    let abs_delta_y = delta.y.abs();

                    // Determine drag axis if not yet determined (use 10px threshold)
                    if self.drag_axis.is_none() {
                        if abs_delta_x > 10.0 || abs_delta_y > 10.0 {
                            if abs_delta_x > abs_delta_y {
                                self.drag_axis = Some(DragAxis::Horizontal);
                            } else {
                                self.drag_axis = Some(DragAxis::Vertical);
                            }
                        }
                    }

                    // Apply scaling based on determined axis
                    match self.drag_axis {
                        Some(DragAxis::Horizontal) => {
                            // Scale X-axis (time) - drag right = expand, drag left = compress
                            let scale_change = delta.x * self.x_scale_sensitivity;
                            self.x_scale = (self.x_scale + scale_change).clamp(0.1, 10.0);
                        }
                        Some(DragAxis::Vertical) => {
                            // Scale Y-axis (price) - drag down = expand, drag up = compress
                            let scale_change = delta.y * self.y_scale_sensitivity;
                            self.y_scale = (self.y_scale + scale_change).clamp(0.1, 10.0);
                        }
                        None => {
                            // Still determining axis, don't scale yet
                        }
                    }

                    self.last_mouse_pos = Some(current_pos);
                }
            }
        } else if self.right_dragging && !is_secondary_down {
            // Right-click released
            self.right_dragging = false;
            self.drag_start_pos = None;
            self.last_mouse_pos = None;
            self.drag_axis = None;
        }

        // Handle LEFT-CLICK drag for panning (only if not right-dragging)
        if is_primary_down && response.dragged() && !self.right_dragging {
            if let Some(current_pos) = response.interact_pointer_pos() {
                if let Some(last_pos) = self.last_mouse_pos {
                    let delta = current_pos - last_pos;
                    self.pan_x += delta.x;
                    self.pan_y += delta.y;
                }
                self.last_mouse_pos = Some(current_pos);
                self.dragging = true;
            }
        } else if self.dragging && !is_primary_down {
            // Left-click released
            self.dragging = false;
            self.last_mouse_pos = None;
        }

        // Draw visual indicator for axis scaling
        if self.right_dragging && self.drag_axis.is_some() {
            let painter = ui.painter();
            if let (Some(start_pos), Some(current_pos)) = (self.drag_start_pos, self.last_mouse_pos) {
                match self.drag_axis {
                    Some(DragAxis::Horizontal) => {
                        // Draw horizontal line
                        painter.line_segment(
                            [Pos2::new(chart_rect.min.x, start_pos.y), Pos2::new(chart_rect.max.x, start_pos.y)],
                            egui::Stroke::new(2.0, Color32::from_rgb(100, 150, 255))
                        );
                        // Draw scale indicator text
                        painter.text(
                            Pos2::new(current_pos.x, start_pos.y - 20.0),
                            egui::Align2::CENTER_BOTTOM,
                            format!("X-Scale: {:.2}x", self.x_scale),
                            egui::FontId::proportional(14.0),
                            Color32::WHITE
                        );
                    }
                    Some(DragAxis::Vertical) => {
                        // Draw vertical line
                        painter.line_segment(
                            [Pos2::new(start_pos.x, chart_rect.min.y), Pos2::new(start_pos.x, chart_rect.max.y)],
                            egui::Stroke::new(2.0, Color32::from_rgb(100, 150, 255))
                        );
                        // Draw scale indicator text
                        painter.text(
                            Pos2::new(start_pos.x + 10.0, current_pos.y),
                            egui::Align2::LEFT_CENTER,
                            format!("Y-Scale: {:.2}x", self.y_scale),
                            egui::FontId::proportional(14.0),
                            Color32::WHITE
                        );
                    }
                    None => {}
                }
            }
        }

        // Handle keyboard shortcuts for navigation
        ui.input(|i| {
            // Arrow keys for panning
            if i.key_pressed(egui::Key::ArrowLeft) {
                self.pan_x += 20.0;
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                self.pan_x -= 20.0;
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                self.pan_y += 20.0;
            }
            if i.key_pressed(egui::Key::ArrowDown) {
                self.pan_y -= 20.0;
            }

            // Plus/minus keys for zooming
            if i.key_pressed(egui::Key::PlusEquals) {
                self.zoom_level = (self.zoom_level * 1.2).min(self.max_zoom);
            }
            if i.key_pressed(egui::Key::Minus) {
                self.zoom_level = (self.zoom_level / 1.2).max(self.min_zoom);
            }

            // Home key to reset view
            if i.key_pressed(egui::Key::Home) {
                self.zoom_level = 1.0;
                self.x_scale = 1.0;
                self.y_scale = 1.0;
                self.pan_x = 0.0;
                self.pan_y = 0.0;
            }
        });
    }

    fn draw_candle_statistics(&mut self, ui: &mut Ui, candles: &[FootprintCandle], start_pos: Pos2, width: f32, height: f32, pan_x: f32, candle_width: f32) {
        let stats_rect = Rect::from_min_size(start_pos, Vec2::new(width, height));

        // Use provided candle_width instead of calculating
        let stats_per_row = (width / 100.0).max(1.0) as usize; // Minimum 100px per stat column

        // Draw background
        ui.painter().rect_filled(stats_rect, 0.0, Color32::from_rgb(30, 30, 30));

        // Header labels
        let header_y = start_pos.y + 5.0;
        ui.painter().text(
            Pos2::new(start_pos.x + 10.0, header_y),
            egui::Align2::LEFT_TOP,
            "Delta",
            egui::FontId::monospace(10.0),
            Color32::WHITE
        );
        ui.painter().text(
            Pos2::new(start_pos.x + 10.0, header_y + 15.0),
            egui::Align2::LEFT_TOP,
            "Volume",
            egui::FontId::monospace(10.0),
            Color32::WHITE
        );
        ui.painter().text(
            Pos2::new(start_pos.x + 10.0, header_y + 30.0),
            egui::Align2::LEFT_TOP,
            "CVD",
            egui::FontId::monospace(10.0),
            Color32::WHITE
        );

        // Calculate which candles are visible based on pan_x (matching candlestick visibility logic)
        let chart_start_x = start_pos.x + 80.0;
        let chart_width = width - 80.0;
        let visible_start_index = (-pan_x / candle_width).max(0.0) as usize;
        let visible_end_index = ((chart_width - pan_x) / candle_width).min(candles.len() as f32) as usize;

        // Reset and recalculate CVD for the selected symbol from the beginning of visible candles
        // This ensures CVD is cumulative across all candles
        let mut running_cvd: i64 = 0;

        // Draw statistics for each visible candle
        for (i, candle) in candles.iter().enumerate().skip(visible_start_index).take(visible_end_index - visible_start_index) {
            let x = chart_start_x + i as f32 * candle_width + pan_x;

            // Only draw if within visible bounds
            if x + candle_width < chart_start_x || x > start_pos.x + width {
                continue;
            }

            // Calculate delta, volume, CVD from all cells
            let mut total_ask_volume = 0u64;
            let mut total_bid_volume = 0u64;

            for cell in candle.cells.values() {
                total_ask_volume += cell.ask_volume;
                total_bid_volume += cell.bid_volume;
            }

            let delta = total_ask_volume as i64 - total_bid_volume as i64;
            let total_volume = candle.max_volume();

            // Calculate TRUE Cumulative Volume Delta (CVD)
            // CVD = sum of all deltas from start to current candle
            running_cvd += delta;

            // Delta
            let delta_color = if delta > 0 { Color32::GREEN } else { Color32::RED };
            ui.painter().text(
                Pos2::new(x + candle_width / 2.0, header_y),
                egui::Align2::CENTER_TOP,
                format!("{:+}", delta),
                egui::FontId::monospace(8.0),
                delta_color
            );

            // Volume
            ui.painter().text(
                Pos2::new(x + candle_width / 2.0, header_y + 15.0),
                egui::Align2::CENTER_TOP,
                format!("{}", total_volume),
                egui::FontId::monospace(8.0),
                Color32::LIGHT_BLUE
            );

            // CVD (TRUE cumulative value)
            let cvd_color = if running_cvd > 0 { Color32::GREEN } else { Color32::RED };
            ui.painter().text(
                Pos2::new(x + candle_width / 2.0, header_y + 30.0),
                egui::Align2::CENTER_TOP,
                format!("{:+}", running_cvd),
                egui::FontId::monospace(8.0),
                cvd_color
            );

            // Draw separator lines
            if i < candles.len() - 1 {
                ui.painter().line_segment(
                    [Pos2::new(x + candle_width, start_pos.y), Pos2::new(x + candle_width, start_pos.y + height)],
                    egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 60))
                );
            }
        }
    }

    fn draw_price_axis(&self, ui: &mut Ui, chart_rect: Rect, min_price: f64, max_price: f64, axis_x: f32, axis_width: f32) {
        let price_range = max_price - min_price;
        if price_range <= 0.0 {
            return;
        }

        let axis_rect = Rect::from_min_size(
            Pos2::new(axis_x, chart_rect.min.y),
            Vec2::new(axis_width, chart_rect.height())
        );

        // Draw background
        ui.painter().rect_filled(axis_rect, 0.0, Color32::from_rgb(40, 40, 40));

        // Calculate price tick intervals
        let num_ticks = 10;
        let tick_interval = price_range / num_ticks as f64;

        for i in 0..=num_ticks {
            let price = min_price + (i as f64 * tick_interval);
            let y = chart_rect.max.y - ((price - min_price) / price_range * chart_rect.height() as f64) as f32;

            // Draw tick line
            ui.painter().line_segment(
                [Pos2::new(axis_x + axis_width - 5.0, y), Pos2::new(axis_x + axis_width, y)],
                egui::Stroke::new(1.0, Color32::GRAY)
            );

            // Draw price label
            ui.painter().text(
                Pos2::new(axis_x + axis_width - 10.0, y),
                egui::Align2::RIGHT_CENTER,
                format!("{:.4}", price),
                egui::FontId::monospace(8.0),
                Color32::WHITE
            );
        }
    }

    fn draw_time_axis(&self, ui: &mut Ui, candles: &[FootprintCandle], chart_rect: Rect, axis_y: f32, axis_height: f32, candle_width: f32) {
        let axis_rect = Rect::from_min_size(
            Pos2::new(chart_rect.min.x, axis_y),
            Vec2::new(chart_rect.width(), axis_height)
        );

        // Draw background
        ui.painter().rect_filled(axis_rect, 0.0, Color32::from_rgb(40, 40, 40));

        // Draw time labels for visible candles
        let visible_start_index = (-self.pan_x / candle_width).max(0.0) as usize;
        let visible_end_index = ((chart_rect.width() - self.pan_x) / candle_width).min(candles.len() as f32) as usize;

        // Show every nth candle timestamp to avoid overcrowding
        let label_interval = ((visible_end_index - visible_start_index) / 6).max(1);

        for (i, candle) in candles.iter().enumerate().skip(visible_start_index).take(visible_end_index - visible_start_index) {
            if i % label_interval == 0 {
                let x = chart_rect.min.x + i as f32 * candle_width + self.pan_x;

                if x >= chart_rect.min.x && x <= chart_rect.max.x {
                    // Convert timestamp to readable format
                    let datetime = chrono::DateTime::from_timestamp(candle.timestamp as i64 / 1000, 0)
                        .unwrap_or_else(|| chrono::Utc::now());
                    let time_str = datetime.format("%H:%M").to_string();

                    // Draw tick line
                    ui.painter().line_segment(
                        [Pos2::new(x, axis_y), Pos2::new(x, axis_y + 5.0)],
                        egui::Stroke::new(1.0, Color32::GRAY)
                    );

                    // Draw time label
                    ui.painter().text(
                        Pos2::new(x, axis_y + 8.0),
                        egui::Align2::CENTER_TOP,
                        time_str,
                        egui::FontId::monospace(8.0),
                        Color32::WHITE
                    );
                }
            }
        }
    }

    fn update_symbols_for_category(&mut self) {
        let symbols_by_category = BinanceSymbols::get_symbols_by_category();

        self.symbols = match self.symbol_category.as_str() {
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

        // Update selected symbol if it's not in the new list
        if !self.symbols.contains(&self.selected_symbol) {
            self.selected_symbol = self.symbols.first().unwrap_or(&"BTCUSDT".to_string()).clone();
        }
    }
}