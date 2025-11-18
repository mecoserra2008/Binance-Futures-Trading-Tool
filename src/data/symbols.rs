use std::collections::HashMap;

/// Comprehensive list of Binance futures trading symbols
pub struct BinanceSymbols;

impl BinanceSymbols {
    /// Get all major Binance futures symbols
    pub fn get_all_symbols() -> Vec<String> {
        vec![
            // Major cryptocurrencies
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "BNBUSDT".to_string(),
            "ADAUSDT".to_string(),
            "XRPUSDT".to_string(),
            "SOLUSDT".to_string(),
            "DOTUSDT".to_string(),
            "DOGEUSDT".to_string(),
            "AVAXUSDT".to_string(),
            "SHIBUSDT".to_string(),
            "MATICUSDT".to_string(),
            "LTCUSDT".to_string(),
            "ATOMUSDT".to_string(),
            "LINKUSDT".to_string(),
            "ETCUSDT".to_string(),
            "BCHUSDT".to_string(),
            "FILUSDT".to_string(),
            "TRXUSDT".to_string(),
            "NEARUSDT".to_string(),
            "ALGOUSDT".to_string(),

            // Layer 1 & DeFi
            "UNIUSDT".to_string(),
            "AAVEUSDT".to_string(),
            "MKRUSDT".to_string(),
            "COMPUSDT".to_string(),
            "SUSHIUSDT".to_string(),
            "CRVUSDT".to_string(),
            "1INCHUSDT".to_string(),
            "YFIUSDT".to_string(),
            "SNXUSDT".to_string(),
            "BALUSDT".to_string(),

            // Layer 2 Solutions
            "ARBUSDT".to_string(),
            "OPUSDT".to_string(),
            "MASKUSDT".to_string(),
            "IMXUSDT".to_string(),
            "METISUSDT".to_string(),

            // Gaming & Metaverse
            "AXSUSDT".to_string(),
            "SANDUSDT".to_string(),
            "MANAUSDT".to_string(),
            "ENJUSDT".to_string(),
            "GALAUSDT".to_string(),
            "CHZUSDT".to_string(),
            "FLOWUSDT".to_string(),
            "APECOINUSDT".to_string(),

            // AI & Big Data
            "FETUSDT".to_string(),
            "OCEANUSDT".to_string(),
            "AGIXUSDT".to_string(),
            "RNDRУСDT".to_string(),

            // Meme Coins
            "PEPEUSDT".to_string(),
            "FLOKIUSDT".to_string(),
            "BONKUSDT".to_string(),
            "1000RATSUSDT".to_string(),

            // Infrastructure
            "ICPUSDT".to_string(),
            "THETAUSDT".to_string(),
            "VETUSDT".to_string(),
            "IOTAUSDT".to_string(),
            "HBARUSDT".to_string(),
            "EGLDUSDT".to_string(),
            "FTMUSDT".to_string(),
            "ONEUSDT".to_string(),
            "ZILUSDT".to_string(),
            "WAVESUSDT".to_string(),

            // Privacy Coins
            "XMRUSDT".to_string(),
            "ZECUSDT".to_string(),
            "DASHUSDT".to_string(),

            // Enterprise & Institutional
            "XLMUSDT".to_string(),
            "XTZUSDT".to_string(),
            "EOSUSDT".to_string(),
            "IOTXUSDT".to_string(),

            // Newer Trending Assets
            "SUIUSDT".to_string(),
            "APTUSDT".to_string(),
            "LDOUSDT".to_string(),
            "INJUSDT".to_string(),
            "STXUSDT".to_string(),
            "TIAUSDT".to_string(),
            "SEIUSDT".to_string(),
            "JUPUSDT".to_string(),
            "WIFUSDT".to_string(),
            "TNSRUSDT".to_string(),

            // Cross-chain & Interoperability
            "DOTUSDT".to_string(),
            "KSMUSDT".to_string(),
            "RUNEUSDT".to_string(),
            "THORUSDT".to_string(),

            // Stablecoins (USDT pairs)
            "BUSDUSDT".to_string(),
            "USDCUSDT".to_string(),
            "DAIUSDT".to_string(),
            "TUSDUSDT".to_string(),
            "FDUSDUSDT".to_string(),

            // High volatility / High volume pairs
            "GMTUSDT".to_string(),
            "STEPNUSDT".to_string(),
            "GALUSDT".to_string(),
            "JASMYUSDT".to_string(),
            "WOOUSDT".to_string(),
            "ALPHAUSDT".to_string(),
            "DYDXUSDT".to_string(),
            "GMXUSDT".to_string(),
            "BLURUSDT".to_string(),
            "IDUSDT".to_string(),
            "ARBUSDT".to_string(),
            "MAGICUSDT".to_string(),
            "RDNTUSDT".to_string(),
            "EDUUSDT".to_string(),
            "SUIUSDT".to_string(),
            "1000XECUSDT".to_string(),
            "PENDLEUSDT".to_string(),
            "ARKMUSDT".to_string(),
            "WLDUSDT".to_string(),
            "FXSUSDT".to_string(),
            "MAVUSDT".to_string(),
            "MDTUSDT".to_string(),
            "XVGUSDT".to_string(),
            "CELRUSDT".to_string(),
            "KEYUSDT".to_string(),
            "CYBERUSDT".to_string(),
            "HIFIUSDT".to_string(),
            "ARKUSDT".to_string(),
            "GLMRUSDT".to_string(),
            "BICOUSDT".to_string(),
            "STRAXUSDT".to_string(),
            "LOOMUSDT".to_string(),
            "BIGTIMEUSDT".to_string(),
            "BONDUSDT".to_string(),
            "ORBSUSDT".to_string(),
            "STGUSDT".to_string(),
            "TOKENUSDT".to_string(),
            "BADGERUSDT".to_string(),
            "RLCUSDT".to_string(),
            "COMBOUSDT".to_string(),
            "NMRUSDT".to_string(),
            "MAVUSDT".to_string(),
            "PHBUSDT".to_string(),
            "LEVERUSDT".to_string(),
            "DGBUSDT".to_string(),
            "POLYXUSDT".to_string(),
            "ACHUSDT".to_string(),
            "IMXUSDT".to_string(),
            "UNFIUSDT".to_string(),
            "FRONTUSDT".to_string(),
            "AGLDUSDT".to_string(),
            "ROSEUSDT".to_string(),
            "AUDIOUSDT".to_string(),
            "LINAUSDT".to_string(),
            "REQUSDT".to_string(),
            "CVXUSDT".to_string(),
            "ENSUSDT".to_string(),
            "PEOPLEUSDT".to_string(),
            "ANTUSDT".to_string(),
            "REIUSDT".to_string(),
            "OPUSDT".to_string(),
            "SLPUSDT".to_string(),
            "AMBUSDT".to_string(),
            "LEVERFIUSDT".to_string(),
            "RDNTUSDT".to_string(),
            "HFTUSDT".to_string(),
            "PHBUSDT".to_string(),
            "HOOKUSDT".to_string(),
            "MAGICUSDT".to_string(),
            "HIGHUSDT".to_string(),
            "MINAUSDT".to_string(),
            "ASTRUSDT".to_string(),
            "AGIXUSDT".to_string(),
            "NKNUSDT".to_string(),
            "GMXUSDT".to_string(),
            "MEMECOINUSDT".to_string(),
            "CFXUSDT".to_string(),
            "STXUSDT".to_string(),
            "COCOSUSDT".to_string(),
            "KLAYUSDT".to_string(),
            "QNTUSDT".to_string(),
            "IDEXUSDT".to_string(),
            "EDUUSDT".to_string(),
            "IDUSDT".to_string(),
            "USTCUSDT".to_string(),
            "APEUSDT".to_string(),
            "GMRTUSDT".to_string(),
            "ANCUSDT".to_string(),
            "XNOUSDT".to_string(),
            "WOOУСDT".to_string(),
            "INJUSDT".to_string(),
            "STEEMUSDT".to_string(),
            "LSKUSDT".to_string(),
            "UNFIUSDT".to_string(),
            "BEAMUSDT".to_string(),
            "PYRUSDT".to_string(),
            "NCTUSDT".to_string(),
            "PUNDIXUSDT".to_string(),
            "TLMUSDT".to_string(),
            "BADGERUSDT".to_string(),
            "FISUSDT".to_string(),
            "OMGUSDT".to_string(),
            "PONDUSDT".to_string(),
            "DEGOUSDT".to_string(),
            "ALICEUSDT".to_string(),
            "LINAUSDT".to_string(),
            "PERPUSDT".to_string(),
            "SUPERUSDT".to_string(),
            "CFXUSDT".to_string(),
            "EPSUSDT".to_string(),
            "AUTOUSDT".to_string(),
            "TKOUSDT".to_string(),
            "PAXGUSDT".to_string(),
            "QUICKUSDT".to_string(),
            "1000LUNCUSDT".to_string(),
            "USTCUSDT".to_string(),
            "SIDUSDT".to_string(),
            "RAREUSDT".to_string(),
            "ADXUSDT".to_string(),
            "AUCTIONUSDT".to_string(),
            "DARUSDT".to_string(),
            "BNXUSDT".to_string(),
            "RGTUSDT".to_string(),
            "MOVRUSDT".to_string(),
            "CITYUSDT".to_string(),
            "ENSUSDT".to_string(),
            "KP3RUSDT".to_string(),
            "QIUSDT".to_string(),
            "PORTOUSDT".to_string(),
            "POWRUSDT".to_string(),
            "VGXUSDT".to_string(),
            "JASMYUSDT".to_string(),
            "AMPUSDT".to_string(),
            "PLAUSDT".to_string(),
            "PYRUSDT".to_string(),
            "RNDRUSDT".to_string(),
            "ALCXUSDT".to_string(),
            "SANTOSUSDT".to_string(),
            "MCUSDT".to_string(),
            "ANYUSDT".to_string(),
            "BICOUSDT".to_string(),
            "FLUXUSDT".to_string(),
            "FXSUSDT".to_string(),
            "VOXELUSDT".to_string(),
            "HIGHUSDT".to_string(),
            "CVXUSDT".to_string(),
            "PEOPLEUSDT".to_string(),
            "OOKIUSDT".to_string(),
            "SPELLUSDT".to_string(),
            "1000FLOKIUSDT".to_string(),
            "RADUSDT".to_string(),
            "BANDUSDT".to_string(),
            "COTIUSDT".to_string(),
            "CKBUSDT".to_string(),
            "TWTUSDT".to_string(),
            "FIROUSDT".to_string(),
            "LITUSDT".to_string(),
            "SFPUSDT".to_string(),
            "DODOUSDT".to_string(),
            "CAKEUSDT".to_string(),
            "ACMUSDT".to_string(),
            "BADGERUSDT".to_string(),
            "FISUSDT".to_string(),
            "RAYUSDT".to_string(),
            "C98USDT".to_string(),
            "MASKUSDT".to_string(),
            "ATAUSDT".to_string(),
            "DOCKUSDT".to_string(),
            "POLYUSDT".to_string(),
            "MDXUSDT".to_string(),
            "DENTUSDT".to_string(),
            "NUUSDT".to_string(),
            "CKBUSDT".to_string(),
            "REQUSDT".to_string(),
            "WAXPUSDT".to_string(),
            "OGNUSDT".to_string(),
            "LRCUSDT".to_string(),
            "PNTUSDT".to_string(),
            "BTCSTUSDT".to_string(),
            "TRUUSDT".to_string(),
            "LOOKSUSDT".to_string(),
            "APEUSDT".to_string(),
            "STGUSDT".to_string(),
            "GALUSDT".to_string(),
            "JASMYUSDT".to_string(),
            "LDOUSDT".to_string(),
            "EPXUSDT".to_string(),
            "LEVERFIUSDT".to_string(),
            "STEEMUSDT".to_string(),
            "REIUSDT".to_string(),
            "OPSUSDT".to_string(),
            "HFTUSDT".to_string(),
            "PHBUSDT".to_string(),
            "HOOKUSDT".to_string(),
            "MAGICUSDT".to_string(),
            "TUSDT".to_string(),
            "HIGHUSDT".to_string(),
            "MINAUSDT".to_string(),
            "ASTRUSDT".to_string(),
            "AGIXUSDT".to_string(),
            "NKNUSDT".to_string(),
            "NEOUSDT".to_string(),
            "METAUSDT".to_string(),
            "UTKUSDT".to_string(),
            "SSVUSDT".to_string(),
            "GASUSDT".to_string(),
            "POWRUSDT".to_string(),
            "AVAXUSDT".to_string(),
            "TUSDT".to_string(),
            "API3USDT".to_string(),
            "XMRUSDT".to_string(),
            "WOOUSDT".to_string(),
            "FTTUSDT".to_string(),
            "BTCDOMUSDT".to_string(),
            "XRPUSDT".to_string(),
            "ADAUSDT".to_string(),
            "DOTUSDT".to_string(),
        ]
    }

