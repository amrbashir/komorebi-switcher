use std::collections::HashMap;

use windows::Win32::Foundation::HWND;
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

use crate::egui_glue::EguiWindow;
use crate::window_registry_info::WindowRegistryInfo;

#[derive(Debug, Clone)]
pub enum AppMessage {
    UpdateWorkspaces(Vec<crate::komorebi::Workspace>),
    MenuEvent(muda::MenuEvent),
    SystemSettingsChanged,
    DpiChanged,
    StartMoveResize,
    CreateResizeWindow {
        host: isize,
        info: WindowRegistryInfo,
    },
    CloseWindow(WindowId),
    NotifyWindowInfoChanges(WindowRegistryInfo),
}

pub struct App {
    pub wgpu_instance: wgpu::Instance,
    pub proxy: EventLoopProxy<AppMessage>,
    pub taskbar_hwnd: HWND,
    pub windows: HashMap<WindowId, EguiWindow>,
    pub tray_icon: Option<crate::tray_icon::TrayIcon>,
}

impl App {
    pub fn new(taskbar_hwnd: HWND, proxy: EventLoopProxy<AppMessage>) -> anyhow::Result<Self> {
        let wgpu_instance = egui_wgpu::wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });

        let tray_icon = crate::tray_icon::TrayIcon::new(proxy.clone()).ok();

        Ok(Self {
            wgpu_instance,
            taskbar_hwnd,
            windows: Default::default(),
            proxy,
            tray_icon,
        })
    }

    fn handle_app_message(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: &AppMessage,
    ) -> anyhow::Result<()> {
        match event {
            AppMessage::CreateResizeWindow { host, info } => {
                self.create_resize_window(event_loop, HWND(*host as _), *info)?
            }

            AppMessage::CloseWindow(window_id) => {
                self.windows.remove(window_id);
            }

            _ => {}
        }

        Ok(())
    }
}

impl ApplicationHandler<AppMessage> for App {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if cause == StartCause::Init {
            self.create_main_window(event_loop).unwrap_or_else(|e| {
                tracing::error!("Failed to create main window: {e}");
                std::process::exit(1);
            });
        }
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppMessage) {
        if let Err(e) = self.handle_app_message(event_loop, &event) {
            tracing::error!("Error while handling AppMessage: {e}")
        }

        if let Some(tray) = &self.tray_icon {
            if let Err(e) = tray.handle_app_message(event_loop, &event) {
                tracing::error!("Error while handling AppMessage for tray: {e}")
            }
        }

        for window in self.windows.values_mut() {
            let ctx = window.surface.egui_renderer.egui_ctx();
            if let Err(e) = window.view.handle_app_message(ctx, event_loop, &event) {
                tracing::error!("Error while handling AppMessage for window: {e}")
            }

            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if event == WindowEvent::CloseRequested {
            tracing::info!("Closing window {window_id:?}");
            self.windows.remove(&window_id);

            if self.windows.is_empty() {
                tracing::info!("Exiting event loop");
                event_loop.exit();
            }
        }

        let Some(window) = self.windows.get_mut(&window_id) else {
            return;
        };

        if let Err(e) = window.handle_window_event(event_loop, event) {
            tracing::error!("Error while handing `WindowEevent`: {e}")
        }
    }
}
