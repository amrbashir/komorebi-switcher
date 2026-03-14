use std::sync::{Arc, RwLock};

use winit::dpi::PhysicalSize;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::{WindowAttributes, WindowId};

use crate::config::Config;
use crate::komorebi::State;
use crate::windows::app::{App, AppMessage};
use crate::windows::egui_glue::{EguiView, EguiWindow};

impl App {
    pub fn create_settings_window(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        #[cfg(debug_assertions)]
        let class_name = "komorebi-switcher-debug::settings-window";
        #[cfg(not(debug_assertions))]
        let class_name = "komorebi-switcher::settings-window";

        let attrs = WindowAttributes::default()
            .with_title("Settings")
            .with_class_name(class_name)
            .with_inner_size(PhysicalSize::new(450, 600))
            .with_resizable(true)
            .with_no_redirection_bitmap(true);

        let window = event_loop.create_window(attrs)?;
        let window = Arc::new(window);

        let state = SettingsWindowView {
            window_id: window.id(),
            proxy: self.proxy.clone(),
            config_: self.config.clone(),
            config: self.config.read().unwrap().clone(),
            komorebi_state: self.komorebi_state.clone(),
        };

        let window = EguiWindow::new(window, &self.wgpu_instance, state)?;

        self.windows.insert(window.id(), None, window);

        Ok(())
    }
}

struct SettingsWindowView {
    window_id: WindowId,
    proxy: EventLoopProxy<AppMessage>,
    config_: Arc<RwLock<Config>>,
    config: Config,
    komorebi_state: State,
}

impl SettingsWindowView {
    fn close_window(&self) -> anyhow::Result<()> {
        let message = AppMessage::CloseWindow(self.window_id);
        self.proxy.send_event(message).map_err(Into::into)
    }

    fn save(&mut self) -> anyhow::Result<()> {
        let mut config = self.config_.write().unwrap();
        *config = self.config.clone();
        config.save()?;

        self.proxy.send_event(AppMessage::ClearPreviewConfig)?;
        self.close_window()
    }

    fn cancel(&mut self) -> anyhow::Result<()> {
        self.proxy.send_event(AppMessage::ClearPreviewConfig)?;
        self.close_window()
    }

    fn preview_config(&self) {
        let message = AppMessage::PreviewConfig(self.config.clone());
        if let Err(e) = self.proxy.send_event(message) {
            tracing::error!("Failed to send preview config: {e}");
        }
    }

