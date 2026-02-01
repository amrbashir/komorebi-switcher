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
    #[serde(default)]
    pub show_layout_button: Option<bool>,
    #[serde(default)]
    pub hide_empty_workspaces: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub monitors: HashMap<String, WindowConfig>,

    #[serde(default)]
    pub show_layout_button: bool,

    #[serde(default)]
    pub hide_empty_workspaces: bool,
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

            #[allow(unused_mut)]
            let mut config = Config::default();

            #[cfg(target_os = "windows")]
            {
                tracing::info!("Migrating config from Windows registry if any");

                let migrated = Self::migrate_from_registry()?;
                config.monitors = migrated;
            }

            config.save()?;

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

    #[allow(dead_code)]
    pub fn get_monitor(&self, monitor_id: &str) -> WindowConfig {
        self.monitors.get(monitor_id).copied().unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn get_monitor_or_default(&mut self, monitor_id: &str) -> &mut WindowConfig {
        self.monitors.entry(monitor_id.to_string()).or_default()
    }

    #[allow(dead_code)]
    pub fn set_monitor(&mut self, monitor_id: &str, config: WindowConfig) {
        self.monitors.insert(monitor_id.to_string(), config);
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
                    show_layout_button: None,
                    hide_empty_workspaces: None,
                },
            );
        }

        Ok(monitors)
    }
}
