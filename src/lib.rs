//! A compact, single-tab Pomodoro timer.
mod store;

use nsh_core::element::{ImageSrc, desktop, input};
use nsh_core::prelude::*;
use std::time::{Duration, Instant};
use store::{Phase, TimerState};

#[native_module("pomoterra/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

const BLUE: u32 = 0x0083c9;
const GREY: u32 = 0x686868;
const TIMER_ICON: &str = "<path d='M12 3a9 9 0 1 1-6.36 2.64M12 1v4M4 3l3 3M12 7v5l3 2'/>";
const LAPTOP: &str =
    "<rect x='4' y='4' width='16' height='13' rx='1.5'/><path d='M2 20h20l-2-3H4z'/>";
const PLAY: &str = "<path d='M8 5l11 7-11 7z'/>";
const PAUSE: &str = "<path d='M8 5v14M16 5v14'/>";
const RESET: &str = "<path d='M17 7a8 8 0 1 0 3 6M12 3l5 4-5 4'/>";
const SKIP: &str = "<path d='M5 4l12 8-12 8zM19 4v16'/>";
const VOLUME: &str = "<path d='M3 9h4l5-4v14l-5-4H3zM16 8a6 6 0 0 1 0 8M19 5a10 10 0 0 1 0 14'/>";
const MUTED: &str = "<path d='M3 9h4l5-4v14l-5-4H3zM17 9l5 6M22 9l-5 6'/>";
const EXPAND: &str = "<path d='M9 3H3v6M15 21h6v-6M3 21l18-18M15 3h6v6M3 15v6h6'/>";

fn icon(paths: &str, color: u32, width: f32) -> Svg {
    svg(ImageSrc::svg_markup(&format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='#{color:06x}' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'>{paths}</svg>"
    )))
    .w(px(width))
    .h(px(width))
}

fn control(label: &'static str, paths: &'static str, action: impl Fn() + 'static) -> View {
    view()
        .w(px(38.0))
        .h(px(44.0))
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .role(AccessibilityRole::Button)
        .label(label)
        .hover(|s| s.bg(rgb(0xf2f7fa)))
        .active(|s| s.opacity(0.5))
        .on_press(action)
        .child(icon(paths, GREY, 23.0))
}

