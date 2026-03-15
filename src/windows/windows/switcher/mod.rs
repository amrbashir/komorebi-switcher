use std::num::NonZero;
use std::sync::{Arc, RwLock};

use muda::ContextMenu;
use raw_window_handle::{RawWindowHandle, Win32WindowHandle};
use windows::Win32::Foundation::*;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::UI::ViewManagement::{UIColorType, UISettings};
use winit::dpi::PhysicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::WindowAttributes;

use crate::config::Config;
use crate::komorebi::CycleDirection;
use crate::windows::app::{App, AppMessage};
use crate::windows::context_menu::AppContextMenu;
use crate::windows::egui_glue::{EguiView, EguiWindow};
use crate::windows::registry;
use crate::windows::taskbar::Taskbar;
use crate::windows::utils::egui_color_from_color;
use crate::windows::widgets::{LayoutButton, WorkspaceButton};

mod host;

impl App {
    /// Creates the switcher window for the given monitor.
    ///
    /// The switcher consists of a host window (child of taskbar), and a winit
    /// window as a child of the host and hosts the egui view.
    pub fn create_switcher_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        taskbar: Taskbar,
        monitor_state: crate::komorebi::Monitor,
        context_menu: AppContextMenu,
    ) -> anyhow::Result<EguiWindow> {
        let monitor_config = {
            let config = self.config.read().unwrap();
            let mut monitor_config = config.get_monitor(&monitor_state.id);
            monitor_config.adjust_for_auto_size(&monitor_state.id)?;
            monitor_config
        };

        let host = unsafe { host::create_host(taskbar.hwnd, self.proxy.clone(), &monitor_config) }?;

        let mut attrs = WindowAttributes::default();
        attrs = attrs.with_inner_size(PhysicalSize::new(
            monitor_config.width,
            monitor_config.height,
        ));

        let parent = unsafe { NonZero::new_unchecked(host.0 as _) };
        let parent = Win32WindowHandle::new(parent);
        let parent = RawWindowHandle::Win32(parent);
        attrs = unsafe { attrs.with_parent_window(Some(parent)) };

        #[cfg(debug_assertions)]
        let class_name = "komorebi-switcher-debug::window";
        #[cfg(not(debug_assertions))]
        let class_name = "komorebi-switcher::window";

        attrs = attrs
            .with_decorations(false)
            .with_transparent(true)
            .with_active(false)
            .with_class_name(class_name)
            .with_undecorated_shadow(false)
            .with_no_redirection_bitmap(true)
            .with_clip_children(false);

        let window = event_loop.create_window(attrs)?;
        let window = Arc::new(window);

        let state = SwitcherWindowView::new(
            host,
            taskbar,
            monitor_state,
            self.config.clone(),
            context_menu,
        )?;

        EguiWindow::new(window, &self.wgpu_instance, state)
    }
}

pub struct SwitcherWindowView {
    config: Arc<RwLock<crate::config::Config>>,
    preview_config: Option<Config>,
    host: HWND,
    taskbar: Taskbar,
    context_menu: AppContextMenu,
    monitor_state: crate::komorebi::Monitor,
    accent_light2_color: Option<egui::Color32>,
    accent_color: Option<egui::Color32>,
    forgreound_color: Option<egui::Color32>,
    prev_bounds: Option<egui::Rect>,
    applied_font: Option<(String, u16)>,
}

impl SwitcherWindowView {
    fn new(
        host: HWND,
        taskbar: Taskbar,
        monitor_state: crate::komorebi::Monitor,
        config: Arc<RwLock<crate::config::Config>>,
        context_menu: AppContextMenu,
    ) -> anyhow::Result<Self> {
        let mut view = Self {
            host,
            taskbar,
            monitor_state,
            context_menu,
            accent_color: None,
            accent_light2_color: None,
            forgreound_color: None,
            config,
            preview_config: None,
            prev_bounds: None,
            applied_font: None,
        };

        // Update system colors initially.
        if let Err(e) = view.update_system_colors() {
            tracing::error!("Failed to get system colors: {e}");
        }

        Ok(view)
    }
}

/// Getters
impl SwitcherWindowView {
    /// Gets the effective config for the switcher, which is the preview config
    /// if set, or the actual config otherwise.
    fn effective_config(&self) -> Config {
        self.preview_config.clone().unwrap_or_else(|| {
            let config = self.config.read().unwrap();
            config.clone()
        })
    }

    /// Gets the height of the taskbar.
    /// Used for auto-sizing the switcher panel to match the taskbar height.
    fn taskbar_height(&self) -> anyhow::Result<i32> {
        let mut rect = RECT::default();
        unsafe { GetClientRect(self.taskbar.hwnd, &mut rect) }?;
        Ok(rect.bottom - rect.top)
    }

    /// Determines if the system is in dark mode.
    // FIXME: use egui internal dark mode detection
    fn is_system_dark_mode(&self) -> bool {
        self.forgreound_color
            .map(|c| c == egui::Color32::WHITE)
            .unwrap_or(false)
    }

