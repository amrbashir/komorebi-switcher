use std::fmt::Display;

pub fn find_font(family: &str, weight: u16) -> Option<font_kit::font::Font> {
    use font_kit::family_name::FamilyName;
    use font_kit::properties::{Properties, Weight};
    use font_kit::source::SystemSource;

    SystemSource::new()
        .select_best_match(
            &[FamilyName::Title(family.to_string())],
            &Properties {
                weight: Weight(weight as f32),
                ..Default::default()
            },
        )
        .ok()?
        .load()
        .ok()
}

pub fn error_dialog<T: Display>(error: T) {
    rfd::MessageDialog::new()
        .set_title("komorebi-switcher")
        .set_description(error.to_string())
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}
