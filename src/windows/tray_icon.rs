use tray_icon::TrayIconBuilder;

use crate::windows::context_menu::AppContextMenu;

#[allow(dead_code)]
pub struct TrayIcon {
    #[allow(unused)]
    icon: tray_icon::TrayIcon,
    context_menu: AppContextMenu,
}

impl TrayIcon {
    pub fn new(context_menu: AppContextMenu) -> anyhow::Result<Self> {
        let icon = tray_icon::Icon::from_resource(1, Some((32, 32)))?;

        TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip(std::env!("CARGO_PKG_NAME"))
            .with_menu(Box::new(context_menu.menu.clone()))
            .build()
            .map_err(Into::into)
            .map(|icon| Self { icon, context_menu })
    }
}
