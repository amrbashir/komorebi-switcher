use std::num::NonZero;
use std::sync::Arc;

use muda::ContextMenu;
use raw_window_handle::{RawWindowHandle, Win32WindowHandle};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::MapWindowPoints;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::UI::ViewManagement::{UIColorType, UISettings};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::{Window, WindowAttributes};

use crate::app::{App, AppMessage};
use crate::egui_glue::{EguiView, EguiWindow};
use crate::komorebi::listen_for_workspaces;
use crate::widgets::WorkspaceButton;

impl App {
    pub fn create_main_window(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        tracing::info!("Creating main window");

        let host = unsafe { crate::host::create_host(self.taskbar_hwnd, self.proxy.clone()) }?;

        let mut attrs = WindowAttributes::default();

        // get host width/height
        let mut rect = RECT::default();
        unsafe { GetClientRect(host, &mut rect) }?;
        let width = rect.right - rect.left;
        let heigth = rect.bottom - rect.top;

        attrs = attrs.with_inner_size(PhysicalSize::new(width, heigth));

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

        let state = MainWindowView::new(window.clone(), host, self.taskbar_hwnd)?;

        let proxy = self.proxy.clone();

        std::thread::spawn(move || listen_for_workspaces(proxy));

        let window = EguiWindow::new(window, &self.wgpu_instance, state)?;

        self.windows.insert(window.id(), window);

        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
enum DraggingState {
    /// Not in drag mode
    None,
    /// In drag mode but user hasn't started dragging yet
    Started,
    /// In drag mode and actually dragging
    Dragging,
}

pub struct MainWindowView {
    window: Arc<Window>,
    host: HWND,
    taskbar_hwnd: HWND,
    size: (i32, i32),
    workspaces: Vec<crate::komorebi::Workspace>,
    context_menu: muda::Menu,
    dragging_state: DraggingState,
    accent_light2_color: Option<egui::Color32>,
    accent_color: Option<egui::Color32>,
    forgreound_color: Option<egui::Color32>,
}

impl MainWindowView {
    fn new(window: Arc<Window>, host: HWND, taskbar_hwnd: HWND) -> anyhow::Result<Self> {
        let workspaces = crate::komorebi::read_workspaces().unwrap_or_default();

        let mut view = Self {
            window,
            host,
            taskbar_hwnd,
            size: (0, 0),
            workspaces,
            context_menu: Self::create_context_menu()?,
            dragging_state: DraggingState::None,
            accent_color: None,
            accent_light2_color: None,
            forgreound_color: None,
        };

        if let Err(e) = view.update_system_colors() {
            tracing::error!("Failed to get system colors: {e}");
        }

        Ok(view)
    }

    const M_MOVE_ID: &str = "move";
    const M_QUIT_ID: &str = "quit";

    fn create_context_menu() -> anyhow::Result<muda::Menu> {
        muda::Menu::with_items(&[
            &muda::MenuItem::with_id(Self::M_MOVE_ID, "Move", true, None),
            &muda::MenuItem::with_id(Self::M_QUIT_ID, "Quit", true, None),
        ])
        .map_err(Into::into)
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

    fn host_client_rect(&self) -> anyhow::Result<RECT> {
        let mut rect = RECT::default();
        unsafe { GetClientRect(self.host, &mut rect) }?;
        Ok(rect)
    }

    fn host_window_rect(&self) -> anyhow::Result<RECT> {
        let mut rect = RECT::default();
        unsafe { GetWindowRect(self.host, &mut rect) }?;
        Ok(rect)
    }

    fn taskbar_client_rect(&self) -> anyhow::Result<RECT> {
        let mut rect = RECT::default();
        unsafe { GetClientRect(self.taskbar_hwnd, &mut rect) }?;
        Ok(rect)
    }

    const WORKSPACES_MARGIN: egui::Margin = egui::Margin::same(1);

    fn resize_host_to_rect(&mut self, rect: egui::Rect, ppp: f32) -> anyhow::Result<()> {
        let rect = rect + Self::WORKSPACES_MARGIN;
        let rect = rect * ppp;

        let width = rect.width() as i32;
        // height always matches the taskbar height
        let taskbar_rect = self.taskbar_client_rect()?;
        let height = taskbar_rect.bottom - taskbar_rect.top;

        let (curr_width, curr_height) = self.size;

        if curr_width != width || curr_height != height {
            self.size = (width, height);

            tracing::debug!("Resizing host to match content rect");

            unsafe { SetWindowPos(self.host, None, 0, 0, width, height, SWP_NOMOVE) }?;
        }
        Ok(())
    }

    fn start_host_dragging(&mut self) -> anyhow::Result<()> {
        // start dragging mode
        self.dragging_state = DraggingState::Started;

        // get host width and height
        let rect = self.host_client_rect()?;
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        // set the cursor position to the center of the host window
        let x = rect.left + width / 2;
        let y = rect.top + height / 2;
        let points = &mut [POINT { x, y }];
        unsafe { MapWindowPoints(Some(self.host), None, points) };
        unsafe { SetCursorPos(points[0].x, points[0].y)? };

        Ok(())
    }

    fn drag_host_window(&mut self) -> anyhow::Result<()> {
        // get current cursor pos
        let mut pos = POINT::default();
        unsafe { GetCursorPos(&mut pos) }?;

        let points = POINTS {
            x: pos.x as i16,
            y: pos.y as i16,
        };

        unsafe {
            // release cursor capture
            ReleaseCapture()?;

            // simulate left click on the host window invisible title bar
            PostMessageW(
                Some(self.host),
                WM_NCLBUTTONDOWN,
                WPARAM(HTCAPTION as _),
                LPARAM(&points as *const _ as _),
            )?;
        }

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
                r.top == y
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
        // if dragging
        if self.dragging_state == DraggingState::Started {
            // change the cursor
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);

            // start dragging on the first left click
            if ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary)) {
                self.dragging_state = DraggingState::Dragging;
                if let Err(e) = self.drag_host_window() {
                    tracing::error!("Failed to start host darggign: {e}");
                }
            }
        }

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

                    if ui.add(btn).clicked() && self.dragging_state == DraggingState::None {
                        crate::komorebi::change_workspace(workspace.idx);
                    }
                }
            })
        })
        .response
    }

    fn transparent_panel(&self, ctx: &egui::Context) -> egui::CentralPanel {
        let color = if self.dragging_state != DraggingState::None {
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 125)
        } else {
            egui::Color32::TRANSPARENT
        };

        let visuals = egui::Visuals {
            panel_fill: color,
            ..egui::Visuals::dark()
        };
        ctx.set_visuals(visuals);

        let frame = egui::Frame::central_panel(&ctx.style()).inner_margin(Self::WORKSPACES_MARGIN);

        egui::CentralPanel::default().frame(frame)
    }
}

impl EguiView for MainWindowView {
    fn handle_window_event(
        &mut self,
        _ctx: &egui::Context,
        _event_loop: &ActiveEventLoop,
        event: winit::event::WindowEvent,
    ) -> anyhow::Result<()> {
        // exit dragging mode on cursor enter if we are in dragging mode
        // this is done here so the dragging mode background
        // doesn't change until dragging is over
        if self.dragging_state == DraggingState::Dragging
            && matches!(event, WindowEvent::CursorEntered { .. })
        {
            self.dragging_state = DraggingState::None;
        }

        Ok(())
    }

    fn handle_app_message(
        &mut self,
        ctx: &egui::Context,
        _event_loop: &ActiveEventLoop,
        message: &AppMessage,
    ) -> anyhow::Result<()> {
        match message {
            AppMessage::UpdateWorkspaces(workspaces) => self.workspaces = workspaces.clone(),
            AppMessage::MenuEvent(e) if e.id() == Self::M_MOVE_ID => self.start_host_dragging()?,
            AppMessage::MenuEvent(e) if e.id() == Self::M_QUIT_ID => self.close_host()?,
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
