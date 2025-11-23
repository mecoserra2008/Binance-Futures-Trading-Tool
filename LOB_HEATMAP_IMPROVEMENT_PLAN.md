# LOB Heatmap & Trading Platform Enhancement Plan

## Executive Summary

This comprehensive improvement plan details the implementation of a professional-grade LOB (Limit Order Book) heatmap visualization system integrated with the existing footprint chart, along with major platform enhancements including detachable DOM, advanced timeframe management, interactive axis controls, and technical analysis tools.

**Current Architecture**: Rust-based real-time trading platform using:
- **GUI Framework**: egui 0.24 (immediate-mode)
- **Runtime**: Tokio async multi-threaded
- **Data Source**: Binance WebSocket (aggTrade, forceOrder streams)
- **Base Timeframe**: 1-minute candles (60,000ms)
- **Current Visualization**: Footprint chart with volume-at-price cells

---

## Table of Contents

1. [Phase 1: LOB Depth Data Integration](#phase-1-lob-depth-data-integration)
2. [Phase 2: LOB Heatmap Visualization Layer](#phase-2-lob-heatmap-visualization-layer)
3. [Phase 3: Advanced Axis Control System](#phase-3-advanced-axis-control-system)
4. [Phase 4: Detachable DOM Window](#phase-4-detachable-dom-window)
5. [Phase 5: Enhanced Timeframe Management](#phase-5-enhanced-timeframe-management)
6. [Phase 6: Technical Analysis Tools](#phase-6-technical-analysis-tools)
7. [Implementation Timeline & Dependencies](#implementation-timeline--dependencies)
8. [Performance Considerations](#performance-considerations)

---

## Phase 1: LOB Depth Data Integration

### 1.1 Current State Analysis

**File**: `src/data/websocket.rs` (WebSocketManager)

**Current Streams**:
- `aggTrade` - Aggregated trade data (price, quantity, buyer/seller)
- `forceOrder` - Liquidation events

**Missing**:
- `depth@100ms` or `depth` - Real-time order book snapshots
- Depth updates with bid/ask levels

### 1.2 New Data Structures

**File to Create**: `src/data/orderbook.rs`

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ordered_float::OrderedFloat;

/// Represents a single price level in the order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthLevel {
    pub price: f64,
    pub quantity: f64,
    pub timestamp: u64,
}

/// Complete order book snapshot for a symbol
#[derive(Debug, Clone)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: BTreeMap<OrderedFloat<f64>, f64>,  // price -> quantity
    pub asks: BTreeMap<OrderedFloat<f64>, f64>,
    pub last_update_id: u64,
    pub timestamp: u64,
}

impl OrderBook {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_update_id: 0,
            timestamp: 0,
        }
    }

    /// Apply depth update from WebSocket
    pub fn apply_update(&mut self, update: DepthUpdate) {
        // Update bids
        for level in update.bids {
            if level.quantity == 0.0 {
                self.bids.remove(&OrderedFloat(level.price));
            } else {
                self.bids.insert(OrderedFloat(level.price), level.quantity);
            }
        }

        // Update asks
        for level in update.asks {
            if level.quantity == 0.0 {
                self.asks.remove(&OrderedFloat(level.price));
            } else {
                self.asks.insert(OrderedFloat(level.price), level.quantity);
            }
        }

        self.last_update_id = update.last_update_id;
        self.timestamp = update.event_time;
    }

    /// Get top N levels for heatmap visualization
    pub fn get_depth_snapshot(&self, num_levels: usize) -> DepthSnapshot {
        let bids: Vec<(f64, f64)> = self.bids.iter()
            .rev()
            .take(num_levels)
            .map(|(p, q)| (p.0, *q))
            .collect();

        let asks: Vec<(f64, f64)> = self.asks.iter()
            .take(num_levels)
            .map(|(p, q)| (p.0, *q))
            .collect();

        DepthSnapshot {
            bids,
            asks,
            timestamp: self.timestamp,
        }
    }

    /// Calculate cumulative depth for heatmap intensity
    pub fn get_cumulative_depth(&self, price_levels: &[i64], tick_size: f64) -> Vec<(i64, f64, f64)> {
        let mut result = Vec::new();

        for &price_tick in price_levels {
            let price = price_tick as f64 * tick_size;
            let bid_depth = self.get_bid_depth_at_price(price);
            let ask_depth = self.get_ask_depth_at_price(price);
            result.push((price_tick, bid_depth, ask_depth));
        }

        result
    }

    fn get_bid_depth_at_price(&self, price: f64) -> f64 {
        self.bids.iter()
            .filter(|(p, _)| p.0 <= price)
            .map(|(_, q)| q)
            .sum()
    }

    fn get_ask_depth_at_price(&self, price: f64) -> f64 {
        self.asks.iter()
            .filter(|(p, _)| p.0 >= price)
            .map(|(_, q)| q)
            .sum()
    }
}

/// Incremental depth update from WebSocket
#[derive(Debug, Clone, Deserialize)]
pub struct DepthUpdate {
    #[serde(rename = "e")]
    pub event_type: String,  // "depthUpdate"

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "U")]
    pub first_update_id: u64,

    #[serde(rename = "u")]
    pub last_update_id: u64,

    #[serde(rename = "b")]
    pub bids: Vec<BidAsk>,

    #[serde(rename = "a")]
    pub asks: Vec<BidAsk>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BidAsk {
    #[serde(rename = "0")]
    pub price: f64,

    #[serde(rename = "1")]
    pub quantity: f64,
}

/// Snapshot for heatmap rendering
#[derive(Debug, Clone)]
pub struct DepthSnapshot {
    pub bids: Vec<(f64, f64)>,  // (price, quantity)
    pub asks: Vec<(f64, f64)>,
    pub timestamp: u64,
}

/// Historical depth data for heatmap background
#[derive(Debug, Clone)]
pub struct DepthHistory {
    pub symbol: String,
    pub snapshots: Vec<TimedDepthSnapshot>,
    pub max_history: usize,  // Number of snapshots to keep
}

#[derive(Debug, Clone)]
pub struct TimedDepthSnapshot {
    pub timestamp: u64,
    pub depth: DepthSnapshot,
    pub aggregated_bids: BTreeMap<i64, f64>,  // price_tick -> cumulative quantity
    pub aggregated_asks: BTreeMap<i64, f64>,
}

impl DepthHistory {
    pub fn new(symbol: String, max_history: usize) -> Self {
        Self {
            symbol,
            snapshots: Vec::with_capacity(max_history),
            max_history,
        }
    }

    pub fn add_snapshot(&mut self, snapshot: TimedDepthSnapshot) {
        self.snapshots.push(snapshot);
        if self.snapshots.len() > self.max_history {
            self.snapshots.remove(0);
        }
    }

    /// Get depth intensity at specific time and price for heatmap rendering
    pub fn get_intensity_at(&self, timestamp: u64, price_tick: i64) -> Option<(f64, f64)> {
        // Find closest snapshot
        let snapshot = self.snapshots.iter()
            .min_by_key(|s| (s.timestamp as i64 - timestamp as i64).abs())?;

        let bid_qty = snapshot.aggregated_bids.get(&price_tick).copied().unwrap_or(0.0);
        let ask_qty = snapshot.aggregated_asks.get(&price_tick).copied().unwrap_or(0.0);

        Some((bid_qty, ask_qty))
    }
}
```

### 1.3 WebSocket Integration

**File to Modify**: `src/data/websocket.rs`

**Changes Required**:

1. Add depth stream subscription:

```rust
// In WebSocketManager
pub async fn subscribe_to_depth(
    &mut self,
    symbols: &[String],
    update_speed: &str,  // "100ms" or "1000ms"
) -> Result<()> {
    let streams: Vec<String> = symbols.iter()
        .map(|s| format!("{}@depth@{}", s.to_lowercase(), update_speed))
        .collect();

    // Send subscription message
    let subscribe_msg = json!({
        "method": "SUBSCRIBE",
        "params": streams,
        "id": 1
    });

    // ... send via WebSocket
}
```

2. Add depth message handler:

```rust
// In message processing loop
match event_type.as_str() {
    "depthUpdate" => {
        let depth_update: DepthUpdate = serde_json::from_str(&msg)?;
        if let Some(sender) = &self.depth_sender {
            sender.try_send(depth_update)?;
        }
    },
    "aggTrade" => { /* existing */ },
    "forceOrder" => { /* existing */ },
    _ => {}
}
```

3. Add new channel sender:

```rust
pub struct WebSocketManager {
    // ... existing fields
    depth_sender: Option<mpsc::Sender<DepthUpdate>>,
}
```

### 1.4 Order Book Manager

**File to Create**: `src/data/orderbook_manager.rs`

```rust
use tokio::sync::mpsc;
use std::collections::HashMap;
use crate::data::orderbook::{OrderBook, DepthUpdate, DepthHistory};

pub struct OrderBookManager {
    orderbooks: HashMap<String, OrderBook>,
    depth_histories: HashMap<String, DepthHistory>,
    depth_receiver: mpsc::Receiver<DepthUpdate>,

    // Channels to send processed data to GUI
    snapshot_sender: mpsc::Sender<DepthSnapshot>,

    max_levels: usize,  // Max depth levels to maintain
    snapshot_interval_ms: u64,  // How often to snapshot for history
}

impl OrderBookManager {
    pub fn new(
        depth_receiver: mpsc::Receiver<DepthUpdate>,
        snapshot_sender: mpsc::Sender<DepthSnapshot>,
    ) -> Self {
        Self {
            orderbooks: HashMap::new(),
            depth_histories: HashMap::new(),
            depth_receiver,
            snapshot_sender,
            max_levels: 100,  // Configurable
            snapshot_interval_ms: 100,  // 100ms snapshots for heatmap
        }
    }

    pub async fn start(mut self) {
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
        let orderbook = self.orderbooks
            .entry(update.symbol.clone())
            .or_insert_with(|| OrderBook::new(update.symbol.clone()));

        orderbook.apply_update(update);
    }

    fn capture_snapshots(&mut self) {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;

        for (symbol, orderbook) in &self.orderbooks {
            let snapshot = orderbook.get_depth_snapshot(self.max_levels);

            // Store in history
            let history = self.depth_histories
                .entry(symbol.clone())
                .or_insert_with(|| DepthHistory::new(symbol.clone(), 500));  // 50 seconds at 100ms

            // Aggregate to price ticks
            let timed_snapshot = self.create_timed_snapshot(snapshot.clone(), timestamp);
            history.add_snapshot(timed_snapshot);

            // Send to GUI
            let _ = self.snapshot_sender.try_send(snapshot);
        }
    }

    fn create_timed_snapshot(&self, snapshot: DepthSnapshot, timestamp: u64) -> TimedDepthSnapshot {
        // Implementation details for aggregation to price ticks
        // ...
    }
}
```

### 1.5 Integration with Main Application

**File to Modify**: `src/main.rs`

**Add to main():**

```rust
// Create depth data channel
let (depth_tx, depth_rx) = mpsc::channel::<DepthUpdate>(10000);
let (depth_snapshot_tx, depth_snapshot_rx) = mpsc::channel::<DepthSnapshot>(1000);

// Update WebSocketManager initialization
let mut ws_manager = WebSocketManager::new(
    orderflow_tx.clone(),
    Some(liquidation_tx.clone()),
    Some(depth_tx),  // NEW
);

// Spawn OrderBookManager task
let orderbook_manager = OrderBookManager::new(depth_rx, depth_snapshot_tx);
tokio::spawn(async move {
    orderbook_manager.start().await;
});

// Pass depth_snapshot_rx to ScreenerApp
let app = ScreenerApp::new(
    // ... existing receivers
    Some(depth_snapshot_rx),  // NEW
);
```

---

## Phase 2: LOB Heatmap Visualization Layer

### 2.1 Heatmap Rendering Architecture

**Approach**: Render LOB heatmap as a background layer behind footprint cells using egui's painter API.

**Rendering Order**:
1. **Layer 1 (Background)**: LOB depth heatmap (color gradient based on cumulative volume)
2. **Layer 2 (Foreground)**: Footprint candle cells (existing implementation)
3. **Layer 3 (Overlay)**: OHLC wicks, text labels, axes

### 2.2 Color Gradient System

**File to Create**: `src/gui/heatmap_colors.rs`

```rust
use egui::Color32;

#[derive(Debug, Clone, Copy)]
pub struct HeatmapColorScheme {
    pub bid_color_low: Color32,
    pub bid_color_high: Color32,
    pub ask_color_low: Color32,
    pub ask_color_high: Color32,
    pub intensity: f32,  // 0.0 to 1.0 (user-controllable)
}

impl Default for HeatmapColorScheme {
    fn default() -> Self {
        Self {
            bid_color_low: Color32::from_rgba_premultiplied(0, 100, 0, 20),    // Transparent green
            bid_color_high: Color32::from_rgba_premultiplied(0, 255, 100, 180), // Bright green
            ask_color_low: Color32::from_rgba_premultiplied(100, 0, 0, 20),     // Transparent red
            ask_color_high: Color32::from_rgba_premultiplied(255, 50, 50, 180), // Bright red
            intensity: 0.7,  // Default 70% intensity
        }
    }
}

impl HeatmapColorScheme {
    /// Get interpolated color based on volume percentage (0.0 to 1.0)
    pub fn get_bid_color(&self, volume_pct: f32) -> Color32 {
        self.interpolate_color(
            self.bid_color_low,
            self.bid_color_high,
            volume_pct * self.intensity
        )
    }

    pub fn get_ask_color(&self, volume_pct: f32) -> Color32 {
        self.interpolate_color(
            self.ask_color_low,
            self.ask_color_high,
            volume_pct * self.intensity
        )
    }

    fn interpolate_color(&self, color1: Color32, color2: Color32, t: f32) -> Color32 {
        let t = t.clamp(0.0, 1.0);
        let r = (color1.r() as f32 * (1.0 - t) + color2.r() as f32 * t) as u8;
        let g = (color1.g() as f32 * (1.0 - t) + color2.g() as f32 * t) as u8;
        let b = (color1.b() as f32 * (1.0 - t) + color2.b() as f32 * t) as u8;
        let a = (color1.a() as f32 * (1.0 - t) + color2.a() as f32 * t) as u8;
        Color32::from_rgba_premultiplied(r, g, b, a)
    }

    /// Preset color schemes
    pub fn green_red_default() -> Self { Self::default() }

    pub fn blue_orange() -> Self {
        Self {
            bid_color_low: Color32::from_rgba_premultiplied(0, 50, 100, 20),
            bid_color_high: Color32::from_rgba_premultiplied(50, 150, 255, 180),
            ask_color_low: Color32::from_rgba_premultiplied(100, 50, 0, 20),
            ask_color_high: Color32::from_rgba_premultiplied(255, 150, 50, 180),
            intensity: 0.7,
        }
    }

    pub fn monochrome() -> Self {
        Self {
            bid_color_low: Color32::from_rgba_premultiplied(0, 0, 0, 20),
            bid_color_high: Color32::from_rgba_premultiplied(150, 150, 150, 180),
            ask_color_low: Color32::from_rgba_premultiplied(0, 0, 0, 20),
            ask_color_high: Color32::from_rgba_premultiplied(150, 150, 150, 180),
            intensity: 0.7,
        }
    }
}
```

### 2.3 Heatmap Data Processor

**File to Modify**: `src/gui/footprint_panel.rs`

**Add to FootprintPanel struct:**

```rust
pub struct FootprintPanel {
    // ... existing fields

    // LOB Heatmap fields
    depth_snapshots: HashMap<String, VecDeque<DepthSnapshot>>,
    heatmap_enabled: bool,
    heatmap_color_scheme: HeatmapColorScheme,
    heatmap_intensity: f32,  // User-controllable slider value
    heatmap_opacity: f32,    // Overall opacity
    heatmap_aggregation_levels: usize,  // How many price ticks to aggregate

    // Volume normalization for color scaling
    max_bid_volume: f64,
    max_ask_volume: f64,
}
```

**Add heatmap rendering method:**

```rust
impl FootprintPanel {
    fn render_lob_heatmap(
        &self,
        ui: &mut egui::Ui,
        chart_rect: egui::Rect,
        visible_candles: &[FootprintCandle],
        price_to_screen: impl Fn(f64) -> f32,
        time_to_screen: impl Fn(u64) -> f32,
    ) {
        if !self.heatmap_enabled {
            return;
        }

        let painter = ui.painter();

        // Get depth history for selected symbol
        let depth_snapshots = match self.depth_snapshots.get(&self.selected_symbol) {
            Some(snapshots) => snapshots,
            None => return,
        };

        // Iterate through visible candles and render heatmap cells behind them
        for candle in visible_candles {
            let candle_start_time = candle.candle.timestamp;
            let candle_end_time = candle_start_time + 60000;  // 1-minute candle

            // Get corresponding depth snapshot (nearest timestamp)
            if let Some(depth) = self.get_depth_at_timestamp(depth_snapshots, candle_start_time) {
                self.render_depth_for_candle(
                    painter,
                    candle,
                    depth,
                    &price_to_screen,
                    &time_to_screen,
                    chart_rect,
                );
            }
        }
    }

    fn render_depth_for_candle(
        &self,
        painter: &egui::Painter,
        candle: &FootprintCandle,
        depth: &DepthSnapshot,
        price_to_screen: impl Fn(f64) -> f32,
        time_to_screen: impl Fn(u64) -> f32,
        chart_rect: egui::Rect,
    ) {
        let candle_x = time_to_screen(candle.candle.timestamp);
        let candle_width = 20.0 * self.zoom_level;  // Match footprint candle width

        // Render bid heatmap (left side of candle)
        for (price, quantity) in &depth.bids {
            let y = price_to_screen(*price);
            let volume_pct = (*quantity / self.max_bid_volume) as f32;
            let color = self.heatmap_color_scheme.get_bid_color(volume_pct);

            let rect = egui::Rect::from_min_size(
                egui::pos2(candle_x, y - self.price_scale as f32 / 2.0),
                egui::vec2(candle_width / 2.0, self.price_scale as f32),
            );

            painter.rect_filled(rect, 0.0, color);
        }

        // Render ask heatmap (right side of candle)
        for (price, quantity) in &depth.asks {
            let y = price_to_screen(*price);
            let volume_pct = (*quantity / self.max_ask_volume) as f32;
            let color = self.heatmap_color_scheme.get_ask_color(volume_pct);

            let rect = egui::Rect::from_min_size(
                egui::pos2(candle_x + candle_width / 2.0, y - self.price_scale as f32 / 2.0),
                egui::vec2(candle_width / 2.0, self.price_scale as f32),
            );

            painter.rect_filled(rect, 0.0, color);
        }
    }

    fn get_depth_at_timestamp(
        &self,
        snapshots: &VecDeque<DepthSnapshot>,
        timestamp: u64,
    ) -> Option<&DepthSnapshot> {
        snapshots.iter()
            .min_by_key(|s| (s.timestamp as i64 - timestamp as i64).abs())
    }

    fn update_max_volumes(&mut self) {
        // Calculate max volumes across all snapshots for normalization
        self.max_bid_volume = 0.0;
        self.max_ask_volume = 0.0;

        if let Some(snapshots) = self.depth_snapshots.get(&self.selected_symbol) {
            for snapshot in snapshots {
                for (_, qty) in &snapshot.bids {
                    self.max_bid_volume = self.max_bid_volume.max(*qty);
                }
                for (_, qty) in &snapshot.asks {
                    self.max_ask_volume = self.max_ask_volume.max(*qty);
                }
            }
        }
    }
}
```

### 2.4 User Controls for Heatmap

**Add to FootprintPanel UI rendering:**

```rust
// In render_controls() method
ui.horizontal(|ui| {
    ui.label("LOB Heatmap:");
    ui.checkbox(&mut self.heatmap_enabled, "Enabled");

    if self.heatmap_enabled {
        ui.label("Intensity:");
        if ui.add(egui::Slider::new(&mut self.heatmap_intensity, 0.0..=1.0)
            .text(""))
            .changed()
        {
            self.heatmap_color_scheme.intensity = self.heatmap_intensity;
        }

        ui.label("Color Scheme:");
        egui::ComboBox::from_id_source("heatmap_colors")
            .selected_text(self.get_color_scheme_name())
            .show_ui(ui, |ui| {
                if ui.selectable_label(false, "Green/Red").clicked() {
                    self.heatmap_color_scheme = HeatmapColorScheme::green_red_default();
                }
                if ui.selectable_label(false, "Blue/Orange").clicked() {
                    self.heatmap_color_scheme = HeatmapColorScheme::blue_orange();
                }
                if ui.selectable_label(false, "Monochrome").clicked() {
                    self.heatmap_color_scheme = HeatmapColorScheme::monochrome();
                }
            });
    }
});
```

---

## Phase 3: Advanced Axis Control System

### 3.1 Mouse Interaction State Machine

**File to Modify**: `src/gui/footprint_panel.rs`

**Add interaction state tracking:**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum MouseInteractionMode {
    None,
    PanningView,          // Left-click drag
    ScalingXAxis,         // Right-click drag horizontal
    ScalingYAxis,         // Right-click drag vertical
    ScalingBothAxes,      // Right-click drag diagonal
}

pub struct FootprintPanel {
    // ... existing fields

    // Advanced mouse controls
    interaction_mode: MouseInteractionMode,
    last_mouse_pos: Option<egui::Pos2>,
    right_click_start_pos: Option<egui::Pos2>,

    // Independent axis scales
    x_scale: f32,  // Horizontal (time) scale factor
    y_scale: f32,  // Vertical (price) scale factor

    // Scale sensitivity
    x_scale_sensitivity: f32,  // Default 0.01
    y_scale_sensitivity: f32,  // Default 0.01
}
```

### 3.2 Right-Click Drag Scale Implementation

**Add to FootprintPanel::render():**

```rust
impl FootprintPanel {
    fn handle_mouse_interactions(&mut self, ui: &mut egui::Ui, chart_rect: egui::Rect) {
        let response = ui.interact(
            chart_rect,
            ui.id().with("chart_interaction"),
            egui::Sense::click_and_drag(),
        );

        // Get current mouse position
        let current_mouse_pos = ui.input(|i| i.pointer.interact_pos()).unwrap_or_default();

        // Handle right-click drag for scaling
        if response.dragged_by(egui::PointerButton::Secondary) {
            if let Some(last_pos) = self.last_mouse_pos {
                let delta = current_mouse_pos - last_pos;

                // Determine scaling mode based on drag direction
                let abs_dx = delta.x.abs();
                let abs_dy = delta.y.abs();

                if abs_dx > 2.0 || abs_dy > 2.0 {  // Threshold to avoid jitter
                    if abs_dx > abs_dy * 1.5 {
                        // Primarily horizontal drag - scale X axis
                        self.interaction_mode = MouseInteractionMode::ScalingXAxis;
                        self.x_scale *= 1.0 + (delta.x * self.x_scale_sensitivity);
                        self.x_scale = self.x_scale.clamp(0.1, 10.0);
                    } else if abs_dy > abs_dx * 1.5 {
                        // Primarily vertical drag - scale Y axis
                        self.interaction_mode = MouseInteractionMode::ScalingYAxis;
                        self.y_scale *= 1.0 + (delta.y * self.y_scale_sensitivity);
                        self.y_scale = self.y_scale.clamp(0.1, 10.0);
                    } else {
                        // Diagonal drag - scale both axes
                        self.interaction_mode = MouseInteractionMode::ScalingBothAxes;
                        let avg_delta = (delta.x + delta.y) / 2.0;
                        let scale_factor = 1.0 + (avg_delta * self.x_scale_sensitivity);
                        self.x_scale *= scale_factor;
                        self.y_scale *= scale_factor;
                        self.x_scale = self.x_scale.clamp(0.1, 10.0);
                        self.y_scale = self.y_scale.clamp(0.1, 10.0);
                    }
                }
            }
        }

        // Handle left-click drag for panning (existing functionality)
        else if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(last_pos) = self.last_mouse_pos {
                let delta = current_mouse_pos - last_pos;
                self.interaction_mode = MouseInteractionMode::PanningView;
                self.pan_x += delta.x;
                self.pan_y += delta.y;
            }
        } else {
            self.interaction_mode = MouseInteractionMode::None;
        }

        // Handle scroll wheel zoom (existing, but now affects both axes)
        if let Some(scroll_delta) = ui.input(|i| i.scroll_delta.y) {
            if scroll_delta.abs() > 0.1 {
                let zoom_factor = 1.0 + (scroll_delta * 0.001);
                self.zoom_level *= zoom_factor;
                self.zoom_level = self.zoom_level.clamp(0.1, 10.0);

                // Also update individual axis scales
                self.x_scale *= zoom_factor;
                self.y_scale *= zoom_factor;
                self.x_scale = self.x_scale.clamp(0.1, 10.0);
                self.y_scale = self.y_scale.clamp(0.1, 10.0);
            }
        }

        self.last_mouse_pos = Some(current_mouse_pos);

        // Reset on right-click release
        if response.drag_released_by(egui::PointerButton::Secondary) {
            self.interaction_mode = MouseInteractionMode::None;
        }
    }

    fn apply_axis_scales_to_rendering(&self) -> (f32, f32) {
        // Returns (effective_candle_width_scale, effective_price_height_scale)
        (self.x_scale * self.zoom_level, self.y_scale * self.zoom_level)
    }
}
```

### 3.3 Visual Feedback for Scaling

**Add scale indicators:**

```rust
fn render_scale_indicator(&self, ui: &mut egui::Ui, chart_rect: egui::Rect) {
    if self.interaction_mode != MouseInteractionMode::None {
        let text = match self.interaction_mode {
            MouseInteractionMode::ScalingXAxis =>
                format!("X Scale: {:.2}x", self.x_scale),
            MouseInteractionMode::ScalingYAxis =>
                format!("Y Scale: {:.2}x", self.y_scale),
            MouseInteractionMode::ScalingBothAxes =>
                format!("Scale: {:.2}x / {:.2}x", self.x_scale, self.y_scale),
            _ => String::new(),
        };

        if !text.is_empty() {
            let painter = ui.painter();
            let text_pos = chart_rect.center();
            painter.text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(18.0),
                egui::Color32::YELLOW,
            );
        }
    }
}
```

### 3.4 Configuration Options

**Add to config.toml:**

```toml
[gui.footprint]
x_scale_sensitivity = 0.01
y_scale_sensitivity = 0.01
enable_independent_axis_scaling = true
show_scale_indicators = true
```

---

## Phase 4: Detachable DOM Window

### 4.1 Multi-Window Architecture

**Current Limitation**: egui/eframe supports only single-window applications by default.

**Solution**: Use `eframe::App` with multiple native windows OR implement a separate DOM panel that can be "popped out" into a new process/window.

**File to Create**: `src/gui/dom_window.rs`

```rust
use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::data::orderbook::OrderBook;

pub struct DomWindow {
    orderbook: Arc<Mutex<OrderBook>>,
    aggregation_level: f64,  // Price tick size for aggregation
    show_limit_orders: bool,
    show_traded_volume: bool,

    // Display settings
    num_levels_to_show: usize,  // Default 20 levels each side
    color_intensity: f32,

    // Historical traded volume at each price
    traded_volume_history: HashMap<i64, VolumeAtPrice>,  // price_tick -> volume
}

impl DomWindow {
    pub fn new(orderbook: Arc<Mutex<OrderBook>>) -> Self {
        Self {
            orderbook,
            aggregation_level: 0.01,
            show_limit_orders: true,
            show_traded_volume: true,
            num_levels_to_show: 20,
            color_intensity: 0.7,
            traded_volume_history: HashMap::new(),
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Depth of Market (DOM)");

            // Control panel
            self.render_controls(ui);

            ui.separator();

            // DOM visualization
            self.render_dom_table(ui);
        });
    }

    fn render_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Aggregation Level:");
            egui::ComboBox::from_id_source("dom_aggregation")
                .selected_text(format!("{}", self.aggregation_level))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.aggregation_level, 0.01, "0.01");
                    ui.selectable_value(&mut self.aggregation_level, 0.1, "0.1");
                    ui.selectable_value(&mut self.aggregation_level, 1.0, "1.0");
                    ui.selectable_value(&mut self.aggregation_level, 10.0, "10.0");
                });

            ui.checkbox(&mut self.show_limit_orders, "Show Limit Orders");
            ui.checkbox(&mut self.show_traded_volume, "Show Traded Volume");
        });

        ui.horizontal(|ui| {
            ui.label("Levels:");
            ui.add(egui::Slider::new(&mut self.num_levels_to_show, 10..=50));

            ui.label("Color Intensity:");
            ui.add(egui::Slider::new(&mut self.color_intensity, 0.0..=1.0));
        });
    }

    fn render_dom_table(&mut self, ui: &mut egui::Ui) {
        let orderbook = self.orderbook.lock().unwrap();

        // Get aggregated levels
        let (bid_levels, ask_levels) = self.aggregate_orderbook(&orderbook);

        use egui_extras::{TableBuilder, Column};

        TableBuilder::new(ui)
            .striped(true)
            .column(Column::exact(80.0))   // Bid Volume
            .column(Column::exact(100.0))  // Bid Price
            .column(Column::exact(100.0))  // Ask Price
            .column(Column::exact(80.0))   // Ask Volume
            .column(Column::exact(100.0))  // Traded Volume (optional)
            .header(25.0, |mut header| {
                header.col(|ui| { ui.heading("Bid Vol"); });
                header.col(|ui| { ui.heading("Bid Price"); });
                header.col(|ui| { ui.heading("Ask Price"); });
                header.col(|ui| { ui.heading("Ask Vol"); });
                if self.show_traded_volume {
                    header.col(|ui| { ui.heading("Traded"); });
                }
            })
            .body(|mut body| {
                let max_bid_vol = bid_levels.iter().map(|(_, v)| *v).fold(0.0, f64::max);
                let max_ask_vol = ask_levels.iter().map(|(_, v)| *v).fold(0.0, f64::max);

                for i in 0..self.num_levels_to_show {
                    body.row(20.0, |mut row| {
                        // Bid side
                        if let Some((price, volume)) = bid_levels.get(i) {
                            let color_pct = (*volume / max_bid_vol) as f32 * self.color_intensity;
                            let bg_color = egui::Color32::from_rgba_premultiplied(
                                0, (100.0 * color_pct) as u8, 0, (150.0 * color_pct) as u8
                            );

                            row.col(|ui| {
                                ui.colored_label(egui::Color32::GREEN, format!("{:.2}", volume));
                            });
                            row.col(|ui| {
                                ui.colored_label(egui::Color32::GREEN, format!("{:.2}", price));
                            });
                        } else {
                            row.col(|ui| { ui.label(""); });
                            row.col(|ui| { ui.label(""); });
                        }

                        // Ask side
                        if let Some((price, volume)) = ask_levels.get(i) {
                            let color_pct = (*volume / max_ask_vol) as f32 * self.color_intensity;
                            let bg_color = egui::Color32::from_rgba_premultiplied(
                                (100.0 * color_pct) as u8, 0, 0, (150.0 * color_pct) as u8
                            );

                            row.col(|ui| {
                                ui.colored_label(egui::Color32::RED, format!("{:.2}", price));
                            });
                            row.col(|ui| {
                                ui.colored_label(egui::Color32::RED, format!("{:.2}", volume));
                            });
                        } else {
                            row.col(|ui| { ui.label(""); });
                            row.col(|ui| { ui.label(""); });
                        }

                        // Traded volume (if enabled)
                        if self.show_traded_volume {
                            row.col(|ui| {
                                // Display historical traded volume at this price level
                                // Implementation depends on data structure
                                ui.label("...");
                            });
                        }
                    });
                }
            });
    }

    fn aggregate_orderbook(&self, orderbook: &OrderBook) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
        let tick_size = self.aggregation_level;

        let mut aggregated_bids: BTreeMap<i64, f64> = BTreeMap::new();
        let mut aggregated_asks: BTreeMap<i64, f64> = BTreeMap::new();

        // Aggregate bids
        for (price, qty) in &orderbook.bids {
            let price_tick = (price.0 / tick_size).round() as i64;
            *aggregated_bids.entry(price_tick).or_insert(0.0) += qty;
        }

        // Aggregate asks
        for (price, qty) in &orderbook.asks {
            let price_tick = (price.0 / tick_size).round() as i64;
            *aggregated_asks.entry(price_tick).or_insert(0.0) += qty;
        }

        // Convert to sorted vectors
        let bid_levels: Vec<(f64, f64)> = aggregated_bids.iter()
            .rev()
            .take(self.num_levels_to_show)
            .map(|(tick, qty)| (*tick as f64 * tick_size, *qty))
            .collect();

        let ask_levels: Vec<(f64, f64)> = aggregated_asks.iter()
            .take(self.num_levels_to_show)
            .map(|(tick, qty)| (*tick as f64 * tick_size, *qty))
            .collect();

        (bid_levels, ask_levels)
    }
}
```

### 4.2 Multi-Window Launcher

**Approach**: Due to egui limitations with multi-window support, we'll implement a "pop-out" feature that:
1. Opens a new native OS window
2. Shares data via Arc<Mutex<T>>
3. Runs independent egui context

**File to Modify**: `src/gui/footprint_panel.rs`

**Add pop-out button:**

```rust
impl FootprintPanel {
    fn render_dom_popout_button(&mut self, ui: &mut egui::Ui) {
        if ui.button("📊 Pop Out DOM").clicked() {
            self.launch_dom_window();
        }
    }

    fn launch_dom_window(&self) {
        // Get shared orderbook reference
        let orderbook = Arc::clone(&self.orderbook_shared);

        // Spawn new window in separate thread
        std::thread::spawn(move || {
            let options = eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default()
                    .with_inner_size([600.0, 800.0])
                    .with_title("DOM - Depth of Market"),
                ..Default::default()
            };

            eframe::run_native(
                "DOM Window",
                options,
                Box::new(|_cc| Box::new(DomWindow::new(orderbook))),
            ).expect("Failed to launch DOM window");
        });
    }
}
```

### 4.3 Shared State Management

**File to Modify**: `src/gui/app.rs`

**Add shared orderbook:**

```rust
pub struct ScreenerApp {
    // ... existing fields

    // Shared orderbook for DOM window
    shared_orderbooks: HashMap<String, Arc<Mutex<OrderBook>>>,
}

impl ScreenerApp {
    pub fn new(..., orderbook_receiver: Option<mpsc::Receiver<OrderBook>>) -> Self {
        // Initialize shared orderbooks
        let shared_orderbooks = HashMap::new();

        Self {
            // ... existing initialization
            shared_orderbooks,
            // ...
        }
    }

    fn process_incoming_data(&mut self) {
        // ... existing processing

        // Update shared orderbooks
        if let Some(receiver) = &mut self.orderbook_receiver {
            while let Ok(orderbook) = receiver.try_recv() {
                let symbol = orderbook.symbol.clone();

                if let Some(shared_ob) = self.shared_orderbooks.get(&symbol) {
                    *shared_ob.lock().unwrap() = orderbook;
                } else {
                    self.shared_orderbooks.insert(
                        symbol.clone(),
                        Arc::new(Mutex::new(orderbook))
                    );
                }
            }
        }
    }
}
```

### 4.4 Traded Volume History Integration

**File to Create**: `src/analysis/traded_volume_tracker.rs`

```rust
use std::collections::HashMap;
use crate::data::orderflow::OrderflowEvent;

pub struct TradedVolumeTracker {
    symbol: String,

    // price_tick -> VolumeAtPrice
    volume_at_price: HashMap<i64, VolumeAtPrice>,

    tick_size: f64,
}

#[derive(Debug, Clone, Default)]
pub struct VolumeAtPrice {
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub total_volume: f64,
    pub trade_count: u64,
}

impl TradedVolumeTracker {
    pub fn new(symbol: String, tick_size: f64) -> Self {
        Self {
            symbol,
            volume_at_price: HashMap::new(),
            tick_size,
        }
    }

    pub fn process_trade(&mut self, event: &OrderflowEvent) {
        let price_tick = (event.price / self.tick_size).round() as i64;

        let entry = self.volume_at_price
            .entry(price_tick)
            .or_insert_with(VolumeAtPrice::default);

        entry.total_volume += event.quantity;
        entry.trade_count += 1;

        if event.is_buyer_maker {
            entry.sell_volume += event.quantity;
        } else {
            entry.buy_volume += event.quantity;
        }
    }

    pub fn get_volume_at_tick(&self, price_tick: i64) -> Option<&VolumeAtPrice> {
        self.volume_at_price.get(&price_tick)
    }

    pub fn clear_old_data(&mut self, max_price_levels: usize) {
        if self.volume_at_price.len() > max_price_levels {
            // Keep only most recent/relevant price levels
            // Implementation: sort by volume or recency, keep top N
        }
    }
}
```

---

## Phase 5: Enhanced Timeframe Management

### 5.1 Current Limitations

**Current Implementation**:
- Base timeframe: 1-minute (hardcoded)
- Aggregation: On-demand for 5m, 15m, 1h, 4h, 1d
- Issue: Changing timeframe clears `current_candles`, losing in-progress data

**Problems to Solve**:
1. Maintain candle integrity when switching timeframes
2. Support sub-minute timeframes (e.g., 15s, 30s)
3. Preserve historical candle data across scale changes
4. Efficient reaggregation without data loss

### 5.2 Improved Timeframe System

**File to Create**: `src/analysis/timeframe_manager.rs`

```rust
use std::collections::{HashMap, VecDeque};
use crate::analysis::footprint::FootprintCandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Timeframe {
    Seconds(u64),   // Sub-minute: 15s, 30s
    Minutes(u64),   // 1m, 5m, 15m, 30m
    Hours(u64),     // 1h, 4h, 12h
    Days(u64),      // 1d
}

impl Timeframe {
    pub fn to_millis(&self) -> u64 {
        match self {
            Timeframe::Seconds(s) => s * 1000,
            Timeframe::Minutes(m) => m * 60 * 1000,
            Timeframe::Hours(h) => h * 60 * 60 * 1000,
            Timeframe::Days(d) => d * 24 * 60 * 60 * 1000,
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "15s" => Some(Timeframe::Seconds(15)),
            "30s" => Some(Timeframe::Seconds(30)),
            "1m" => Some(Timeframe::Minutes(1)),
            "5m" => Some(Timeframe::Minutes(5)),
            "15m" => Some(Timeframe::Minutes(15)),
            "30m" => Some(Timeframe::Minutes(30)),
            "1h" => Some(Timeframe::Hours(1)),
            "4h" => Some(Timeframe::Hours(4)),
            "12h" => Some(Timeframe::Hours(12)),
            "1d" => Some(Timeframe::Days(1)),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Timeframe::Seconds(s) => format!("{}s", s),
            Timeframe::Minutes(m) => format!("{}m", m),
            Timeframe::Hours(h) => format!("{}h", h),
            Timeframe::Days(d) => format!("{}d", d),
        }
    }
}

pub struct TimeframeManager {
    // Base data storage (finest granularity)
    base_timeframe: Timeframe,
    base_candles: HashMap<String, VecDeque<FootprintCandle>>,

    // Cached aggregated candles per timeframe
    cached_candles: HashMap<Timeframe, HashMap<String, VecDeque<FootprintCandle>>>,

    // Maximum candles to keep per symbol per timeframe
    max_candles: usize,
}

impl TimeframeManager {
    pub fn new(base_timeframe: Timeframe, max_candles: usize) -> Self {
        Self {
            base_timeframe,
            base_candles: HashMap::new(),
            cached_candles: HashMap::new(),
            max_candles,
        }
    }

    /// Add a new base candle
    pub fn add_base_candle(&mut self, symbol: &str, candle: FootprintCandle) {
        let candles = self.base_candles
            .entry(symbol.to_string())
            .or_insert_with(VecDeque::new);

        candles.push_back(candle);

        // Maintain max size
        while candles.len() > self.max_candles {
            candles.pop_front();
        }

        // Invalidate cached aggregations for this symbol
        self.invalidate_cache_for_symbol(symbol);
    }

    /// Get candles for a specific timeframe (with caching)
    pub fn get_candles(&mut self, symbol: &str, timeframe: Timeframe) -> Option<&VecDeque<FootprintCandle>> {
        // If base timeframe requested, return directly
        if timeframe == self.base_timeframe {
            return self.base_candles.get(symbol);
        }

        // Check cache first
        if let Some(cached) = self.cached_candles.get(&timeframe) {
            if let Some(candles) = cached.get(symbol) {
                return Some(candles);
            }
        }

        // Need to aggregate
        self.aggregate_and_cache(symbol, timeframe);

        // Return cached result
        self.cached_candles
            .get(&timeframe)
            .and_then(|cache| cache.get(symbol))
    }

    fn aggregate_and_cache(&mut self, symbol: &str, target_timeframe: Timeframe) {
        let base_candles = match self.base_candles.get(symbol) {
            Some(candles) => candles,
            None => return,
        };

        let timeframe_ms = target_timeframe.to_millis();
        let mut aggregated_candles = VecDeque::new();

        // Group base candles by target timeframe
        let mut current_group: Vec<FootprintCandle> = Vec::new();
        let mut current_period_start: Option<u64> = None;

        for candle in base_candles {
            let period_start = (candle.candle.timestamp / timeframe_ms) * timeframe_ms;

            if let Some(current_start) = current_period_start {
                if period_start != current_start {
                    // New period - aggregate current group
                    if !current_group.is_empty() {
                        let aggregated = Self::aggregate_candles(&current_group);
                        aggregated_candles.push_back(aggregated);
                        current_group.clear();
                    }
                }
            }

            current_period_start = Some(period_start);
            current_group.push(candle.clone());
        }

        // Aggregate remaining group
        if !current_group.is_empty() {
            let aggregated = Self::aggregate_candles(&current_group);
            aggregated_candles.push_back(aggregated);
        }

        // Store in cache
        self.cached_candles
            .entry(target_timeframe)
            .or_insert_with(HashMap::new)
            .insert(symbol.to_string(), aggregated_candles);
    }

    fn aggregate_candles(candles: &[FootprintCandle]) -> FootprintCandle {
        // Merge logic (similar to existing FootprintAnalyzer::aggregate_candles_for_timeframe)
        // but preserving all data structures

        let first = &candles[0];
        let last = &candles[candles.len() - 1];

        let mut aggregated = FootprintCandle {
            candle: first.candle.clone(),
            volume_profile: first.volume_profile.clone(),
            delta: 0.0,
            cvd: last.cvd,  // Preserve CVD from last candle
            imbalance_levels: Vec::new(),
            significant_levels: Vec::new(),
        };

        // Merge OHLC
        aggregated.candle.open = first.candle.open;
        aggregated.candle.close = last.candle.close;
        aggregated.candle.high = candles.iter()
            .map(|c| c.candle.high)
            .fold(f64::NEG_INFINITY, f64::max);
        aggregated.candle.low = candles.iter()
            .map(|c| c.candle.low)
            .fold(f64::INFINITY, f64::min);
        aggregated.candle.volume = candles.iter()
            .map(|c| c.candle.volume)
            .sum();

        // Merge volume profiles
        let mut merged_price_levels = BTreeMap::new();
        for candle in candles {
            for (price_tick, vol) in &candle.volume_profile.price_levels {
                let entry = merged_price_levels
                    .entry(*price_tick)
                    .or_insert_with(|| VolumeAtPrice::default());
                entry.buy_volume += vol.buy_volume;
                entry.sell_volume += vol.sell_volume;
                entry.total_volume += vol.total_volume;
                entry.trade_count += vol.trade_count;
            }
        }

        aggregated.volume_profile.price_levels = merged_price_levels;

        // Recalculate metrics
        aggregated.delta = aggregated.volume_profile.price_levels.values()
            .map(|v| v.buy_volume - v.sell_volume)
            .sum();

        // Recalculate POC, VWAP, imbalances, etc.
        // ... (detailed implementation)

        aggregated
    }

    fn invalidate_cache_for_symbol(&mut self, symbol: &str) {
        for (_, cache) in &mut self.cached_candles {
            cache.remove(symbol);
        }
    }
}
```

### 5.3 Integration with FootprintPanel

**File to Modify**: `src/gui/footprint_panel.rs`

**Replace candle storage:**

```rust
pub struct FootprintPanel {
    // REMOVE:
    // current_candles: HashMap<String, FootprintCandle>,
    // completed_candles: HashMap<String, VecDeque<FootprintCandle>>,

    // ADD:
    timeframe_manager: TimeframeManager,
    selected_timeframe: Timeframe,
    available_timeframes: Vec<Timeframe>,
}

impl FootprintPanel {
    pub fn new() -> Self {
        let available_timeframes = vec![
            Timeframe::Seconds(15),
            Timeframe::Seconds(30),
            Timeframe::Minutes(1),
            Timeframe::Minutes(5),
            Timeframe::Minutes(15),
            Timeframe::Minutes(30),
            Timeframe::Hours(1),
            Timeframe::Hours(4),
            Timeframe::Days(1),
        ];

        Self {
            timeframe_manager: TimeframeManager::new(Timeframe::Minutes(1), 10000),
            selected_timeframe: Timeframe::Minutes(1),
            available_timeframes,
            // ... other fields
        }
    }

    fn render_timeframe_selector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Timeframe:");

            for tf in &self.available_timeframes {
                let is_selected = *tf == self.selected_timeframe;
                if ui.selectable_label(is_selected, tf.to_string()).clicked() {
                    self.selected_timeframe = *tf;
                    // No need to clear data - TimeframeManager handles caching!
                }
            }
        });
    }

    fn get_visible_candles(&mut self) -> Option<&VecDeque<FootprintCandle>> {
        self.timeframe_manager.get_candles(
            &self.selected_symbol,
            self.selected_timeframe
        )
    }
}
```

### 5.4 Candle Integrity Preservation

**Key Design Principle**: Never clear data when changing timeframes. Instead:

1. **Base Layer**: Store all candles at finest granularity (1-minute or sub-minute)
2. **Aggregation Layer**: Cache aggregated timeframes on-demand
3. **Cache Invalidation**: Only invalidate when new base data arrives
4. **Seamless Switching**: Timeframe changes instantly use cached data

**Benefits**:
- No data loss
- Fast timeframe switching
- Historical candle integrity maintained
- Scalable to many timeframes

---

## Phase 6: Technical Analysis Tools

### 6.1 Drawing Tools Framework

**File to Create**: `src/gui/drawing_tools.rs`

```rust
use egui::{Pos2, Color32, Stroke};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DrawingTool {
    TrendLine(TrendLine),
    HorizontalLine(HorizontalLine),
    FibonacciRetracement(FibonacciRetracement),
    Rectangle(Rectangle),
    Text(TextAnnotation),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendLine {
    pub id: String,
    pub start_time: u64,
    pub start_price: f64,
    pub end_time: u64,
    pub end_price: f64,
    pub color: [u8; 4],
    pub width: f32,
    pub style: LineStyle,
    pub extend_right: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizontalLine {
    pub id: String,
    pub price: f64,
    pub color: [u8; 4],
    pub width: f32,
    pub style: LineStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FibonacciRetracement {
    pub id: String,
    pub start_time: u64,
    pub start_price: f64,
    pub end_time: u64,
    pub end_price: f64,
    pub levels: Vec<f32>,  // 0.236, 0.382, 0.5, 0.618, 0.786, 1.0
    pub show_labels: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rectangle {
    pub id: String,
    pub start_time: u64,
    pub start_price: f64,
    pub end_time: u64,
    pub end_price: f64,
    pub fill_color: [u8; 4],
    pub border_color: [u8; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnnotation {
    pub id: String,
    pub time: u64,
    pub price: f64,
    pub text: String,
    pub font_size: f32,
    pub color: [u8; 4],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

pub struct DrawingToolsManager {
    pub tools: Vec<DrawingTool>,
    pub active_tool: Option<ActiveTool>,
    pub selected_tool_id: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ActiveTool {
    TrendLine,
    HorizontalLine,
    FibonacciRetracement,
    Rectangle,
    Text,
}

impl DrawingToolsManager {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            active_tool: None,
            selected_tool_id: None,
        }
    }

    pub fn start_drawing(&mut self, tool: ActiveTool) {
        self.active_tool = Some(tool);
    }

    pub fn finish_drawing(&mut self, tool: DrawingTool) {
        self.tools.push(tool);
        self.active_tool = None;
    }

    pub fn delete_selected(&mut self) {
        if let Some(id) = &self.selected_tool_id {
            self.tools.retain(|t| self.get_tool_id(t) != id);
            self.selected_tool_id = None;
        }
    }

    fn get_tool_id(&self, tool: &DrawingTool) -> &str {
        match tool {
            DrawingTool::TrendLine(t) => &t.id,
            DrawingTool::HorizontalLine(h) => &h.id,
            DrawingTool::FibonacciRetracement(f) => &f.id,
            DrawingTool::Rectangle(r) => &r.id,
            DrawingTool::Text(t) => &t.id,
        }
    }

    pub fn render_tools(
        &self,
        ui: &mut egui::Ui,
        chart_rect: egui::Rect,
        price_to_screen: impl Fn(f64) -> f32,
        time_to_screen: impl Fn(u64) -> f32,
    ) {
        let painter = ui.painter();

        for tool in &self.tools {
            match tool {
                DrawingTool::TrendLine(line) => {
                    self.render_trend_line(painter, line, &price_to_screen, &time_to_screen);
                }
                DrawingTool::HorizontalLine(line) => {
                    self.render_horizontal_line(painter, line, &price_to_screen, chart_rect);
                }
                DrawingTool::FibonacciRetracement(fib) => {
                    self.render_fibonacci(painter, fib, &price_to_screen, &time_to_screen);
                }
                DrawingTool::Rectangle(rect) => {
                    self.render_rectangle(painter, rect, &price_to_screen, &time_to_screen);
                }
                DrawingTool::Text(text) => {
                    self.render_text(painter, text, &price_to_screen, &time_to_screen);
                }
            }
        }
    }

    fn render_trend_line(
        &self,
        painter: &egui::Painter,
        line: &TrendLine,
        price_to_screen: impl Fn(f64) -> f32,
        time_to_screen: impl Fn(u64) -> f32,
    ) {
        let start_pos = Pos2::new(
            time_to_screen(line.start_time),
            price_to_screen(line.start_price),
        );
        let end_pos = Pos2::new(
            time_to_screen(line.end_time),
            price_to_screen(line.end_price),
        );

        let color = Color32::from_rgba_premultiplied(
            line.color[0],
            line.color[1],
            line.color[2],
            line.color[3],
        );

        let stroke = match line.style {
            LineStyle::Solid => Stroke::new(line.width, color),
            LineStyle::Dashed => Stroke::new(line.width, color),  // Need custom dash impl
            LineStyle::Dotted => Stroke::new(line.width, color),  // Need custom dot impl
        };

        painter.line_segment([start_pos, end_pos], stroke);

        // Extend right if enabled
        if line.extend_right {
            // Calculate slope and extend
            // ... implementation
        }
    }

    fn render_horizontal_line(
        &self,
        painter: &egui::Painter,
        line: &HorizontalLine,
        price_to_screen: impl Fn(f64) -> f32,
        chart_rect: egui::Rect,
    ) {
        let y = price_to_screen(line.price);
        let color = Color32::from_rgba_premultiplied(
            line.color[0],
            line.color[1],
            line.color[2],
            line.color[3],
        );

        painter.line_segment(
            [Pos2::new(chart_rect.min.x, y), Pos2::new(chart_rect.max.x, y)],
            Stroke::new(line.width, color),
        );
    }

    fn render_fibonacci(
        &self,
        painter: &egui::Painter,
        fib: &FibonacciRetracement,
        price_to_screen: impl Fn(f64) -> f32,
        time_to_screen: impl Fn(u64) -> f32,
    ) {
        let price_range = fib.end_price - fib.start_price;

        for &level in &fib.levels {
            let price = fib.start_price + (price_range * level as f64);
            let y = price_to_screen(price);
            let x_start = time_to_screen(fib.start_time);
            let x_end = time_to_screen(fib.end_time);

            // Draw level line
            painter.line_segment(
                [Pos2::new(x_start, y), Pos2::new(x_end, y)],
                Stroke::new(1.0, Color32::GRAY),
            );

            // Draw label
            if fib.show_labels {
                painter.text(
                    Pos2::new(x_end + 5.0, y),
                    egui::Align2::LEFT_CENTER,
                    format!("{:.1}% ({:.2})", level * 100.0, price),
                    egui::FontId::proportional(12.0),
                    Color32::WHITE,
                );
            }
        }
    }

    fn render_rectangle(&self, painter: &egui::Painter, rect: &Rectangle, price_to_screen: impl Fn(f64) -> f32, time_to_screen: impl Fn(u64) -> f32) {
        // Implementation
    }

    fn render_text(&self, painter: &egui::Painter, text: &TextAnnotation, price_to_screen: impl Fn(f64) -> f32, time_to_screen: impl Fn(u64) -> f32) {
        // Implementation
    }
}
```

### 6.2 Technical Indicators

**File to Create**: `src/analysis/indicators.rs`

```rust
use crate::data::market_data::Candle;

pub trait Indicator {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64>;
    fn get_name(&self) -> &str;
}

// Moving Average (SMA)
pub struct SimpleMovingAverage {
    period: usize,
}

impl SimpleMovingAverage {
    pub fn new(period: usize) -> Self {
        Self { period }
    }
}

impl Indicator for SimpleMovingAverage {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64> {
        let mut result = Vec::new();

        for i in 0..candles.len() {
            if i < self.period - 1 {
                result.push(f64::NAN);
            } else {
                let sum: f64 = candles[i - self.period + 1..=i]
                    .iter()
                    .map(|c| c.close)
                    .sum();
                result.push(sum / self.period as f64);
            }
        }

        result
    }

    fn get_name(&self) -> &str {
        "SMA"
    }
}

// Exponential Moving Average (EMA)
pub struct ExponentialMovingAverage {
    period: usize,
}

impl ExponentialMovingAverage {
    pub fn new(period: usize) -> Self {
        Self { period }
    }
}

impl Indicator for ExponentialMovingAverage {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64> {
        let mut result = Vec::new();
        let multiplier = 2.0 / (self.period as f64 + 1.0);

        // First EMA = SMA
        let first_sma: f64 = candles[0..self.period]
            .iter()
            .map(|c| c.close)
            .sum::<f64>() / self.period as f64;

        for i in 0..candles.len() {
            if i < self.period - 1 {
                result.push(f64::NAN);
            } else if i == self.period - 1 {
                result.push(first_sma);
            } else {
                let ema = (candles[i].close - result[i - 1]) * multiplier + result[i - 1];
                result.push(ema);
            }
        }

        result
    }

    fn get_name(&self) -> &str {
        "EMA"
    }
}

// Bollinger Bands
pub struct BollingerBands {
    period: usize,
    std_dev: f64,
}

pub struct BollingerBandsResult {
    pub upper: Vec<f64>,
    pub middle: Vec<f64>,
    pub lower: Vec<f64>,
}

impl BollingerBands {
    pub fn new(period: usize, std_dev: f64) -> Self {
        Self { period, std_dev }
    }

    pub fn calculate(&self, candles: &[Candle]) -> BollingerBandsResult {
        let sma = SimpleMovingAverage::new(self.period);
        let middle = sma.calculate(candles);

        let mut upper = Vec::new();
        let mut lower = Vec::new();

        for i in 0..candles.len() {
            if i < self.period - 1 {
                upper.push(f64::NAN);
                lower.push(f64::NAN);
            } else {
                let prices: Vec<f64> = candles[i - self.period + 1..=i]
                    .iter()
                    .map(|c| c.close)
                    .collect();

                let std = self.calculate_std_dev(&prices, middle[i]);

                upper.push(middle[i] + (self.std_dev * std));
                lower.push(middle[i] - (self.std_dev * std));
            }
        }

        BollingerBandsResult {
            upper,
            middle,
            lower,
        }
    }

    fn calculate_std_dev(&self, prices: &[f64], mean: f64) -> f64 {
        let variance: f64 = prices.iter()
            .map(|&price| (price - mean).powi(2))
            .sum::<f64>() / prices.len() as f64;
        variance.sqrt()
    }
}

// RSI (Relative Strength Index)
pub struct RSI {
    period: usize,
}

impl RSI {
    pub fn new(period: usize) -> Self {
        Self { period }
    }

    pub fn calculate(&self, candles: &[Candle]) -> Vec<f64> {
        let mut result = Vec::new();

        if candles.len() < self.period + 1 {
            return vec![f64::NAN; candles.len()];
        }

        // Calculate price changes
        let mut gains = Vec::new();
        let mut losses = Vec::new();

        for i in 1..candles.len() {
            let change = candles[i].close - candles[i - 1].close;
            gains.push(change.max(0.0));
            losses.push((-change).max(0.0));
        }

        // First RSI uses simple average
        let mut avg_gain: f64 = gains[0..self.period].iter().sum::<f64>() / self.period as f64;
        let mut avg_loss: f64 = losses[0..self.period].iter().sum::<f64>() / self.period as f64;

        for i in 0..candles.len() {
            if i < self.period {
                result.push(f64::NAN);
            } else {
                if i == self.period {
                    // First RSI
                } else {
                    // Subsequent RSI using smoothed averages
                    avg_gain = ((avg_gain * (self.period - 1) as f64) + gains[i - 1]) / self.period as f64;
                    avg_loss = ((avg_loss * (self.period - 1) as f64) + losses[i - 1]) / self.period as f64;
                }

                let rs = if avg_loss == 0.0 {
                    100.0
                } else {
                    avg_gain / avg_loss
                };

                let rsi = 100.0 - (100.0 / (1.0 + rs));
                result.push(rsi);
            }
        }

        result
    }
}

// MACD (Moving Average Convergence Divergence)
pub struct MACD {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
}

pub struct MACDResult {
    pub macd_line: Vec<f64>,
    pub signal_line: Vec<f64>,
    pub histogram: Vec<f64>,
}

impl MACD {
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            signal_period,
        }
    }

    pub fn calculate(&self, candles: &[Candle]) -> MACDResult {
        let fast_ema = ExponentialMovingAverage::new(self.fast_period).calculate(candles);
        let slow_ema = ExponentialMovingAverage::new(self.slow_period).calculate(candles);

        let mut macd_line = Vec::new();
        for i in 0..candles.len() {
            macd_line.push(fast_ema[i] - slow_ema[i]);
        }

        // Signal line is EMA of MACD line
        let signal_line = self.calculate_signal_line(&macd_line);

        let mut histogram = Vec::new();
        for i in 0..candles.len() {
            histogram.push(macd_line[i] - signal_line[i]);
        }

        MACDResult {
            macd_line,
            signal_line,
            histogram,
        }
    }

    fn calculate_signal_line(&self, macd_values: &[f64]) -> Vec<f64> {
        // EMA calculation on MACD values
        // ... implementation
        vec![0.0; macd_values.len()]  // Placeholder
    }
}
```

### 6.3 Indicator Overlay on Footprint

**File to Modify**: `src/gui/footprint_panel.rs`

```rust
pub struct FootprintPanel {
    // ... existing fields

    // Indicators
    active_indicators: Vec<Box<dyn Indicator>>,
    indicator_values: HashMap<String, Vec<f64>>,
    show_indicators_panel: bool,

    // Bollinger Bands
    show_bollinger: bool,
    bollinger_period: usize,
    bollinger_std_dev: f64,

    // Moving Averages
    show_sma: bool,
    sma_periods: Vec<usize>,

    show_ema: bool,
    ema_periods: Vec<usize>,
}

impl FootprintPanel {
    fn render_indicators_controls(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Indicators", |ui| {
            // Bollinger Bands
            ui.checkbox(&mut self.show_bollinger, "Bollinger Bands");
            if self.show_bollinger {
                ui.horizontal(|ui| {
                    ui.label("Period:");
                    ui.add(egui::DragValue::new(&mut self.bollinger_period).speed(1));
                    ui.label("Std Dev:");
                    ui.add(egui::DragValue::new(&mut self.bollinger_std_dev).speed(0.1));
                });
            }

            // Moving Averages
            ui.checkbox(&mut self.show_sma, "Simple Moving Average");
            if self.show_sma {
                ui.horizontal(|ui| {
                    ui.label("Periods:");
                    // Period selector
                });
            }

            ui.checkbox(&mut self.show_ema, "Exponential Moving Average");
            // ... similar controls
        });
    }

    fn render_indicators_overlay(
        &self,
        ui: &mut egui::Ui,
        chart_rect: egui::Rect,
        candles: &[FootprintCandle],
        price_to_screen: impl Fn(f64) -> f32,
        time_to_screen: impl Fn(u64) -> f32,
    ) {
        let painter = ui.painter();

        // Convert FootprintCandles to basic Candles for indicator calculation
        let basic_candles: Vec<Candle> = candles.iter()
            .map(|fc| fc.candle.clone())
            .collect();

        // Bollinger Bands
        if self.show_bollinger {
            let bb = BollingerBands::new(self.bollinger_period, self.bollinger_std_dev);
            let result = bb.calculate(&basic_candles);

            self.render_line_indicator(
                painter,
                &result.upper,
                candles,
                &price_to_screen,
                &time_to_screen,
                Color32::from_rgb(100, 100, 255),
            );
            self.render_line_indicator(
                painter,
                &result.middle,
                candles,
                &price_to_screen,
                &time_to_screen,
                Color32::from_rgb(200, 200, 200),
            );
            self.render_line_indicator(
                painter,
                &result.lower,
                candles,
                &price_to_screen,
                &time_to_screen,
                Color32::from_rgb(100, 100, 255),
            );
        }

        // SMAs
        if self.show_sma {
            for &period in &self.sma_periods {
                let sma = SimpleMovingAverage::new(period);
                let values = sma.calculate(&basic_candles);
                self.render_line_indicator(
                    painter,
                    &values,
                    candles,
                    &price_to_screen,
                    &time_to_screen,
                    Color32::from_rgb(255, 200, 0),
                );
            }
        }
    }

    fn render_line_indicator(
        &self,
        painter: &egui::Painter,
        values: &[f64],
        candles: &[FootprintCandle],
        price_to_screen: impl Fn(f64) -> f32,
        time_to_screen: impl Fn(u64) -> f32,
        color: Color32,
    ) {
        let mut points = Vec::new();

        for (i, &value) in values.iter().enumerate() {
            if !value.is_nan() && i < candles.len() {
                let x = time_to_screen(candles[i].candle.timestamp);
                let y = price_to_screen(value);
                points.push(Pos2::new(x, y));
            }
        }

        if points.len() > 1 {
            painter.add(egui::Shape::line(
                points,
                Stroke::new(2.0, color),
            ));
        }
    }
}
```

---

## Implementation Timeline & Dependencies

### Phase 1: LOB Data Integration (Week 1-2)
**Priority**: High
**Dependencies**: None
**Deliverables**:
- OrderBook data structures
- WebSocket depth stream integration
- OrderBookManager with historical snapshots
- Basic depth data flow to GUI

### Phase 2: LOB Heatmap Visualization (Week 2-3)
**Priority**: High
**Dependencies**: Phase 1
**Deliverables**:
- Heatmap color system
- Background rendering layer
- User controls for intensity/colors
- Volume normalization

### Phase 3: Advanced Axis Controls (Week 3-4)
**Priority**: Medium
**Dependencies**: None
**Deliverables**:
- Right-click drag scaling for X/Y axes
- Visual feedback indicators
- Configuration options
- Smooth interaction experience

### Phase 4: Detachable DOM Window (Week 4-5)
**Priority**: Medium
**Dependencies**: Phase 1
**Deliverables**:
- DomWindow component
- Multi-window launcher
- Shared state management
- Traded volume history integration

### Phase 5: Enhanced Timeframe Management (Week 5-6)
**Priority**: High
**Dependencies**: None
**Deliverables**:
- TimeframeManager with caching
- Support for sub-minute timeframes
- Seamless timeframe switching
- Candle integrity preservation

### Phase 6: Technical Analysis Tools (Week 6-8)
**Priority**: Medium
**Dependencies**: Phase 5
**Deliverables**:
- Drawing tools framework
- Technical indicators (SMA, EMA, Bollinger, RSI, MACD)
- Indicator overlays on footprint
- Tool persistence and management

---

## Performance Considerations

### 1. Memory Management

**Challenge**: Large datasets (10,000+ candles × 100+ price levels × depth snapshots)

**Solutions**:
- Implement rolling window limits (max 10,000 base candles)
- Use VecDeque for automatic FIFO eviction
- Lazy aggregation with caching
- Clear unused timeframe caches periodically

### 2. Rendering Performance

**Challenge**: egui immediate-mode rendering recalculates every frame

**Solutions**:
- Cache coordinate transformations
- Cull off-screen elements before painting
- Use `egui::Response::hovered()` to limit interaction checks
- Consider frame rate limiting (30fps vs 60fps option)

### 3. WebSocket Data Volume

**Challenge**: Depth updates at 100ms = 10 messages/sec × 300 symbols = 3000 msg/sec

**Solutions**:
- Use `try_recv()` with batch processing
- Aggregate depth updates before sending to GUI
- Implement message prioritization (current symbol first)
- Consider 1000ms depth updates instead of 100ms

### 4. Indicator Calculation

**Challenge**: Recalculating indicators on every frame

**Solutions**:
- Cache indicator values
- Only recalculate when new candle completes
- Use incremental updates where possible (EMA)
- Parallelize calculations with rayon

---

## Configuration Updates

**File to Modify**: `config.toml`

```toml
[analysis]
footprint_timeframes = ["15s", "30s", "1m", "5m", "15m", "30m", "1h", "4h", "1d"]
base_timeframe = "1m"
max_candles_per_symbol = 10000

[analysis.lob]
enable_depth_data = true
depth_update_speed = "100ms"  # or "1000ms"
max_depth_levels = 100
snapshot_interval_ms = 100
history_snapshots = 500  # 50 seconds at 100ms

[gui.footprint]
enable_lob_heatmap = true
heatmap_default_intensity = 0.7
heatmap_color_scheme = "green_red"  # or "blue_orange", "monochrome"

x_scale_sensitivity = 0.01
y_scale_sensitivity = 0.01
enable_independent_axis_scaling = true
show_scale_indicators = true

[gui.dom]
enable_detachable = true
default_aggregation_level = 0.01
default_num_levels = 20
show_traded_volume = true

[gui.indicators]
enable_drawing_tools = true
enable_technical_indicators = true
default_bollinger_period = 20
default_bollinger_std_dev = 2.0
default_sma_periods = [20, 50, 200]
default_ema_periods = [12, 26]

[performance]
target_fps = 60
enable_render_culling = true
max_visible_candles = 200
```

---

## Summary

This plan provides a comprehensive roadmap for implementing:

1. **LOB Heatmap**: Real-time order book visualization with customizable color gradients
2. **Advanced Axis Controls**: Tradinglite-style right-click scaling for independent X/Y axis control
3. **Detachable DOM**: Separate window showing limit orders and traded volume at each aggregation level
4. **Enhanced Timeframe Management**: Seamless switching with candle integrity preservation
5. **Technical Analysis Tools**: Drawing tools (lines, fibonacci, etc.) and indicators (MA, Bollinger, RSI, MACD)

The implementation leverages the existing Rust/egui architecture while adding sophisticated new capabilities for professional trading analysis.
