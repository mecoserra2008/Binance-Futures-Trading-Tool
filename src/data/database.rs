use rusqlite::{Connection, Result as SqliteResult, params};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use super::{OrderflowEvent, VolumeProfile, OrderImbalance, LiquidationEvent, Candle, DailyStats};

pub struct DatabaseManager {
    connection: Arc<Mutex<Connection>>,
}

impl DatabaseManager {
    pub async fn new(db_path: &str) -> Result<Arc<Self>> {
        let conn = Connection::open(db_path)
            .context("Failed to open database connection")?;
        
        println!("Setting up database PRAGMA settings...");
        // Enable WAL mode for better concurrent access
        let _ = conn.execute("PRAGMA journal_mode=WAL", []);
        let _ = conn.execute("PRAGMA synchronous=NORMAL", []);
        let _ = conn.execute("PRAGMA cache_size=10000", []);
        let _ = conn.execute("PRAGMA temp_store=memory", []);

        Ok(Arc::new(Self {
            connection: Arc::new(Mutex::new(conn)),
        }))
    }

    pub async fn initialize_schema(&self) -> Result<()> {
        let conn = self.connection.lock().await;
        
        println!("Creating candles table...");
        // Create candles table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS candles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                open_price REAL NOT NULL,
                high_price REAL NOT NULL,
                low_price REAL NOT NULL,
                close_price REAL NOT NULL,
                volume REAL NOT NULL,
                buy_volume REAL NOT NULL,
                sell_volume REAL NOT NULL,
                timeframe TEXT NOT NULL,
                trade_count INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER,
                UNIQUE(symbol, timestamp, timeframe)
            )
            "#,
            [],
        )?;

        println!("Creating volume profile table...");
        // Create volume profile table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS volume_profile (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                price_level REAL NOT NULL,
                buy_volume REAL NOT NULL,
                sell_volume REAL NOT NULL,
                total_volume REAL NOT NULL,
                trade_count INTEGER NOT NULL DEFAULT 0,
                timeframe TEXT NOT NULL,
                created_at INTEGER
            )
            "#,
            [],
        )?;

        println!("Creating order imbalances table...");
        // Create order imbalances table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS order_imbalances (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                bid_volume REAL NOT NULL,
                ask_volume REAL NOT NULL,
                imbalance_ratio REAL NOT NULL,
                window_duration_seconds INTEGER NOT NULL,
                created_at INTEGER
            )
            "#,
            [],
        )?;

        println!("Creating liquidations table...");
        // Create liquidations table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS liquidations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                side TEXT NOT NULL,
                price REAL NOT NULL,
                quantity REAL NOT NULL,
                is_forced INTEGER NOT NULL,
                notional_value REAL NOT NULL,
                created_at INTEGER
            )
            "#,
            [],
        )?;

        println!("Creating daily stats table...");
        // Create daily stats table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS daily_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                date TEXT NOT NULL,
                avg_volume REAL NOT NULL,
                total_volume REAL NOT NULL,
                avg_price REAL NOT NULL,
                high_price REAL NOT NULL,
                low_price REAL NOT NULL,
                trade_count INTEGER NOT NULL,
                created_at INTEGER,
                updated_at INTEGER,
                UNIQUE(symbol, date)
            )
            "#,
            [],
        )?;

        println!("Creating raw trades table...");
        // Create raw trades table for temporary storage
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS raw_trades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                price REAL NOT NULL,
                quantity REAL NOT NULL,
                is_buyer_maker INTEGER NOT NULL,
                trade_id INTEGER NOT NULL,
                created_at INTEGER
            )
            "#,
            [],
        )?;
        
        println!("Database schema initialized successfully!");
        Ok(())
    }

    pub async fn insert_orderflow_event(&self, event: &OrderflowEvent) -> Result<()> {
        let conn = self.connection.lock().await;
        
        conn.execute(
            r#"
            INSERT OR REPLACE INTO raw_trades 
            (symbol, timestamp, price, quantity, is_buyer_maker, trade_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                event.symbol,
                event.timestamp,
                event.price,
                event.quantity,
                if event.is_buyer_maker { 1 } else { 0 },
                event.trade_id
            ],
        )?;

        Ok(())
    }

    pub async fn insert_candle(&self, candle: &Candle) -> Result<()> {
        let conn = self.connection.lock().await;
        
        conn.execute(
            r#"
            INSERT OR REPLACE INTO candles 
            (symbol, timestamp, open_price, high_price, low_price, close_price, 
             volume, buy_volume, sell_volume, timeframe, trade_count)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                candle.symbol,
                candle.timestamp,
                candle.open_price,
                candle.high_price,
                candle.low_price,
                candle.close_price,
                candle.volume,
                candle.buy_volume,
                candle.sell_volume,
                candle.timeframe,
                candle.trade_count
            ],
        )?;

        Ok(())
    }

    pub async fn insert_volume_profile(&self, profile: &VolumeProfile) -> Result<()> {
        let conn = self.connection.lock().await;
        
        // Clear existing profile data for this timestamp and symbol
        conn.execute(
            "DELETE FROM volume_profile WHERE symbol = ?1 AND timestamp = ?2 AND timeframe = ?3",
            params![profile.symbol, profile.timestamp, profile.timeframe],
        )?;

        // Insert new profile data
        for (price, volume_data) in &profile.price_levels {
            conn.execute(
                r#"
                INSERT INTO volume_profile 
                (symbol, timestamp, price_level, buy_volume, sell_volume, total_volume, trade_count, timeframe)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    profile.symbol,
                    profile.timestamp,
                    price.0,
                    volume_data.buy_volume,
                    volume_data.sell_volume,
                    volume_data.total_volume,
                    volume_data.trade_count,
                    profile.timeframe
                ],
            )?;
        }

        Ok(())
    }

    pub async fn insert_order_imbalance(&self, imbalance: &OrderImbalance) -> Result<()> {
        let conn = self.connection.lock().await;
        
        conn.execute(
            r#"
            INSERT INTO order_imbalances 
            (symbol, timestamp, bid_volume, ask_volume, imbalance_ratio, window_duration_seconds)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                imbalance.symbol,
                imbalance.timestamp,
                imbalance.bid_volume,
                imbalance.ask_volume,
                imbalance.imbalance_ratio,
                imbalance.window_duration_seconds
            ],
        )?;

        Ok(())
    }

    pub async fn insert_liquidation(&self, liquidation: &LiquidationEvent) -> Result<()> {
        let conn = self.connection.lock().await;
        
        conn.execute(
            r#"
            INSERT INTO liquidations 
            (symbol, timestamp, side, price, quantity, is_forced, notional_value)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                liquidation.symbol,
                liquidation.timestamp,
                liquidation.side,
                liquidation.price,
                liquidation.quantity,
                if liquidation.is_forced { 1 } else { 0 },
                liquidation.notional_value
            ],
        )?;

        Ok(())
    }

    pub async fn insert_or_update_daily_stats(&self, stats: &DailyStats) -> Result<()> {
        let conn = self.connection.lock().await;
        
        conn.execute(
            r#"
            INSERT OR REPLACE INTO daily_stats 
            (symbol, date, avg_volume, total_volume, avg_price, high_price, low_price, trade_count)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                stats.symbol,
                stats.date,
                stats.avg_volume,
                stats.total_volume,
                stats.avg_price,
                stats.high_price,
                stats.low_price,
                stats.trade_count
            ],
        )?;

        Ok(())
    }

    pub async fn get_daily_stats(&self, symbol: &str, date: &str) -> Result<Option<DailyStats>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT symbol, date, avg_volume, total_volume, avg_price, high_price, low_price, trade_count 
             FROM daily_stats WHERE symbol = ?1 AND date = ?2"
        )?;

        let result = stmt.query_row(params![symbol, date], |row| {
            Ok(DailyStats {
                symbol: row.get(0)?,
                date: row.get(1)?,
                avg_volume: row.get(2)?,
                total_volume: row.get(3)?,
                avg_price: row.get(4)?,
                high_price: row.get(5)?,
                low_price: row.get(6)?,
                trade_count: row.get(7)?,
            })
        });

        match result {
            Ok(stats) => Ok(Some(stats)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn get_recent_candles(&self, symbol: &str, timeframe: &str, limit: usize) -> Result<Vec<Candle>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            r#"
            SELECT symbol, timestamp, open_price, high_price, low_price, close_price,
                   volume, buy_volume, sell_volume, timeframe, trade_count
            FROM candles 
            WHERE symbol = ?1 AND timeframe = ?2 
            ORDER BY timestamp DESC 
            LIMIT ?3
            "#
        )?;

        let candle_iter = stmt.query_map(params![symbol, timeframe, limit], |row| {
            Ok(Candle {
                symbol: row.get(0)?,
                timestamp: row.get(1)?,
                open_price: row.get(2)?,
                high_price: row.get(3)?,
                low_price: row.get(4)?,
                close_price: row.get(5)?,
                volume: row.get(6)?,
                buy_volume: row.get(7)?,
                sell_volume: row.get(8)?,
                timeframe: row.get(9)?,
                trade_count: row.get(10)?,
            })
        })?;

        let mut candles = Vec::new();
        for candle in candle_iter {
            candles.push(candle?);
        }

        candles.reverse(); // Return in chronological order
        Ok(candles)
    }

    pub async fn cleanup_old_data(&self, days_to_keep: u32) -> Result<()> {
        let conn = self.connection.lock().await;
        let cutoff_timestamp = chrono::Utc::now().timestamp() - (days_to_keep as i64 * 24 * 60 * 60);

        // Clean up old raw trades
        conn.execute(
            "DELETE FROM raw_trades WHERE timestamp < ?1",
            params![cutoff_timestamp],
        )?;

        // Clean up old imbalances
        conn.execute(
            "DELETE FROM order_imbalances WHERE timestamp < ?1",
            params![cutoff_timestamp],
        )?;

        // Clean up old liquidations (keep longer for analysis)
        let liquidation_cutoff = cutoff_timestamp - (30 * 24 * 60 * 60); // Keep 30 extra days
        conn.execute(
            "DELETE FROM liquidations WHERE timestamp < ?1",
            params![liquidation_cutoff],
        )?;

        // Vacuum database to reclaim space
        conn.execute("VACUUM", [])?;

        Ok(())
    }

    pub async fn get_connection_stats(&self) -> Result<String> {
        let conn = self.connection.lock().await;
        
        let mut stats = String::new();
        
        // Get table sizes
        let tables = ["candles", "volume_profile", "order_imbalances", "liquidations", "daily_stats", "raw_trades"];
        
        for table in &tables {
            let mut stmt = conn.prepare(&format!("SELECT COUNT(*) FROM {}", table))?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            stats.push_str(&format!("{}: {} rows\n", table, count));
        }

        Ok(stats)
    }
}