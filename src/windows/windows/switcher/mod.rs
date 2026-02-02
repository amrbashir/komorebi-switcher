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

use crate::config::Config;
use crate::komorebi::CycleDirection;
use crate::windows::app::{App, AppMessage};
use crate::windows::context_menu::AppContextMenu;
use crate::windows::egui_glue::{EguiView, EguiWindow};
use crate::windows::registry;
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
        let monitor_config = {
            let config = self.config.read().unwrap();
            let mut monitor_config = config.get_monitor(&monitor_state.id);
            monitor_config.adjust_for_auto_size(&monitor_state.id)?;
            monitor_config
        };

        let host = unsafe { host::create_host(taskbar.hwnd, self.proxy.clone(), &monitor_config) }?;

        let mut attrs = WindowAttributes::default();
        attrs = attrs.with_inner_size(PhysicalSize::new(
            monitor_config.width,
            monitor_config.height,
        ));

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
    prev_bounds: Option<egui::Rect>,
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
            prev_bounds: None,
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
        let monitor_config = config.get_monitor(&self.monitor_state.id);

        // Add margins to rect and scale by ppp
        let rect = rect + Self::WORKSPACES_MARGIN;
        let rect = rect * ppp;

        let x = monitor_config.x;
        let y = monitor_config.y;

        let height = if monitor_config.auto_height {
            self.taskbar_height()?
        } else {
            monitor_config.height
        };

        let width = if monitor_config.auto_width {
            rect.width() as i32
        } else {
            monitor_config.width
        };

        let current_bounds = egui::Rect::from_min_size(
            egui::pos2(x as f32, y as f32),
            egui::vec2(width as f32, height as f32),
        );

        // If bounds changed, resize host window
        if self.prev_bounds != Some(current_bounds) {
            tracing::debug!("Resizing host to match content rect");

            unsafe {
                SetWindowPos(
                    self.host,
                    None,
                    x,
                    y,
                    width,
                    height,
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                )
            }?;

            // Store auto size if needed
            if monitor_config.auto_width || monitor_config.auto_height {
                let _ = registry::store_auto_size(&self.monitor_state.id, width, height);
            }

            // Update previous bounds, to avoid redundant resizes next time
            self.prev_bounds = Some(current_bounds);
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

                let config = self.effective_config();
                let monitor_config = config.get_monitor(&self.monitor_state.id);
                let hide_empty_workspaces = match monitor_config.hide_empty_workspaces {
                    Some(hide) => hide,
                    None => config.hide_empty_workspaces,
                };

                // show workspace buttons
                for workspace in self.monitor_state.workspaces.iter() {
                    if hide_empty_workspaces && workspace.is_empty && !workspace.focused {
                        continue;
                    }

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

            AppMessage::PreviewConfig(config) => self.preview_config = Some(config.clone()),

            AppMessage::ClearPreviewConfig => self.preview_config = None,
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

            if let Err(e) = self.resize_host_to_rect(response.rect, ctx.pixels_per_point()) {
                tracing::error!("Failed to resize host to rect: {e}");
            }
        });
    }
}
