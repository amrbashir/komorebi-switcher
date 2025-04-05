use tray_icon::menu::{Menu, MenuItem};
use tray_icon::TrayIconBuilder;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};

use crate::app::AppMessage;

pub struct TrayIcon {
    #[allow(unused)]
    icon: tray_icon::TrayIcon,
    proxy: EventLoopProxy<AppMessage>,
}

impl TrayIcon {
    /// The ID of the "Quit" menu item.
    const M_QUIT_ID: &str = "quit";
    /// The ID of the "Move & Resize" menu item.
    const M_MOVE_RESIZE_ID: &str = "move-resize";

    pub fn new(proxy: EventLoopProxy<AppMessage>) -> anyhow::Result<Self> {
        let icon = tray_icon::Icon::from_resource(1, Some((32, 32)))?;

        let quit = MenuItem::with_id(Self::M_QUIT_ID, "Quit", true, None);
        let move_resize = MenuItem::with_id(Self::M_MOVE_RESIZE_ID, "Move & Resize", true, None);
        let menu = Menu::with_items(&[&move_resize, &quit])?;

        TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip(std::env!("CARGO_PKG_NAME"))
            .with_menu(Box::new(menu.clone()))
            .build()
            .map_err(Into::into)
            .map(|icon| Self { icon, proxy })
    }

    pub fn handle_app_message(
        &self,
        event_loop: &ActiveEventLoop,
        event: &AppMessage,
    ) -> anyhow::Result<()> {
        match event {
            AppMessage::MenuEvent(event) if event.id() == Self::M_QUIT_ID => event_loop.exit(),
            AppMessage::MenuEvent(event) if event.id() == Self::M_MOVE_RESIZE_ID => {
                self.proxy.send_event(AppMessage::StartMoveResize)?
            }
            _ => {}
        }

        Ok(())
    }
}
