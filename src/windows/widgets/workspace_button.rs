use crate::komorebi::Workspace;

pub struct WorkspaceButton<'a> {
    workspace: &'a Workspace,
    text_color: Option<egui::Color32>,
    line_on_top: bool,
    line_focused_color: Option<egui::Color32>,
    dark_mode: Option<bool>,
}

impl<'a> WorkspaceButton<'a> {
    pub fn new(workspace: &'a Workspace) -> Self {
        Self {
            workspace,
            text_color: None,
            line_on_top: false,
            line_focused_color: None,
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

    pub fn line_on_top(mut self, line_on_top: bool) -> Self {
        self.line_on_top = line_on_top;
        self
    }

    // pub fn line_focused_color(mut self, color: egui::Color32) -> Self {
    //     self.line_focused_color.replace(color);
    //     self
    // }

    pub fn line_focused_color_opt(mut self, color: Option<egui::Color32>) -> Self {
        self.line_focused_color = color;
        self
    }
}

impl egui::Widget for WorkspaceButton<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        const RADIUS: f32 = 4.0;
        const MIN_SIZE: egui::Vec2 = egui::vec2(28.0, 28.0);
        const INDICATOR_FOCUSED_WIDTH: f32 = 14.0;
        const INDICATOR_BASE_WIDTH: f32 = 6.0;
        const INDICATOR_HEIGHT: f32 = 3.5;
        const TEXT_PADDING: egui::Vec2 = egui::vec2(16.0, 8.0);

        let dark_mode = self.dark_mode.unwrap_or_else(|| ui.visuals().dark_mode);

        let font_id = egui::FontId::default();
        let text_color = self.text_color.unwrap_or(if dark_mode {
            egui::Color32::WHITE
        } else {
            egui::Color32::BLACK
        });

        let text = self.workspace.name.clone();
        let text_galley = ui
            .painter()
            .layout_no_wrap(text, font_id.clone(), text_color);

        let size = MIN_SIZE.max(text_galley.rect.size() + TEXT_PADDING);

        let (rect, response) = ui.allocate_at_least(size, egui::Sense::CLICK | egui::Sense::HOVER);

        let painter = ui.painter();

        // draw background
        if response.hovered() || self.workspace.focused {
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

        // draw indicator

        // animate opacity
        let target_opacity = (self.workspace.focused || !self.workspace.is_empty) as i32 as f32;
        let opacity = egui_animation::animate_eased(
            ui.ctx(),
            format!("Opacity{}", self.workspace.index),
            target_opacity,
            0.3,
            egui_animation::easing::sine_out,
        );

        // animate width
        let target_line_width = if !response.is_pointer_button_down_on() && self.workspace.focused {
            INDICATOR_FOCUSED_WIDTH
        } else {
            INDICATOR_BASE_WIDTH
        };
        let line_width = egui_animation::animate_eased(
            ui.ctx(),
            format!("Width{}", self.workspace.index),
            target_line_width,
            0.2,
            egui_animation::easing::sine_out,
        );

        if line_width != target_line_width || opacity != target_opacity {
            ui.ctx().request_repaint();
        }

        let x = rect.center().x - line_width / 2.0;
        let mut line_rect = rect.with_min_x(x).with_max_x(x + line_width);

        if self.line_on_top {
            line_rect = line_rect.with_max_y(rect.min.y + INDICATOR_HEIGHT);
        } else {
            line_rect = line_rect.with_min_y(rect.max.y - INDICATOR_HEIGHT);
        };

        let color = if self.workspace.focused {
            let c = self.line_focused_color.unwrap_or(egui::Color32::CYAN);
            egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (opacity * 255.0) as u8)
        } else if dark_mode {
            egui::Color32::from_rgba_unmultiplied(180, 173, 170, (opacity * 125.0) as u8)
        } else {
            egui::Color32::from_rgba_unmultiplied(31, 31, 31, (opacity * 150.0) as u8)
        };

        painter.rect_filled(line_rect, RADIUS, color);

        // draw text
        let text_color = if response.hovered() || self.workspace.focused {
            text_color
        } else {
            text_color.gamma_multiply(0.75)
        };

        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            &self.workspace.name,
            font_id,
            text_color,
        );

        response
    }
}
