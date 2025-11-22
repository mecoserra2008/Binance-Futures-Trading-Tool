use egui::{Pos2, Color32, Stroke};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DrawingTool {
    TrendLine(TrendLine),
    HorizontalLine(HorizontalLine),
    VerticalLine(VerticalLine),
    FibonacciRetracement(FibonacciRetracement),
    Rectangle(Rectangle),
    Text(TextAnnotation),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendLine {
    pub id: String,
    pub start_time: u64,
    pub start_price: f64,
    pub end_time: u64,
    pub end_price: f64,
    pub color: [u8; 4],
    pub width: f32,
    pub style: LineStyle,
    pub extend_right: bool,
    pub extend_left: bool,
}

impl TrendLine {
    pub fn new(start_time: u64, start_price: f64) -> Self {
        Self {
            id: format!("trend_{}", chrono::Utc::now().timestamp_millis()),
            start_time,
            start_price,
            end_time: start_time,
            end_price: start_price,
            color: [255, 255, 0, 255], // Yellow
            width: 2.0,
            style: LineStyle::Solid,
            extend_right: false,
            extend_left: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizontalLine {
    pub id: String,
    pub price: f64,
    pub color: [u8; 4],
    pub width: f32,
    pub style: LineStyle,
}

impl HorizontalLine {
    pub fn new(price: f64) -> Self {
        Self {
            id: format!("hline_{}", chrono::Utc::now().timestamp_millis()),
            price,
            color: [100, 100, 100, 255], // Gray
            width: 1.5,
            style: LineStyle::Solid,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerticalLine {
    pub id: String,
    pub time: u64,
    pub color: [u8; 4],
    pub width: f32,
    pub style: LineStyle,
}

impl VerticalLine {
    pub fn new(time: u64) -> Self {
        Self {
            id: format!("vline_{}", chrono::Utc::now().timestamp_millis()),
            time,
            color: [100, 100, 100, 255], // Gray
            width: 1.5,
            style: LineStyle::Solid,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FibonacciRetracement {
    pub id: String,
    pub start_time: u64,
    pub start_price: f64,
    pub end_time: u64,
    pub end_price: f64,
    pub levels: Vec<f32>,  // 0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0
    pub show_labels: bool,
    pub color: [u8; 4],
}

impl FibonacciRetracement {
    pub fn new(start_time: u64, start_price: f64) -> Self {
        Self {
            id: format!("fib_{}", chrono::Utc::now().timestamp_millis()),
            start_time,
            start_price,
            end_time: start_time,
            end_price: start_price,
            levels: vec![0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0],
            show_labels: true,
            color: [138, 43, 226, 255], // BlueViolet
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rectangle {
    pub id: String,
    pub start_time: u64,
    pub start_price: f64,
    pub end_time: u64,
    pub end_price: f64,
    pub fill_color: [u8; 4],
    pub border_color: [u8; 4],
    pub border_width: f32,
}

impl Rectangle {
    pub fn new(start_time: u64, start_price: f64) -> Self {
        Self {
            id: format!("rect_{}", chrono::Utc::now().timestamp_millis()),
            start_time,
            start_price,
            end_time: start_time,
            end_price: start_price,
            fill_color: [100, 100, 255, 50], // Transparent blue
            border_color: [100, 100, 255, 255], // Blue
            border_width: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnnotation {
    pub id: String,
    pub time: u64,
    pub price: f64,
    pub text: String,
    pub font_size: f32,
    pub color: [u8; 4],
    pub background: bool,
}

impl TextAnnotation {
    pub fn new(time: u64, price: f64, text: String) -> Self {
        Self {
            id: format!("text_{}", chrono::Utc::now().timestamp_millis()),
            time,
            price,
            text,
            font_size: 14.0,
            color: [255, 255, 255, 255], // White
            background: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveTool {
    None,
    TrendLine,
    HorizontalLine,
    VerticalLine,
    FibonacciRetracement,
    Rectangle,
    Text,
    Delete,
}

pub struct DrawingToolsManager {
    pub tools: Vec<DrawingTool>,
    pub active_tool: ActiveTool,
    pub selected_tool_id: Option<String>,
    pub drawing_in_progress: Option<DrawingTool>,
}

impl DrawingToolsManager {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            active_tool: ActiveTool::None,
            selected_tool_id: None,
            drawing_in_progress: None,
        }
    }

    pub fn start_drawing(&mut self, tool: ActiveTool, time: u64, price: f64) {
        self.active_tool = tool;

        self.drawing_in_progress = match tool {
            ActiveTool::TrendLine => Some(DrawingTool::TrendLine(TrendLine::new(time, price))),
            ActiveTool::HorizontalLine => Some(DrawingTool::HorizontalLine(HorizontalLine::new(price))),
            ActiveTool::VerticalLine => Some(DrawingTool::VerticalLine(VerticalLine::new(time))),
            ActiveTool::FibonacciRetracement => Some(DrawingTool::FibonacciRetracement(FibonacciRetracement::new(time, price))),
            ActiveTool::Rectangle => Some(DrawingTool::Rectangle(Rectangle::new(time, price))),
            ActiveTool::Text => Some(DrawingTool::Text(TextAnnotation::new(time, price, "Text".to_string()))),
            _ => None,
        };
    }

    pub fn update_drawing(&mut self, time: u64, price: f64) {
        if let Some(tool) = &mut self.drawing_in_progress {
            match tool {
                DrawingTool::TrendLine(ref mut line) => {
                    line.end_time = time;
                    line.end_price = price;
                }
                DrawingTool::FibonacciRetracement(ref mut fib) => {
                    fib.end_time = time;
                    fib.end_price = price;
                }
                DrawingTool::Rectangle(ref mut rect) => {
                    rect.end_time = time;
                    rect.end_price = price;
                }
                _ => {}
            }
        }
    }

    pub fn finish_drawing(&mut self) {
        if let Some(tool) = self.drawing_in_progress.take() {
            self.tools.push(tool);
            self.active_tool = ActiveTool::None;
        }
    }

    pub fn cancel_drawing(&mut self) {
        self.drawing_in_progress = None;
        self.active_tool = ActiveTool::None;
    }

    pub fn delete_selected(&mut self) {
        if let Some(id) = &self.selected_tool_id {
            let id_to_delete = id.clone();
            self.tools.retain(|t| {
                let tool_id = match t {
                    DrawingTool::TrendLine(tool) => &tool.id,
                    DrawingTool::HorizontalLine(tool) => &tool.id,
                    DrawingTool::VerticalLine(tool) => &tool.id,
                    DrawingTool::FibonacciRetracement(tool) => &tool.id,
                    DrawingTool::Rectangle(tool) => &tool.id,
                    DrawingTool::Text(tool) => &tool.id,
                };
                tool_id != &id_to_delete
            });
            self.selected_tool_id = None;
        }
    }

    pub fn select_tool_at(&mut self, time: u64, price: f64, tolerance: f64) -> bool {
        // Find tool near the click position
        for tool in &self.tools {
            if self.is_tool_near(tool, time, price, tolerance) {
                self.selected_tool_id = Some(self.get_tool_id(tool).to_string());
                return true;
            }
        }
        self.selected_tool_id = None;
        false
    }

    fn is_tool_near(&self, tool: &DrawingTool, time: u64, price: f64, tolerance: f64) -> bool {
        match tool {
            DrawingTool::HorizontalLine(line) => {
                (line.price - price).abs() < tolerance
            }
            DrawingTool::VerticalLine(line) => {
                (line.time as i64 - time as i64).abs() < (tolerance * 60000.0) as i64
            }
            DrawingTool::Text(text) => {
                (text.price - price).abs() < tolerance &&
                (text.time as i64 - time as i64).abs() < (tolerance * 60000.0) as i64
            }
            _ => false, // More complex hit testing for lines and shapes
        }
    }

    fn get_tool_id<'a>(&self, tool: &'a DrawingTool) -> &'a str {
        match tool {
            DrawingTool::TrendLine(t) => &t.id,
            DrawingTool::HorizontalLine(h) => &h.id,
            DrawingTool::VerticalLine(v) => &v.id,
            DrawingTool::FibonacciRetracement(f) => &f.id,
            DrawingTool::Rectangle(r) => &r.id,
            DrawingTool::Text(t) => &t.id,
        }
    }

    pub fn render_tools(
        &self,
        ui: &mut egui::Ui,
        chart_rect: egui::Rect,
        price_to_screen: &impl Fn(f64) -> f32,
        time_to_screen: &impl Fn(u64) -> f32,
    ) {
        let painter = ui.painter();

        // Render completed tools
        for tool in &self.tools {
            let is_selected = self.selected_tool_id.as_ref()
                .map(|id| id == self.get_tool_id(tool))
                .unwrap_or(false);

            self.render_tool(painter, tool, price_to_screen, time_to_screen, chart_rect, is_selected);
        }

        // Render tool in progress
        if let Some(tool) = &self.drawing_in_progress {
            self.render_tool(painter, tool, price_to_screen, time_to_screen, chart_rect, false);
        }
    }

    fn render_tool(
        &self,
        painter: &egui::Painter,
        tool: &DrawingTool,
        price_to_screen: &impl Fn(f64) -> f32,
        time_to_screen: &impl Fn(u64) -> f32,
        chart_rect: egui::Rect,
        is_selected: bool,
    ) {
        match tool {
            DrawingTool::TrendLine(line) => {
                self.render_trend_line(painter, line, price_to_screen, time_to_screen, is_selected);
            }
            DrawingTool::HorizontalLine(line) => {
                self.render_horizontal_line(painter, line, price_to_screen, chart_rect, is_selected);
            }
            DrawingTool::VerticalLine(line) => {
                self.render_vertical_line(painter, line, time_to_screen, chart_rect, is_selected);
            }
            DrawingTool::FibonacciRetracement(fib) => {
                self.render_fibonacci(painter, fib, price_to_screen, time_to_screen, chart_rect, is_selected);
            }
            DrawingTool::Rectangle(rect) => {
                self.render_rectangle(painter, rect, price_to_screen, time_to_screen, is_selected);
            }
            DrawingTool::Text(text) => {
                self.render_text(painter, text, price_to_screen, time_to_screen, is_selected);
            }
        }
    }

    fn render_trend_line(
        &self,
        painter: &egui::Painter,
        line: &TrendLine,
        price_to_screen: &impl Fn(f64) -> f32,
        time_to_screen: &impl Fn(u64) -> f32,
        is_selected: bool,
    ) {
        let start_pos = Pos2::new(
            time_to_screen(line.start_time),
            price_to_screen(line.start_price),
        );
        let end_pos = Pos2::new(
            time_to_screen(line.end_time),
            price_to_screen(line.end_price),
        );

        let mut color = Color32::from_rgba_premultiplied(
            line.color[0],
            line.color[1],
            line.color[2],
            line.color[3],
        );

        let width = if is_selected { line.width + 1.0 } else { line.width };

        painter.line_segment([start_pos, end_pos], Stroke::new(width, color));

        if is_selected {
            // Draw selection handles
            painter.circle_filled(start_pos, 4.0, Color32::WHITE);
            painter.circle_filled(end_pos, 4.0, Color32::WHITE);
        }
    }

    fn render_horizontal_line(
        &self,
        painter: &egui::Painter,
        line: &HorizontalLine,
        price_to_screen: &impl Fn(f64) -> f32,
        chart_rect: egui::Rect,
        is_selected: bool,
    ) {
        let y = price_to_screen(line.price);
        let color = Color32::from_rgba_premultiplied(
            line.color[0],
            line.color[1],
            line.color[2],
            line.color[3],
        );

        let width = if is_selected { line.width + 1.0 } else { line.width };

        painter.line_segment(
            [Pos2::new(chart_rect.min.x, y), Pos2::new(chart_rect.max.x, y)],
            Stroke::new(width, color),
        );

        // Draw price label
        let text = format!("{:.2}", line.price);
        painter.text(
            Pos2::new(chart_rect.max.x - 60.0, y - 10.0),
            egui::Align2::LEFT_BOTTOM,
            text,
            egui::FontId::proportional(12.0),
            color,
        );
    }

    fn render_vertical_line(
        &self,
        painter: &egui::Painter,
        line: &VerticalLine,
        time_to_screen: &impl Fn(u64) -> f32,
        chart_rect: egui::Rect,
        is_selected: bool,
    ) {
        let x = time_to_screen(line.time);
        let color = Color32::from_rgba_premultiplied(
            line.color[0],
            line.color[1],
            line.color[2],
            line.color[3],
        );

        let width = if is_selected { line.width + 1.0 } else { line.width };

        painter.line_segment(
            [Pos2::new(x, chart_rect.min.y), Pos2::new(x, chart_rect.max.y)],
            Stroke::new(width, color),
        );
    }

    fn render_fibonacci(
        &self,
        painter: &egui::Painter,
        fib: &FibonacciRetracement,
        price_to_screen: &impl Fn(f64) -> f32,
        time_to_screen: &impl Fn(u64) -> f32,
        chart_rect: egui::Rect,
        is_selected: bool,
    ) {
        let price_range = fib.end_price - fib.start_price;
        let color = Color32::from_rgba_premultiplied(
            fib.color[0],
            fib.color[1],
            fib.color[2],
            fib.color[3],
        );

        for &level in &fib.levels {
            let price = fib.start_price + (price_range * level as f64);
            let y = price_to_screen(price);
            let x_start = time_to_screen(fib.start_time);
            let x_end = time_to_screen(fib.end_time);

            painter.line_segment(
                [Pos2::new(x_start, y), Pos2::new(x_end, y)],
                Stroke::new(1.0, color),
            );

            if fib.show_labels {
                let text = format!("{:.1}% ({:.2})", level * 100.0, price);
                painter.text(
                    Pos2::new(x_end + 5.0, y),
                    egui::Align2::LEFT_CENTER,
                    text,
                    egui::FontId::proportional(11.0),
                    color,
                );
            }
        }
    }

    fn render_rectangle(
        &self,
        painter: &egui::Painter,
        rect: &Rectangle,
        price_to_screen: &impl Fn(f64) -> f32,
        time_to_screen: &impl Fn(u64) -> f32,
        is_selected: bool,
    ) {
        let min_x = time_to_screen(rect.start_time.min(rect.end_time));
        let max_x = time_to_screen(rect.start_time.max(rect.end_time));
        let min_y = price_to_screen(rect.start_price.max(rect.end_price));
        let max_y = price_to_screen(rect.start_price.min(rect.end_price));

        let rect_shape = egui::Rect::from_min_max(
            Pos2::new(min_x, min_y),
            Pos2::new(max_x, max_y),
        );

        let fill_color = Color32::from_rgba_premultiplied(
            rect.fill_color[0],
            rect.fill_color[1],
            rect.fill_color[2],
            rect.fill_color[3],
        );

        let border_color = Color32::from_rgba_premultiplied(
            rect.border_color[0],
            rect.border_color[1],
            rect.border_color[2],
            rect.border_color[3],
        );

        painter.rect_filled(rect_shape, 0.0, fill_color);
        painter.rect_stroke(rect_shape, 0.0, Stroke::new(rect.border_width, border_color));
    }

    fn render_text(
        &self,
        painter: &egui::Painter,
        text: &TextAnnotation,
        price_to_screen: &impl Fn(f64) -> f32,
        time_to_screen: &impl Fn(u64) -> f32,
        is_selected: bool,
    ) {
        let pos = Pos2::new(
            time_to_screen(text.time),
            price_to_screen(text.price),
        );

        let color = Color32::from_rgba_premultiplied(
            text.color[0],
            text.color[1],
            text.color[2],
            text.color[3],
        );

        if text.background {
            let galley = painter.layout_no_wrap(
                text.text.clone(),
                egui::FontId::proportional(text.font_size),
                color,
            );
            let rect = egui::Rect::from_min_size(pos, galley.size());
            painter.rect_filled(rect.expand(4.0), 2.0, Color32::from_black_alpha(180));
        }

        painter.text(
            pos,
            egui::Align2::LEFT_TOP,
            &text.text,
            egui::FontId::proportional(text.font_size),
            color,
        );
    }

    pub fn clear_all(&mut self) {
        self.tools.clear();
        self.selected_tool_id = None;
        self.drawing_in_progress = None;
        self.active_tool = ActiveTool::None;
    }

    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

impl Default for DrawingToolsManager {
    fn default() -> Self {
        Self::new()
    }
}
