pub struct LayoutButton<'a> {
    text: &'a str,
    text_color: Option<egui::Color32>,
    dark_mode: Option<bool>,
}

impl<'a> LayoutButton<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            text_color: None,
            dark_mode: None,
        }
    }

    pub fn dark_mode(mut self, dark_mode: Option<bool>) -> Self {
        self.dark_mode = dark_mode;
        self
    }

    // pub fn text_color(mut self, olor: egui::Color32) -> Self {
    //     self.text_color.replace(color);
    //     self
    // }

    pub fn text_color_opt(mut self, color: Option<egui::Color32>) -> Self {
        self.text_color = color;
        self
    }
}

impl<'a> egui::Widget for LayoutButton<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        const RADIUS: f32 = 4.0;
        const MIN_SIZE: egui::Vec2 = egui::vec2(28.0, 28.0);
        const TEXT_PADDING: egui::Vec2 = egui::vec2(16.0, 8.0);

        let dark_mode = self.dark_mode.unwrap_or_else(|| ui.visuals().dark_mode);

        let font_id = egui::FontId::default();
        let text_color = self.text_color.unwrap_or(if dark_mode {
            egui::Color32::WHITE
        } else {
            egui::Color32::BLACK
        });

        let text = self.text.to_string();
        let text_galley = ui
            .painter()
            .layout_no_wrap(text, font_id.clone(), text_color);

        let size = MIN_SIZE.max(text_galley.rect.size() + TEXT_PADDING);

        let (rect, response) = ui.allocate_at_least(size, egui::Sense::CLICK | egui::Sense::HOVER);

        let painter = ui.painter();

        // draw background
        if response.hovered() {
            let color = if dark_mode {
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 1)
            } else {
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30)
            };

            let stroke_color = if dark_mode {
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 2)
            } else {
                egui::Color32::from_rgba_unmultiplied(33, 33, 33, 33)
            };

            let stroke = egui::Stroke {
                width: 1.0,
                color: stroke_color,
            };

            painter.rect(rect, RADIUS, color, stroke, egui::StrokeKind::Inside);
        }

        // draw text
        let text_color = if response.hovered() {
            text_color
        } else {
            text_color.gamma_multiply(0.75)
        };

        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            self.text,
            font_id,
            text_color,
        );

        response
    }
}
