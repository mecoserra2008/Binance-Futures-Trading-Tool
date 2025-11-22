use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use anyhow::{Result, anyhow};
use tracing::{info, warn, error, debug};
use std::collections::HashMap;
use url::Url;

use crate::config::Settings;
use super::{OrderflowEvent, LiquidationEvent, DepthUpdate};

#[derive(Debug, Deserialize)]
struct BinanceAggTradeMessage {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E")]
    event_time: u64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "a")]
    aggregate_trade_id: u64,
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "q")]
    quantity: String,
    #[serde(rename = "f")]
    first_trade_id: u64,
    #[serde(rename = "l")]
    last_trade_id: u64,
    #[serde(rename = "T")]
    trade_time: u64,
    #[serde(rename = "m")]
    is_buyer_maker: bool,
    #[serde(rename = "M")]
    ignore: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct BinanceLiquidationMessage {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E")]
    event_time: u64,
    #[serde(rename = "o")]
    order: LiquidationOrder,
}

#[derive(Debug, Deserialize)]
struct LiquidationOrder {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "S")]
    side: String,
    #[serde(rename = "o")]
    order_type: String,
    #[serde(rename = "f")]
    time_in_force: String,
    #[serde(rename = "q")]
    original_quantity: String,
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "ap")]
    average_price: String,
    #[serde(rename = "X")]
    order_status: String,
    #[serde(rename = "l")]
    last_filled_quantity: String,
    #[serde(rename = "z")]
    filled_accumulated_quantity: String,
    #[serde(rename = "T")]
    trade_time: u64,
}

#[derive(Debug, Serialize)]
struct StreamSubscription {
    method: String,
    params: Vec<String>,
    id: u64,
}

pub struct WebSocketManager {
    settings: Settings,
    orderflow_sender: mpsc::Sender<OrderflowEvent>,
    liquidation_sender: Option<mpsc::Sender<LiquidationEvent>>,
    depth_sender: Option<mpsc::Sender<DepthUpdate>>,
    active_symbols: Vec<String>,
}

impl WebSocketManager {
    pub fn new(settings: Settings, orderflow_sender: mpsc::Sender<OrderflowEvent>) -> Self {
        Self {
            settings,
            orderflow_sender,
            liquidation_sender: None,
            depth_sender: None,
            active_symbols: Vec::new(),
        }
    }

    pub fn set_liquidation_sender(&mut self, sender: mpsc::Sender<LiquidationEvent>) {
        self.liquidation_sender = Some(sender);
    }

    pub fn set_depth_sender(&mut self, sender: mpsc::Sender<DepthUpdate>) {
        self.depth_sender = Some(sender);
    }

    pub async fn start(&mut self) -> Result<()> {
        // Get ALL active symbols from Binance API for orderflow (like liquidations)
        self.active_symbols = self.settings.get_active_symbols().await?;
        info!("Found {} active USDT perpetual symbols for orderflow streams", self.active_symbols.len());

        // Start trade streams
        let trade_handle = self.start_trade_streams().await?;

        // Start liquidation stream
        let liquidation_handle = self.start_liquidation_stream().await?;

        // Start depth streams if enabled
        let depth_handle = if self.depth_sender.is_some() {
            Some(self.start_depth_streams().await?)
        } else {
            info!("Depth streams disabled (no depth sender configured)");
            None
        };

        // Wait for all streams
        tokio::select! {
            result = trade_handle => {
                if let Err(e) = result {
                    error!("Trade stream error: {}", e);
                }
            }
            result = liquidation_handle => {
                if let Err(e) = result {
                    error!("Liquidation stream error: {}", e);
                }
            }
            result = async { if let Some(h) = depth_handle { h.await } else { Ok(Ok(())) } } => {
                if let Err(e) = result {
                    error!("Depth stream error: {:?}", e);
                }
            }
        }

        Ok(())
    }

