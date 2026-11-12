use std::fmt::Display;

#[cfg(target_os = "macos")]
use font_kit::family_name::FamilyName;
#[cfg(target_os = "macos")]
use font_kit::properties::{Properties, Weight};
#[cfg(target_os = "macos")]
use font_kit::source::SystemSource;

#[cfg(target_os = "macos")]
pub fn find_font(family: &str, weight: u16) -> Option<font_kit::font::Font> {
	let family = FamilyName::Title(family.to_string());
	let properties = Properties {
		weight: Weight(weight as f32),
		..Default::default()
	};
	let system_source = SystemSource::new();
	let handle = system_source
		.select_best_match(&[family], &properties)
		.ok()?;
	handle.load().ok()
}

pub fn error_dialog<T: Display>(error: T) {
	rfd::MessageDialog::new()
		.set_title(crate::APP_NAME)
		.set_description(error.to_string())
		.set_level(rfd::MessageLevel::Error)
		.set_buttons(rfd::MessageButtons::Ok)
		.show();
}
