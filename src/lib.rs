//! A compact Pomodoro timer with configurable rules and scheduled breaks.
rust_i18n::i18n!("locales", fallback = "en");

mod audio;
mod components;
mod i18n;
mod icons;
mod persistence;
mod store;

#[cfg(all(
    feature = "native",
    not(feature = "gpui"),
    any(target_os = "linux", target_os = "windows", target_os = "macos")
))]
mod desktop;

use chrono::{Datelike, Local, Timelike};
use components::{
    GREY, Tab, choose_profile, config_tab, history_tab, profile_picker, quick_profile_switcher,
    session_note, settings_tab, tab_header, timer_controls, timer_display,
};
use nsh_core::prelude::*;
use persistence::Library;
use std::time::{Duration, Instant};
use store::TimerState;

#[native_module("pomoterra/native")]
fn native(registry: &mut Registry) {
    registry.assets(icons::ASSETS);
    registry.register::<App>();
}

/// Remove only successfully saved records; failed writes stay queued for retry.
fn flush_completed(timer: Signal<TimerState>, library: Signal<Library>, profile_id: i64) {
    let pending = timer.get().completed.clone();
    if pending.is_empty() {
        return;
    }
    let data = library.get();
    let Some(profile) = data.profiles.iter().find(|p| p.id == profile_id) else {
        return;
    };
    let Some(database) = data.database.as_ref() else {
        return;
    };
    let mut saved = 0;
    for focus in &pending {
        match database.record(profile, focus) {
            Ok(()) => saved += 1,
            Err(error) => {
                library.update(|data| data.error = error);
                break;
            }
        }
    }
    if saved > 0 {
        timer.update(|state| {
            state.completed.drain(..saved);
        });
        match database.sessions() {
            Ok(sessions) => library.update(|data| {
                data.sessions = sessions;
                data.error.clear();
            }),
            Err(error) => library.update(|data| data.error = error),
        }
    }
}

/// Minimum desktop content size in logical pixels, also used at startup.
/// Native window decorations are added by each platform, not included here.
pub const MIN_WINDOW_SIZE: (i32, i32) = (340, 552);

// NSH 0.4 has no window-size constraints API. Apply GTK's native minimum
// on its UI thread so this works with both `cargo run` and `nsh run`.
#[cfg(all(feature = "native", not(feature = "gpui"), target_os = "linux"))]
fn constrain_window_size() {
    use nsh_platform_linux::gtk4::{self as gtk, prelude::*};

    gtk::glib::idle_add_once(|| {
        for window in gtk::Window::list_toplevels() {
            if let Ok(window) = window.downcast::<gtk::ApplicationWindow>() {
                desktop::install(&window);
            }
        }
    });
}

