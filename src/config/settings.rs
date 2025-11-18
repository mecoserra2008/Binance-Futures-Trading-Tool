use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub binance: BinanceConfig,
    pub database: DatabaseConfig,
    pub analysis: AnalysisConfig,
    pub gui: GuiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceConfig {
    pub websocket_base_url: String,
    pub api_base_url: String,
    pub max_reconnect_attempts: u32,
    pub reconnect_delay_ms: u64,
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub max_connections: u32,
    pub backup_interval_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub volume_threshold_percentage: f64,
    pub imbalance_window_seconds: u64,
    pub footprint_timeframes: Vec<String>,
    pub liquidation_size_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiConfig {
    pub refresh_rate_ms: u64,
    pub max_displayed_rows: usize,
    pub color_scheme: ColorScheme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    pub background: String,
    pub text: String,
    pub buy_color: String,
    pub sell_color: String,
    pub neutral_color: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            binance: BinanceConfig {
                websocket_base_url: "wss://fstream.binance.com".to_string(),
                api_base_url: "https://fapi.binance.com".to_string(),
                max_reconnect_attempts: 10,
                reconnect_delay_ms: 5000,
                symbols: vec![
                    "BTCUSDT".to_string(),
                    "ETHUSDT".to_string(),
                    "ADAUSDT".to_string(),
                    "DOTUSDT".to_string(),
                    "LINKUSDT".to_string(),
                    "LTCUSDT".to_string(),
                    "BCHUSDT".to_string(),
                    "XLMUSDT".to_string(),
                    "EOSUSDT".to_string(),
                    "TRXUSDT".to_string(),
                ],
            },
            database: DatabaseConfig {
                path: "data.db".to_string(),
                max_connections: 10,
                backup_interval_hours: 24,
            },
            analysis: AnalysisConfig {
                volume_threshold_percentage: 0.5,
                imbalance_window_seconds: 60,
                footprint_timeframes: vec!["1m".to_string(), "5m".to_string(), "15m".to_string()],
                liquidation_size_threshold: 100000.0,
            },
            gui: GuiConfig {
                refresh_rate_ms: 16, // 60fps
                max_displayed_rows: 100,
                color_scheme: ColorScheme {
                    background: "#1e1e1e".to_string(),
                    text: "#e0e0e0".to_string(),
                    buy_color: "#00ff88".to_string(),
                    sell_color: "#ff4444".to_string(),
                    neutral_color: "#ffaa00".to_string(),
                },
            },
        }
    }
}

impl Settings {
    pub fn new() -> anyhow::Result<Self> {
        // Try to load from config file, fallback to default
        match std::fs::read_to_string("config.toml") {
            Ok(content) => {
                toml::from_str(&content).map_err(|e| anyhow::anyhow!("Config parse error: {}", e))
            }
            Err(_) => {
                let default_settings = Settings::default();
                // Save default config
                let toml_content = toml::to_string_pretty(&default_settings)?;
                std::fs::write("config.toml", toml_content)?;
                Ok(default_settings)
            }
        }
    }

    pub async fn get_active_symbols(&self) -> anyhow::Result<Vec<String>> {
        // Fetch active USDT perpetual symbols from Binance API
        let client = reqwest::Client::new();
        let url = format!("{}/fapi/v1/exchangeInfo", self.binance.api_base_url);
        
        let response: serde_json::Value = client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        let mut symbols = Vec::new();
        if let Some(symbols_array) = response["symbols"].as_array() {
            for symbol in symbols_array {
                if let (Some(symbol_name), Some(status), Some(contract_type), Some(quote_asset)) = (
                    symbol["symbol"].as_str(),
                    symbol["status"].as_str(),
                    symbol["contractType"].as_str(),
                    symbol["quoteAsset"].as_str(),
                ) {
                    if status == "TRADING" 
                        && contract_type == "PERPETUAL" 
                        && quote_asset == "USDT" {
                        symbols.push(symbol_name.to_string());
                    }
                }
            }
        }

        Ok(symbols)
    }
}