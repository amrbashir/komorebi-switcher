use std::time::Duration;

use windows_reactor::*;

use super::super::utils::TRANSPARENT_BRUSH;

const ACTIVE_INDICATOR_WIDTH: f64 = 14.0;
const INACTIVE_INDICATOR_WIDTH: f64 = 6.0;

#[derive(Clone, PartialEq)]
pub struct WorkspaceButtonInput {
	pub id: String,
	pub text: String,
	pub focused: bool,
	pub is_empty: bool,
	pub active_indicator: Brush,
	pub busy_indicator: Brush,
	pub font_family: Option<String>,
	pub font_weight: u16,
	pub monitor_index: usize,
	pub workspace_index: usize,
}

pub struct WorkspaceButton {
	hovered: bool,
	pressed: bool,
}

pub enum Message {
	Enter,
	Exit,
	Press(PointerEventInfo),
	Release,
	Cancel,
}

impl Component for WorkspaceButton {
	type Input = WorkspaceButtonInput;
	type Message = Message;

	fn create(_: &Self::Input, _: &ComponentContext<Self>) -> Self {
		Self {
			hovered: false,
			pressed: false,
		}
	}

	fn update(&mut self, message: Message, context: &ComponentContext<Self>) {
		match message {
			Message::Enter => {
				self.hovered = true;
			}

			Message::Exit => {
				self.hovered = false;
				self.pressed = false;
			}

			Message::Press(event) => {
				self.pressed = event.is_left_button_pressed;
			}

			Message::Release => {
				if self.pressed {
					let input = context.input();
					crate::komorebi::change_workspace(input.monitor_index, input.workspace_index);
				}

				self.pressed = false;
			}

			Message::Cancel => {
				self.pressed = false;
			}
		}
	}

	fn view(&self, input: &Self::Input, context: &mut ViewContext<Self>) -> View {
		let highlighted = input.focused || self.hovered;
		let indicator = if input.focused {
			Some(input.active_indicator)
		} else if !input.is_empty {
			Some(input.busy_indicator)
		} else {
			None
		};

		let indicator_width = if input.focused && !self.pressed {
			ACTIVE_INDICATOR_WIDTH
		} else {
			INACTIVE_INDICATOR_WIDTH
		};

		let mut text = TextBlock::new()
			.text(&input.text)
			.font_size(14.0)
			.font_weight(FontWeight::new(input.font_weight).unwrap_or(FontWeight::NORMAL))
			.foreground(ThemeBrush::PrimaryText)
			.opacity(if highlighted { 1.0 } else { 0.75 })
			.horizontal_alignment(HorizontalAlignment::Center)
			.vertical_alignment(VerticalAlignment::Center)
			// Include the outer one-DIP border in the original 8-by-4 text inset.
			.margin(Thickness::xy(7.0, 3.0));

		if let Some(family) = input
			.font_family
			.as_deref()
			.filter(|family| !family.trim().is_empty())
		{
			text = text.font_family(family);
		}

		Border::new()
			.corner_radius(4.0)
			.background(if highlighted {
				ThemeBrush::CardBackground.into()
			} else {
				TRANSPARENT_BRUSH
			})
			.border_brush(if highlighted {
				ThemeBrush::CardStroke.into()
			} else {
				TRANSPARENT_BRUSH
			})
			.border_thickness(1.0)
			.automation_id(&input.id)
			.automation_name(&input.text)
			.min_width(28.0)
			.min_height(28.0)
			.is_tab_stop(false)
			.allow_focus_on_interaction(false)
			.focus_on_pointer_release(false)
			.on_pointer_entered(context.callback(|_| Message::Enter))
			.on_pointer_exited(context.callback(|_| Message::Exit))
			.on_pointer_pressed(context.callback(Message::Press))
			.on_pointer_released(context.callback(|_| Message::Release))
			.on_pointer_canceled(context.callback(|_| Message::Cancel))
			.content(
				Grid::new().children((
					text,
					Border::new()
						.height(3.5)
						.width(indicator_width)
						.width_transition(Duration::from_millis(200))
						.horizontal_alignment(HorizontalAlignment::Center)
						.vertical_alignment(VerticalAlignment::Bottom)
						// Keep the underline on the outer bottom edge, across the border inset.
						.margin(Thickness::new(0.0, 0.0, 0.0, -1.0))
						.corner_radius(1.75)
						.background(indicator.unwrap_or(TRANSPARENT_BRUSH))
						.opacity(match indicator {
							Some(Brush::Theme(ThemeBrush::PrimaryText)) => 0.5,
							Some(_) => 1.0,
							None => 0.0,
						})
						.opacity_transition(Duration::from_millis(300)),
				)),
			)
			.into()
	}
}
