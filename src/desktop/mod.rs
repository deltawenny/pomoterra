//! Native desktop integration; mobile and web do not compile this module.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "windows", target_os = "macos"))]
mod native_tray;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub(crate) use linux::install;
#[cfg(target_os = "macos")]
pub(crate) use macos::install;
#[cfg(target_os = "windows")]
pub(crate) use windows::install;

/// Render a catalog glyph to RGBA, shared by all three desktop backends.
fn icon_rgba(size: usize) -> Result<Vec<u8>, String> {
    let font = fontdue::Font::from_bytes(
        lucide_icons::LUCIDE_FONT_BYTES,
        fontdue::FontSettings::default(),
    )
    .map_err(str::to_owned)?;
    let (metrics, bitmap) =
        font.rasterize(char::from(lucide_icons::Icon::Timer), size as f32 * 0.8);
    let mut data = vec![0; size * size * 4];
    let left = size.saturating_sub(metrics.width) / 2;
    let top = size.saturating_sub(metrics.height) / 2;
    for y in 0..metrics.height.min(size) {
        for x in 0..metrics.width.min(size) {
            let offset = ((top + y) * size + left + x) * 4;
            data[offset..offset + 4].copy_from_slice(&[
                0x26,
                0x86,
                0xeb,
                bitmap[y * metrics.width + x],
            ]);
        }
    }
    Ok(data)
}
