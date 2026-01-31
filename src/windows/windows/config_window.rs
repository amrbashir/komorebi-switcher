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
    pub fn create_config_window(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        #[cfg(debug_assertions)]
        let class_name = "komorebi-switcher-debug::config-window";
        #[cfg(not(debug_assertions))]
        let class_name = "komorebi-switcher::config-window";

        let attrs = WindowAttributes::default()
            .with_title("Configuration")
            .with_class_name(class_name)
            .with_inner_size(PhysicalSize::new(450, 600))
            .with_resizable(true)
            .with_no_redirection_bitmap(true);

        let window = event_loop.create_window(attrs)?;
        let window = Arc::new(window);

        let state = ConfigWindowView {
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

struct ConfigWindowView {
    window_id: WindowId,
    proxy: EventLoopProxy<AppMessage>,
    config_: Arc<RwLock<Config>>,
    config: Config,
    komorebi_state: State,
}

impl ConfigWindowView {
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

    fn global_settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Global Settings");

        ui.add(egui::Checkbox::new(
            &mut self.config.show_layout_button,
            "Show layout button",
        ));
    }

    fn layout_button_ui(&mut self, ui: &mut egui::Ui, monitor_id: &str) {
        let monitor_config = self.config.get_or_insert(monitor_id);

        ui.label("Show layout button");

        #[derive(Copy, Clone, PartialEq, strum::Display)]
        enum ShowLayoutButton {
            Inherit,
            Show,
            Hide,
        }

        let mut selected = match monitor_config.show_layout_button {
            None => ShowLayoutButton::Inherit,
            Some(true) => ShowLayoutButton::Show,
            Some(false) => ShowLayoutButton::Hide,
        };

        let before = selected;

        egui::ComboBox::from_label("")
            .selected_text(format!("{}", selected))
            .show_ui(ui, |ui| {
                for option in [
                    ShowLayoutButton::Inherit,
                    ShowLayoutButton::Show,
                    ShowLayoutButton::Hide,
                ] {
                    ui.selectable_value(&mut selected, option, format!("{}", option));
                }
            });

        if before != selected {
            monitor_config.show_layout_button = match selected {
                ShowLayoutButton::Inherit => None,
                ShowLayoutButton::Show => Some(true),
                ShowLayoutButton::Hide => Some(false),
            };
        }
    }

    fn monitor_settings_ui(&mut self, ui: &mut egui::Ui, monitor_id: &str) {
        let monitor_config = self.config.get_or_insert(monitor_id);

        ui.label("X");
        ui.add(egui::DragValue::new(&mut monitor_config.x));
        ui.end_row();

        ui.label("Y");
        ui.add(egui::DragValue::new(&mut monitor_config.y));
        ui.end_row();

        ui.label("Width");
        ui.horizontal(|ui| {
            ui.add_enabled(
                !monitor_config.auto_width,
                egui::DragValue::new(&mut monitor_config.width),
            );
            ui.checkbox(&mut monitor_config.auto_width, "Auto");
        });
        ui.end_row();

        ui.label("Height");
        ui.horizontal(|ui| {
            ui.add_enabled(
                !monitor_config.auto_height,
                egui::DragValue::new(&mut monitor_config.height),
            );
            ui.checkbox(&mut monitor_config.auto_height, "Auto");
        });
        ui.end_row();

        self.layout_button_ui(ui, monitor_id);
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

impl EguiView for ConfigWindowView {
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
