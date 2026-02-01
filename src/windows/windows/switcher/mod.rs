use std::num::NonZero;
use std::sync::{Arc, RwLock};

use muda::ContextMenu;
use raw_window_handle::{RawWindowHandle, Win32WindowHandle};
use windows::Win32::Foundation::*;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::UI::ViewManagement::{UIColorType, UISettings};
use winit::dpi::PhysicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::WindowAttributes;

use crate::config::{Config, WindowConfig};
use crate::komorebi::CycleDirection;
use crate::windows::app::{App, AppMessage};
use crate::windows::context_menu::AppContextMenu;
use crate::windows::egui_glue::{EguiView, EguiWindow};
use crate::windows::taskbar::Taskbar;
use crate::windows::widgets::{LayoutButton, WorkspaceButton};

mod host;

impl App {
    pub fn create_switcher_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        taskbar: Taskbar,
        monitor_state: crate::komorebi::Monitor,
        context_menu: AppContextMenu,
    ) -> anyhow::Result<EguiWindow> {
        let window_info = {
            let config = self.config.read().unwrap();
            config.get_monitor(&monitor_state.id)
        };

        let host = unsafe { host::create_host(taskbar.hwnd, self.proxy.clone(), &window_info) }?;

        let mut attrs = WindowAttributes::default();
        attrs = attrs.with_inner_size(PhysicalSize::new(window_info.width, window_info.height));

        let parent = unsafe { NonZero::new_unchecked(host.0 as _) };
        let parent = Win32WindowHandle::new(parent);
        let parent = RawWindowHandle::Win32(parent);
        attrs = unsafe { attrs.with_parent_window(Some(parent)) };

        #[cfg(debug_assertions)]
        let class_name = "komorebi-switcher-debug::window";
        #[cfg(not(debug_assertions))]
        let class_name = "komorebi-switcher::window";

        attrs = attrs
            .with_decorations(false)
            .with_transparent(true)
            .with_active(false)
            .with_class_name(class_name)
            .with_undecorated_shadow(false)
            .with_no_redirection_bitmap(true)
            .with_clip_children(false);

        let window = event_loop.create_window(attrs)?;
        let window = Arc::new(window);

        let state = SwitcherWindowView::new(
            host,
            taskbar,
            monitor_state,
            self.config.clone(),
            context_menu,
        )?;

        EguiWindow::new(window, &self.wgpu_instance, state)
    }
}

struct ContextMenuState {
    menu: AppContextMenu,
}

pub struct SwitcherWindowView {
    config: Arc<RwLock<crate::config::Config>>,
    preview_config: Option<Config>,
    host: HWND,
    taskbar: Taskbar,
    context_menu: ContextMenuState,
    monitor_state: crate::komorebi::Monitor,
    accent_light2_color: Option<egui::Color32>,
    accent_color: Option<egui::Color32>,
    forgreound_color: Option<egui::Color32>,
}

impl SwitcherWindowView {
    fn new(
        host: HWND,
        taskbar: Taskbar,
        monitor_state: crate::komorebi::Monitor,
        config: Arc<RwLock<crate::config::Config>>,
        context_menu: AppContextMenu,
    ) -> anyhow::Result<Self> {
        let mut view = Self {
            host,
            taskbar,
            monitor_state,
            context_menu: ContextMenuState { menu: context_menu },
            accent_color: None,
            accent_light2_color: None,
            forgreound_color: None,
            config,
            preview_config: None,
        };

        if let Err(e) = view.update_system_colors() {
            tracing::error!("Failed to get system colors: {e}");
        }

        Ok(view)
    }

    fn effective_config(&self) -> Config {
        self.preview_config.clone().unwrap_or_else(|| {
            let config = self.config.read().unwrap();
            config.clone()
        })
    }

    fn show_context_menu(&self) {
        tracing::debug!("Showing context menu");

        let hwnd = self.host.0 as isize;
        unsafe {
            self.context_menu
                .menu
                .menu
                .show_context_menu_for_hwnd(hwnd, None)
        };
    }

    fn update_system_colors(&mut self) -> anyhow::Result<()> {
        let settings = UISettings::new()?;

        let color = settings.GetColorValue(UIColorType::Accent)?;
        let color = egui::Color32::from_rgb(color.R, color.G, color.B);
        self.accent_color.replace(color);

        let color = settings.GetColorValue(UIColorType::AccentLight2)?;
        let color = egui::Color32::from_rgb(color.R, color.G, color.B);
        self.accent_light2_color.replace(color);

        let color = settings.GetColorValue(UIColorType::Foreground)?;
        let color = egui::Color32::from_rgb(color.R, color.G, color.B);
        self.forgreound_color.replace(color);

        Ok(())
    }

    fn taskbar_height(&self) -> anyhow::Result<i32> {
        let mut rect = RECT::default();
        unsafe { GetClientRect(self.taskbar.hwnd, &mut rect) }?;
        Ok(rect.bottom - rect.top)
    }

    const WORKSPACES_MARGIN: egui::Margin = egui::Margin::same(1);

