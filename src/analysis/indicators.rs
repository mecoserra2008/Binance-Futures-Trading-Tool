use crate::data::market_data::Candle;

/// Trait for all technical indicators
pub trait Indicator {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64>;
    fn get_name(&self) -> &str;
}

/// Simple Moving Average (SMA)
#[derive(Debug, Clone)]
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
        let mut result = Vec::with_capacity(candles.len());

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

/// Exponential Moving Average (EMA)
#[derive(Debug, Clone)]
pub struct ExponentialMovingAverage {
    period: usize,
}

impl ExponentialMovingAverage {
    pub fn new(period: usize) -> Self {
        Self { period }
    }

    fn calculate_ema(&self, candles: &[Candle]) -> Vec<f64> {
        let mut result = Vec::with_capacity(candles.len());
        let multiplier = 2.0 / (self.period as f64 + 1.0);

        if candles.len() < self.period {
            return vec![f64::NAN; candles.len()];
        }

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
}

impl Indicator for ExponentialMovingAverage {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64> {
        self.calculate_ema(candles)
    }

    fn get_name(&self) -> &str {
        "EMA"
    }
}

/// Bollinger Bands
#[derive(Debug, Clone)]
pub struct BollingerBands {
    period: usize,
    std_dev: f64,
}

#[derive(Debug, Clone)]
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

        let mut upper = Vec::with_capacity(candles.len());
        let mut lower = Vec::with_capacity(candles.len());

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

/// Relative Strength Index (RSI)
#[derive(Debug, Clone)]
pub struct RSI {
    period: usize,
}

impl RSI {
    pub fn new(period: usize) -> Self {
        Self { period }
    }

    pub fn calculate(&self, candles: &[Candle]) -> Vec<f64> {
        let mut result = Vec::with_capacity(candles.len());

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

        result.push(f64::NAN); // First candle has no RSI

        for i in 0..candles.len() - 1 {
            if i < self.period {
                result.push(f64::NAN);
            } else {
                if i > self.period {
                    // Subsequent RSI using smoothed averages
                    avg_gain = ((avg_gain * (self.period - 1) as f64) + gains[i]) / self.period as f64;
                    avg_loss = ((avg_loss * (self.period - 1) as f64) + losses[i]) / self.period as f64;
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

impl Indicator for RSI {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64> {
        self.calculate(candles)
    }

    fn get_name(&self) -> &str {
        "RSI"
    }
}

/// Moving Average Convergence Divergence (MACD)
#[derive(Debug, Clone)]
pub struct MACD {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
}

#[derive(Debug, Clone)]
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

        let mut macd_line = Vec::with_capacity(candles.len());
        for i in 0..candles.len() {
            macd_line.push(fast_ema[i] - slow_ema[i]);
        }

        // Signal line is EMA of MACD line
        let signal_line = self.calculate_signal_line(&macd_line);

        let mut histogram = Vec::with_capacity(candles.len());
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
        let mut result = Vec::with_capacity(macd_values.len());
        let multiplier = 2.0 / (self.signal_period as f64 + 1.0);

        let valid_start = macd_values.iter()
            .position(|&v| !v.is_nan())
            .unwrap_or(0);

        if valid_start + self.signal_period > macd_values.len() {
            return vec![f64::NAN; macd_values.len()];
        }

        // First signal = SMA of MACD
        let first_sma: f64 = macd_values[valid_start..valid_start + self.signal_period]
            .iter()
            .sum::<f64>() / self.signal_period as f64;

        for i in 0..macd_values.len() {
            if i < valid_start + self.signal_period - 1 {
                result.push(f64::NAN);
            } else if i == valid_start + self.signal_period - 1 {
                result.push(first_sma);
            } else {
                let signal = (macd_values[i] - result[i - 1]) * multiplier + result[i - 1];
                result.push(signal);
            }
        }

        result
    }
}

/// Weighted Moving Average (WMA)
#[derive(Debug, Clone)]
pub struct WeightedMovingAverage {
    period: usize,
}

impl WeightedMovingAverage {
    pub fn new(period: usize) -> Self {
        Self { period }
    }
}

impl Indicator for WeightedMovingAverage {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64> {
        let mut result = Vec::with_capacity(candles.len());
        let weight_sum: usize = (1..=self.period).sum();

        for i in 0..candles.len() {
            if i < self.period - 1 {
                result.push(f64::NAN);
            } else {
                let weighted_sum: f64 = candles[i - self.period + 1..=i]
                    .iter()
                    .enumerate()
                    .map(|(j, c)| c.close * (j + 1) as f64)
                    .sum();
                result.push(weighted_sum / weight_sum as f64);
            }
        }

        result
    }

    fn get_name(&self) -> &str {
        "WMA"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_candles() -> Vec<Candle> {
        vec![
            Candle { timestamp: 0, open: 100.0, high: 105.0, low: 99.0, close: 102.0, volume: 1000.0 },
            Candle { timestamp: 1, open: 102.0, high: 106.0, low: 101.0, close: 104.0, volume: 1100.0 },
            Candle { timestamp: 2, open: 104.0, high: 107.0, low: 103.0, close: 105.0, volume: 1200.0 },
            Candle { timestamp: 3, open: 105.0, high: 108.0, low: 104.0, close: 106.0, volume: 1300.0 },
            Candle { timestamp: 4, open: 106.0, high: 109.0, low: 105.0, close: 107.0, volume: 1400.0 },
        ]
    }

    #[test]
    fn test_sma() {
        let candles = create_test_candles();
        let sma = SimpleMovingAverage::new(3);
        let result = sma.calculate(&candles);

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 103.666).abs() < 0.01); // (102+104+105)/3
    }

    #[test]
    fn test_ema() {
        let candles = create_test_candles();
        let ema = ExponentialMovingAverage::new(3);
        let result = ema.calculate(&candles);

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(!result[2].is_nan());
    }

    #[test]
    fn test_bollinger_bands() {
        let candles = create_test_candles();
        let bb = BollingerBands::new(3, 2.0);
        let result = bb.calculate(&candles);

        assert_eq!(result.upper.len(), 5);
        assert_eq!(result.middle.len(), 5);
        assert_eq!(result.lower.len(), 5);

        // First two should be NAN
        assert!(result.upper[0].is_nan());
        assert!(result.middle[1].is_nan());

        // Upper should be greater than middle, middle greater than lower
        assert!(result.upper[2] > result.middle[2]);
        assert!(result.middle[2] > result.lower[2]);
    }

    #[test]
    fn test_rsi() {
        let candles = create_test_candles();
        let rsi = RSI::new(3);
        let result = rsi.calculate(&candles);

        assert_eq!(result.len(), 5);
        // RSI should be between 0 and 100
        for (i, &val) in result.iter().enumerate() {
            if !val.is_nan() {
                assert!(val >= 0.0 && val <= 100.0, "RSI[{}] = {} out of range", i, val);
            }
        }
    }

    #[test]
    fn test_macd() {
        let candles = create_test_candles();
        let macd = MACD::new(2, 3, 2);
        let result = macd.calculate(&candles);

        assert_eq!(result.macd_line.len(), 5);
        assert_eq!(result.signal_line.len(), 5);
        assert_eq!(result.histogram.len(), 5);
    }

    #[test]
    fn test_wma() {
        let candles = create_test_candles();
        let wma = WeightedMovingAverage::new(3);
        let result = wma.calculate(&candles);

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(!result[2].is_nan());

        // WMA should weight recent prices more heavily
        // So it should be closer to the most recent price than SMA
    }
}