    /// Get symbols organized by category
    pub fn get_symbols_by_category() -> HashMap<String, Vec<String>> {
        let mut categories = HashMap::new();

        categories.insert("Major".to_string(), vec![
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "BNBUSDT".to_string(),
            "ADAUSDT".to_string(),
            "XRPUSDT".to_string(),
            "SOLUSDT".to_string(),
            "DOTUSDT".to_string(),
            "DOGEUSDT".to_string(),
            "AVAXUSDT".to_string(),
            "MATICUSDT".to_string(),
            "LTCUSDT".to_string(),
            "ATOMUSDT".to_string(),
            "LINKUSDT".to_string(),
            "BCHUSDT".to_string(),
        ]);

        categories.insert("DeFi".to_string(), vec![
            "UNIUSDT".to_string(),
            "AAVEUSDT".to_string(),
            "MKRUSDT".to_string(),
            "COMPUSDT".to_string(),
            "SUSHIUSDT".to_string(),
            "CRVUSDT".to_string(),
            "1INCHUSDT".to_string(),
            "YFIUSDT".to_string(),
            "SNXUSDT".to_string(),
            "BALUSDT".to_string(),
        ]);

        categories.insert("Layer2".to_string(), vec![
            "ARBUSDT".to_string(),
            "OPUSDT".to_string(),
            "MASKUSDT".to_string(),
            "IMXUSDT".to_string(),
            "METISUSDT".to_string(),
        ]);

        categories.insert("Gaming".to_string(), vec![
            "AXSUSDT".to_string(),
            "SANDUSDT".to_string(),
            "MANAUSDT".to_string(),
            "ENJUSDT".to_string(),
            "GALAUSDT".to_string(),
            "CHZUSDT".to_string(),
            "FLOWUSDT".to_string(),
            "APECOINUSDT".to_string(),
        ]);

        categories.insert("AI".to_string(), vec![
            "FETUSDT".to_string(),
            "OCEANUSDT".to_string(),
            "AGIXUSDT".to_string(),
            "RNDRУСDT".to_string(),
        ]);

        categories.insert("Meme".to_string(), vec![
            "SHIBUSDT".to_string(),
            "PEPEUSDT".to_string(),
            "FLOKIUSDT".to_string(),
            "BONKUSDT".to_string(),
            "1000RATSUSDT".to_string(),
            "DOGEUSDT".to_string(),
        ]);

        categories.insert("Infrastructure".to_string(), vec![
            "ICPUSDT".to_string(),
            "THETAUSDT".to_string(),
            "VETUSDT".to_string(),
            "IOTAUSDT".to_string(),
            "HBARUSDT".to_string(),
            "EGLDUSDT".to_string(),
            "FTMUSDT".to_string(),
        ]);

        categories.insert("New".to_string(), vec![
            "SUIUSDT".to_string(),
            "APTUSDT".to_string(),
            "LDOUSDT".to_string(),
            "INJUSDT".to_string(),
            "STXUSDT".to_string(),
            "TIAUSDT".to_string(),
            "SEIUSDT".to_string(),
            "JUPUSDT".to_string(),
            "WIFUSDT".to_string(),
            "TNSRUSDT".to_string(),
        ]);

        categories
    }

