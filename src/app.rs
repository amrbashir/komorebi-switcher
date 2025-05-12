use windows::Win32::Foundation::HWND;
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

use crate::egui_glue::EguiWindow;
use crate::multi_map::MultiMap;
use crate::utils::RECTExt;
use crate::window_registry_info::WindowRegistryInfo;

#[derive(Debug, Clone)]
pub enum AppMessage {
    UpdateKomorebiState(crate::komorebi::State),
    MenuEvent(muda::MenuEvent),
    SystemSettingsChanged,
    DpiChanged,
    StartMoveResize(String),
    CreateResizeWindow {
        host: isize,
        info: WindowRegistryInfo,
        subkey: String,
        window_id: WindowId,
    },
    CloseWindow(WindowId),
    NotifyWindowInfoChanges(WindowId, WindowRegistryInfo),
}

pub struct App {
    pub wgpu_instance: wgpu::Instance,
    pub proxy: EventLoopProxy<AppMessage>,
    pub windows: MultiMap<WindowId, Option<String>, EguiWindow>,
    pub tray_icon: Option<crate::tray_icon::TrayIcon>,
    pub komorebi_state: crate::komorebi::State,
}

impl App {
    pub fn new(proxy: EventLoopProxy<AppMessage>) -> anyhow::Result<Self> {
        let wgpu_instance = egui_wgpu::wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });

        let tray_icon = crate::tray_icon::TrayIcon::new(proxy.clone()).ok();

        let komorebi_state = crate::komorebi::read_state().unwrap_or_default();

        {
            let proxy = proxy.clone();
            std::thread::spawn(move || crate::komorebi::listen_for_state(proxy));
        }

        Ok(Self {
            wgpu_instance,
            windows: Default::default(),
            proxy,
            tray_icon,
            komorebi_state,
        })
    }

    fn create_switchers(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let taskbars = crate::taskbar::all();

        tracing::debug!("Found {} taskbars: {taskbars:?}", taskbars.len());

        for monitor in self.komorebi_state.monitors.clone().into_iter() {
            // skip already existing window for this monitor
            let monitor_id = monitor.id.clone();
            if self.windows.contains_key_alt(&Some(monitor_id.clone())) {
                continue;
            }

            let Some(taskbar) = taskbars.iter().find(|tb| monitor.rect.contains(&tb.rect)) else {
                tracing::warn!(
                    "Failed to find taskbar for monitor: {}-{} {:?}",
                    monitor.name,
                    monitor.id,
                    monitor.rect
                );
                continue;
            };

            tracing::info!(
                "Creating switcher window for monitor: {}-{} {:?}",
                monitor.name,
                monitor.id,
                monitor.rect,
            );

            let window = self.create_switcher_window(event_loop, *taskbar, monitor)?;

            self.windows.insert(window.id(), Some(monitor_id), window);
        }

        Ok(())
    }

    fn handle_app_message(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: &AppMessage,
    ) -> anyhow::Result<()> {
        match event {
            AppMessage::CreateResizeWindow {
                host,
                info,
                subkey,
                window_id,
            } => self.create_resize_window(
                event_loop,
                *window_id,
                HWND(*host as _),
                *info,
                subkey.clone(),
            )?,

            AppMessage::CloseWindow(window_id) => {
                self.windows.remove(window_id);
            }

            AppMessage::UpdateKomorebiState(state) => {
                // Update the komorebi state
                self.komorebi_state = state.clone();

                // Create switcher windows for new monitors if needed
                self.create_switchers(event_loop)?;

                // Remove the windows for monitors that no longer exist
                self.windows.retain(|_, key, _| {
                    let Some(key) = key else {
                        return true;
                    };

                    let monitor = state.monitors.iter().any(|m| &m.id == key);

                    if !monitor {
                        tracing::info!("Removing switcher window for {key}");
                    }

                    monitor
                });

                // Update tray icon
                if let Some(tray) = &mut self.tray_icon {
                    tray.destroy_items_for_switchers()?;

                    let switchers_ids = self
                        .windows
                        .iter()
                        .filter_map(|(_, (key, _))| key.clone())
                        .collect::<Vec<_>>();
                    tray.create_items_for_switchers(switchers_ids)?;
                }
            }

            _ => {}
        }

        Ok(())
    }
}

impl ApplicationHandler<AppMessage> for App {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if cause == StartCause::Init {
            if let Err(e) = self.create_switchers(event_loop) {
                tracing::error!("Error while creating switchers: {e}");
            };

            if let Some(tray) = &mut self.tray_icon {
                let switchers_ids = self
                    .windows
                    .iter()
                    .filter_map(|(_, (key, _))| key.clone())
                    .collect::<Vec<_>>();
                if let Err(e) = tray.create_items_for_switchers(switchers_ids) {
                    tracing::error!("Error while creating tray items for switchers: {e}");
                }
            }
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
        if event == WindowEvent::Destroyed {
            tracing::info!("Window {window_id:?} destroyed");
            self.windows.remove(&window_id);
            return;
        }

        if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
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
