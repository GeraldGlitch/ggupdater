use eframe::egui;

pub const DEVELOPER_LOGO_PNG: &[u8] = include_bytes!("../assets/developer_logo.png");
pub const LOCAL_BANNER_PNG: &[u8] = include_bytes!("../assets/GG.png");

/// Decodifica bytes de imagen (PNG/JPG/WebP) a RGBA8.
pub fn decode_rgba(bytes: &[u8]) -> Option<(usize, usize, Vec<u8>)> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    Some((width as usize, height as usize, image.into_raw()))
}

pub fn to_color_image(bytes: &[u8]) -> Option<egui::ColorImage> {
    let (width, height, rgba) = decode_rgba(bytes)?;
    Some(egui::ColorImage::from_rgba_unmultiplied([width, height], &rgba))
}