    /// Get high-volume symbols (most liquid)
    pub fn get_high_volume_symbols() -> Vec<String> {
        vec![
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "BNBUSDT".to_string(),
            "SOLUSDT".to_string(),
            "XRPUSDT".to_string(),
            "DOGEUSDT".to_string(),
            "ADAUSDT".to_string(),
            "AVAXUSDT".to_string(),
            "DOTUSDT".to_string(),
            "MATICUSDT".to_string(),
            "LINKUSDT".to_string(),
            "LTCUSDT".to_string(),
            "UNIUSDT".to_string(),
            "ATOMUSDT".to_string(),
            "NEARUSDT".to_string(),
            "FILUSDT".to_string(),
            "ETCUSDT".to_string(),
            "BCHUSDT".to_string(),
            "AAVEUSDT".to_string(),
            "ARBUSDT".to_string(),
        ]
    }

    /// Get default symbols for quick start
    pub fn get_default_symbols() -> Vec<String> {
        vec![
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "BNBUSDT".to_string(),
            "SOLUSDT".to_string(),
            "XRPUSDT".to_string(),
            "ADAUSDT".to_string(),
            "DOGEUSDT".to_string(),
            "AVAXUSDT".to_string(),
            "DOTUSDT".to_string(),
            "MATICUSDT".to_string(),
            "LINKUSDT".to_string(),
            "LTCUSDT".to_string(),
            "ATOMUSDT".to_string(),
            "UNIUSDT".to_string(),
            "AAVEUSDT".to_string(),
            "NEARUSDT".to_string(),
            "SHIBUSDT".to_string(),
            "TRXUSDT".to_string(),
            "BCHUSDT".to_string(),
            "ETCUSDT".to_string(),
        ]
    }

