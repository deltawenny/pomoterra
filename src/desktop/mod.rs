//! Native desktop integration; mobile and web do not compile this module.
use std::sync::atomic::{AtomicBool, Ordering};

static CLOSE_TO_TRAY: AtomicBool = AtomicBool::new(true);
static SHOW_REQUESTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn update_settings(settings: &crate::persistence::AppSettings) {
    CLOSE_TO_TRAY.store(settings.close_to_tray, Ordering::Relaxed);
}

pub(crate) fn close_to_tray() -> bool {
    CLOSE_TO_TRAY.load(Ordering::Relaxed)
}

pub(crate) fn request_show() {
    SHOW_REQUESTED.store(true, Ordering::Release);
}

/// Notification services may block; never call them on the timer/UI thread.
pub(crate) fn notify_completion(phase: crate::store::Phase) {
    let body = match phase {
        crate::store::Phase::Focus => "Focus session complete. Time for a break.",
        crate::store::Phase::Break => "Break complete. Your next focus session is ready.",
    };
    let _ = std::thread::Builder::new()
        .name("completion-notification".into())
        .spawn(move || {
            if let Err(error) = notify_rust::Notification::new()
                .summary("Pomoterra")
                .body(body)
                .appname("Pomoterra")
                .show()
            {
                eprintln!("Unable to show session notification: {error}");
            }
        });
}

pub(crate) fn take_show_request() -> bool {
    SHOW_REQUESTED.swap(false, Ordering::AcqRel)
}
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "windows", target_os = "macos"))]
mod native_tray;
mod tray_image;
mod tray_menu;

pub(crate) fn take_playback_request() -> Option<bool> {
    tray_menu::take_playback_request()
}

pub(crate) fn take_profile_request() -> Option<i64> {
    tray_menu::take_profile_request()
}
#[cfg(target_os = "windows")]
mod windows;

pub(crate) fn update_timer(
    timer: &crate::store::TimerState,
    library: &crate::persistence::Library,
    active: i64,
) {
    tray_image::publish(timer);
    tray_menu::publish(timer, library, active);
}

#[cfg(target_os = "linux")]
pub(crate) use linux::install;
#[cfg(target_os = "macos")]
pub(crate) use macos::install;
#[cfg(target_os = "windows")]
pub(crate) use windows::install;
