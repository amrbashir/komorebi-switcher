use std::fmt::Display;

pub fn error_dialog<T: Display>(error: T) {
    rfd::MessageDialog::new()
        .set_title("komorebi-switcher")
        .set_description(error.to_string())
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}
