use anyhow::Context;
use windows_registry::CURRENT_USER;

use crate::config::MonitorConfig;

#[cfg(debug_assertions)]
const APP_REG_KEY: &str = "SOFTWARE\\amrbashir\\komorebi-switcher-debug";
#[cfg(not(debug_assertions))]
const APP_REG_KEY: &str = "SOFTWARE\\amrbashir\\komorebi-switcher";

impl MonitorConfig {
	pub fn adjust_for_auto_size(&mut self, monitor_id: &str) -> anyhow::Result<()> {
		if self.auto_width || self.auto_height {
			if let Some((stored_w, stored_h)) = get_auto_size(monitor_id) {
				if self.auto_width {
					self.width = stored_w;
				}
				if self.auto_height {
					self.height = stored_h;
				}
			}
		}

		Ok(())
	}
}

const WINDOW_SIZE_LAST_WIDTH: &str = "window-size-last-width";
const WINDOW_SIZE_LAST_HEIGHT: &str = "window-size-last-height";

fn get_auto_size(monitor_id: &str) -> Option<(i32, i32)> {
	let key = CURRENT_USER.open(format!("{APP_REG_KEY}\\{monitor_id}"));

	let key = match key {
		Ok(key) => key,
		Err(_) => return None,
	};

	let width: u32 = key.get_u32(WINDOW_SIZE_LAST_WIDTH).unwrap_or(200);
	let height: u32 = key.get_u32(WINDOW_SIZE_LAST_HEIGHT).unwrap_or(40);

	Some((width as i32, height as i32))
}

pub fn store_auto_size(monitor_id: &str, width: i32, height: i32) -> anyhow::Result<()> {
	let key = CURRENT_USER
		.create(format!("{APP_REG_KEY}\\{monitor_id}"))
		.context("failed to open registry key")?;

	key.set_u32(WINDOW_SIZE_LAST_WIDTH, width as u32)?;
	key.set_u32(WINDOW_SIZE_LAST_HEIGHT, height as u32)?;

	Ok(())
}
