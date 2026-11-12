use std::collections::HashSet;
use std::ops::RangeInclusive;
use std::rc::Rc;

use windows_reactor::*;

use crate::config::{Config, MonitorConfig};
use crate::komorebi::Monitor;

const COMPACT_BREAKPOINT: f64 = 640.0;
const FIELD_MIN_WIDTH: f64 = 280.0;
const FIELD_MAX_WIDTH: f64 = 360.0;
const MULTI_CONTROL_FIELD_MIN_WIDTH: f64 = 180.0;
const MULTI_CONTROL_FIELD_MAX_WIDTH: f64 = 240.0;

const DEFAULT_FONT_WEIGHT: u16 = 400;
const FONT_WEIGHT_RANGE: RangeInclusive<f64> = 100.0..=900.0;
const POSITION_RANGE: RangeInclusive<f64> = i32::MIN as f64..=i32::MAX as f64;
const SIZE_RANGE: RangeInclusive<f64> = 1.0..=i32::MAX as f64;
const FONT_FAMILY_PLACEHOLDER: &str = "e.g. Segoe UI";
const COLOR_PLACEHOLDER: &str = "#RRGGBBAA or rgba(...)";

#[derive(Clone, PartialEq)]
pub struct SettingsInput {
	pub config: Config,
	pub monitors: Vec<Monitor>,
	pub preview: Callback<Config>,
	pub saved: Callback<Config>,
	pub closed: Callback<()>,
}

pub struct SettingsWindow {
	config: Config,
	expanded: HashSet<String>,
	compact: bool,
	error: String,
	closed: Callback<()>,
}

pub enum GlobalEdit {
	ShowLayout(bool),
	HideEmpty(bool),
	FontFamily(Rc<str>),
	FontWeight(Option<f64>),
	ActiveColor(Rc<str>),
	BusyColor(Rc<str>),
}

pub enum MonitorEdit {
	ShowLayout(Option<bool>),
	HideEmpty(Option<bool>),
	X(Option<f64>),
	Y(Option<f64>),
	Width(Option<f64>),
	Height(Option<f64>),
	AutoWidth(bool),
	AutoHeight(bool),
	FontFamily(Rc<str>),
	FontWeight(Option<f64>),
	ActiveColor(Rc<str>),
	BusyColor(Rc<str>),
	InheritFontFamily(bool),
	InheritFontWeight(bool),
	InheritActiveColor(bool),
	InheritBusyColor(bool),
}

pub enum Message {
	Resize(WindowSize),
	GlobalEdit(GlobalEdit),
	MonitorEdit(String, MonitorEdit),
	Expand(String, bool),
	Save,
	Cancel,
}

impl SettingsWindow {
	fn edit(&mut self, context: &ComponentContext<Self>, apply: impl FnOnce(&mut Self)) {
		let before = self.config.clone();
		apply(self);
		if self.config != before {
			context.input().preview.call(self.config.clone());
		}
	}

	fn edit_global(&mut self, edit: GlobalEdit) {
		match edit {
			GlobalEdit::FontWeight(None) => {}
			GlobalEdit::ShowLayout(value) => self.config.show_layout_button = value,
			GlobalEdit::HideEmpty(value) => self.config.hide_empty_workspaces = value,
			GlobalEdit::FontFamily(value) => self.config.font_family = Self::optional_text(value),
			GlobalEdit::FontWeight(Some(value)) => {
				self.config.font_weight = Some(Self::font_weight(value));
			}
			GlobalEdit::ActiveColor(value) => {
				self.config.colors.active_indicator = Self::optional_text(value)
			}
			GlobalEdit::BusyColor(value) => {
				self.config.colors.busy_indicator = Self::optional_text(value)
			}
		}
	}

