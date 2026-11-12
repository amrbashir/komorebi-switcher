use windows_reactor::{ApplicationViewContext, Key, Menu, MenuItem};

use super::app::{App, AppMessage};

#[derive(Debug, Clone, Copy, strum::AsRefStr, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum MenuCommand {
	Settings,
	Refresh,
	Quit,
}

pub fn create(context: &ApplicationViewContext<App>) -> Menu {
	Menu::new(
		[
			MenuItem::item(MenuCommand::Settings.as_ref(), "Settings"),
			MenuItem::item(MenuCommand::Refresh.as_ref(), "Refresh"),
			MenuItem::separator("separator"),
			MenuItem::disabled("title", crate::APP_NAME),
			MenuItem::disabled("version", concat!("v", env!("CARGO_PKG_VERSION"))),
			MenuItem::separator("quit-separator"),
			MenuItem::item(MenuCommand::Quit.as_ref(), "Quit"),
		],
		context.callback(|key: Key| {
			let command = key
				.as_str()
				.expect("menu keys are strings")
				.parse::<MenuCommand>()
				.expect("menu only emits actionable item keys");

			AppMessage::MenuCommand(command)
		}),
	)
}
