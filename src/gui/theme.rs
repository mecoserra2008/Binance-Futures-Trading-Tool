use egui::{Color32, Visuals, Style, Stroke, Rounding};

pub struct ScreenerTheme;

impl ScreenerTheme {
    // Color palette
    pub const BACKGROUND: Color32 = Color32::from_rgb(30, 30, 30);           // #1e1e1e
    pub const PANEL_BACKGROUND: Color32 = Color32::from_rgb(35, 35, 35);      // #232323
    pub const SURFACE: Color32 = Color32::from_rgb(40, 40, 40);               // #282828
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(224, 224, 224);       // #e0e0e0
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(180, 180, 180);     // #b4b4b4
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(140, 140, 140);         // #8c8c8c
    
    // Trading colors
    pub const BUY_COLOR: Color32 = Color32::from_rgb(0, 255, 136);            // #00ff88
    pub const SELL_COLOR: Color32 = Color32::from_rgb(255, 68, 68);           // #ff4444
    pub const NEUTRAL_COLOR: Color32 = Color32::from_rgb(255, 170, 0);        // #ffaa00
    
    // Accent colors
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(100, 181, 246);        // #64b5f6
    pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(156, 39, 176);       // #9c27b0
    pub const WARNING: Color32 = Color32::from_rgb(255, 193, 7);              // #ffc107
    pub const ERROR: Color32 = Color32::from_rgb(244, 67, 54);                // #f44336
    
    // Grid and border colors
    pub const GRID_COLOR: Color32 = Color32::from_rgb(60, 60, 60);            // #3c3c3c
    pub const BORDER_COLOR: Color32 = Color32::from_rgb(80, 80, 80);          // #505050
    pub const SEPARATOR_COLOR: Color32 = Color32::from_rgb(70, 70, 70);       // #464646

    pub fn apply_dark_theme(ctx: &egui::Context) {
        let mut style = Style::default();
        
        // Set up dark visuals
        let mut visuals = Visuals::dark();
        
        // Background colors
        visuals.panel_fill = Self::PANEL_BACKGROUND;
        visuals.window_fill = Self::BACKGROUND;
        visuals.extreme_bg_color = Self::SURFACE;
        visuals.faint_bg_color = Self::SURFACE;
        
        // Text colors (note: these fields may vary by egui version)
        // visuals.text_color = Self::TEXT_PRIMARY;
        // visuals.weak_text_color = Self::TEXT_SECONDARY;
        
        // Widget colors
        visuals.widgets.noninteractive.bg_fill = Self::SURFACE;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Self::TEXT_SECONDARY);
        