	fn edit_monitor(&mut self, id: &str, edit: MonitorEdit) {
		if matches!(
			edit,
			MonitorEdit::X(None)
				| MonitorEdit::Y(None)
				| MonitorEdit::Width(None)
				| MonitorEdit::Height(None)
				| MonitorEdit::FontWeight(None)
		) {
			return;
		}
		let inherited_text = match &edit {
			MonitorEdit::InheritFontFamily(false) => {
				Some(self.config.font_family.clone().unwrap_or_default())
			}
			MonitorEdit::InheritActiveColor(false) => Some(
				self.config
					.colors
					.active_indicator
					.clone()
					.unwrap_or_default(),
			),
			MonitorEdit::InheritBusyColor(false) => Some(
				self.config
					.colors
					.busy_indicator
					.clone()
					.unwrap_or_default(),
			),
			_ => None,
		};
		let font_weight = self.config.font_weight.unwrap_or(DEFAULT_FONT_WEIGHT);
		let monitor = self.config.get_monitor_mut(id);
		match edit {
			MonitorEdit::X(None)
			| MonitorEdit::Y(None)
			| MonitorEdit::Width(None)
			| MonitorEdit::Height(None)
			| MonitorEdit::FontWeight(None) => {}
			MonitorEdit::ShowLayout(value) => monitor.show_layout_button = value,
			MonitorEdit::HideEmpty(value) => monitor.hide_empty_workspaces = value,
			MonitorEdit::AutoWidth(value) => monitor.auto_width = value,
			MonitorEdit::AutoHeight(value) => monitor.auto_height = value,
			MonitorEdit::X(Some(value)) => {
				monitor.x = value.round() as i32;
			}
			MonitorEdit::Y(Some(value)) => {
				monitor.y = value.round() as i32;
			}
			MonitorEdit::Width(Some(value)) => {
				monitor.width = (value.round() as i32).max(1);
			}
			MonitorEdit::Height(Some(value)) => {
				monitor.height = (value.round() as i32).max(1);
			}
			MonitorEdit::FontFamily(value) => monitor.font_family = Some(value.to_string()),
			MonitorEdit::FontWeight(Some(value)) => {
				monitor.font_weight = Some(Self::font_weight(value));
			}
			MonitorEdit::ActiveColor(value) => {
				monitor.colors.active_indicator = Some(value.to_string())
			}
			MonitorEdit::BusyColor(value) => {
				monitor.colors.busy_indicator = Some(value.to_string())
			}
			MonitorEdit::InheritFontFamily(value) => {
				monitor.font_family = if value { None } else { inherited_text }
			}
			MonitorEdit::InheritFontWeight(value) => {
				monitor.font_weight = if value { None } else { Some(font_weight) }
			}
			MonitorEdit::InheritActiveColor(value) => {
				monitor.colors.active_indicator = if value { None } else { inherited_text }
			}
			MonitorEdit::InheritBusyColor(value) => {
				monitor.colors.busy_indicator = if value { None } else { inherited_text }
			}
		}
	}

	fn optional_text(value: Rc<str>) -> Option<String> {
		(!value.is_empty()).then(|| value.to_string())
	}

	fn font_weight(value: f64) -> u16 {
		value
			.round()
			.clamp(*FONT_WEIGHT_RANGE.start(), *FONT_WEIGHT_RANGE.end()) as u16
	}

