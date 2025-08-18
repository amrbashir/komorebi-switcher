use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows_registry::CURRENT_USER;

#[cfg(debug_assertions)]
const APP_REG_KEY: &str = "SOFTWARE\\amrbashir\\komorebi-switcher-debug";
#[cfg(not(debug_assertions))]
const APP_REG_KEY: &str = "SOFTWARE\\amrbashir\\komorebi-switcher";

const WINDOW_POS_X_KEY: &str = "window-pos-x";
const WINDOW_POS_Y_KEY: &str = "window-pos-y";
const WINDOW_SIZE_WIDTH_KEY: &str = "window-size-width";
const WINDOW_SIZE_HEIGHT_KEY: &str = "window-size-height";
const WINDOW_SIZE_AUTO_WIDTH_KEY: &str = "window-size-auto-width";
const WINDOW_SIZE_AUTO_HEIGHT_KEY: &str = "window-size-auto-height";

#[derive(Debug, Clone, Copy)]
pub struct WindowRegistryInfo {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub auto_width: bool,
    pub auto_height: bool,
}

impl Default for WindowRegistryInfo {
    fn default() -> Self {
        WindowRegistryInfo {
            x: 5,
            y: 0,
            width: 200,
            height: 40,
            auto_width: true,
            auto_height: true,
        }
    }
}

impl WindowRegistryInfo {
    pub fn load(subkey: &str) -> anyhow::Result<Self> {
        tracing::debug!("Loading window info from registry for {subkey}...");

        let key = CURRENT_USER.create(APP_REG_KEY)?;
        let key = key.create(subkey)?;

        let defaults = WindowRegistryInfo::default();

        let x = get_str(&key, WINDOW_POS_X_KEY, defaults.x);
        let y = get_str(&key, WINDOW_POS_Y_KEY, defaults.y);
        let width = get_str(&key, WINDOW_SIZE_WIDTH_KEY, defaults.width);
        let height = get_str(&key, WINDOW_SIZE_HEIGHT_KEY, defaults.height);
        let auto_width = get_bool(&key, WINDOW_SIZE_AUTO_WIDTH_KEY, defaults.auto_width);
        let auto_height = get_bool(&key, WINDOW_SIZE_AUTO_HEIGHT_KEY, defaults.auto_height);

        let info = WindowRegistryInfo {
            x,
            y,
            width,
            height,
            auto_width,
            auto_height,
        };

        tracing::debug!("Loaded window info from registry: {info:?}");

        Ok(info)
    }

    pub fn save(&self, switcher_subkey: &str) -> anyhow::Result<()> {
        tracing::debug!("Storing window info into registry for {switcher_subkey}: {self:?}");

        let key = CURRENT_USER.create(APP_REG_KEY)?;
        let key = key.create(switcher_subkey)?;
        key.set_string(WINDOW_POS_X_KEY, &self.x.to_string())?;
        key.set_string(WINDOW_POS_Y_KEY, &self.y.to_string())?;
        // avoid saving zero/negative width and height
        if self.width > 0 {
            key.set_string(WINDOW_POS_X_KEY, &self.x.to_string())?;
        }
        if self.height > 0 {
            key.set_string(WINDOW_POS_Y_KEY, &self.y.to_string())?;
        }
        key.set_u32(WINDOW_SIZE_AUTO_WIDTH_KEY, self.auto_width as _)?;
        key.set_u32(WINDOW_SIZE_AUTO_HEIGHT_KEY, self.auto_height as _)?;

        Ok(())
    }

    pub fn apply(&mut self, hwnd: HWND) -> anyhow::Result<()> {
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

/// Helper functions to get string from the registry
/// and set default values if they don't exist.
#[inline]
fn get_str(k: &windows_registry::Key, key: &str, default: i32) -> i32 {
    k.get_string(key)
        .inspect_err(|_| {
            tracing::warn!("Registry {key} not found, creating it with default value: {default}");
            let _ = k.set_string(key, &default.to_string());
        })
        .map_err(|e| anyhow::anyhow!(e))
        .and_then(|v| v.parse::<i32>().map_err(Into::into))
        .unwrap_or(default)
}

/// Helper functions to get bool from the registry
/// and set default values if they don't exist.
#[inline]
fn get_bool(k: &windows_registry::Key, key: &str, default: bool) -> bool {
    k.get_u32(key)
        .inspect_err(|_| {
            tracing::warn!("Registry {key} not found, creating it with default value: {default}");
            let _ = k.set_u32(key, default as _);
        })
        .map(|v| v != 0)
        .unwrap_or(default)
}
