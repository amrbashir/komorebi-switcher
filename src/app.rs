use std::collections::HashMap;

use windows::Win32::Foundation::HWND;
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

use crate::egui_glue::EguiWindow;

pub enum AppMessage {
    UpdateWorkspaces(Vec<crate::komorebi::Workspace>),
    MenuEvent(muda::MenuEvent),
    SystemSettingsChanged,
    DpiChanged,
}

pub struct App {
    pub wgpu_instance: wgpu::Instance,
    pub proxy: EventLoopProxy<AppMessage>,
    pub taskbar_hwnd: HWND,
    pub windows: HashMap<WindowId, EguiWindow>,
}

impl App {
    pub fn new(taskbar_hwnd: HWND, proxy: EventLoopProxy<AppMessage>) -> Self {
        let wgpu_instance = egui_wgpu::wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });

        Self {
            wgpu_instance,
            taskbar_hwnd,
            windows: Default::default(),
            proxy,
        }
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
        for window in self.windows.values_mut() {
            let ctx = window.surface.egui_renderer.egui_ctx();
            if let Err(e) = window.view.handle_app_message(ctx, event_loop, &event) {
                tracing::error!("Error while handling AppMessage: {e}")
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
            tracing::info!("Closing main window");
            self.windows.remove(&window_id);
            tracing::info!("Exiting event loop");
            event_loop.exit();
        }

        let Some(window) = self.windows.get_mut(&window_id) else {
            return;
        };

        if let Err(e) = window.handle_window_event(event_loop, event) {
            tracing::error!("Error while handing `WindowEevent`: {e}")
        }
    }
}