/// The operating system supplies the window chrome.
#[component]
pub fn App() -> impl IntoElement {
    #[cfg(all(feature = "native", not(feature = "gpui"), target_os = "linux"))]
    constrain_window_size();
    #[cfg(all(
        feature = "native",
        not(feature = "gpui"),
        any(target_os = "windows", target_os = "macos")
    ))]
    desktop::install();

    let loaded = Library::load();
    let active_id = loaded
        .database
        .as_ref()
        .and_then(|db| db.selected())
        .filter(|id| loaded.profiles.iter().any(|p| p.id == *id))
        .or_else(|| loaded.profiles.first().map(|p| p.id))
        .unwrap_or(0);
    let mut initial_timer = TimerState::default();
    if let Some(profile) = loaded.profiles.iter().find(|p| p.id == active_id) {
        initial_timer.configure(|rules| *rules = profile.rules.clone());
    }
    let library = signal(loaded);
    i18n::provide(library);
    let active = signal(active_id);
    let timer = signal(initial_timer);
    #[cfg(all(
        feature = "native",
        not(feature = "gpui"),
        any(target_os = "linux", target_os = "windows", target_os = "macos")
    ))]
    effect(move || desktop::update_settings(&library.get().settings));
    #[cfg(all(
        feature = "native",
        not(feature = "gpui"),
        any(target_os = "linux", target_os = "windows", target_os = "macos")
    ))]
    effect(move || desktop::update_timer(&timer.get(), &library.get(), active.get()));
    // Observe only rule changes, not each countdown tick, before writing SQLite.
    let saved_rules = signal(timer.get().rules.clone());
    effect(move || {
        let rules = timer.get().rules.clone();
        if saved_rules.get() != rules {
            saved_rules.set(rules.clone());
            library.update(|data| {
                if data
                    .profiles
                    .iter()
                    .find(|p| p.id == active.get())
                    .is_some_and(|p| p.rules != rules)
                {
                    data.edit(active.get(), |profile| profile.rules = rules);
                }
            });
        }
    });
    let announcements = signal(true);
    let selected = signal(Tab::Timer);

    set_interval(Duration::from_millis(200), move || {
        #[cfg(all(
            feature = "native",
            not(feature = "gpui"),
            any(target_os = "linux", target_os = "windows", target_os = "macos")
        ))]
        if let Some(id) = desktop::take_profile_request() {
            // Ignore stale menu entries and selecting the already-active profile.
            if id != active.get() && library.get().settings.quick_profiles.contains(&id) {
                choose_profile(timer, library, active, id);
            }
        }
        #[cfg(all(
            feature = "native",
            not(feature = "gpui"),
            any(target_os = "linux", target_os = "windows", target_os = "macos")
        ))]
        if let Some(running) = desktop::take_playback_request() {
            if timer.get().running != running {
                timer.update(|state| state.toggle(Instant::now()));
            }
        }
        if timer.get().running {
            let mut finished = false;
            let mut finished_phase = None;
            timer.update(|state| {
                let now = Instant::now();
                let phase = state.phase;
                finished = state.tick(now);
                if finished {
                    finished_phase = Some(phase);
                }
                let local = Local::now();
                state.check_special_break(
                    local.date_naive().num_days_from_ce(),
                    local.hour() * 60 + local.minute(),
                    now,
                );
            });
            if let Some(phase) = finished_phase {
                let settings = library.get().settings;
                if announcements.get() {
                    audio::play_completion();
                    announce_for_accessibility("Session complete. Your next session is ready.");
                }
                let show = match phase {
                    store::Phase::Focus => settings.show_after_focus,
                    store::Phase::Break => settings.show_after_break,
                };
                #[cfg(all(
                    feature = "native",
                    not(feature = "gpui"),
                    any(target_os = "linux", target_os = "windows", target_os = "macos")
                ))]
                {
                    if show {
                        desktop::request_show();
                    }
                    if settings.system_notifications {
                        desktop::notify_completion(
                            phase,
                            i18n::Language::from_preference(&library.get().settings.language),
                        );
                    }
                }
                let continue_next = match phase {
                    store::Phase::Focus => settings.continue_after_focus,
                    store::Phase::Break => settings.continue_after_break,
                };
                if continue_next {
                    timer.update(|state| state.toggle(Instant::now()));
                }
            }
        }
        // Also catches completion discovered by pause and retries failed writes.
        if !timer.get().completed.is_empty() {
            flush_completed(timer, library, active.get());
        }
    });

    view()
        .flex_1()
        .bg(rgb(0xffffff))
        .text_color(rgb(GREY))
        .p_safe_area()
        .child(tab_header(selected, timer))
        .child(when(
            move || !library.get().error.is_empty(),
            move || {
                text(move || library.get().error.clone())
                    .text_size(px(12.0))
                    .text_color(rgb(0xb34230))
                    .px(px(16.0))
                    .py(px(8.0))
            },
        ))
        .child(
            view()
                .flex_1()
                .style_when(move || selected.get() != Tab::Settings, |s| s.hidden())
                .child(nsh_core::element::scroll().flex_1().child(settings_tab(
                    timer,
                    library,
                    active,
                    announcements,
                ))),
        )
        .child(
            view()
                .flex_1()
                .style_when(move || selected.get() != Tab::History, |s| s.hidden())
                .child(history_tab(library)),
        )
        .child(
            view()
                .flex_1()
                .style_when(move || selected.get() != Tab::Config, |s| s.hidden())
                .child(
                    nsh_core::element::scroll()
                        .flex_1()
                        .child(config_tab(timer, library, active)),
                ),
        )
        .child(
            view()
                .flex_1()
                .style_when(move || selected.get() != Tab::Timer, |s| s.hidden())
                .child(
                    view()
                        .px(px(16.0))
                        .pt(px(8.0))
                        .flex_shrink_0()
                        .child(profile_picker(timer, library, active, || {}))
                        .child(quick_profile_switcher(timer, library, active)),
                )
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
