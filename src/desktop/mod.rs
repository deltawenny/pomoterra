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

/// Scale the bundled 64px application logo for native tray backends.
fn icon_rgba(size: usize) -> Result<Vec<u8>, String> {
    const SOURCE: &[u8; 64 * 64 * 4] = include_bytes!("../../assets/icons/pomoterra.rgba");
    if size == 0 || size > 256 {
        return Err("Tray icon size must be between 1 and 256".into());
    }
    let mut data = vec![0; size * size * 4];
    // Area averaging preserves the logo's thin lines at small tray sizes.
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0_u32; 4];
            let mut count = 0;
            for sy in y * 64 / size..((y + 1) * 64 / size).max(y * 64 / size + 1) {
                for sx in x * 64 / size..((x + 1) * 64 / size).max(x * 64 / size + 1) {
                    let pixel = &SOURCE[(sy * 64 + sx) * 4..][..4];
                    for channel in 0..4 {
                        sum[channel] += u32::from(pixel[channel]);
                    }
                    count += 1;
                }
            }
            for channel in 0..4 {
                data[(y * size + x) * 4 + channel] = (sum[channel] / count) as u8;
            }
        }
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    #[test]
    fn logo_pixels_have_expected_dimensions_and_transparency() {
        for size in [16, 24, 32, 48, 64, 128, 256] {
            let pixels = super::icon_rgba(size).unwrap();
            assert_eq!(pixels.len(), size * size * 4);
            assert_eq!(pixels[3], 0);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] == 255));
        }
        assert!(super::icon_rgba(0).is_err());
        assert!(super::icon_rgba(257).is_err());
    }
}