    fn resize_host_to_rect(&mut self, rect: egui::Rect, ppp: f32) -> anyhow::Result<()> {
        let config = self.effective_config();
        let mut window_info = config.get_monitor(&self.monitor_state.id);

        let rect = rect + Self::WORKSPACES_MARGIN;
        let rect = rect * ppp;

        let height = if window_info.auto_height {
            self.taskbar_height()?
        } else {
            window_info.height
        };

        let width = if window_info.auto_width {
            rect.width() as i32
        } else {
            window_info.width
        };

        let curr_width = window_info.width;
        let curr_height = window_info.height;

        if curr_width != width || curr_height != height {
            window_info.width = width;
            window_info.height = height;

            tracing::debug!("Resizing host to match content rect");

            let mut config = self.config.write().unwrap();
            config.set_monitor(&self.monitor_state.id, window_info);
            config.save()?;
            unsafe { SetWindowPos(self.host, None, 0, 0, width, height, SWP_NOMOVE) }?;
        }

        Ok(())
    }

    fn is_system_dark_mode(&self) -> bool {
        // FIXME: use egui internal dark mode detection
        self.forgreound_color
            .map(|c| c == egui::Color32::WHITE)
            .unwrap_or(false)
    }

    fn line_focused_color(&self) -> Option<egui::Color32> {
        if self.is_system_dark_mode() {
            self.accent_light2_color
        } else {
            self.accent_color
        }
    }

    fn workspaces_row(&mut self, ui: &mut egui::Ui) -> egui::Response {
        // show context menu on right click
        if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) {
            self.show_context_menu();
        }

        ui.horizontal_centered(|ui| {
            ui.scope(|ui| {
                ui.style_mut().spacing.item_spacing = egui::vec2(4., 4.);

                // show workspace buttons
                for workspace in self.monitor_state.workspaces.iter() {
                    let btn = WorkspaceButton::new(workspace)
                        .dark_mode(Some(self.is_system_dark_mode()))
                        .line_focused_color_opt(self.line_focused_color())
                        .text_color_opt(self.forgreound_color);

                    if ui.add(btn).clicked() {
                        crate::komorebi::change_workspace(
                            self.monitor_state.index,
                            workspace.index,
                        );
                    }
                }

                // show layout button for focused workspace
                let config = self.effective_config();
                let monitor_config = config.get_monitor(&self.monitor_state.id);
                let show_layout_button = match monitor_config.show_layout_button {
                    Some(show) => show,
                    None => config.show_layout_button,
                };
                if show_layout_button {
                    if let Some(focused_ws) = self.monitor_state.focused_workspace() {
                        ui.add(egui::Label::new("|"));

                        let btn = LayoutButton::new(&focused_ws.layout)
                            .dark_mode(Some(self.is_system_dark_mode()))
                            .text_color_opt(self.forgreound_color);

                        if ui.add(btn).clicked() {
                            crate::komorebi::cycle_layout(CycleDirection::Next);
                        }
                    }
                }
            })
        })
        .response
    }

    fn transparent_panel(&self, ctx: &egui::Context) -> egui::CentralPanel {
        let visuals = egui::Visuals {
            panel_fill: egui::Color32::TRANSPARENT,
            ..egui::Visuals::dark()
        };
        ctx.set_visuals(visuals);

        let frame = egui::Frame::central_panel(&ctx.style()).inner_margin(Self::WORKSPACES_MARGIN);

        egui::CentralPanel::default().frame(frame)
    }
}

impl WindowConfig {
    pub fn apply_bounds_to(&self, hwnd: windows::Win32::Foundation::HWND) -> anyhow::Result<()> {
        let height = if self.auto_height {
            let parent = unsafe { GetParent(hwnd) }?;
            let mut rect = RECT::default();
            unsafe { GetClientRect(parent, &mut rect) }?;
            rect.bottom - rect.top
        } else {
            self.height
        };

        let width = if self.auto_width {
            let child = unsafe { GetWindow(hwnd, GW_CHILD) }?;
            let mut rect = RECT::default();
            unsafe { GetClientRect(child, &mut rect) }?;
            rect.right - rect.left
        } else {
            self.width
        };

        unsafe {
            SetWindowPos(
                hwnd,
                None,
                self.x,
                self.y,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            )
            .map_err(Into::into)
        }
    }
}

impl EguiView for SwitcherWindowView {
    fn handle_app_message(
        &mut self,
        ctx: &egui::Context,
        _event_loop: &ActiveEventLoop,
        message: &AppMessage,
    ) -> anyhow::Result<()> {
        match message {
            AppMessage::UpdateKomorebiState(state) => {
                self.monitor_state = state
                    .monitors
                    .iter()
                    .find(|m| m.id == self.monitor_state.id)
                    .cloned()
                    .unwrap_or_default();
            }

            AppMessage::PreviewConfig(config) => {
                self.preview_config = Some(config.clone());

                let monitor_config = config.get_monitor(&self.monitor_state.id);
                monitor_config.apply_bounds_to(self.host)?;
            }

            AppMessage::ClearPreviewConfig => {
                self.preview_config = None;
            }

            AppMessage::SystemSettingsChanged => self.update_system_colors()?,

            AppMessage::DpiChanged => {
                let dpi = unsafe { GetDpiForWindow(self.host) } as f32;
                let ppp = dpi / USER_DEFAULT_SCREEN_DPI as f32;
                ctx.set_pixels_per_point(ppp);
            }

            _ => {}
        }

        Ok(())
    }

    fn update(&mut self, ctx: &egui::Context) {
        self.transparent_panel(ctx).show(ctx, |ui| {
            let response = self.workspaces_row(ui);

            // Skip resizing if we are in preview mode, to avoid infinite resize loop
            if self.preview_config.is_some() {
                return;
            }

            if let Err(e) = self.resize_host_to_rect(response.rect, ctx.pixels_per_point()) {
                tracing::error!("Failed to resize host to rect: {e}");
            }
        });
    }
}
