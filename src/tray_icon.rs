use muda::Submenu;
use tray_icon::menu::{Menu, MenuItem};
use tray_icon::TrayIconBuilder;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};

use crate::app::AppMessage;

pub struct TrayIcon {
    #[allow(unused)]
    icon: tray_icon::TrayIcon,
    proxy: EventLoopProxy<AppMessage>,
    #[allow(unused)]
    menu: Menu,
    quit: MenuItem,
    move_resize: Submenu,
    move_resize_items: Vec<MenuItem>,
}

impl TrayIcon {
    pub fn new(proxy: EventLoopProxy<AppMessage>) -> anyhow::Result<Self> {
        let icon = tray_icon::Icon::from_resource(1, Some((32, 32)))?;

        let quit = MenuItem::new("Quit", true, None);
        let move_resize = Submenu::new("Move & Resize", true);
        let menu = Menu::with_items(&[&move_resize, &quit])?;

        TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip(std::env!("CARGO_PKG_NAME"))
            .with_menu(Box::new(menu.clone()))
            .build()
            .map_err(Into::into)
            .map(|icon| Self {
                icon,
                proxy,
                menu,
                quit,
                move_resize,
                move_resize_items: vec![],
            })
    }

    pub fn destroy_items_for_switchers(&mut self) -> anyhow::Result<()> {
        for item in &self.move_resize_items {
            self.move_resize.remove(item)?;
        }

        self.move_resize_items.clear();

        Ok(())
    }

    pub fn create_items_for_switchers(&mut self, switchers: Vec<String>) -> anyhow::Result<()> {
        for switcher in switchers {
            let item = MenuItem::with_id(&switcher, &switcher, true, None);
            self.move_resize.append(&item)?;
            self.move_resize_items.push(item);
        }

        Ok(())
    }

    pub fn handle_app_message(
        &self,
        event_loop: &ActiveEventLoop,
        event: &AppMessage,
    ) -> anyhow::Result<()> {
        match event {
            AppMessage::MenuEvent(event) if event.id() == self.quit.id() => event_loop.exit(),
            AppMessage::MenuEvent(event)
                if self
                    .move_resize_items
                    .iter()
                    .any(|item| item.id() == event.id()) =>
            {
                self.proxy
                    .send_event(AppMessage::StartMoveResize(event.id().as_ref().to_string()))?;
            }
            _ => {}
        }

        Ok(())
    }
}
