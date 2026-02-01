use muda::{Menu, PredefinedMenuItem};
use tray_icon::menu::MenuItem;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};

use crate::windows::app::AppMessage;

#[derive(Clone)]
pub struct AppContextMenu {
    pub proxy: EventLoopProxy<AppMessage>,
    pub menu: Menu,
    pub settings: MenuItem,
    pub refresh: MenuItem,
    pub quit: MenuItem,
}

impl AppContextMenu {
    pub fn new(proxy: EventLoopProxy<AppMessage>) -> anyhow::Result<Self> {
        let settings = MenuItem::new("Settings", true, None);
        let refresh = MenuItem::new("Refresh", true, None);
        let separator = PredefinedMenuItem::separator();

        #[cfg(debug_assertions)]
        let title = concat!(env!("CARGO_PKG_NAME"), " (debug)");
        #[cfg(not(debug_assertions))]
        let title = env!("CARGO_PKG_NAME");

        let title = MenuItem::new(title, false, None);
        let version = MenuItem::new(concat!("v", env!("CARGO_PKG_VERSION")), false, None);
        let quit = MenuItem::new("Quit", true, None);

        let menu = Menu::with_items(&[
            &settings, &refresh, &separator, &title, &version, &separator, &quit,
        ])?;

        Ok(Self {
            proxy,
            menu,
            settings,
            refresh,
            quit,
        })
    }

    pub fn handle_app_message(
        &self,
        event_loop: &ActiveEventLoop,
        event: &AppMessage,
    ) -> anyhow::Result<()> {
        match event {
            AppMessage::MenuEvent(event) if *event.id() == self.settings.id() => {
                self.proxy.send_event(AppMessage::CreateSettingsWindow)?
            }
            AppMessage::MenuEvent(event) if *event.id() == self.refresh.id() => {
                self.proxy.send_event(AppMessage::RecreateSwitcherWindows)?
            }
            AppMessage::MenuEvent(event) if *event.id() == self.quit.id() => event_loop.exit(),
            _ => {}
        }

        Ok(())
    }
}
