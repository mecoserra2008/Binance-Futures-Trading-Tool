/// Mathematical utilities for financial calculations

pub fn calculate_percentage_change(old_value: f64, new_value: f64) -> f64 {
    if old_value == 0.0 {
        0.0
    } else {
        ((new_value - old_value) / old_value) * 100.0
    }
}

pub fn calculate_vwap(prices: &[f64], volumes: &[f64]) -> f64 {
    if prices.len() != volumes.len() || prices.is_empty() {
        return 0.0;
    }

    let total_volume: f64 = volumes.iter().sum();
    if total_volume == 0.0 {
        return 0.0;
    }

    let volume_weighted_sum: f64 = prices.iter()
        .zip(volumes.iter())
        .map(|(price, volume)| price * volume)
        .sum();

    volume_weighted_sum / total_volume
}

pub fn calculate_standard_deviation(values: &[f64]) -> f64 {
    if values.len() <= 1 {
        return 0.0;
    }

    let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
    let variance: f64 = values.iter()
        .map(|value| {
            let diff = value - mean;
            diff * diff
        })
        .sum::<f64>() / (values.len() - 1) as f64;

    variance.sqrt()
}

pub fn calculate_ema(values: &[f64], period: usize) -> Vec<f64> {
    if values.is_empty() || period == 0 {
        return Vec::new();
    }

    let alpha = 2.0 / (period + 1) as f64;
    let mut ema = Vec::with_capacity(values.len());
    
    // Initialize with first value
    ema.push(values[0]);
    
    for i in 1..values.len() {
        let new_ema = alpha * values[i] + (1.0 - alpha) * ema[i - 1];
        ema.push(new_ema);
    }
    
    ema
}

pub fn calculate_sma(values: &[f64], period: usize) -> Vec<f64> {
    if values.len() < period || period == 0 {
        return Vec::new();
    }

    let mut sma = Vec::new();
    
    for i in period - 1..values.len() {
        let sum: f64 = values[i - period + 1..=i].iter().sum();
        sma.push(sum / period as f64);
    }
    
    sma
}

pub fn normalize_price(price: f64, tick_size: f64) -> f64 {
    if tick_size <= 0.0 {
        return price;
    }
    
    (price / tick_size).round() * tick_size
}

pub fn calculate_order_imbalance(bid_volume: f64, ask_volume: f64) -> f64 {
    let total = bid_volume + ask_volume;
    if total == 0.0 {
        0.0
    } else {
        (bid_volume - ask_volume) / total
    }
}

pub fn calculate_volume_delta(buy_volume: f64, sell_volume: f64) -> f64 {
    buy_volume - sell_volume
}

pub fn calculate_relative_strength(current_value: f64, baseline_value: f64) -> f64 {
    if baseline_value == 0.0 {
        0.0
    } else {
        current_value / baseline_value
    }
}

pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

pub fn lerp(start: f64, end: f64, t: f64) -> f64 {
    start + t * (end - start)
}