	fn global_callback<T: 'static>(
		context: &ViewContext<Self>,
		edit: impl Fn(T) -> GlobalEdit + 'static,
	) -> Callback<T> {
		context.callback(move |value| Message::GlobalEdit(edit(value)))
	}

	fn monitor_callback<T: 'static>(
		context: &ViewContext<Self>,
		id: &str,
		edit: impl Fn(T) -> MonitorEdit + 'static,
	) -> Callback<T> {
		let id = id.to_owned();
		context.callback(move |value| Message::MonitorEdit(id.clone(), edit(value)))
	}

	fn card(content: View) -> View {
		Border::new()
			.min_height(64.0)
			.padding(12.0)
			.corner_radius(4.0)
			.background(ThemeBrush::CardBackground)
			.border_brush(ThemeBrush::CardStroke)
			.border_thickness(1.0)
			.content(content)
			.into()
	}

	/// The final control grows; in compact multi-control rows it wraps below the label and preceding controls.
	fn row(&self, label: &str, controls: impl IntoIterator<Item = View>) -> View {
		let controls: Vec<_> = controls.into_iter().collect();
		let count = controls.len();
		let wrap_last = self.compact && count > 1;
		let column_count = if wrap_last { count } else { count + 1 };
		let (minimum, maximum) = match (self.compact, count > 1) {
			(true, _) => (0.0, FIELD_MAX_WIDTH),
			(false, true) => (MULTI_CONTROL_FIELD_MIN_WIDTH, MULTI_CONTROL_FIELD_MAX_WIDTH),
			(false, false) => (FIELD_MIN_WIDTH, FIELD_MAX_WIDTH),
		};
		let last_column_width = GridLength::Star(2.0).min(minimum).max(maximum);

		let mut columns = vec![GridLength::STAR];
		columns.extend(std::iter::repeat_n(
			GridLength::Auto,
			count.saturating_sub(1),
		));

		if !wrap_last && count > 0 {
			columns.push(last_column_width);
		}
		let mut children = vec![TextBlock::new()
			.text(label)
			.text_wrapping(TextWrapping::Wrap)
			.vertical_alignment(VerticalAlignment::Center)
			.into()];

		for (index, control) in controls.into_iter().enumerate() {
			let wrapped = wrap_last && index + 1 == count;
			let (column, row, span) = if wrapped {
				(0, 1, column_count as i32)
			} else {
				((index + 1) as i32, 0, 1)
			};
			children.push(
				Border::new()
					.grid_column(column)
					.grid_row(row)
					.grid_column_span(span)
					.vertical_alignment(VerticalAlignment::Center)
					.content(control)
					.into(),
			);
		}
		Self::card(
			Grid::new()
				.columns(columns)
				.rows(if wrap_last {
					vec![GridLength::Auto, GridLength::Auto]
				} else {
					vec![GridLength::Auto]
				})
				.column_spacing(16.0)
				.row_spacing(8.0)
				.children(children)
				.into(),
		)
	}

	fn toggle(&self, label: &str, value: bool, changed: Callback<bool>) -> View {
		self.row(
			label,
			[ToggleSwitch::new()
				.automation_name(label)
				.horizontal_alignment(HorizontalAlignment::Right)
				.min_width(0.0)
				.on_content("")
				.off_content("")
				.is_on(value)
				.on_toggled(changed)
				.into()],
		)
	}

	fn inheritable_toggle(
		&self,
		label: &str,
		value: Option<bool>,
		changed: Callback<Option<bool>>,
	) -> View {
		self.row(
			label,
			[ComboBox::new()
				.automation_name(label)
				.horizontal_alignment(HorizontalAlignment::Stretch)
				.items_source(["Inherit", "Enable", "Disable"])
				.selected_index(Some(match value {
					None => 0,
					Some(true) => 1,
					Some(false) => 2,
				}))
				.on_selection_changed(Callback::new(move |value| {
					changed.call(match value {
						Some(1) => Some(true),
						Some(2) => Some(false),
						_ => None,
					})
				}))
				.into()],
		)
	}

	fn number(
		label: &str,
		value: f64,
		range: RangeInclusive<f64>,
		changed: Callback<Option<f64>>,
	) -> NumberBox {
		NumberBox::new()
			.automation_name(label)
			.value(value)
			.minimum(*range.start())
			.maximum(*range.end())
			.spin_button_placement_mode(NumberBoxSpinButtonPlacementMode::Compact)
			.on_value_changed(changed)
	}

	fn text(
		label: &str,
		value: Option<&str>,
		placeholder: &str,
		changed: Callback<Rc<str>>,
	) -> TextBox {
		TextBox::new()
			.automation_name(label)
			.text(value.unwrap_or_default())
			.placeholder_text(placeholder)
			.on_text_changed(changed)
	}

	/// Builds the editor with its enabled state, independently of the inheritance toggle callback.
	fn inherit(
		&self,
		label: &str,
		inherited: bool,
		editor: impl FnOnce(bool) -> View,
		inherit_changed: Callback<bool>,
	) -> View {
		let toggle = ToggleSwitch::new()
			.automation_name(format!("Inherit {label}"))
			.vertical_alignment(VerticalAlignment::Center)
			.min_width(0.0)
			.on_content("Inherit")
			.off_content("Inherit")
			.is_on(inherited)
			.on_toggled(inherit_changed);
		self.row(label, [toggle.into(), editor(!inherited)])
	}

	fn global_settings(&self, context: &ViewContext<Self>) -> View {
		StackPanel::new()
			.spacing(8.0)
			.children((
				TextBlock::new()
					.text("Global settings")
					.font_size(20.0)
					.font_weight(FontWeight::SEMI_BOLD),
				self.toggle(
					"Show layout button",
					self.config.show_layout_button,
					Self::global_callback(context, GlobalEdit::ShowLayout),
				),
				self.toggle(
					"Hide empty workspaces",
					self.config.hide_empty_workspaces,
					Self::global_callback(context, GlobalEdit::HideEmpty),
				),
				self.row(
					"Font family",
					[Self::text(
						"Font family",
						self.config.font_family.as_deref(),
						FONT_FAMILY_PLACEHOLDER,
						Self::global_callback(context, GlobalEdit::FontFamily),
					)
					.into()],
				),
				self.row(
					"Font weight",
					[Self::number(
						"Font weight",
						self.config
							.font_weight
							.unwrap_or(DEFAULT_FONT_WEIGHT)
							.into(),
						FONT_WEIGHT_RANGE,
						Self::global_callback(context, GlobalEdit::FontWeight),
					)
					.into()],
				),
				self.row(
					"Active indicator",
					[Self::text(
						"Active indicator",
						self.config.colors.active_indicator.as_deref(),
						COLOR_PLACEHOLDER,
						Self::global_callback(context, GlobalEdit::ActiveColor),
					)
					.into()],
				),
				self.row(
					"Busy indicator",
					[Self::text(
						"Busy indicator",
						self.config.colors.busy_indicator.as_deref(),
						COLOR_PLACEHOLDER,
						Self::global_callback(context, GlobalEdit::BusyColor),
					)
					.into()],
				),
			))
			.into()
	}

	fn monitor_geometry(
		&self,
		id: &str,
		config: &MonitorConfig,
		context: &ViewContext<Self>,
	) -> View {
		StackPanel::new()
			.spacing(8.0)
			.children((
				self.row(
					"X",
					[Self::number(
						"X",
						config.x.into(),
						POSITION_RANGE,
						Self::monitor_callback(context, id, MonitorEdit::X),
					)
					.into()],
				),
				self.row(
					"Y",
					[Self::number(
						"Y",
						config.y.into(),
						POSITION_RANGE,
						Self::monitor_callback(context, id, MonitorEdit::Y),
					)
					.into()],
				),
				self.row(
					"Width",
					[Self::number(
						"Width",
						config.width.into(),
						SIZE_RANGE,
						Self::monitor_callback(context, id, MonitorEdit::Width),
					)
					.is_enabled(!config.auto_width)
					.into()],
				),
				self.toggle(
					"Auto width",
					config.auto_width,
					Self::monitor_callback(context, id, MonitorEdit::AutoWidth),
				),
				self.row(
					"Height",
					[Self::number(
						"Height",
						config.height.into(),
						SIZE_RANGE,
						Self::monitor_callback(context, id, MonitorEdit::Height),
					)
					.is_enabled(!config.auto_height)
					.into()],
				),
				self.toggle(
					"Auto height",
					config.auto_height,
					Self::monitor_callback(context, id, MonitorEdit::AutoHeight),
				),
			))
			.into()
	}

	fn monitor_behavior(
		&self,
		id: &str,
		config: &MonitorConfig,
		context: &ViewContext<Self>,
	) -> View {
		StackPanel::new()
			.spacing(8.0)
			.children((
				self.inheritable_toggle(
					"Show layout button",
					config.show_layout_button,
					Self::monitor_callback(context, id, MonitorEdit::ShowLayout),
				),
				self.inheritable_toggle(
					"Hide empty workspaces",
					config.hide_empty_workspaces,
					Self::monitor_callback(context, id, MonitorEdit::HideEmpty),
				),
			))
			.into()
	}

	fn monitor_appearance(
		&self,
		id: &str,
		config: &MonitorConfig,
		context: &ViewContext<Self>,
	) -> View {
		StackPanel::new()
			.spacing(8.0)
			.children((
				self.inherit(
					"Font family",
					config.font_family.is_none(),
					|enabled| {
						Self::text(
							"Font family",
							config.font_family.as_deref(),
							FONT_FAMILY_PLACEHOLDER,
							Self::monitor_callback(context, id, MonitorEdit::FontFamily),
						)
						.is_enabled(enabled)
						.into()
					},
					Self::monitor_callback(context, id, MonitorEdit::InheritFontFamily),
				),
				self.inherit(
					"Font weight",
					config.font_weight.is_none(),
					|enabled| {
						Self::number(
							"Font weight",
							config.font_weight.unwrap_or(DEFAULT_FONT_WEIGHT).into(),
							FONT_WEIGHT_RANGE,
							Self::monitor_callback(context, id, MonitorEdit::FontWeight),
						)
						.is_enabled(enabled)
						.into()
					},
					Self::monitor_callback(context, id, MonitorEdit::InheritFontWeight),
				),
				self.inherit(
					"Active indicator",
					config.colors.active_indicator.is_none(),
					|enabled| {
						Self::text(
							"Active indicator",
							config.colors.active_indicator.as_deref(),
							COLOR_PLACEHOLDER,
							Self::monitor_callback(context, id, MonitorEdit::ActiveColor),
						)
						.is_enabled(enabled)
						.into()
					},
					Self::monitor_callback(context, id, MonitorEdit::InheritActiveColor),
				),
				self.inherit(
					"Busy indicator",
					config.colors.busy_indicator.is_none(),
					|enabled| {
						Self::text(
							"Busy indicator",
							config.colors.busy_indicator.as_deref(),
							COLOR_PLACEHOLDER,
							Self::monitor_callback(context, id, MonitorEdit::BusyColor),
						)
						.is_enabled(enabled)
						.into()
					},
					Self::monitor_callback(context, id, MonitorEdit::InheritBusyColor),
				),
			))
			.into()
	}

	fn monitor_section(&self, monitor: &Monitor, context: &ViewContext<Self>) -> View {
		let config = self.config.get_monitor(&monitor.id);
		let id = monitor.id.clone();
		Expander::new()
			.header(format!("{} ({})", monitor.name, monitor.id))
			.is_expanded(self.expanded.contains(&id))
			.horizontal_alignment(HorizontalAlignment::Stretch)
			.horizontal_content_alignment(HorizontalAlignment::Stretch)
			.on_is_expanded_changed(
				context.callback(move |expanded| Message::Expand(id.clone(), expanded)),
			)
			.content(StackPanel::new().spacing(8.0).children((
				self.monitor_geometry(&monitor.id, &config, context),
				self.monitor_behavior(&monitor.id, &config, context),
				self.monitor_appearance(&monitor.id, &config, context),
			)))
			.into()
	}

	fn actions(&self, context: &ViewContext<Self>) -> View {
		StackPanel::new()
			.orientation(Orientation::Horizontal)
			.spacing(8.0)
			.children((
				Button::new()
					.automation_id("settings-save")
					.content("Save")
					.style(ButtonStyle::Accent)
					.on_click(context.callback(|_| Message::Save)),
				Button::new()
					.automation_id("settings-cancel")
					.content("Cancel")
					.on_click(context.callback(|_| Message::Cancel)),
			))
			.into()
	}

	fn footer(&self, context: &ViewContext<Self>) -> View {
		let mut children = Vec::new();

		if !self.error.is_empty() {
			children.push(
				TextBlock::new()
					.text(&self.error)
					.foreground(ThemeBrush::SystemCritical)
					.into(),
			);
		}

		children.push(self.actions(context));

		StackPanel::new()
			.grid_row(1)
			.margin(16.0)
			.spacing(8.0)
			.children(children)
			.into()
	}
}

