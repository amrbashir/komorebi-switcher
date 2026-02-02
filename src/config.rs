use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

fn default_width() -> i32 {
    200
}

fn default_height() -> i32 {
    40
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MonitorConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_layout_button: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide_empty_workspaces: Option<bool>,

    #[serde(default = "default_true")]
    pub auto_width: bool,
    #[serde(default = "default_true")]
    pub auto_height: bool,

    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,

    #[serde(default = "default_width")]
    pub width: i32,
    #[serde(default = "default_height")]
    pub height: i32,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            show_layout_button: None,
            hide_empty_workspaces: None,
            auto_width: true,
            auto_height: true,
            x: 0,
            y: 0,
            width: default_width(),
            height: default_height(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub show_layout_button: bool,
    #[serde(default)]
    pub hide_empty_workspaces: bool,

    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub monitors: HashMap<String, MonitorConfig>,
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
                if !migrated.is_empty() {
                    config.monitors = migrated;
                    config.save()?;
                }
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

    #[allow(dead_code)]
    pub fn get_monitor(&self, monitor_id: &str) -> MonitorConfig {
        self.monitors.get(monitor_id).copied().unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn get_monitor_or_default(&mut self, monitor_id: &str) -> &mut MonitorConfig {
        self.monitors.entry(monitor_id.to_string()).or_default()
    }

    #[allow(dead_code)]
    pub fn set_monitor(&mut self, monitor_id: &str, config: MonitorConfig) {
        self.monitors.insert(monitor_id.to_string(), config);
    }
}

fn default_true() -> bool {
    true
}
