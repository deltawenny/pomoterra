//! A compact Pomodoro timer with configurable rules and scheduled breaks.
mod components;
mod icons;
mod store;

use chrono::{Datelike, Local, Timelike};
use components::{GREY, Tab, config_tab, session_note, tab_header, timer_controls, timer_display};
use nsh_core::prelude::*;
use std::time::{Duration, Instant};
use store::TimerState;

#[native_module("pomoterra/native")]
fn native(registry: &mut Registry) {
    registry.assets(icons::ASSETS);
    registry.register::<App>();
}

const MAX_WIDTH: i32 = 340;
const MAX_HEIGHT: i32 = 515 - 27;

// NSH 0.4 has no window-size constraints API. Apply GTK's native minimum
// on its UI thread so this works with both `cargo run` and `nsh run`.
#[cfg(target_os = "linux")]
fn constrain_window_size() {
    use nsh_platform_linux::gtk4::{self as gtk, prelude::*};

    gtk::glib::idle_add_once(|| {
        for window in gtk::Window::list_toplevels() {
            if let Ok(window) = window.downcast::<gtk::ApplicationWindow>() {
                window.set_size_request(MAX_WIDTH, MAX_HEIGHT);
            }
        }
    });
}

/// The operating system supplies the window chrome.
#[component]
pub fn App() -> impl IntoElement {
    #[cfg(target_os = "linux")]
    constrain_window_size();

    let timer = signal(TimerState::default());
    let announcements = signal(true);
    let selected = signal(Tab::Timer);

    set_interval(Duration::from_millis(200), move || {
        if timer.get().running {
            let mut finished = false;
            timer.update(|state| {
                let now = Instant::now();
                finished = state.tick(now);
                let local = Local::now();
                state.check_special_break(
                    local.date_naive().num_days_from_ce(),
                    local.hour() * 60 + local.minute(),
                    now,
                );
            });
            if finished && announcements.get() {
                announce_for_accessibility("Session complete. Your next session is ready.");
            }
        }
    });

    view()
        .flex_1()
        .bg(rgb(0xffffff))
        .text_color(rgb(GREY))
        .p_safe_area()
        .child(tab_header(selected))
        .child(
            view()
                .flex_1()
                .style_when(move || selected.get() != Tab::Config, |s| s.hidden())
                .child(config_tab(timer)),
        )
        .child(
            view()
                .flex_1()
                .style_when(move || selected.get() != Tab::Timer, |s| s.hidden())
                .child(timer_display(timer))
                .child(
                    view()
                        .px(px(16.0))
                        .pb(px(17.0))
                        .flex_shrink_0()
                        .gap(px(18.0))
                        .child(session_note(timer))
                        .child(timer_controls(timer, announcements)),
                ),
        )
}