impl Component for SettingsWindow {
	type Input = SettingsInput;
	type Message = Message;

	fn create(input: &Self::Input, _: &ComponentContext<Self>) -> Self {
		Self {
			config: input.config.clone(),
			compact: true,
			expanded: input
				.monitors
				.first()
				.map(|m| m.id.clone())
				.into_iter()
				.collect(),
			error: String::new(),
			closed: input.closed.clone(),
		}
	}

	fn update(&mut self, message: Message, context: &ComponentContext<Self>) {
		match message {
			Message::Resize(size) => self.compact = size.width < COMPACT_BREAKPOINT,
			Message::GlobalEdit(edit) => self.edit(context, |settings| settings.edit_global(edit)),
			Message::MonitorEdit(id, edit) => {
				self.edit(context, |settings| settings.edit_monitor(&id, edit))
			}

			Message::Expand(id, expanded) => {
				if expanded {
					self.expanded.insert(id);
				} else {
					self.expanded.remove(&id);
				}
			}

			Message::Save => match self.config.save() {
				Ok(()) => {
					context.input().saved.call(self.config.clone());
					let _ = context.close_window();
				}

				Err(error) => self.error = format!("Could not save settings: {error}"),
			},
			Message::Cancel => {
				let _ = context.close_window();
			}
		}
	}

	fn view(&self, input: &Self::Input, context: &mut ViewContext<Self>) -> View {
		context.on_window_size(context.callback(Message::Resize));
		context.window_visuals(
			WindowVisuals::new()
				.client_size(520.0, 640.0)
				.backdrop(WindowBackdrop::Mica),
		);

		let mut sections = vec![keyed("global", self.global_settings(context))];

		for monitor in &input.monitors {
			sections.push(keyed(
				monitor.id.clone(),
				self.monitor_section(monitor, context),
			));
		}

		let content = Grid::new()
			.rows([GridLength::STAR, GridLength::Auto])
			.children((
				ScrollViewer::new().content(
					StackPanel::new()
						.margin(16.0)
						.spacing(20.0)
						.keyed_children(sections),
				),
				self.footer(context),
			));

		context.window_frame(format!("Settings - {}", crate::APP_NAME), content)
	}
}

impl Drop for SettingsWindow {
	fn drop(&mut self) {
		self.closed.call(());
	}
}
