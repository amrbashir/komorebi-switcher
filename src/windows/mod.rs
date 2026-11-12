use windows_reactor::App;

mod app;
mod menu;
mod message_window;
mod registry;
mod taskbar;
mod utils;
mod widgets;
#[allow(clippy::module_inception)]
mod windows;

pub fn run() -> anyhow::Result<()> {
	let config = crate::config::Config::load()?;
	let state = crate::komorebi::read_state().unwrap_or_default();

	App::run_application::<app::App>((config, state))?;

	Ok(())
}
