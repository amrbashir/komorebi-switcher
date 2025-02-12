use std::num::NonZero;
use std::sync::Arc;

use egui::vec2;
use raw_window_handle::{RawWindowHandle, Win32WindowHandle};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, SetWindowPos, SWP_NOMOVE};
use winit::dpi::PhysicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::{Window, WindowAttributes};

use crate::_egui_glue::{EguiView, EguiWindow};
use crate::app::{App, AppMessage};
use crate::komorebi::listen_for_workspaces;

impl App {
    pub fn create_main_window(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let mut attrs = WindowAttributes::default();

        let (w, h) = self.host_size()?;
        attrs = attrs.with_inner_size(PhysicalSize::new(w, h));

        let parent = unsafe { NonZero::new_unchecked(self.taskbar_host.0 as _) };
        let parent = Win32WindowHandle::new(parent);
        let parent = RawWindowHandle::Win32(parent);

        attrs = unsafe { attrs.with_parent_window(Some(parent)) };
        attrs = attrs
            .with_decorations(false)
            .with_transparent(true)
            .with_class_name("komorebi-switcher::window")
            .with_undecorated_shadow(false)
            .with_clip_children(false);

        let window = event_loop.create_window(attrs)?;
        let window = Arc::new(window);

        let state = MainWindowView::new(window.clone(), self.taskbar_host);

        let proxy = self.proxy.clone();

        std::thread::spawn(move || listen_for_workspaces(proxy));

        let window = EguiWindow::new(window, &self.wgpu_instance, state);

        self.windows.insert(window.id(), window);

        Ok(())
    }
}

pub struct MainWindowView {
    window: Arc<Window>,
    taskbar_host: HWND,
    curr_width: i32,
    workspaces: Vec<crate::komorebi::Workspace>,
}

impl MainWindowView {
    fn new(window: Arc<Window>, taskbar_host: HWND) -> Self {
        let workspaces = crate::komorebi::read_workspaces().unwrap_or_default();

        Self {
            window,
            taskbar_host,
            curr_width: 0,
            workspaces,
        }
    }

    fn resize_host_to_rect(&mut self, rect: egui::Rect) {
        let width = rect.width() as f64 + 2.0 /* default margin 1 on each side */;
        let width = self.window.scale_factor() * width;
        let width = width as i32;

        if width != self.curr_width {
            self.curr_width = width;

            let mut rect = RECT::default();
            if unsafe { GetClientRect(self.taskbar_host, &mut rect) }.is_ok() {
                let _ = unsafe {
                    SetWindowPos(
                        self.taskbar_host,
                        None,
                        0,
                        0,
                        width,
                        rect.bottom - rect.top,
                        SWP_NOMOVE,
                    )
                };
            }
        }
    }

    fn workspace_button(
        workspace: &crate::komorebi::Workspace,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        let style = ui.style_mut();

        let fill_color = if workspace.focused {
            style.visuals.selection.bg_fill
        } else if workspace.is_empty {
            egui::Color32::TRANSPARENT
        } else {
            egui::Color32::DARK_GRAY
        };

        let hover_color = style.visuals.selection.bg_fill;

        let active_border_color = egui::Color32::LIGHT_GRAY;

        let inactive_border_color = if workspace.focused {
            active_border_color
        } else {
            egui::Color32::GRAY
        };

        style.visuals.widgets.inactive = egui::style::WidgetVisuals {
            bg_fill: fill_color,
            weak_bg_fill: fill_color,
            bg_stroke: egui::Stroke {
                width: 1.,
                color: inactive_border_color,
            },
            ..style.visuals.widgets.hovered
        };

        style.visuals.widgets.hovered = egui::style::WidgetVisuals {
            bg_fill: hover_color,
            weak_bg_fill: hover_color,
            bg_stroke: egui::Stroke {
                width: 1.,
                color: active_border_color,
            },
            ..style.visuals.widgets.hovered
        };

        let btn = egui::Button::new(&workspace.name)
            .min_size(vec2(24., 24.))
            .corner_radius(2);

        ui.add(btn)
    }

    fn draw_workspaces_row(&mut self, ui: &mut egui::Ui) -> egui::InnerResponse<()> {
        ui.horizontal_centered(|ui| {
            for workspace in self.workspaces.iter() {
                if Self::workspace_button(workspace, ui).clicked() {
                    let _ = crate::komorebi::change_workspace(workspace.idx);
                }
            }
        })
    }
}

impl EguiView for MainWindowView {
    fn handle_app_message(&mut self, message: &AppMessage) {
        match message {
            AppMessage::UpdateWorkspaces(workspaces) => self.workspaces = workspaces.clone(),
        }
    }

    fn update(&mut self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::default();
        visuals.panel_fill = egui::Color32::TRANSPARENT;
        ctx.set_visuals(visuals);

        let margin = egui::Margin::symmetric(1, 0);
        let frame = egui::Frame::central_panel(&ctx.style()).inner_margin(margin);

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            let response = self.draw_workspaces_row(ui);
            self.resize_host_to_rect(response.response.rect);
        });
    }
}
