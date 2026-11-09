//! Native desktop integration; mobile and web do not compile this module.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "windows", target_os = "macos"))]
mod native_tray;
mod tray_image;
#[cfg(target_os = "windows")]
mod windows;

pub(crate) fn update_timer(timer: &crate::store::TimerState) {
    tray_image::publish(timer);
}

#[cfg(target_os = "linux")]
pub(crate) use linux::install;
#[cfg(target_os = "macos")]
pub(crate) use macos::install;
#[cfg(target_os = "windows")]
pub(crate) use windows::install;
