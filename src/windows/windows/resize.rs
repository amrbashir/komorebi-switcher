use std::sync::Arc;

use windows::Win32::Foundation::HWND;
use winit::dpi::PhysicalSize;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::{WindowAttributes, WindowId};

use crate::app::{App, AppMessage};
use crate::egui_glue::{EguiView, EguiWindow};
use crate::window_registry_info::WindowRegistryInfo;

impl App {
    pub fn create_resize_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        host: HWND,
        initial_info: WindowRegistryInfo,
        subkey: String,
    ) -> anyhow::Result<()> {
        #[cfg(debug_assertions)]
        let class_name = "komorebi-switcher-debug::resize-window";
        #[cfg(not(debug_assertions))]
        let class_name = "komorebi-switcher::resize-window";

        let attrs = WindowAttributes::default()
            .with_title("Move and Resize")
            .with_class_name(class_name)
            .with_inner_size(PhysicalSize::new(300, 200))
            .with_owner_window(host.0 as _)
            .with_no_redirection_bitmap(true);

        let window = event_loop.create_window(attrs)?;
        let window = Arc::new(window);

        let state = ResizeWindowView {
            window_id,
            self_window_id: window.id(),
            proxy: self.proxy.clone(),
            host,
            initial_info,
            info: initial_info,
            subkey,
        };

        let window = EguiWindow::new(window, &self.wgpu_instance, state)?;

        self.windows.insert(window.id(), None, window);

        Ok(())
    }
}

struct ResizeWindowView {
    window_id: WindowId,
    self_window_id: WindowId,
    host: HWND,
    proxy: EventLoopProxy<AppMessage>,
    initial_info: WindowRegistryInfo,
    info: WindowRegistryInfo,
    subkey: String,
}

impl ResizeWindowView {
    fn close_window(&self) -> anyhow::Result<()> {
        let message = AppMessage::CloseWindow(self.self_window_id);
        self.proxy.send_event(message).map_err(Into::into)
    }

    fn notify_window_info_changes(&self) -> anyhow::Result<()> {
        let message = AppMessage::NotifyWindowInfoChanges(self.window_id, self.info);
        self.proxy.send_event(message).map_err(Into::into)
    }

    fn save(&mut self) -> anyhow::Result<()> {
        self.info.apply(self.host)?;
        self.info.save(&self.subkey)?;
        self.notify_window_info_changes()?;
        self.close_window()
    }

    fn cancel(&mut self) -> anyhow::Result<()> {
        self.initial_info.apply(self.host)?;
        self.close_window()
    }
}

impl EguiView for ResizeWindowView {
    fn handle_window_event(
        &mut self,
        _ctx: &egui::Context,
        _event_loop: &ActiveEventLoop,
        event: winit::event::WindowEvent,
    ) -> anyhow::Result<()> {
        // Handle close event from the window manager or user clicking the window close button
        if let winit::event::WindowEvent::CloseRequested = event {
            self.cancel()?;
        }

        Ok(())
    }

    fn update(&mut self, ctx: &egui::Context) {
        if let Err(e) = self.info.apply(self.host) {
            tracing::error!("Failed to apply current move and resize info: {e}");
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("grid")
                .num_columns(2)
                .min_col_width(ui.available_width() / 2.0)
                .max_col_width(ui.available_width() / 2.0)
                .show(ui, |ui| {
                    ui.label("x");
                    ui.add(egui::DragValue::new(&mut self.info.x));
                    ui.end_row();

                    ui.label("y");
                    ui.add(egui::DragValue::new(&mut self.info.y));
                    ui.end_row();

                    ui.label("width");
                    ui.horizontal(|ui| {
                        ui.add_enabled(
                            !self.info.auto_width,
                            egui::DragValue::new(&mut self.info.width),
                        );
                        ui.checkbox(&mut self.info.auto_width, "Auto width");
                    });
                    ui.end_row();

                    ui.label("height");
                    ui.horizontal(|ui| {
                        ui.add_enabled(
                            !self.info.auto_height,
                            egui::DragValue::new(&mut self.info.height),
                        );
                        ui.checkbox(&mut self.info.auto_height, "Auto height");
                    });
                    ui.end_row();
                });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    if let Err(e) = self.save() {
                        tracing::error!("Failed to save resize: {e}");
                    }
                }

                if ui.button("Cancel").clicked() {
                    if let Err(e) = self.cancel() {
                        tracing::error!("Failed to cancel resize: {e}");
                    }
                }
            });
        });
    }
}
