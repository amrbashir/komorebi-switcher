use windows::Win32::Foundation::HWND;
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

use crate::config::WindowConfig;
use crate::windows::egui_glue::EguiWindow;
use crate::windows::utils::{HwndWithDrop, MultiMap};

#[derive(Debug, Clone)]
pub enum AppMessage {
    UpdateKomorebiState(crate::komorebi::State),
    MenuEvent(muda::MenuEvent),
    SystemSettingsChanged,
    DpiChanged,
    StartMoveResize(String),
    CreateResizeWindow {
        host: isize,
        info: WindowConfig,
        monitor_id: String,
        window_id: WindowId,
    },
    CloseWindow(WindowId),
    NotifyWindowInfoChanges(WindowId, WindowConfig),
    RecreateSwitcherWindows,
    TaskbarRecreated,
    UpdateWindowConfig {
        monitor_id: String,
        config: WindowConfig,
    },
}

pub struct App {
    pub wgpu_instance: wgpu::Instance,
    pub proxy: EventLoopProxy<AppMessage>,
    pub windows: MultiMap<WindowId, Option<String>, EguiWindow>,
    pub tray_icon: Option<crate::windows::tray_icon::TrayIcon>,
    pub komorebi_state: crate::komorebi::State,
    #[allow(unused)]
    pub message_window: HwndWithDrop,
    pub config: crate::config::Config,
}

impl App {
    pub fn new(proxy: EventLoopProxy<AppMessage>) -> anyhow::Result<Self> {
        let wgpu_instance = egui_wgpu::wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });

        let tray_icon = crate::windows::tray_icon::TrayIcon::new(proxy.clone()).ok();

        let komorebi_state = crate::komorebi::read_state().unwrap_or_default();

        let message_window = unsafe { crate::windows::message_window::create(proxy.clone())? };
        let message_window = HwndWithDrop(message_window);

        let config = crate::config::Config::load()?;

        // Start listening for komorebi state changes
        {
            let proxy = proxy.clone();
            std::thread::spawn(move || {
                crate::komorebi::listen_for_state(move |new_state| {
                    if let Err(e) = proxy.send_event(AppMessage::UpdateKomorebiState(new_state)) {
                        tracing::error!("Failed to send komorebi state update: {e}");
                    }
                })
            });
        }

        Ok(Self {
            wgpu_instance,
            windows: Default::default(),
            proxy,
            tray_icon,
            komorebi_state,
            message_window,
            config,
        })
    }

    fn create_switchers(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let taskbars = crate::windows::taskbar::all();

        tracing::debug!("Found {} taskbars: {taskbars:?}", taskbars.len());

        for monitor in self.komorebi_state.monitors.clone().into_iter() {
            // skip already existing window for this monitor
            let monitor_id = monitor.id.clone();
            if self.windows.contains_key_alt(&Some(monitor_id.clone())) {
                continue;
            }

            let Some(taskbar) = taskbars.iter().find(|tb| monitor.rect.contains(tb.rect)) else {
                tracing::warn!(
                    "Failed to find taskbar for monitor: {}-{} {:?}",
                    monitor.name,
                    monitor.id,
                    monitor.rect
                );
                continue;
            };

            tracing::info!(
                "Creating switcher window for monitor: {}-{} {:?} on taskbar: {:?}",
                monitor.name,
                monitor.id,
                monitor.rect,
                taskbar.hwnd
            );

            let window = self.create_switcher_window(event_loop, *taskbar, monitor)?;

            self.windows.insert(window.id(), Some(monitor_id), window);
        }

        Ok(())
    }

    fn switcher_menu_ids(&self) -> Vec<String> {
        self.komorebi_state
            .monitors
            .iter()
            .filter(|m| self.windows.contains_key_alt(&Some(m.id.clone())))
            .map(|m| format!("{}-{}", m.name, m.id))
            .collect::<Vec<_>>()
    }

    fn recreate_tray_menu_items(&mut self) -> anyhow::Result<()> {
        let switchers_ids = self.switcher_menu_ids();

        if let Some(tray) = &mut self.tray_icon {
            tray.destroy_items_for_switchers()?;
            tray.create_items_for_switchers(switchers_ids)?;
        }

        Ok(())
    }

    fn handle_app_message(
        &mut self,
        event_loop: &ActiveEventLoop,
        message: &AppMessage,
    ) -> anyhow::Result<()> {
        match message {
            AppMessage::CreateResizeWindow {
                host,
                info,
                monitor_id,
                window_id,
            } => {
                let host = HWND(*host as _);
                self.create_resize_window(event_loop, *window_id, host, *info, monitor_id.clone())?
            }

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

                // Update tray icon menu items
                self.recreate_tray_menu_items()?;
            }

            AppMessage::RecreateSwitcherWindows | AppMessage::TaskbarRecreated => {
                tracing::info!("Received {message:?}, closing and recreating all switchers");

                // Close all existing switcher windows
                self.windows.clear();

                // Recreate switchers with new taskbar windows
                self.create_switchers(event_loop)?;

                // Update tray icon menu items
                self.recreate_tray_menu_items()?;
            }

            AppMessage::UpdateWindowConfig { monitor_id, config } => {
                self.config.set(monitor_id.clone(), *config);
                self.config.save()?;
            }

            _ => {}
        }

        Ok(())
    }
}

impl ApplicationHandler<AppMessage> for App {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if cause == StartCause::Init {
            // On Init, create switcher windows for all monitors
            if let Err(e) = self.create_switchers(event_loop) {
                tracing::error!("Error while creating switchers: {e}");
            };

            // Then, create tray icon menu items for switchers
            let switchers_ids = self.switcher_menu_ids();
            if let Some(tray) = &mut self.tray_icon {
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
