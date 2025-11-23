use egui::Color32;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HeatmapColorScheme {
    pub bid_color_low: [u8; 4],
    pub bid_color_high: [u8; 4],
    pub ask_color_low: [u8; 4],
    pub ask_color_high: [u8; 4],
    pub intensity: f32,  // 0.0 to 1.0 (user-controllable)
}

impl Default for HeatmapColorScheme {
    fn default() -> Self {
        Self::green_red_default()
    }
}

impl HeatmapColorScheme {
    /// Default green/red color scheme
    pub fn green_red_default() -> Self {
        Self {
            bid_color_low: [0, 100, 0, 20],      // Transparent green
            bid_color_high: [0, 255, 100, 180],  // Bright green
            ask_color_low: [100, 0, 0, 20],      // Transparent red
            ask_color_high: [255, 50, 50, 180],  // Bright red
            intensity: 0.7,  // Default 70% intensity
        }
    }

    /// Blue/orange color scheme
    pub fn blue_orange() -> Self {
        Self {
            bid_color_low: [0, 50, 100, 20],
            bid_color_high: [50, 150, 255, 180],
            ask_color_low: [100, 50, 0, 20],
            ask_color_high: [255, 150, 50, 180],
            intensity: 0.7,
        }
    }

    /// Monochrome grayscale
    pub fn monochrome() -> Self {
        Self {
            bid_color_low: [0, 0, 0, 20],
            bid_color_high: [150, 150, 150, 180],
            ask_color_low: [0, 0, 0, 20],
            ask_color_high: [150, 150, 150, 180],
            intensity: 0.7,
        }
    }

    /// Purple/yellow scheme
    pub fn purple_yellow() -> Self {
        Self {
            bid_color_low: [75, 0, 130, 20],     // Purple
            bid_color_high: [147, 112, 219, 180],
            ask_color_low: [139, 128, 0, 20],    // Yellow
            ask_color_high: [255, 215, 0, 180],
            intensity: 0.7,
        }
    }

    /// Get interpolated bid color based on volume percentage (0.0 to 1.0)
    pub fn get_bid_color(&self, volume_pct: f32) -> Color32 {
        self.interpolate_color(
            self.bid_color_low,
            self.bid_color_high,
            volume_pct * self.intensity
        )
    }

    /// Get interpolated ask color based on volume percentage (0.0 to 1.0)
    pub fn get_ask_color(&self, volume_pct: f32) -> Color32 {
        self.interpolate_color(
            self.ask_color_low,
            self.ask_color_high,
            volume_pct * self.intensity
        )
    }

    /// Linear interpolation between two colors
    fn interpolate_color(&self, color1: [u8; 4], color2: [u8; 4], t: f32) -> Color32 {
        let t = t.clamp(0.0, 1.0);
        let r = (color1[0] as f32 * (1.0 - t) + color2[0] as f32 * t) as u8;
        let g = (color1[1] as f32 * (1.0 - t) + color2[1] as f32 * t) as u8;
        let b = (color1[2] as f32 * (1.0 - t) + color2[2] as f32 * t) as u8;
        let a = (color1[3] as f32 * (1.0 - t) + color2[3] as f32 * t) as u8;
        Color32::from_rgba_premultiplied(r, g, b, a)
    }

    /// Get all available color schemes
    pub fn all_schemes() -> Vec<(&'static str, Self)> {
        vec![
            ("Green/Red", Self::green_red_default()),
            ("Blue/Orange", Self::blue_orange()),
            ("Monochrome", Self::monochrome()),
            ("Purple/Yellow", Self::purple_yellow()),
        ]
    }

    /// Get scheme by name
    pub fn from_name(name: &str) -> Self {
        match name {
            "Green/Red" => Self::green_red_default(),
            "Blue/Orange" => Self::blue_orange(),
            "Monochrome" => Self::monochrome(),
            "Purple/Yellow" => Self::purple_yellow(),
            _ => Self::default(),
        }
    }

    /// Get scheme name
    pub fn get_name(&self) -> &'static str {
        // Compare with known schemes
        if self.approx_eq(&Self::green_red_default()) {
            "Green/Red"
        } else if self.approx_eq(&Self::blue_orange()) {
            "Blue/Orange"
        } else if self.approx_eq(&Self::monochrome()) {
            "Monochrome"
        } else if self.approx_eq(&Self::purple_yellow()) {
            "Purple/Yellow"
        } else {
            "Custom"
        }
    }

    fn approx_eq(&self, other: &Self) -> bool {
        self.bid_color_low == other.bid_color_low &&
        self.bid_color_high == other.bid_color_high &&
        self.ask_color_low == other.ask_color_low &&
        self.ask_color_high == other.ask_color_high
    }

    /// Set intensity (0.0 to 1.0)
    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_interpolation() {
        let scheme = HeatmapColorScheme::default();

        // Test 0% - should be close to low color
        let color_0 = scheme.get_bid_color(0.0);
        assert_eq!(color_0.r(), 0);

        // Test 100% - should be close to high color (with intensity)
        let color_100 = scheme.get_bid_color(1.0);
        // With 70% intensity, should be between low and high

        // Test 50% - should be middle
        let color_50 = scheme.get_bid_color(0.5);
        assert!(color_50.r() > color_0.r());
        assert!(color_50.r() < 255);
    }

    #[test]
    fn test_intensity() {
        let mut scheme = HeatmapColorScheme::default();

        scheme.set_intensity(0.5);
        assert_eq!(scheme.intensity, 0.5);

        scheme.set_intensity(1.5);  // Should clamp to 1.0
        assert_eq!(scheme.intensity, 1.0);

        scheme.set_intensity(-0.5);  // Should clamp to 0.0
        assert_eq!(scheme.intensity, 0.0);
    }

    #[test]
    fn test_all_schemes() {
        let schemes = HeatmapColorScheme::all_schemes();
        assert_eq!(schemes.len(), 4);

        for (name, scheme) in schemes {
            assert!(!name.is_empty());
            assert!(scheme.intensity > 0.0);
        }
    }

    #[test]
    fn test_from_name() {
        let green_red = HeatmapColorScheme::from_name("Green/Red");
        assert_eq!(green_red.get_name(), "Green/Red");

        let blue_orange = HeatmapColorScheme::from_name("Blue/Orange");
        assert_eq!(blue_orange.get_name(), "Blue/Orange");

        let invalid = HeatmapColorScheme::from_name("Invalid");
        assert_eq!(invalid.get_name(), "Green/Red");  // Returns default
    }
}