    async fn start_trade_streams(&self) -> Result<tokio::task::JoinHandle<Result<()>>> {
        let settings = self.settings.clone();
        let orderflow_sender = self.orderflow_sender.clone();
        let symbols = self.active_symbols.clone();

        let handle = tokio::spawn(async move {
            // Split symbols into chunks to avoid WebSocket limits
            const MAX_STREAMS_PER_CONNECTION: usize = 200; // Conservative limit
            let symbol_chunks: Vec<Vec<String>> = symbols
                .chunks(MAX_STREAMS_PER_CONNECTION)
                .map(|chunk| chunk.to_vec())
                .collect();

            info!("Splitting {} symbols into {} WebSocket connections", symbols.len(), symbol_chunks.len());

            let mut connection_handles = Vec::new();

            for (i, chunk) in symbol_chunks.into_iter().enumerate() {
                let settings_clone = settings.clone();
                let sender_clone = orderflow_sender.clone();

                let handle: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
                    let mut retry_count = 0;
                    let max_retries = settings_clone.binance.max_reconnect_attempts;

                    loop {
                        match Self::connect_trade_streams(&settings_clone, &sender_clone, &chunk).await {
                            Ok(_) => {
                                info!("Trade stream connection {} connected successfully with {} symbols", i, chunk.len());
                                retry_count = 0;
                            }
                            Err(e) => {
                                retry_count += 1;
                                error!("Trade stream connection {} failed (attempt {}/{}): {}",
                                       i, retry_count, max_retries, e);

                                if retry_count >= max_retries {
                                    return Err(anyhow!("Max retry attempts reached for trade stream connection {}", i));
                                }

                                let delay = Duration::from_millis(
                                    settings_clone.binance.reconnect_delay_ms * retry_count as u64
                                );
                                warn!("Reconnecting trade stream connection {} in {:?}", i, delay);
                                sleep(delay).await;
                            }
                        }
                    }
                });

                connection_handles.push(handle);
            }

            // Wait for all connections
            for (i, handle) in connection_handles.into_iter().enumerate() {
                if let Err(e) = handle.await {
                    error!("Trade stream connection {} failed: {:?}", i, e);
                }
            }

            Ok(())
        });

        Ok(handle)
    }

    async fn connect_trade_streams(
        settings: &Settings,
        orderflow_sender: &mpsc::Sender<OrderflowEvent>,
        symbols: &[String],
    ) -> Result<()> {
        let stream_names: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@aggTrade", s.to_lowercase()))
            .collect();

        let url = format!("{}/ws", settings.binance.websocket_base_url);
        let (ws_stream, _) = connect_async(&url).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to streams
        let subscription = StreamSubscription {
            method: "SUBSCRIBE".to_string(),
            params: stream_names.clone(),
            id: 1,
        };

        let subscribe_msg = Message::Text(serde_json::to_string(&subscription)?);
        ws_sender.send(subscribe_msg).await?;

        info!("Subscribed to {} trade streams", symbols.len());
        info!("First 10 symbols: {:?}", &symbols[..symbols.len().min(10)]);
        info!("Subscription message size: {} characters", serde_json::to_string(&subscription)?.len());
        debug!("First few stream names: {:?}", &stream_names[..stream_names.len().min(5)]);

        // Process incoming messages
        let mut message_count = 0;
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    message_count += 1;
                    if message_count % 100 == 0 {
                        info!("WebSocket received {} messages. Latest size: {} chars", message_count, text.len());
                    }
                    if text.contains("error") {
                        error!("WebSocket error message: {}", text);
                    }
                    if let Err(e) = Self::process_trade_message(&text, orderflow_sender).await {
                        debug!("Failed to process trade message: {}", e);
                    }
                }
                Ok(Message::Ping(ping)) => {
                    ws_sender.send(Message::Pong(ping)).await?;
                }
                Ok(Message::Close(_)) => {
                    warn!("WebSocket connection closed by server");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Err(anyhow!("Trade stream connection lost"))
    }

    async fn process_trade_message(
        text: &str,
        orderflow_sender: &mpsc::Sender<OrderflowEvent>,
    ) -> Result<()> {
        // Handle subscription confirmation messages
        if text.contains("\"result\":null") || text.contains("\"id\":") {
            info!("WebSocket subscription confirmed: {}", text);
            return Ok(());
        }

        let trade_msg: BinanceAggTradeMessage = serde_json::from_str(text)
            .map_err(|e| {
                error!("Failed to parse aggTrade message: {}. Message: {}", e, text);
                e
            })?;

        if trade_msg.event_type == "aggTrade" {
            let price = trade_msg.price.parse::<f64>()?;
            let quantity = trade_msg.quantity.parse::<f64>()?;

            let event = OrderflowEvent {
                symbol: trade_msg.symbol.clone(),
                timestamp: trade_msg.trade_time,
                price,
                quantity,
                is_buyer_maker: trade_msg.is_buyer_maker,
                trade_id: trade_msg.aggregate_trade_id,
            };

            debug!("Parsed aggTrade for {}: price={}, qty={}, buyer_maker={}",
                   event.symbol, event.price, event.quantity, event.is_buyer_maker);

            if let Err(e) = orderflow_sender.try_send(event.clone()) {
                error!("Failed to send orderflow event: {}", e);
            } else {
                debug!("Successfully sent orderflow event to channel");
            }
        } else {
            debug!("Received non-aggTrade event: {}", trade_msg.event_type);
        }

        Ok(())
    }

    async fn start_liquidation_stream(&self) -> Result<tokio::task::JoinHandle<Result<()>>> {
        let settings = self.settings.clone();
        let liquidation_sender = self.liquidation_sender.clone();

        let handle = tokio::spawn(async move {
            let mut retry_count = 0;
            let max_retries = settings.binance.max_reconnect_attempts;

            loop {
                match Self::connect_liquidation_stream(&settings, &liquidation_sender).await {
                    Ok(_) => {
                        info!("Liquidation stream connected successfully");
                        retry_count = 0;
                    }
                    Err(e) => {
                        retry_count += 1;
                        error!("Liquidation stream connection failed (attempt {}/{}): {}", 
                               retry_count, max_retries, e);

                        if retry_count >= max_retries {
                            return Err(anyhow!("Max retry attempts reached for liquidation stream"));
                        }

                        let delay = Duration::from_millis(
                            settings.binance.reconnect_delay_ms * retry_count as u64
                        );
                        warn!("Reconnecting liquidation stream in {:?}", delay);
                        sleep(delay).await;
                    }
                }
            }
        });

        Ok(handle)
    }

    async fn connect_liquidation_stream(
        settings: &Settings,
        liquidation_sender: &Option<mpsc::Sender<LiquidationEvent>>,
    ) -> Result<()> {
        let url = format!("{}/ws/!forceOrder@arr", settings.binance.websocket_base_url);
        let (ws_stream, _) = connect_async(&url).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        info!("Connected to liquidation stream");

        // Process incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Some(sender) = liquidation_sender {
                        if let Err(e) = Self::process_liquidation_message(&text, sender).await {
                            debug!("Failed to process liquidation message: {}", e);
                        }
                    }
                }
                Ok(Message::Ping(ping)) => {
                    ws_sender.send(Message::Pong(ping)).await?;
                }
                Ok(Message::Close(_)) => {
                    warn!("Liquidation WebSocket connection closed by server");
                    break;
                }
                Err(e) => {
                    error!("Liquidation WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Err(anyhow!("Liquidation stream connection lost"))
    }

    async fn process_liquidation_message(
        text: &str,
        liquidation_sender: &mpsc::Sender<LiquidationEvent>,
    ) -> Result<()> {
        let liquidation_msg: BinanceLiquidationMessage = serde_json::from_str(text)?;

        if liquidation_msg.event_type == "forceOrder" {
            let order = liquidation_msg.order;
            
            let event = LiquidationEvent {
                symbol: order.symbol,
                timestamp: liquidation_msg.event_time,
                side: order.side,
                price: order.price.parse()?,
                quantity: order.original_quantity.parse()?,
                is_forced: true,
                notional_value: order.price.parse::<f64>()? * order.original_quantity.parse::<f64>()?,
            };

            if let Err(e) = liquidation_sender.try_send(event) {
                debug!("Failed to send liquidation event: {}", e);
            }
        }

        Ok(())
    }

    async fn start_depth_streams(&self) -> Result<tokio::task::JoinHandle<Result<()>>> {
        let settings = self.settings.clone();
        let depth_sender = self.depth_sender.clone();
        let symbols = self.active_symbols.clone();

        let handle = tokio::spawn(async move {
            // Split symbols into chunks to avoid WebSocket limits
            const MAX_STREAMS_PER_CONNECTION: usize = 200;
            let symbol_chunks: Vec<Vec<String>> = symbols
                .chunks(MAX_STREAMS_PER_CONNECTION)
                .map(|chunk| chunk.to_vec())
                .collect();

            info!("Splitting {} symbols into {} depth WebSocket connections", symbols.len(), symbol_chunks.len());

            let mut connection_handles = Vec::new();

            for (i, chunk) in symbol_chunks.into_iter().enumerate() {
                let settings_clone = settings.clone();
                let sender_clone = depth_sender.clone();

                let handle: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
                    let mut retry_count = 0;
                    let max_retries = settings_clone.binance.max_reconnect_attempts;

                    loop {
                        match Self::connect_depth_streams(&settings_clone, &sender_clone, &chunk).await {
                            Ok(_) => {
                                info!("Depth stream connection {} connected successfully with {} symbols", i, chunk.len());
                                retry_count = 0;
                            }
                            Err(e) => {
                                retry_count += 1;
                                error!("Depth stream connection {} failed (attempt {}/{}): {}",
                                       i, retry_count, max_retries, e);

                                if retry_count >= max_retries {
                                    return Err(anyhow!("Max retry attempts reached for depth stream connection {}", i));
                                }

                                let delay = Duration::from_millis(
                                    settings_clone.binance.reconnect_delay_ms * retry_count as u64
                                );
                                warn!("Reconnecting depth stream connection {} in {:?}", i, delay);
                                sleep(delay).await;
                            }
                        }
                    }
                });

                connection_handles.push(handle);
            }

            // Wait for all connections
            for (i, handle) in connection_handles.into_iter().enumerate() {
                if let Err(e) = handle.await {
                    error!("Depth stream connection {} failed: {:?}", i, e);
                }
            }

            Ok(())
        });

        Ok(handle)
    }

    async fn connect_depth_streams(
        settings: &Settings,
        depth_sender: &Option<mpsc::Sender<DepthUpdate>>,
        symbols: &[String],
    ) -> Result<()> {
        let stream_names: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@depth@100ms", s.to_lowercase()))
            .collect();

        let url = format!("{}/ws", settings.binance.websocket_base_url);
        let (ws_stream, _) = connect_async(&url).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to streams
        let subscription = StreamSubscription {
            method: "SUBSCRIBE".to_string(),
            params: stream_names.clone(),
            id: 2,
        };

        let subscribe_msg = Message::Text(serde_json::to_string(&subscription)?);
        ws_sender.send(subscribe_msg).await?;

        info!("Subscribed to {} depth streams", symbols.len());
        debug!("First few depth stream names: {:?}", &stream_names[..stream_names.len().min(5)]);

        // Process incoming messages
        let mut message_count = 0;
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    message_count += 1;
                    if message_count % 1000 == 0 {
                        info!("Depth WebSocket received {} messages", message_count);
                    }
                    if text.contains("error") {
                        error!("Depth WebSocket error message: {}", text);
                    }
                    if let Some(sender) = depth_sender {
                        if let Err(e) = Self::process_depth_message(&text, sender).await {
                            debug!("Failed to process depth message: {}", e);
                        }
                    }
                }
                Ok(Message::Ping(ping)) => {
                    ws_sender.send(Message::Pong(ping)).await?;
                }
                Ok(Message::Close(_)) => {
                    warn!("Depth WebSocket connection closed by server");
                    break;
                }
                Err(e) => {
                    error!("Depth WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Err(anyhow!("Depth stream connection lost"))
    }

    async fn process_depth_message(
        text: &str,
        depth_sender: &mpsc::Sender<DepthUpdate>,
    ) -> Result<()> {
        // Handle subscription confirmation messages
        if text.contains("\"result\":null") || text.contains("\"id\":") {
            debug!("Depth WebSocket subscription confirmed");
            return Ok(());
        }

        let depth_update: DepthUpdate = serde_json::from_str(text)
            .map_err(|e| {
                debug!("Failed to parse depth message: {}. Message snippet: {}", e, &text[..text.len().min(100)]);
                e
            })?;

        if depth_update.event_type == "depthUpdate" {
            debug!("Parsed depthUpdate for {}: {} bids, {} asks",
                   depth_update.symbol, depth_update.bids.len(), depth_update.asks.len());

            if let Err(e) = depth_sender.try_send(depth_update) {
                debug!("Failed to send depth update: {}", e);
            }
        }

        Ok(())
    }
}

