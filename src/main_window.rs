use std::num::NonZero;
use std::sync::Arc;

use muda::dpi::PhysicalPosition;
use muda::ContextMenu;
use raw_window_handle::{HasWindowHandle, RawWindowHandle, Win32WindowHandle};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::MapWindowPoints;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
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

        let parent = unsafe { NonZero::new_unchecked(self.host.0 as _) };
        let parent = Win32WindowHandle::new(parent);
        let parent = RawWindowHandle::Win32(parent);

        attrs = unsafe { attrs.with_parent_window(Some(parent)) };
        attrs = attrs
            .with_decorations(false)
            .with_transparent(true)
            .with_active(false)
            .with_class_name("komorebi-switcher::window")
            .with_undecorated_shadow(false)
            .with_clip_children(false);

        let window = event_loop.create_window(attrs)?;
        let window = Arc::new(window);

        let state = MainWindowView::new(window.clone(), self.taskbar_hwnd, self.host)?;

        let proxy = self.proxy.clone();

        std::thread::spawn(move || listen_for_workspaces(proxy));

        let window = EguiWindow::new(window, &self.wgpu_instance, state);

        self.windows.insert(window.id(), window);

        Ok(())
    }
}

pub struct MainWindowView {
    window: Arc<Window>,
    hwnd: HWND,
    taskbar_hwnd: HWND,
    host: HWND,
    curr_width: i32,
    workspaces: Vec<crate::komorebi::Workspace>,
    context_menu: muda::Menu,
    is_dragging: bool,
}

impl MainWindowView {
    fn new(window: Arc<Window>, taskbar_hwnd: HWND, host: HWND) -> anyhow::Result<Self> {
        let workspaces = crate::komorebi::read_workspaces().unwrap_or_default();

        let context_menu = muda::Menu::with_items(&[
            &muda::MenuItem::with_id("move", "Move", true, None),
            &muda::MenuItem::with_id("quit", "Quit", true, None),
        ])?;

        let hwnd = window.window_handle()?;
        let RawWindowHandle::Win32(hwnd) = hwnd.as_raw() else {
            unreachable!("Window handle must be win32")
        };
        let hwnd = HWND(hwnd.hwnd.get() as _);

        Ok(Self {
            hwnd,
            window,
            host,
            taskbar_hwnd,
            curr_width: 0,
            workspaces,
            context_menu,
            is_dragging: false,
        })
    }

    fn host_rect(&self) -> anyhow::Result<RECT> {
        let mut rect = RECT::default();
        unsafe { GetClientRect(self.host, &mut rect) }?;
        Ok(rect)
    }