    fn global_font_family_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Font Family");
            let mut font_family = self.config.font_family.clone().unwrap_or_default();
            let text_edit = egui::TextEdit::singleline(&mut font_family).hint_text("i.e Roboto");
            if ui.add(text_edit).changed() {
                self.config.font_family = (!font_family.is_empty()).then_some(font_family);
            }
        });
    }

    fn global_font_weight_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Font Weight");
            let mut font_weight = self.config.font_weight.unwrap_or(400);
            let drag_value = egui::DragValue::new(&mut font_weight)
                .range(100..=900)
                .speed(10);
            if ui.add(drag_value).changed() {
                self.config.font_weight = Some(font_weight);
            }
        });
    }

    fn global_settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Global Settings");

        ui.add(egui::Checkbox::new(
            &mut self.config.show_layout_button,
            "Show layout button",
        ));

        ui.add(egui::Checkbox::new(
            &mut self.config.hide_empty_workspaces,
            "Hide empty workspaces",
        ));

        self.global_font_family_ui(ui);
        self.global_font_weight_ui(ui);
    }

    fn width_ui(&mut self, ui: &mut egui::Ui, monitor_id: &str) {
        let monitor_config = self.config.get_monitor_or_default(monitor_id);

        ui.label("Width");
        ui.horizontal(|ui| {
            let drag_value = egui::DragValue::new(&mut monitor_config.width);
            ui.add_enabled(!monitor_config.auto_width, drag_value);
            ui.checkbox(&mut monitor_config.auto_width, "Auto");
        });
    }

    fn height_ui(&mut self, ui: &mut egui::Ui, monitor_id: &str) {
        let monitor_config = self.config.get_monitor_or_default(monitor_id);

        ui.label("Height");
        ui.horizontal(|ui| {
            let drag_value = egui::DragValue::new(&mut monitor_config.height);
            ui.add_enabled(!monitor_config.auto_height, drag_value);
            ui.checkbox(&mut monitor_config.auto_height, "Auto");
        });
    }

    fn show_layout_button_ui(&mut self, ui: &mut egui::Ui, monitor_id: &str) {
        let monitor_config = self.config.get_monitor_or_default(monitor_id);

        ui.label("Show layout button");

        let mut selected: ActivationOption = monitor_config.show_layout_button.into();

        let before = selected;

        egui::ComboBox::new("show_layout_button", "")
            .selected_text(format!("{}", selected))
            .show_ui(ui, |ui| {
                for option in [
                    ActivationOption::Inherit,
                    ActivationOption::Enable,
                    ActivationOption::Disable,
                ] {
                    ui.selectable_value(&mut selected, option, format!("{}", option));
                }
            });

        if before != selected {
            monitor_config.show_layout_button = selected.into();
        }
    }

    fn hide_empty_workspaces_ui(&mut self, ui: &mut egui::Ui, monitor_id: &str) {
        let monitor_config = self.config.get_monitor_or_default(monitor_id);

        ui.label("Hide empty workspaces");

        let mut selected: ActivationOption = monitor_config.hide_empty_workspaces.into();

        let before = selected;

        egui::ComboBox::new("hide_empty_workspaces", "")
            .selected_text(format!("{}", selected))
            .show_ui(ui, |ui| {
                for option in [
                    ActivationOption::Inherit,
                    ActivationOption::Enable,
                    ActivationOption::Disable,
                ] {
                    ui.selectable_value(&mut selected, option, format!("{}", option));
                }
            });

        if before != selected {
            monitor_config.hide_empty_workspaces = selected.into();
        }
    }

    fn font_family_ui(&mut self, ui: &mut egui::Ui, monitor_id: &str) {
        let monitor_config = self.config.get_monitor_or_default(monitor_id);
        ui.label("Font Family");
        ui.horizontal(|ui| {
            let mut inherit = monitor_config.font_family.is_none();
            let mut font_family = monitor_config.font_family.clone().unwrap_or_default();
            let text_edit = egui::TextEdit::singleline(&mut font_family).hint_text("i.e Roboto");
            if ui.add_enabled(!inherit, text_edit).changed() {
                monitor_config.font_family = (!font_family.is_empty()).then_some(font_family);
            }
            if ui.checkbox(&mut inherit, "Inherit").changed() {
                monitor_config.font_family = if inherit { None } else { Some(String::new()) };
            }
        });
    }

    fn font_weight_ui(&mut self, ui: &mut egui::Ui, monitor_id: &str) {
        let monitor_config = self.config.get_monitor_or_default(monitor_id);
        ui.label("Font Weight");
        ui.horizontal(|ui| {
            let mut inherit = monitor_config.font_weight.is_none();
            let mut font_weight = monitor_config.font_weight.unwrap_or(400);
            let drag_value = egui::DragValue::new(&mut font_weight)
                .range(100..=900)
                .speed(10);
            if ui.add_enabled(!inherit, drag_value).changed() {
                monitor_config.font_weight = Some(font_weight);
            }
            if ui.checkbox(&mut inherit, "Inherit").changed() {
                monitor_config.font_weight = if inherit { None } else { Some(400) };
            }
        });
    }

    fn monitor_settings_ui(&mut self, ui: &mut egui::Ui, monitor_id: &str) {
        let monitor_config = self.config.get_monitor_or_default(monitor_id);

        ui.label("X");
        ui.add(egui::DragValue::new(&mut monitor_config.x));
        ui.end_row();

        ui.label("Y");
        ui.add(egui::DragValue::new(&mut monitor_config.y));
        ui.end_row();

        self.width_ui(ui, monitor_id);
        ui.end_row();

        self.height_ui(ui, monitor_id);
        ui.end_row();

        self.show_layout_button_ui(ui, monitor_id);
        ui.end_row();

        self.hide_empty_workspaces_ui(ui, monitor_id);
        ui.end_row();

        self.font_family_ui(ui, monitor_id);
        ui.end_row();

        self.font_weight_ui(ui, monitor_id);
        ui.end_row();
    }

    fn actions_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                if let Err(e) = self.save() {
                    tracing::error!("Failed to save config: {e}");
                }
            }

            if ui.button("Cancel").clicked() {
                if let Err(e) = self.cancel() {
                    tracing::error!("Failed to cancel: {e}");
                }
            }
        });
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        // Preview config on each update
        self.preview_config();

        self.global_settings_ui(ui);

        ui.separator();

        ui.heading("Monitors");
        for (index, monitor) in self.komorebi_state.monitors.clone().iter().enumerate() {
            // Each monitor's settings in a collapsible header
            let header = format!("{} ({})", monitor.name, monitor.id);
            egui::CollapsingHeader::new(&header)
                .default_open(index == 0)
                .show_background(true)
                .show(ui, |ui| {
                    egui::Grid::new(&header)
                        .num_columns(2)
                        .min_col_width(ui.available_width() / 2.0)
                        .max_col_width(ui.available_width() / 2.0)
                        .show(ui, |ui| self.monitor_settings_ui(ui, &monitor.id))
                });
        }

        ui.separator();

        self.actions_ui(ui);
    }
}

impl EguiView for SettingsWindowView {
    fn handle_window_event(
        &mut self,
        _ctx: &egui::Context,
        _event_loop: &ActiveEventLoop,
        event: winit::event::WindowEvent,
    ) -> anyhow::Result<()> {
        if let winit::event::WindowEvent::CloseRequested = event {
            self.cancel()?;
        }

        Ok(())
    }

    fn update(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| self.ui(ui));
    }
}

/// Represents an activation option for a setting: Inherit, Enable, or Disable.
#[derive(Copy, Clone, PartialEq, strum::Display)]
enum ActivationOption {
    Inherit,
    Enable,
    Disable,
}

impl From<ActivationOption> for Option<bool> {
    fn from(option: ActivationOption) -> Self {
        match option {
            ActivationOption::Inherit => None,
            ActivationOption::Enable => Some(true),
            ActivationOption::Disable => Some(false),
        }
    }
}

impl From<Option<bool>> for ActivationOption {
    fn from(option: Option<bool>) -> Self {
        match option {
            None => ActivationOption::Inherit,
            Some(true) => ActivationOption::Enable,
            Some(false) => ActivationOption::Disable,
        }
    }
}
