use eframe::egui;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, error};

mod config;
mod data;
mod analysis;
mod gui;
mod utils;

use config::Settings;
use data::*;
use analysis::*;
use gui::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing with debug level
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    info!("Starting Binance Futures Orderflow Screener");

    // Load configuration
    let settings = Settings::new()?;
    
    // Create data channels
    let (orderflow_tx, orderflow_rx) = mpsc::channel::<OrderflowEvent>(10000);
    let (imbalance_tx, imbalance_rx) = mpsc::channel::<OrderImbalance>(1000);
    let (liquidation_tx, liquidation_rx) = mpsc::channel::<LiquidationEvent>(1000);
    let (volume_tx, volume_rx) = mpsc::channel::<VolumeProfile>(1000);
    let (gui_update_tx, gui_update_rx) = mpsc::channel::<GuiUpdate>(1000);
    let (gui_orderflow_tx, gui_orderflow_rx) = mpsc::channel::<OrderflowEvent>(10000);

    // Initialize database
    let db_manager = DatabaseManager::new("data.db").await?;
    db_manager.initialize_schema().await?;

    // Start WebSocket manager
    let mut ws_manager = WebSocketManager::new(settings.clone(), orderflow_tx.clone());
    ws_manager.set_liquidation_sender(liquidation_tx.clone());
    let ws_handle = tokio::spawn(async move {
        if let Err(e) = ws_manager.start().await {
            error!("WebSocket manager error: {}", e);
        }
    });

    // Start analysis engines
    let analysis_handles = start_analysis_engines(
        orderflow_rx,
        imbalance_tx,
        liquidation_tx,
        volume_tx,
        gui_update_tx,
        gui_orderflow_tx,
        db_manager.clone(),
        settings.binance.api_base_url.clone(),
    ).await;

    // Start GUI application
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1920.0, 1080.0])
            .with_title("Binance Futures Orderflow Screener"),
        ..Default::default()
    };

    let app = ScreenerApp::new(
        imbalance_rx,
        liquidation_rx,
        volume_rx,
        gui_update_rx,
        gui_orderflow_rx,
        db_manager,
        settings.binance.symbols.clone(), // Pass the actual subscribed symbols
    ).await?;

    eframe::run_native(
        "Binance Futures Orderflow Screener",
        native_options,
        Box::new(|_cc| Box::new(app)),
    ).map_err(|e| anyhow::anyhow!("GUI error: {}", e))?;

    // Cleanup
    ws_handle.abort();
    for handle in analysis_handles {
        handle.abort();
    }

    Ok(())
}

async fn start_analysis_engines(
    mut orderflow_rx: mpsc::Receiver<OrderflowEvent>,
    imbalance_tx: mpsc::Sender<OrderImbalance>,
    liquidation_tx: mpsc::Sender<LiquidationEvent>,
    volume_tx: mpsc::Sender<VolumeProfile>,
    _gui_update_tx: mpsc::Sender<GuiUpdate>,
    gui_orderflow_tx: mpsc::Sender<OrderflowEvent>,
    _db_manager: Arc<DatabaseManager>,
    api_base_url: String,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();

    // Create channels for distributing orderflow events to multiple analyzers
    let (orderflow_broadcast_tx, orderflow_broadcast_rx1) = mpsc::channel::<OrderflowEvent>(1000);
    let (orderflow_broadcast_tx2, orderflow_broadcast_rx2) = mpsc::channel::<OrderflowEvent>(1000);
    let (orderflow_broadcast_tx3, orderflow_broadcast_rx3) = mpsc::channel::<OrderflowEvent>(1000);

    // Event distributor
    let handle = tokio::spawn(async move {
        let mut event_count = 0;
        while let Some(event) = orderflow_rx.recv().await {
            event_count += 1;
            if event_count % 100 == 0 {
                tracing::info!("Event distributor processed {} events. Latest: {} @ {}", event_count, event.symbol, event.price);
            }

            // Distribute to all analyzers and GUI
            let _ = orderflow_broadcast_tx.try_send(event.clone());
            let _ = orderflow_broadcast_tx2.try_send(event.clone());
            let _ = orderflow_broadcast_tx3.try_send(event.clone());
            let _ = gui_orderflow_tx.try_send(event.clone());
        }
    });
    handles.push(handle);

    // Imbalance analyzer
    let imbalance_analyzer = ImbalanceAnalyzer::new(imbalance_tx.clone());
    let orderflow_rx_arc = Arc::new(tokio::sync::Mutex::new(orderflow_broadcast_rx1));
    let handle = tokio::spawn(async move {
        if let Err(e) = imbalance_analyzer.start(orderflow_rx_arc).await {
            error!("Imbalance analyzer error: {}", e);
        }
    });
    handles.push(handle);

    // Volume analyzer
    let mut volume_analyzer = VolumeAnalyzer::new(volume_tx.clone(), api_base_url);
    let orderflow_rx2_arc = Arc::new(tokio::sync::Mutex::new(orderflow_broadcast_rx2));
    let handle = tokio::spawn(async move {
        if let Err(e) = volume_analyzer.start_with_receiver(orderflow_rx2_arc).await {
            error!("Volume analyzer error: {}", e);
        }
    });
    handles.push(handle);

    // Liquidation detector
    let mut liquidation_detector = LiquidationDetector::new(liquidation_tx.clone());
    let orderflow_rx3_arc = Arc::new(tokio::sync::Mutex::new(orderflow_broadcast_rx3));
    let handle = tokio::spawn(async move {
        if let Err(e) = liquidation_detector.start_with_receiver(orderflow_rx3_arc).await {
            error!("Liquidation detector error: {}", e);
        }
    });
    handles.push(handle);

    handles
}