use std::time::Instant;

use nsh_core::element::desktop;
use nsh_core::prelude::*;

use super::GREY;
use crate::icons::{Icon, icon};
use crate::store::TimerState;

pub(crate) fn timer_controls(timer: Signal<TimerState>, announcements: Signal<bool>) -> View {
    view()
        .h(px(56.0))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(session_counter(timer))
        .child(
            view()
                .flex_1()
                .flex_row()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .gap(px(4.0))
                .child(control("Reset timer", Icon::RotateCcw, move || {
                    timer.update(TimerState::reset)
                }))
                .child(play_pause_button(timer))
                .child(control("Skip session", Icon::SkipForward, move || {
                    timer.update(TimerState::skip)
                }))
                .child(announcement_toggle(announcements)),
        )
        .child(control(
            "Toggle full screen",
            Icon::Maximize,
            desktop::toggle_full_screen,
        ))
}

fn session_counter(timer: Signal<TimerState>) -> View {
    view()
        .w_14()
        .items_center()
        .gap(px(3.0))
        .child(text(move || format!("{} / 4\nSessions", timer.get().session)).text_size(px(14.0)))
}

fn play_pause_button(timer: Signal<TimerState>) -> View {
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
        .label(move || {
            if timer.get().running {
                "Pause timer"
            } else {
                "Start timer"
            }
        })
        .hover(|s| s.bg(rgb(0xf2f7fa)))
        .active(|s| s.opacity(0.5))
        .on_press(move || timer.update(|state| state.toggle(Instant::now())))
        .child(dynamic(move || {
            icon(
                if timer.get().running {
                    Icon::Pause
                } else {
                    Icon::Play
                },
                GREY,
                28.0,
            )
        }))
}

fn announcement_toggle(announcements: Signal<bool>) -> View {
    view()
        .w(px(38.0))
        .h(px(44.0))
        .items_center()
        .justify_center()
        .role(AccessibilityRole::Button)
        .label(move || {
            if announcements.get() {
                "Mute announcements"
            } else {
                "Enable announcements"
            }
        })
        .on_press(move || announcements.update(|value| *value = !*value))
        .child(dynamic(move || {
            icon(
                if announcements.get() {
                    Icon::Volume2
                } else {
                    Icon::VolumeX
                },
                GREY,
                23.0,
            )
        }))
}

fn control(label: &'static str, name: Icon, action: impl Fn() + 'static) -> View {
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
        .child(icon(name, GREY, 23.0))
}
