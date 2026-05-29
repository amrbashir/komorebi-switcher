use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedMonitorConfig {
    pub show_layout_button: bool,
    pub hide_empty_workspaces: bool,
    pub highlight_focused_workspace: bool,
    pub font_family: Option<String>,
    pub font_weight: u16,
    pub active_indicator: Option<String>,
    pub busy_indicator: Option<String>,
    pub auto_width: bool,
    pub auto_height: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

fn default_width() -> i32 {
    200
}

fn default_height() -> i32 {
    40
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColorsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_indicator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub busy_indicator: Option<String>,
}

impl ColorsConfig {
    pub fn is_empty(&self) -> bool {
        self.active_indicator.is_none() && self.busy_indicator.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_layout_button: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide_empty_workspaces: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight_focused_workspace: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<u16>,

    #[serde(default, skip_serializing_if = "ColorsConfig::is_empty")]
    pub colors: ColorsConfig,

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
            highlight_focused_workspace: None,
            font_family: None,
            font_weight: None,
            colors: ColorsConfig::default(),
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

    #[serde(default = "default_true")]
    pub highlight_focused_workspace: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<u16>,

    #[serde(default, skip_serializing_if = "ColorsConfig::is_empty")]
    pub colors: ColorsConfig,

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
        self.monitors.get(monitor_id).cloned().unwrap_or_default()
    }

    pub fn get_monitor_mut(&mut self, monitor_id: &str) -> &mut MonitorConfig {
        self.monitors.entry(monitor_id.to_string()).or_default()
    }

    #[allow(dead_code)]
    pub fn set_monitor(&mut self, monitor_id: &str, config: MonitorConfig) {
        self.monitors.insert(monitor_id.to_string(), config);
    }

    pub fn resolved_monitor_config(&self, monitor_id: &str) -> ResolvedMonitorConfig {
        let mc = self.get_monitor(monitor_id);

        let show_layout_button = mc.show_layout_button.unwrap_or(self.show_layout_button);
        let hide_empty_workspaces = mc
            .hide_empty_workspaces
            .unwrap_or(self.hide_empty_workspaces);

        let highlight_focused_workspace = mc
            .highlight_focused_workspace
            .unwrap_or(self.highlight_focused_workspace);

        let font_family = mc.font_family.or(self.font_family.clone());
        let font_weight = mc.font_weight.or(self.font_weight).unwrap_or(400);

        let active_indicator = mc
            .colors
            .active_indicator
            .or(self.colors.active_indicator.clone());
        let busy_indicator = mc
            .colors
            .busy_indicator
            .or(self.colors.busy_indicator.clone());

        ResolvedMonitorConfig {
            show_layout_button,
            hide_empty_workspaces,
            highlight_focused_workspace,
            font_family,
            font_weight,
            active_indicator,
            busy_indicator,
            auto_width: mc.auto_width,
            auto_height: mc.auto_height,
            x: mc.x,
            y: mc.y,
            width: mc.width,
            height: mc.height,
        }
    }
}

fn default_true() -> bool {
    true
}