    /// Check if a symbol is supported
    pub fn is_supported_symbol(symbol: &str) -> bool {
        Self::get_all_symbols().contains(&symbol.to_string())
    }

    /// Get tick size for a symbol (used for price precision)
    pub fn get_tick_size(symbol: &str) -> f64 {
        match symbol {
            // High-priced coins (BTC, ETH, BNB)
            "BTCUSDT" => 0.1,
            "ETHUSDT" => 0.01,
            "BNBUSDT" => 0.01,

            // Mid-priced coins ($1-100)
            "ADAUSDT" | "XRPUSDT" | "SOLUSDT" | "DOTUSDT" | "AVAXUSDT" |
            "MATICUSDT" | "LTCUSDT" | "ATOMUSDT" | "LINKUSDT" | "UNIUSDT" |
            "AAVEUSDT" | "NEARUSDT" | "FILUSDT" | "BCHUSDT" | "ETCUSDT" => 0.0001,

            // Low-priced coins ($0.001-1)
            "DOGEUSDT" | "SHIBUSDT" | "TRXUSDT" | "ALGOUSDT" => 0.00001,

            // Very low-priced coins
            "1000SHIBUSDT" | "1000FLOKIUSDT" | "1000RATSUSDT" | "1000LUNCUSDT" | "1000XECUSDT" => 0.000001,

            // Default for unknown symbols
            _ => 0.0001,
        }
    }

    /// Get minimum order quantity for a symbol
    pub fn get_min_quantity(symbol: &str) -> f64 {
        match symbol {
            "BTCUSDT" => 0.001,
            "ETHUSDT" => 0.001,
            "BNBUSDT" => 0.01,
            _ => 0.1, // Default minimum quantity
        }
    }
}