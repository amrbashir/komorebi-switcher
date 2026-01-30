use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct WindowConfig {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub auto_width: bool,
    pub auto_height: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub monitors: HashMap<String, WindowConfig>,
}

impl Config {
    #[cfg(debug_assertions)]
    pub const FILENAME: &'static str = "komorebi-switcher.debug.toml";
    #[cfg(not(debug_assertions))]
    pub const FILENAME: &'static str = "komorebi-switcher.toml";

    pub fn path() -> anyhow::Result<PathBuf> {
        dirs::home_dir()
            .context("Could not determine home directory")
            .map(|dir| dir.join(".config"))
            .map(|dir| dir.join(Self::FILENAME))
    }

    pub fn load() -> anyhow::Result<Self> {
        let config_file = Self::path()?;

        if config_file.exists() {
            tracing::info!("Loading config from {}", config_file.display());

            let content = std::fs::read_to_string(&config_file)?;
            toml::from_str(&content).map_err(Into::into)
        } else {
            tracing::info!(
                "Config file not found at {}, using default config",
                config_file.display()
            );

            let mut config = Config::default();
            #[cfg(target_os = "windows")]
            {
                tracing::info!("Migrating config from Windows registry if any");

                let migrated = Self::migrate_from_registry()?;
                config.monitors = migrated;
                config.save()?;
            }

            Ok(config)
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_file = Self::path()?;

        tracing::info!("Saving config to {}", config_file.display());

        if let Some(parent) = config_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&config_file, toml::to_string(self)?)?;
        Ok(())
    }

    pub fn get(&self, monitor_id: &str) -> Option<&WindowConfig> {
        self.monitors.get(monitor_id)
    }

    pub fn set(&mut self, monitor_id: String, config: WindowConfig) {
        self.monitors.insert(monitor_id, config);
    }
}

#[cfg(target_os = "windows")]
impl Config {
    fn migrate_from_registry() -> anyhow::Result<HashMap<String, WindowConfig>> {
        use windows_registry::CURRENT_USER;

        #[cfg(debug_assertions)]
        const APP_REG_KEY: &str = "SOFTWARE\\amrbashir\\komorebi-switcher-debug";
        #[cfg(not(debug_assertions))]
        const APP_REG_KEY: &str = "SOFTWARE\\amrbashir\\komorebi-switcher";

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
                WindowConfig {
                    x,
                    y,
                    width,
                    height,
                    auto_width,
                    auto_height,
                },
            );
        }

        Ok(monitors)
    }
}

#[cfg(target_os = "windows")]
impl WindowConfig {
    pub fn apply(&mut self, hwnd: windows::Win32::Foundation::HWND) -> anyhow::Result<()> {
        use windows::Win32::Foundation::*;
        use windows::Win32::UI::WindowsAndMessaging::*;

        let height = if self.auto_height {
            let parent = unsafe { GetParent(hwnd) }?;
            let mut rect = RECT::default();
            unsafe { GetClientRect(parent, &mut rect) }?;
            rect.bottom - rect.top
        } else {
            self.height
        };

        let width = if self.auto_width {
            let child = unsafe { GetWindow(hwnd, GW_CHILD) }?;
            let mut rect = RECT::default();
            unsafe { GetClientRect(child, &mut rect) }?;
            rect.right - rect.left
        } else {
            self.width
        };

        self.width = width;
        self.height = height;

        unsafe {
            SetWindowPos(
                hwnd,
                None,
                self.x,
                self.y,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            )
            .map_err(Into::into)
        }
    }
}
