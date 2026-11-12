use windows_reactor::*;

use super::super::utils::TRANSPARENT_BRUSH;

#[derive(Clone, PartialEq)]
pub struct LayoutButtonInput {
	pub text: String,
	pub font_family: Option<String>,
	pub font_weight: u16,
}

pub struct LayoutButton {
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

impl Component for LayoutButton {
	type Input = LayoutButtonInput;
	type Message = Message;

	fn create(_: &Self::Input, _: &ComponentContext<Self>) -> Self {
		Self {
			hovered: false,
			pressed: false,
		}
	}

	fn update(&mut self, message: Message, _: &ComponentContext<Self>) {
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
					crate::komorebi::cycle_layout(crate::komorebi::CycleDirection::Next);
				}

				self.pressed = false;
			}

			Message::Cancel => {
				self.pressed = false;
			}
		}
	}

	fn view(&self, input: &Self::Input, context: &mut ViewContext<Self>) -> View {
		let highlighted = self.hovered;

		let mut text = TextBlock::new()
			.text(&input.text)
			.font_size(14.0)
			.font_weight(FontWeight::new(input.font_weight).unwrap_or(FontWeight::NORMAL))
			.foreground(ThemeBrush::PrimaryText)
			.opacity(if highlighted { 1.0 } else { 0.75 })
			.horizontal_alignment(HorizontalAlignment::Center)
			.vertical_alignment(VerticalAlignment::Center);

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
			// Border thickness contributes 1 DIP on each side; padding supplies
			// the remaining 7 horizontally and 3 vertically: 1 + 7 = 8, 1 + 3 = 4.
			.padding(Thickness::xy(7.0, 3.0))
			.automation_id("layout")
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
			.content(text)
			.into()
	}
}
