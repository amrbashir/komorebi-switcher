use std::collections::HashMap;

use anyhow::Context;
use windows_registry::CURRENT_USER;

use crate::config::{Config, MonitorConfig};

#[cfg(debug_assertions)]
const APP_REG_KEY: &str = "SOFTWARE\\amrbashir\\komorebi-switcher-debug";
#[cfg(not(debug_assertions))]
const APP_REG_KEY: &str = "SOFTWARE\\amrbashir\\komorebi-switcher";

impl Config {
    pub fn migrate_from_registry() -> anyhow::Result<HashMap<String, MonitorConfig>> {
        use windows_registry::CURRENT_USER;

        let key = CURRENT_USER.create(APP_REG_KEY)?;
        let subkeys: Vec<String> = key.keys()?.collect();

        let mut monitors = HashMap::new();
        for monitor_id in subkeys {
            let subkey = key.open(&monitor_id)?;

            #[inline]
            fn get_int(k: &windows_registry::Key, key: &str, default: i32) -> i32 {
                k.get_string(key)
                    .map_err(|e| anyhow::anyhow!(e))
                    .and_then(|v| v.parse::<i32>().map_err(Into::into))
                    .unwrap_or(default)
            }

            let x = get_int(&subkey, "window-pos-x", 0);
            let y = get_int(&subkey, "window-pos-y", 0);
            let width = get_int(&subkey, "window-size-width", 200);
            let height = get_int(&subkey, "window-size-height", 40);
            let auto_width = subkey.get_u32("window-size-auto-width").unwrap_or(1) != 0;
            let auto_height = subkey.get_u32("window-size-auto-height").unwrap_or(1) != 0;

            monitors.insert(
                monitor_id,
                MonitorConfig {
                    x,
                    y,
                    width,
                    height,
                    auto_width,
                    auto_height,
                    show_layout_button: None,
                    hide_empty_workspaces: None,
                },
            );
        }

        Ok(monitors)
    }
}

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
    let key = CURRENT_USER.open(format!("{APP_REG_KEY}\\monitors\\{}", monitor_id));

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
        .create(format!("{APP_REG_KEY}\\monitors\\{}", monitor_id))
        .context("failed to open registry key")?;

    key.set_u32(WINDOW_SIZE_LAST_WIDTH, width as u32)?;
    key.set_u32(WINDOW_SIZE_LAST_HEIGHT, height as u32)?;

    Ok(())
}