    fn resize_host_to_rect(&mut self, rect: egui::Rect) {
        let width = rect.width() as f64 + 2.0 /* default margin 1 on each side */;
        let width = self.window.scale_factor() * width;
        let width = width as i32;

        if width != self.curr_width {
            self.curr_width = width;

            if let Ok(rect) = self.host_rect() {
                let _ = unsafe {
                    SetWindowPos(
                        self.host,
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

    fn focus_egui_window(&self) -> anyhow::Result<()> {
        // loop until we get focus
        let mut counter = 0;
        while let Err(err) = unsafe { SetFocus(Some(self.hwnd)) } {
            counter += 1;
            std::thread::sleep(std::time::Duration::from_millis(100));

            if counter >= 3 {
                return Err(err.into());
            }
        }

        Ok(())
    }

    fn start_host_dragging(&mut self) -> anyhow::Result<()> {
        if self.is_dragging {
            return Ok(());
        }

        self.is_dragging = true;

        let _ = unsafe { SetForegroundWindow(self.hwnd) };

        self.focus_egui_window()?;

        unsafe { SetCapture(self.hwnd) };

        let rect = self.host_rect()?;
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        let x = rect.left + width / 2;
        let y = rect.top + height / 2;

        let points = &mut [POINT { x, y }];
        unsafe { MapWindowPoints(Some(self.host), None, points) };

        unsafe { SetCursorPos(points[0].x, points[0].y)? };

        Ok(())
    }

    fn move_host(&self, position: PhysicalPosition<f64>) -> anyhow::Result<()> {
        let points = &mut [POINT {
            x: position.x as _,
            y: position.y as _,
        }];

        unsafe { MapWindowPoints(Some(self.host), Some(self.taskbar_hwnd), points) };

        unsafe {
            SetWindowPos(
                self.host,
                Some(HWND_TOP),
                points[0].x - self.curr_width / 2,
                0,
                0,
                0,
                SWP_NOSIZE,
            )
            .map_err(Into::into)
        }
    }

    fn stop_host_dragging(&mut self) -> anyhow::Result<()> {
        if self.is_dragging {
            unsafe { ReleaseCapture()? };
            self.is_dragging = false;
        }

        Ok(())
    }

    fn close_host(&self) -> anyhow::Result<()> {
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

    fn show_context_menu(&self) {
        let hwnd = self.host.0 as _;
        unsafe { self.context_menu.show_context_menu_for_hwnd(hwnd, None) };
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

        let stroke_width = 1.5;

        style.visuals.widgets.inactive = egui::style::WidgetVisuals {
            bg_fill: fill_color,
            weak_bg_fill: fill_color,
            bg_stroke: egui::Stroke {
                width: stroke_width,
                color: inactive_border_color,
            },
            ..style.visuals.widgets.hovered
        };

        style.visuals.widgets.hovered = egui::style::WidgetVisuals {
            bg_fill: hover_color,
            weak_bg_fill: hover_color,
            bg_stroke: egui::Stroke {
                width: stroke_width,
                color: active_border_color,
            },
            ..style.visuals.widgets.hovered
        };

        let btn = egui::Button::new(&workspace.name)
            .min_size(egui::vec2(24., 24.))
            .corner_radius(2);

        ui.add(btn)
    }

    fn should_show_context_menu(&self, ui: &mut egui::Ui) -> bool {
        !self.is_dragging && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary))
    }

    fn workspaces_row(&mut self, ui: &mut egui::Ui) -> egui::InnerResponse<()> {
        ui.horizontal_centered(|ui| {
            for workspace in self.workspaces.iter() {
                if Self::workspace_button(workspace, ui).clicked() && !self.is_dragging {
                    let _ = crate::komorebi::change_workspace(workspace.idx);
                }
            }
        })
    }
}

fn is_escape_key(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::KeyboardInput {
            event: KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::Escape),
                state: ElementState::Released,
                ..
            },
            ..
        },
    )
}

impl EguiView for MainWindowView {
    fn handle_window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CursorMoved { position, .. } if self.is_dragging => {
                let _ = self.move_host(position);
            }

            _ if is_escape_key(&event) && self.is_dragging => {
                let _ = self.stop_host_dragging();
                self.window.request_redraw();
            }

            _ => {}
        }
    }

    fn handle_app_message(&mut self, _event_loop: &ActiveEventLoop, message: &AppMessage) {
        match message {
            AppMessage::UpdateWorkspaces(workspaces) => self.workspaces = workspaces.clone(),

            AppMessage::MenuEvent(e) if e.id() == "move" => {
                if self.start_host_dragging().is_err() {
                    let _ = unsafe { ReleaseCapture() };
                    self.is_dragging = false;
                }
            }

            AppMessage::MenuEvent(e) if e.id() == "quit" => {
                let _ = self.close_host();
            }

            _ => {}
        }
    }

    fn update(&mut self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::default();

        if !self.is_dragging {
            visuals.panel_fill = egui::Color32::TRANSPARENT;
        } else {
            ctx.set_cursor_icon(egui::CursorIcon::ResizeColumn);
        };

        ctx.set_visuals(visuals);

        let margin = egui::Margin::symmetric(1, 0);

        let frame = egui::Frame::central_panel(&ctx.style()).inner_margin(margin);
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            if self.should_show_context_menu(ui) {
                self.show_context_menu();
            }

            let response = self.workspaces_row(ui);

            self.resize_host_to_rect(response.response.rect);
        });
    }
}
