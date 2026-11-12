use windows::Win32::Foundation::*;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows_reactor::*;

use super::super::utils::TRANSPARENT;
use crate::config::Config;
use crate::komorebi::Monitor;
use crate::windows::taskbar::Taskbar;
use crate::windows::utils::{to_reactor_color, OwnedHWND};
use crate::windows::widgets::{
	LayoutButton, LayoutButtonInput, WorkspaceButton, WorkspaceButtonInput,
};

mod host;

#[derive(Clone, PartialEq)]
pub struct SwitcherInput {
	pub monitor: Monitor,
	pub taskbar: Taskbar,
	pub config: Config,
	pub context_menu: Menu,
}

pub struct SwitcherWindow {
	host: OwnedHWND,
}

pub enum Message {
	Measure,
}

impl SwitcherWindow {
	fn resize(&self, input: &SwitcherInput, content_width: f64) -> anyhow::Result<()> {
		let mut config = input.config.get_monitor(&input.monitor.id);

		if config.auto_width && content_width > 0.0 {
			let dpi = unsafe { GetDpiForWindow(self.host.0) }.max(96);
			config.width = (content_width * f64::from(dpi) / 96.0).ceil().max(1.0) as i32;
		}

		if config.auto_height {
			let mut rect = RECT::default();
			unsafe { GetClientRect(input.taskbar.hwnd, &mut rect) }?;

			config.height = rect.bottom - rect.top;
		}

		unsafe {
			SetWindowPos(
				self.host.0,
				Some(HWND_TOP),
				config.x,
				config.y,
				config.width.max(1),
				config.height.max(1),
				SWP_NOACTIVATE,
			)?;
		}

		if config.auto_width || config.auto_height {
			crate::windows::registry::store_auto_size(
				&input.monitor.id,
				config.width,
				config.height,
			)?;
		}

		Ok(())
	}
}

impl Component for SwitcherWindow {
	type Input = SwitcherInput;
	type Message = Message;

	fn create(input: &Self::Input, _: &ComponentContext<Self>) -> Self {
		let mut monitor_config = input.config.get_monitor(&input.monitor.id);
		let _ = monitor_config.adjust_for_auto_size(&input.monitor.id);

		let hwnd = unsafe { host::create(input.taskbar.hwnd, &monitor_config) }
			.expect("Failed to create taskbar host");

		Self { host: hwnd }
	}

	fn update(&mut self, Message::Measure: Message, context: &ComponentContext<Self>) {
		let Some(window) = context.native_window() else {
			tracing::error!("Cannot measure switcher before its native window is available");
			return;
		};

		match window.desired_size() {
			Ok(size) => {
				if let Err(error) = self.resize(context.input(), size.width) {
					tracing::error!("Failed to resize switcher: {error}");
				}
			}

			Err(error) => tracing::error!("Failed to measure switcher: {error:?}"),
		}
	}

	fn view(&self, input: &Self::Input, context: &mut ViewContext<Self>) -> View {
		context.window_visuals(unsafe {
			WindowVisuals::new()
				.backdrop(WindowBackdrop::Transparent)
				.parent_window(self.host.0 .0 as _)
		});

		context.on_window_size(context.callback(|_| Message::Measure));
		context.window_title(format!("{}: {}", crate::APP_NAME, input.monitor.id));

		let sender = context.sender();
		context.use_effect("measure", input.clone(), move || {
			let _ = sender.send(Message::Measure);
			None
		});

		let config = input.config.get_monitor(&input.monitor.id);

		let font_family = config
			.font_family
			.clone()
			.or_else(|| input.config.font_family.clone());
		let font_weight = config
			.font_weight
			.or(input.config.font_weight)
			.unwrap_or(400);
		let active = config
			.colors
			.active_indicator
			.as_deref()
			.or(input.config.colors.active_indicator.as_deref())
			.and_then(to_reactor_color)
			.map(Brush::from)
			.unwrap_or(ThemeBrush::Accent.into());
		let busy = config
			.colors
			.busy_indicator
			.as_deref()
			.or(input.config.colors.busy_indicator.as_deref())
			.and_then(to_reactor_color)
			.map(Brush::from)
			.unwrap_or(ThemeBrush::PrimaryText.into());

		let hide_empty_workspaces = config
			.hide_empty_workspaces
			.unwrap_or(input.config.hide_empty_workspaces);

		// Workspace buttons
		let mut buttons = Vec::new();
		for workspace in &input.monitor.workspaces {
			if hide_empty_workspaces && workspace.is_empty && !workspace.focused {
				continue;
			}

			let monitor = input.monitor.index;
			let index = workspace.index;

			buttons.push(keyed(
				format!("workspace-{index}"),
				View::component::<WorkspaceButton>(WorkspaceButtonInput {
					id: format!("workspace-{index}"),
					text: workspace.name.clone(),
					focused: workspace.focused,
					is_empty: workspace.is_empty,
					active_indicator: active,
					busy_indicator: busy,
					font_family: font_family.clone(),
					font_weight,
					monitor_index: monitor,
					workspace_index: index,
				}),
			));
		}

		// Layout button and its separator
		if config
			.show_layout_button
			.unwrap_or(input.config.show_layout_button)
		{
			if let Some(workspace) = input.monitor.focused_workspace() {
				let mut separator = TextBlock::new()
					.text("|")
					.font_size(14.0)
					.font_weight(FontWeight::new(font_weight).unwrap_or(FontWeight::NORMAL))
					.vertical_alignment(VerticalAlignment::Center)
					.foreground(ThemeBrush::PrimaryText)
					.opacity(0.75);

				if let Some(family) = &font_family {
					separator = separator.font_family(family);
				}

				buttons.push(keyed("layout-separator", separator));
				buttons.push(keyed(
					"layout",
					View::component::<LayoutButton>(LayoutButtonInput {
						text: workspace.layout.clone(),
						font_family: font_family.clone(),
						font_weight,
					}),
				));
			}
		}

		Border::new()
			.background(TRANSPARENT)
			.allow_focus_on_interaction(false)
			.is_tab_stop(false)
			.content(
				Grid::new()
					.horizontal_alignment(HorizontalAlignment::Left)
					.children((Border::new()
						.padding(8.0)
						.vertical_alignment(VerticalAlignment::Center)
						.content(
							StackPanel::new()
								.orientation(Orientation::Horizontal)
								.spacing(4.0)
								.keyed_children(buttons),
						),)),
			)
			.context_menu(input.context_menu.clone())
	}
}