        visuals.widgets.inactive.bg_fill = Self::SURFACE;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Self::TEXT_SECONDARY);
        
        visuals.widgets.hovered.bg_fill = Self::ACCENT_BLUE.gamma_multiply(0.3);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Self::TEXT_PRIMARY);
        
        visuals.widgets.active.bg_fill = Self::ACCENT_BLUE.gamma_multiply(0.5);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, Self::TEXT_PRIMARY);
        
        visuals.widgets.open.bg_fill = Self::ACCENT_BLUE.gamma_multiply(0.2);
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, Self::TEXT_PRIMARY);
        
        // Selection colors
        visuals.selection.bg_fill = Self::ACCENT_BLUE.gamma_multiply(0.4);
        visuals.selection.stroke = Stroke::new(1.0, Self::ACCENT_BLUE);
        
        // Hyperlink color
        visuals.hyperlink_color = Self::ACCENT_BLUE;
        
        // Window and panel styling
        visuals.window_rounding = Rounding::same(8.0);
        // visuals.panel_border = Stroke::new(1.0, Self::BORDER_COLOR); // Field not available in this version
        
        // Scrollbar styling
        visuals.widgets.noninteractive.rounding = Rounding::same(4.0);
        visuals.widgets.inactive.rounding = Rounding::same(4.0);
        visuals.widgets.hovered.rounding = Rounding::same(4.0);
        visuals.widgets.active.rounding = Rounding::same(4.0);
        
        style.visuals = visuals;
        
        // Spacing and sizing
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.menu_margin = egui::Margin::same(8.0);
        style.spacing.indent = 20.0;
        // Note: scroll_bar_width not available in this egui version
        // style.spacing.scroll_bar_width = 12.0;
        
        ctx.set_style(style);
    }

    pub fn get_volume_color(buy_volume: f64, sell_volume: f64) -> Color32 {
        if buy_volume > sell_volume {
            Self::BUY_COLOR
        } else if sell_volume > buy_volume {
            Self::SELL_COLOR
        } else {
            Self::NEUTRAL_COLOR
        }
    }

    pub fn get_imbalance_color(imbalance_ratio: f64) -> Color32 {
        let abs_ratio = imbalance_ratio.abs();
        if imbalance_ratio > 0.0 {
            // Positive imbalance (more buys)
            Self::BUY_COLOR.gamma_multiply(abs_ratio.min(1.0) as f32)
        } else {
            // Negative imbalance (more sells)
            Self::SELL_COLOR.gamma_multiply(abs_ratio.min(1.0) as f32)
        }
    }

    pub fn get_price_change_color(change: f64) -> Color32 {
        if change > 0.0 {
            Self::BUY_COLOR
        } else if change < 0.0 {
            Self::SELL_COLOR
        } else {
            Self::TEXT_PRIMARY
        }
    }

    pub fn get_liquidation_color(side: &str) -> Color32 {
        match side.to_uppercase().as_str() {
            "LONG" => Self::SELL_COLOR, // Long liquidations are bearish
            "SHORT" => Self::BUY_COLOR, // Short liquidations are bullish
            _ => Self::NEUTRAL_COLOR,
        }
    }

    pub fn get_volume_intensity_color(intensity: f64) -> Color32 {
        // intensity should be between 0.0 and 1.0
        let clamped_intensity = intensity.clamp(0.0, 1.0) as f32;
        
        if clamped_intensity < 0.3 {
            Self::TEXT_MUTED
        } else if clamped_intensity < 0.6 {
            Self::NEUTRAL_COLOR.gamma_multiply(clamped_intensity)
        } else if clamped_intensity < 0.8 {
            Self::WARNING.gamma_multiply(clamped_intensity)
        } else {
            Self::ERROR.gamma_multiply(clamped_intensity)
        }
    }

    pub fn table_header_style() -> egui::style::WidgetVisuals {
        egui::style::WidgetVisuals {
            bg_fill: Self::SURFACE,
            fg_stroke: Stroke::new(1.0, Self::TEXT_PRIMARY),
            rounding: Rounding::same(4.0),
            expansion: 0.0,
            weak_bg_fill: Self::SURFACE,
            bg_stroke: Stroke::new(1.0, Self::BORDER_COLOR),
        }
    }

    pub fn table_row_style(is_even: bool) -> egui::style::WidgetVisuals {
        let bg_color = if is_even {
            Self::BACKGROUND
        } else {
            Self::PANEL_BACKGROUND
        };

        egui::style::WidgetVisuals {
            bg_fill: bg_color,
            fg_stroke: Stroke::new(1.0, Self::TEXT_PRIMARY),
            rounding: Rounding::same(2.0),
            expansion: 0.0,
            weak_bg_fill: bg_color,
            bg_stroke: Stroke::new(0.5, Self::GRID_COLOR),
        }
    }

    pub fn format_volume(volume: f64) -> String {
        if volume >= 1_000_000.0 {
            format!("{:.1}M", volume / 1_000_000.0)
        } else if volume >= 1_000.0 {
            format!("{:.1}K", volume / 1_000.0)
        } else {
            format!("{:.2}", volume)
        }
    }

    pub fn format_price(price: f64, precision: u32) -> String {
        format!("{:.prec$}", price, prec = precision as usize)
    }

    pub fn format_percentage(percentage: f64) -> String {
        format!("{:+.2}%", percentage)
    }

    pub fn format_currency(amount: f64) -> String {
        if amount >= 1_000_000.0 {
            format!("${:.2}M", amount / 1_000_000.0)
        } else if amount >= 1_000.0 {
            format!("${:.1}K", amount / 1_000.0)
        } else {
            format!("${:.2}", amount)
        }
    }

    pub fn format_delta(delta: f64) -> String {
        if delta >= 0.0 {
            format!("+{}", Self::format_volume(delta))
        } else {
            format!("-{}", Self::format_volume(delta.abs()))
        }
    }

    pub fn format_timestamp(timestamp: u64) -> String {
        let dt = chrono::DateTime::from_timestamp((timestamp / 1000) as i64, 0)
            .unwrap_or_else(|| chrono::Utc::now());
        dt.format("%H:%M:%S").to_string()
    }

    pub fn format_imbalance_ratio(ratio: f64) -> String {
        format!("{:+.2}", ratio)
    }
}