// Connection health monitoring
pub struct ConnectionMonitor {
    trade_stream_active: bool,
    liquidation_stream_active: bool,
    last_trade_time: Option<u64>,
    last_liquidation_time: Option<u64>,
}

impl ConnectionMonitor {
    pub fn new() -> Self {
        Self {
            trade_stream_active: false,
            liquidation_stream_active: false,
            last_trade_time: None,
            last_liquidation_time: None,
        }
    }

    pub fn update_trade_activity(&mut self, timestamp: u64) {
        self.trade_stream_active = true;
        self.last_trade_time = Some(timestamp);
    }

    pub fn update_liquidation_activity(&mut self, timestamp: u64) {
        self.liquidation_stream_active = true;
        self.last_liquidation_time = Some(timestamp);
    }

    pub fn check_health(&self, max_silence_seconds: u64) -> bool {
        let current_time = chrono::Utc::now().timestamp() as u64;
        
        if let Some(last_trade) = self.last_trade_time {
            if current_time - last_trade > max_silence_seconds {
                return false;
            }
        }

        true
    }

    pub fn get_status(&self) -> String {
        format!(
            "Trade Stream: {}, Liquidation Stream: {}, Last Trade: {:?}, Last Liquidation: {:?}",
            if self.trade_stream_active { "Active" } else { "Inactive" },
            if self.liquidation_stream_active { "Active" } else { "Inactive" },
            self.last_trade_time,
            self.last_liquidation_time
        )
    }
}