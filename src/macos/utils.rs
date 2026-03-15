pub fn ns_color_from_color(color: &str) -> Option<Retained<NSColor>> {
    let rgba = color::parse_color(color)
        .ok()?
        .to_alpha_color::<color::Srgb>()
        .to_rgba8();
    Some(NSColor::colorWithSRGBRed_green_blue_alpha(
        f64::from(rgba.r) / 255.0,
        f64::from(rgba.g) / 255.0,
        f64::from(rgba.b) / 255.0,
        f64::from(rgba.a) / 255.0,
    ))
}