// Custom widget helpers
pub struct VolumeBar;

impl VolumeBar {
    pub fn show(
        ui: &mut egui::Ui,
        buy_volume: f64,
        sell_volume: f64,
        width: f32,
        height: f32,
    ) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(width, height),
            egui::Sense::hover(),
        );

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let total_volume = buy_volume + sell_volume;

            if total_volume > 0.0 {
                let buy_ratio = buy_volume / total_volume;
                let buy_width = width * buy_ratio as f32;

                // Draw buy volume (left side)
                if buy_width > 0.0 {
                    painter.rect_filled(
                        egui::Rect::from_min_size(rect.min, egui::vec2(buy_width, height)),
                        Rounding::same(2.0),
                        ScreenerTheme::BUY_COLOR.gamma_multiply(0.8),
                    );
                }

                // Draw sell volume (right side)
                let sell_width = width - buy_width;
                if sell_width > 0.0 {
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(rect.min.x + buy_width, rect.min.y),
                            egui::vec2(sell_width, height),
                        ),
                        Rounding::same(2.0),
                        ScreenerTheme::SELL_COLOR.gamma_multiply(0.8),
                    );
                }

                // Draw border
                painter.rect_stroke(
                    rect,
                    Rounding::same(2.0),
                    Stroke::new(1.0, ScreenerTheme::BORDER_COLOR),
                );
            } else {
                // Empty bar
                painter.rect_stroke(
                    rect,
                    Rounding::same(2.0),
                    Stroke::new(1.0, ScreenerTheme::GRID_COLOR),
                );
            }
        }

        response
    }
}

pub struct ImbalanceIndicator;

impl ImbalanceIndicator {
    pub fn show(
        ui: &mut egui::Ui,
        imbalance_ratio: f64,
        width: f32,
        height: f32,
    ) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(width, height),
            egui::Sense::hover(),
        );

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let center_x = rect.center().x;
            let abs_ratio = imbalance_ratio.abs().min(1.0);
            let bar_width = (width * 0.4 * abs_ratio as f32).max(2.0);

            let color = ScreenerTheme::get_imbalance_color(imbalance_ratio);

            if imbalance_ratio > 0.0 {
                // Positive imbalance (buy side)
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(center_x, rect.min.y),
                        egui::vec2(bar_width, height),
                    ),
                    Rounding::same(1.0),
                    color,
                );
            } else if imbalance_ratio < 0.0 {
                // Negative imbalance (sell side)
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(center_x - bar_width, rect.min.y),
                        egui::vec2(bar_width, height),
                    ),
                    Rounding::same(1.0),
                    color,
                );
            }

            // Center line
            painter.line_segment(
                [
                    egui::pos2(center_x, rect.min.y),
                    egui::pos2(center_x, rect.max.y),
                ],
                Stroke::new(1.0, ScreenerTheme::GRID_COLOR),
            );

            // Border
            painter.rect_stroke(
                rect,
                Rounding::same(2.0),
                Stroke::new(1.0, ScreenerTheme::BORDER_COLOR),
            );
        }

        response
    }
}