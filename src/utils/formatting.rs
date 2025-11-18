/// Formatting utilities for display

use chrono::{DateTime, Utc};

pub fn format_large_number(number: f64) -> String {
    if number.abs() >= 1_000_000_000.0 {
        format!("{:.2}B", number / 1_000_000_000.0)
    } else if number.abs() >= 1_000_000.0 {
        format!("{:.2}M", number / 1_000_000.0)
    } else if number.abs() >= 1_000.0 {
        format!("{:.1}K", number / 1_000.0)
    } else {
        format!("{:.2}", number)
    }
}

pub fn format_price_with_precision(price: f64, precision: u32) -> String {
    format!("{:.prec$}", price, prec = precision as usize)
}

pub fn format_percentage_with_sign(percentage: f64) -> String {
    if percentage >= 0.0 {
        format!("+{:.2}%", percentage)
    } else {
        format!("{:.2}%", percentage)
    }
}

pub fn format_duration_from_ms(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    if days > 0 {
        format!("{}d {}h", days, hours % 24)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes % 60)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds % 60)
    } else {
        format!("{}s", seconds)
    }
}

pub fn format_timestamp_detailed(timestamp: u64) -> String {
    let dt = DateTime::from_timestamp((timestamp / 1000) as i64, 0)
        .unwrap_or_else(|| Utc::now());
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn format_time_ago(timestamp: u64) -> String {
    let now = Utc::now().timestamp() as u64 * 1000;
    let elapsed = now.saturating_sub(timestamp);
    
    if elapsed < 60_000 {
        format!("{}s ago", elapsed / 1000)
    } else if elapsed < 3_600_000 {
        format!("{}m ago", elapsed / 60_000)
    } else if elapsed < 86_400_000 {
        format!("{}h ago", elapsed / 3_600_000)
    } else {
        format!("{}d ago", elapsed / 86_400_000)
    }
}

pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

pub fn truncate_string(s: &str, max_length: usize) -> String {
    if s.len() <= max_length {
        s.to_string()
    } else {
        format!("{}...", &s[..max_length.saturating_sub(3)])
    }
}

pub fn capitalize_first_letter(s: &str) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    if !chars.is_empty() {
        chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
    }
    chars.iter().collect()
}