    /// Gets the accent color to use for the switcher, based on the system
    /// accent color and the current theme (light/dark mode).
    fn accent_color(&self) -> Option<egui::Color32> {
        if self.is_system_dark_mode() {
            self.accent_light2_color
        } else {
            self.accent_color
        }
    }
}

/// Actions
impl SwitcherWindowView {
    /// Shows the context menu at the current mouse position.
    ///
    /// Used when right-clicking on the switcher panel.
    fn show_context_menu(&self) {
        let hwnd = self.host.0 as isize;
        let menu = &self.context_menu.menu;
        unsafe { menu.show_context_menu_for_hwnd(hwnd, None) };
    }

    /// Updates system colors from Windows settings, and stores them in the view
    /// state.
    fn update_system_colors(&mut self) -> anyhow::Result<()> {
        let settings = UISettings::new()?;

        let color = settings.GetColorValue(UIColorType::Accent)?;
        let color = egui::Color32::from_rgb(color.R, color.G, color.B);
        self.accent_color.replace(color);

        let color = settings.GetColorValue(UIColorType::AccentLight2)?;
        let color = egui::Color32::from_rgb(color.R, color.G, color.B);
        self.accent_light2_color.replace(color);

        let color = settings.GetColorValue(UIColorType::Foreground)?;
        let color = egui::Color32::from_rgb(color.R, color.G, color.B);
        self.forgreound_color.replace(color);

        Ok(())
    }

    /// Resizes the host window that contains the switcher panel to match the
    /// given content rect.
    fn resize_host_to_rect(
        &mut self,
        rect: egui::Rect,
        ppp: f32,
        monitor_config: &crate::config::MonitorConfig,
    ) -> anyhow::Result<()> {
        // Scale by pixels per point
        let rect = rect * ppp;

        let x = monitor_config.x;
        let y = monitor_config.y;

        let height = if monitor_config.auto_height {
            self.taskbar_height()?
        } else {
            monitor_config.height
        };

        let width = if monitor_config.auto_width {
            rect.width() as i32
        } else {
            monitor_config.width
        };

        let current_bounds = egui::Rect::from_min_size(
            egui::pos2(x as f32, y as f32),
            egui::vec2(width as f32, height as f32),
        );

        // If bounds changed, resize host window
        if self.prev_bounds != Some(current_bounds) {
            tracing::debug!("Resizing host to match content rect");

            unsafe {
                SetWindowPos(
                    self.host,
                    None,
                    x,
                    y,
                    width,
                    height,
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                )
            }?;

            // Store auto size if needed
            if monitor_config.auto_width || monitor_config.auto_height {
                let _ = registry::store_auto_size(&self.monitor_state.id, width, height);
            }

            // Update previous bounds, to avoid redundant resizes next time
            self.prev_bounds = Some(current_bounds);
        }

        Ok(())
    }

    /// Applies the desired font to the egui context if it's not already
    /// applied.
    fn maybe_apply_font(
        &mut self,
        ctx: &egui::Context,
        config: &Config,
        monitor_config: &crate::config::MonitorConfig,
    ) {
        let font_family = monitor_config
            .font_family
            .as_deref()
            .or(config.font_family.as_deref());
        let font_weight = monitor_config
            .font_weight
            .or(config.font_weight)
            .unwrap_or(400);

        // Skip if the desired font is already applied
        let desired = font_family.map(|family| (family.to_string(), font_weight));
        if self.applied_font == desired {
            return;
        }

        // Update applied font to avoid redundant updates next time
        self.applied_font = desired.clone();

        // Load font data for the desired font, if specified. If loading fails, log a
        // warning and fall back to default font.
        let font_data = desired.as_ref().and_then(|(family, weight)| {
            let font = crate::utils::find_font(family, *weight);
            let data = font.and_then(|f| f.copy_font_data());
            let data = data.map(|arc| arc.to_vec());
            if data.is_none() {
                tracing::warn!(
                    "Font '{family}' with weight {weight} not found, falling back to default font"
                );
            }
            data
        });

        // Set font in egui context, or fall back to default.
        let mut fonts = egui::FontDefinitions::default();
        if let Some(data) = font_data {
            const SWITCHER_FONT_NAME: &str = "switcher_custom";

            fonts.font_data.insert(
                SWITCHER_FONT_NAME.to_owned(),
                egui::FontData::from_owned(data).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, SWITCHER_FONT_NAME.to_owned());
        }
        ctx.set_fonts(fonts);
    }
}

/// UI
impl SwitcherWindowView {
    fn layout_button(&self, ui: &mut egui::Ui, focused_workspace: &crate::komorebi::Workspace) {
        let btn = LayoutButton::new(&focused_workspace.layout)
            .dark_mode(Some(self.is_system_dark_mode()))
            .text_color_opt(self.forgreound_color);

        if ui.add(btn).clicked() {
            crate::komorebi::cycle_layout(CycleDirection::Next);
        }
    }

