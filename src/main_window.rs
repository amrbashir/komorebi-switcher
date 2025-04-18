use std::num::NonZero;
use std::sync::Arc;

use muda::{ContextMenu, Menu, MenuItem};
use raw_window_handle::{RawWindowHandle, Win32WindowHandle};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::UI::ViewManagement::{UIColorType, UISettings};
use winit::dpi::PhysicalSize;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::{Window, WindowAttributes};

use crate::app::{App, AppMessage};
use crate::egui_glue::{EguiView, EguiWindow};
use crate::host::create_host;
use crate::komorebi::listen_for_workspaces;
use crate::widgets::WorkspaceButton;
use crate::window_registry_info::WindowRegistryInfo;

impl App {
    pub fn create_main_window(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        tracing::info!("Creating main window");

        let window_info = WindowRegistryInfo::load()?;

        let host = unsafe { create_host(self.taskbar_hwnd, self.proxy.clone(), &window_info) }?;

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

        let state = MainWindowView::new(
            window.clone(),
            host,
            self.taskbar_hwnd,
            self.proxy.clone(),
            window_info,
        )?;

        let proxy = self.proxy.clone();

        std::thread::spawn(move || listen_for_workspaces(proxy));

        let window = EguiWindow::new(window, &self.wgpu_instance, state)?;

        self.windows.insert(window.id(), window);

        Ok(())
    }
}

pub struct MainWindowView {
    window: Arc<Window>,
    host: HWND,
    taskbar_hwnd: HWND,
    proxy: EventLoopProxy<AppMessage>,
    context_menu: muda::Menu,
    workspaces: Vec<crate::komorebi::Workspace>,
    accent_light2_color: Option<egui::Color32>,
    accent_color: Option<egui::Color32>,
    forgreound_color: Option<egui::Color32>,
    window_info: WindowRegistryInfo,
}

impl MainWindowView {
    fn new(
        window: Arc<Window>,
        host: HWND,
        taskbar_hwnd: HWND,
        proxy: EventLoopProxy<AppMessage>,
        window_info: WindowRegistryInfo,
    ) -> anyhow::Result<Self> {
        let workspaces = crate::komorebi::read_workspaces()
            .inspect_err(|e| {
                tracing::error!("Failed to read workspaces: {e}");
            })
            .unwrap_or_default();

        let mut view = Self {
            window,
            host,
            proxy,
            taskbar_hwnd,
            workspaces,
            context_menu: Self::create_context_menu()?,
            accent_color: None,
            accent_light2_color: None,
            forgreound_color: None,
            window_info,
        };

        if let Err(e) = view.update_system_colors() {
            tracing::error!("Failed to get system colors: {e}");
        }

        Ok(view)
    }

    /// The ID of the "Move & Resize" menu item.
    const M_MOVE_RESIZE_ID: &str = "main-window-move-resize";
    /// The ID of the "Quit" menu item.
    const M_QUIT_ID: &str = "main-window-quit";

    fn create_context_menu() -> anyhow::Result<muda::Menu> {
        let quit = MenuItem::with_id(Self::M_QUIT_ID, "Quit", true, None);
        let move_resize = MenuItem::with_id(Self::M_MOVE_RESIZE_ID, "Move & Resize", true, None);
        Menu::with_items(&[&move_resize, &quit]).map_err(Into::into)
    }

    fn show_context_menu(&self) {
        tracing::debug!("Showing context menu");

        let hwnd = self.host.0 as isize;
        unsafe { self.context_menu.show_context_menu_for_hwnd(hwnd, None) };
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

    fn host_window_rect(&self) -> anyhow::Result<RECT> {
        let mut rect = RECT::default();
        unsafe { GetWindowRect(self.host, &mut rect) }?;
        Ok(rect)
    }

    fn taskbar_height(&self) -> anyhow::Result<i32> {
        let mut rect = RECT::default();
        unsafe { GetClientRect(self.taskbar_hwnd, &mut rect) }?;
        Ok(rect.bottom - rect.top)
    }

    const WORKSPACES_MARGIN: egui::Margin = egui::Margin::same(1);

    fn resize_host_to_rect(&mut self, rect: egui::Rect, ppp: f32) -> anyhow::Result<()> {
        let rect = rect + Self::WORKSPACES_MARGIN;
        let rect = rect * ppp;

        let height = if self.window_info.auto_height {
            self.taskbar_height()?
        } else {
            self.window_info.height
        };

        let width = if self.window_info.auto_width {
            rect.width() as i32
        } else {
            self.window_info.width
        };

        let curr_width = self.window_info.width;
        let curr_height = self.window_info.height;

        if curr_width != width || curr_height != height {
            self.window_info.width = width;
            self.window_info.height = height;

            tracing::debug!("Resizing host to match content rect");

            self.window_info.save()?;
            unsafe { SetWindowPos(self.host, None, 0, 0, width, height, SWP_NOMOVE) }?;
        }

        Ok(())
    }

    pub const IN_RESIZE_PROP: PCWSTR = w!("komorebi::in_resize");

    fn start_host_dragging(&self) -> anyhow::Result<()> {
        let host = self.host.0 as isize;
        let info = self.window_info;
        let message = AppMessage::CreateResizeWindow { host, info };
        self.proxy.send_event(message)?;
        unsafe { SetPropW(self.host, Self::IN_RESIZE_PROP, Some(HANDLE(1 as _))) }?;
        Ok(())
    }

    fn update_window_info(&mut self, info: &WindowRegistryInfo) -> anyhow::Result<()> {
        self.window_info = *info;
        unsafe { RemovePropW(self.host, Self::IN_RESIZE_PROP) }?;
        Ok(())
    }

    fn close_host(&self) -> anyhow::Result<()> {
        tracing::info!("Closing host window");

        unsafe {
            PostMessageW(
                Some(self.host),
                WM_CLOSE,
                WPARAM::default(),
                LPARAM::default(),
            )
            .map_err(Into::into)
        }
    }

    fn is_taskbar_on_top(&self) -> bool {
        // TODO: find a more peroformant way to check this
        self.host_window_rect()
            .map(|r| {
                let current_monitor = self.window.current_monitor();
                let y = current_monitor.map(|m| m.position().y).unwrap_or(0);
                r.top <= y
            })
            .unwrap_or(false)
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

                for workspace in self.workspaces.iter() {
                    let btn = WorkspaceButton::new(workspace)
                        .dark_mode(Some(self.is_system_dark_mode()))
                        .line_focused_color_opt(self.line_focused_color())
                        .text_color_opt(self.forgreound_color)
                        .line_on_top(self.is_taskbar_on_top());

                    if ui.add(btn).clicked() {
                        crate::komorebi::change_workspace(workspace.idx);
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

impl EguiView for MainWindowView {
    fn handle_app_message(
        &mut self,
        ctx: &egui::Context,
        _event_loop: &ActiveEventLoop,
        message: &AppMessage,
    ) -> anyhow::Result<()> {
        match message {
            AppMessage::UpdateWorkspaces(workspaces) => self.workspaces = workspaces.clone(),
            AppMessage::MenuEvent(e) if e.id() == Self::M_MOVE_RESIZE_ID => {
                self.start_host_dragging()?
            }
            AppMessage::StartMoveResize => self.start_host_dragging()?,
            AppMessage::MenuEvent(e) if e.id() == Self::M_QUIT_ID => self.close_host()?,
            AppMessage::SystemSettingsChanged => self.update_system_colors()?,
            AppMessage::NotifyWindowInfoChanges(info) => self.update_window_info(info)?,
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
