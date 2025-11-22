use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::PathBuf;
use std::fs;

/// Application settings that persist across sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Chart settings
    pub chart: ChartSettings,

    /// Drawing tools settings
    pub drawing_tools: DrawingToolSettings,

    /// Indicator settings
    pub indicators: IndicatorSettings,

    /// UI settings
    pub ui: UISettings,

    /// Last session state
    pub session: SessionState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartSettings {
    /// Zoom level (0.1 - 10.0)
    pub zoom_level: f32,

    /// X-axis scale (0.1 - 10.0)
    pub x_scale: f32,

    /// Y-axis scale (0.1 - 10.0)
    pub y_scale: f32,

    /// Maximum candles to display
    pub max_candles_display: usize,

    /// Price scale/aggregation
    pub price_scale: f64,
    pub scale_index: usize,

    /// Display toggles
    pub show_volume: bool,
    pub show_delta: bool,
    pub show_imbalance: bool,

    /// Heatmap settings
    pub enable_heatmap: bool,
    pub heatmap_opacity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawingToolSettings {
    /// Show drawing toolbar
    pub show_toolbar: bool,

    /// Persisted drawing tools per symbol (stored separately)
    pub auto_save: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorSettings {
    /// Show indicator panel
    pub show_panel: bool,

    /// SMA settings
    pub show_sma: bool,
    pub sma_period: usize,

    /// EMA settings
    pub show_ema: bool,
    pub ema_period: usize,

    /// Bollinger Bands settings
    pub show_bollinger: bool,
    pub bollinger_period: usize,
    pub bollinger_std_dev: f64,

    /// RSI settings
    pub show_rsi: bool,
    pub rsi_period: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UISettings {
    /// Active panel
    pub active_panel: String,

    /// Symbol category
    pub symbol_category: String,

    /// Window size (if we add multi-window support)
    pub window_width: Option<f32>,
    pub window_height: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Last selected symbol
    pub selected_symbol: String,

    /// Last selected timeframe index
    pub selected_timeframe_index: usize,

    /// Pan offsets
    pub pan_x: f32,
    pub pan_y: f32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            chart: ChartSettings::default(),
            drawing_tools: DrawingToolSettings::default(),
            indicators: IndicatorSettings::default(),
            ui: UISettings::default(),
            session: SessionState::default(),
        }
    }
}

impl Default for ChartSettings {
    fn default() -> Self {
        Self {
            zoom_level: 1.0,
            x_scale: 1.0,
            y_scale: 1.0,
            max_candles_display: 50,
            price_scale: 0.01,
            scale_index: 2,
            show_volume: true,
            show_delta: true,
            show_imbalance: false,
            enable_heatmap: true,
            heatmap_opacity: 0.6,
        }
    }
}

impl Default for DrawingToolSettings {
    fn default() -> Self {
        Self {
            show_toolbar: true,
            auto_save: true,
        }
    }
}

impl Default for IndicatorSettings {
    fn default() -> Self {
        Self {
            show_panel: true,
            show_sma: false,
            sma_period: 20,
            show_ema: false,
            ema_period: 20,
            show_bollinger: false,
            bollinger_period: 20,
            bollinger_std_dev: 2.0,
            show_rsi: false,
            rsi_period: 14,
        }
    }
}

impl Default for UISettings {
    fn default() -> Self {
        Self {
            active_panel: "Footprint".to_string(),
            symbol_category: "High Volume".to_string(),
            window_width: None,
            window_height: None,
        }
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            selected_symbol: "BTCUSDT".to_string(),
            selected_timeframe_index: 2, // 1m
            pan_x: 0.0,
            pan_y: 0.0,
        }
    }
}

impl AppSettings {
    /// Get the settings file path
    fn settings_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("binance-futures-tool");
        fs::create_dir_all(&path).ok();
        path.push("settings.json");
        path
    }

    /// Load settings from disk, or create default if not found
    pub fn load() -> Self {
        let path = Self::settings_path();

        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(contents) => {
                    match serde_json::from_str(&contents) {
                        Ok(settings) => {
                            tracing::info!("Loaded settings from {:?}", path);
                            return settings;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse settings file: {}. Using defaults.", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read settings file: {}. Using defaults.", e);
                }
            }
        }

        // Return default settings if load failed
        let settings = Self::default();
        // Try to save default settings
        settings.save().ok();
        settings
    }

    /// Save settings to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::settings_path();
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        tracing::info!("Saved settings to {:?}", path);
        Ok(())
    }

    /// Auto-save settings (silent on error)
    pub fn auto_save(&self) {
        if let Err(e) = self.save() {
            tracing::warn!("Failed to auto-save settings: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = AppSettings::default();
        assert_eq!(settings.chart.zoom_level, 1.0);
        assert_eq!(settings.session.selected_symbol, "BTCUSDT");
    }

    #[test]
    fn test_serialize_deserialize() {
        let settings = AppSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.chart.zoom_level, settings.chart.zoom_level);
    }
}