    fn workspace_button(
        &self,
        ui: &mut egui::Ui,
        workspace: &crate::komorebi::Workspace,
        monitor_config: &crate::config::MonitorConfig,
        config: &Config,
    ) {
        // Determine active indicator colors,
        // with monitor config taking precedence over global config,
        // and falling back to accent color if not specified.
        let active_indicator_color = match monitor_config.colors.active_indicator {
            Some(ref c) => egui_color_from_color(c),
            None => config
                .colors
                .active_indicator
                .as_ref()
                .and_then(|c| egui_color_from_color(c)),
        };
        let active_indicator_color = active_indicator_color.or_else(|| self.accent_color());

        // Determine busy indicator color,
        // with monitor config taking precedence over global config.
        let busy_indicator_color = match monitor_config.colors.busy_indicator {
            Some(ref c) => egui_color_from_color(c),
            None => config
                .colors
                .busy_indicator
                .as_ref()
                .and_then(|c| egui_color_from_color(c)),
        };

        let btn = WorkspaceButton::new(workspace)
            .dark_mode(Some(self.is_system_dark_mode()))
            .line_active_color_opt(active_indicator_color)
            .line_busy_color_opt(busy_indicator_color)
            .text_color_opt(self.forgreound_color);

        if ui.add(btn).clicked() {
            crate::komorebi::change_workspace(self.monitor_state.index, workspace.index);
        }
    }

    /// Main UI elements, workspaces buttons, layout button ...etc
    fn switcher_ui(
        &mut self,
        ui: &mut egui::Ui,
        config: &Config,
        monitor_config: &crate::config::MonitorConfig,
    ) {
        // Set spacing between buttons
        ui.style_mut().spacing.item_spacing = egui::vec2(4., 4.);

        // Determine whether to show or hide empty workspaces
        let hide_empty_workspaces = match monitor_config.hide_empty_workspaces {
            Some(hide) => hide,
            None => config.hide_empty_workspaces,
        };

        // Draw a button for each workspace
        for workspace in self.monitor_state.workspaces.iter() {
            //Skip empty and unfocused workspaces if the setting is enabled
            if hide_empty_workspaces && workspace.is_empty && !workspace.focused {
                continue;
            }

            self.workspace_button(ui, workspace, monitor_config, config);
        }

        // Show layout button for focused workspace if the setting is enabled
        let show_layout_button = match monitor_config.show_layout_button {
            Some(show) => show,
            None => config.show_layout_button,
        };
        if show_layout_button {
            if let Some(focused_ws) = self.monitor_state.focused_workspace() {
                ui.add(egui::Label::new("|"));

                self.layout_button(ui, focused_ws);
            }
        }
    }

    /// Transparent panel containing the UI elements horizontally with some
    /// margin around it.
    fn switcher_panel(
        &mut self,
        ctx: &egui::Context,
        config: &Config,
        monitor_config: &crate::config::MonitorConfig,
    ) -> egui::InnerResponse<egui::Rect> {
        // Set transparent panel visual for the switcher.
        let visuals = egui::Visuals {
            panel_fill: egui::Color32::TRANSPARENT,
            ..egui::Visuals::dark()
        };
        ctx.set_visuals(visuals);

        // Create a frame with margins
        let frame = egui::Frame::central_panel(&ctx.style());

        // Create central panel with the margined frame
        let total_margin = frame.total_margin();
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            let response = ui.horizontal_centered(|ui| {
                self.switcher_ui(ui, config, monitor_config);
                ui.min_rect()
            });

            // Use the content rect from inside the horizontal layout,
            // expanded by the frame's total margin (inner + outer + stroke).
            response.inner.expand2(total_margin.right_bottom())
        })
    }
}

impl EguiView for SwitcherWindowView {
    fn handle_app_message(
        &mut self,
        ctx: &egui::Context,
        _event_loop: &ActiveEventLoop,
        message: &AppMessage,
    ) -> anyhow::Result<()> {
        match message {
            AppMessage::UpdateKomorebiState(state) => {
                self.monitor_state = state
                    .monitors
                    .iter()
                    .find(|m| m.id == self.monitor_state.id)
                    .cloned()
                    .unwrap_or_default();
            }

            AppMessage::PreviewConfig(config) => self.preview_config = Some(config.clone()),
            AppMessage::ClearPreviewConfig => self.preview_config = None,

            AppMessage::SystemSettingsChanged => self.update_system_colors()?,

            AppMessage::DpiChanged => {
                let dpi = unsafe { GetDpiForWindow(self.host) } as f32;
                let ppp = dpi / USER_DEFAULT_SCREEN_DPI as f32;
                ctx.set_pixels_per_point(ppp);
            }

            _ => {}
        }

        Ok(())
    }

    // Main render loop
    fn update(&mut self, ctx: &egui::Context) {
        // Show context menu on right click
        if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) {
            self.show_context_menu();
        }

        // Load effective config.
        let config = self.effective_config();
        let monitor_config = config.get_monitor(&self.monitor_state.id);

        // Apply font
        self.maybe_apply_font(ctx, &config, &monitor_config);

        // Draw ui
        let response = self.switcher_panel(ctx, &config, &monitor_config);

        // Resize host to match the content rect.
        let ppp = ctx.pixels_per_point();
        if let Err(e) = self.resize_host_to_rect(response.inner, ppp, &monitor_config) {
            tracing::error!("Failed to resize host to rect: {e}");
        }
    }
}