/// The sole Timer tab; the operating system supplies the window chrome.
#[component]
pub fn App() -> impl IntoElement {
    let timer = signal(TimerState::default());
    let announcements = signal(true);
    let note = signal(String::new());
    let editing = signal(false);

    set_interval(Duration::from_millis(200), move || {
        if timer.get().running {
            let mut finished = false;
            timer.update(|state| finished = state.tick(Instant::now()));
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
        .child(
            view()
                .h(px(57.0))
                .flex_shrink_0()
                .border_b_1()
                .border_color(rgb(0xeeeeee))
                .items_center()
                .child(
                    view()
                        .w(px(86.0))
                        .h_full()
                        .items_center()
                        .justify_center()
                        .gap(px(3.0))
                        .border_b_1()
                        .border_color(rgb(BLUE))
                        .child(icon(TIMER_ICON, BLUE, 16.0))
                        .child(text("Timer").text_size(px(13.0)).text_color(rgb(BLUE))),
                ),
        )
        .child(
            view()
                .flex_1()
                .items_center()
                .justify_center()
                .py(px(24.0))
                .child(
                    view()
                        .w(px(220.0))
                        .h(px(220.0))
                        .flex_shrink_0()
                        .items_center()
                        .justify_center()
                        .child(
                            svg(move || {
                                let state = timer.get();
                                let circumference = 2.0 * std::f64::consts::PI * 107.0;
                                let progress = state.remaining as f64 / state.duration() as f64;
                                ImageSrc::svg_markup(&format!(
                                    "<svg xmlns='http://www.w3.org/2000/svg' width='220' height='220' viewBox='0 0 220 220'><circle cx='110' cy='110' r='107' fill='none' stroke='#edf3f7' stroke-width='6'/><circle cx='110' cy='110' r='107' fill='none' stroke='#0083c9' stroke-width='6' stroke-dasharray='{circumference}' stroke-dashoffset='{}' transform='rotate(-90 110 110)'/></svg>",
                                    circumference * (1.0 - progress)
                                ))
                            })
                            .absolute()
                            .w(px(220.0))
                            .h(px(220.0)),
                        )
                        .child(icon(LAPTOP, GREY, 40.0))
                        .child(
                            text(move || timer.get().display())
                                .text_size(px(40.0))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(rgb(BLUE))
                                .mt(px(10.0)),
                        )
                        .child(
                            text(move || match timer.get().phase {
                                Phase::Focus => "Stay Focused",
                                Phase::Break => "Take a Break",
                            })
                            .text_size(px(18.0))
                            .mt(px(3.0)),
                        ),
                ),
        )
        .child(
            view()
                .px(px(16.0))
                .pb(px(17.0))
                .gap(px(18.0))
                .child(
                    view()
                        .h(px(56.0))
                        .bg(rgb(0xfafafa))
                        .border_1()
                        .border_color(rgb(0xefefef))
                        .rounded(px(3.0))
                        .flex_row()
                        .items_center()
                        .child(
                            when(move || editing.get(), move || {
                                    input()
                                        .flex_1()
                                        .h_full()
                                        .px(px(10.0))
                                        .text_size(px(14.0))
                                        .label("Session note")
                                        .placeholder("What are you focusing on?")
                                        .value(move || note.get())
                                        .on_change(move |value: &str| note.set(value.to_owned()))
                                })
                                .otherwise(move || {
                                    text(move || note.get())
                                        .flex_1()
                                        .px(px(10.0))
                                        .text_size(px(14.0))
                                }),
                        )
                        .child(
                            view()
                                .w(px(30.0))
                                .h_full()
                                .items_center()
                                .pt(px(8.0))
                                .role(AccessibilityRole::Button)
                                .label(move || if editing.get() { "Save note" } else { "Edit note" })
                                .on_press(move || editing.update(|value| *value = !*value))
                                .child(icon("<circle cx='12' cy='5' r='1'/><circle cx='12' cy='12' r='1'/><circle cx='12' cy='19' r='1'/>", GREY, 16.0)),
                        ),
                )
                .child(
                    view()
                        .h(px(56.0))
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            view()
                                .w(px(72.0))
                                .items_center()
                                .gap(px(3.0))
                                .child(text(move || format!("{} / 4", timer.get().session)).text_size(px(14.0)))
                                .child(text("Sessions").text_size(px(14.0))),
                        )
                        .child(control("Reset timer", RESET, move || timer.update(TimerState::reset)))
                        .child(
                            view()
                                .w(px(56.0))
                                .h(px(56.0))
                                .flex_shrink_0()
                                .rounded(px(28.0))
                                .border_2()
                                .border_color(rgb(GREY))
                                .items_center()
                                .justify_center()
                                .role(AccessibilityRole::Button)
                                .label(move || if timer.get().running { "Pause timer" } else { "Start timer" })
                                .hover(|s| s.bg(rgb(0xf2f7fa)))
                                .active(|s| s.opacity(0.5))
                                .on_press(move || timer.update(|state| state.toggle(Instant::now())))
                                .child(dynamic(move || icon(if timer.get().running { PAUSE } else { PLAY }, GREY, 28.0))),
                        )
                        .child(control("Skip session", SKIP, move || timer.update(TimerState::skip)))
                        .child(
                            view()
                                .w(px(38.0))
                                .h(px(44.0))
                                .items_center()
                                .justify_center()
                                .role(AccessibilityRole::Button)
                                .label(move || if announcements.get() { "Mute announcements" } else { "Enable announcements" })
                                .on_press(move || announcements.update(|value| *value = !*value))
                                .child(dynamic(move || icon(if announcements.get() { VOLUME } else { MUTED }, GREY, 23.0))),
                        )
                        .child(control("Toggle full screen", EXPAND, desktop::toggle_full_screen)),
                ),
        )
}
