use std::time::Instant;

use nsh_core::element::desktop;
use nsh_core::prelude::*;

use super::{GREY, phase_color};
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
                .child(control(timer, "Reset timer", Icon::RotateCcw, move || {
                    timer.update(TimerState::reset)
                }))
                .child(play_pause_button(timer))
                .child(control(
                    timer,
                    "Skip session",
                    Icon::SkipForward,
                    move || timer.update(TimerState::skip),
                ))
                .child(announcement_toggle(timer, announcements)),
        )
        .child(control(
            timer,
            "Toggle full screen",
            Icon::Maximize,
            desktop::toggle_full_screen,
        ))
}

fn session_counter(timer: Signal<TimerState>) -> View {
    view()
        .w(px(64.0))
        .flex_shrink_0()
        .items_center()
        .gap(px(3.0))
        .child(
            text(move || {
                let state = timer.get();
                format!("{} / {}", state.session, state.rules.rounds)
            })
            .text_size(px(14.0)),
        )
        .child(text(crate::i18n::tr("sessions_106")).text_size(move || {
            px(
                if crate::i18n::language() != crate::i18n::Language::English {
                    12.0
                } else {
                    14.0
                },
            )
        }))
}

fn play_pause_button(timer: Signal<TimerState>) -> View {
    let hovered = signal(false);
    let color = move || {
        if hovered.get() {
            phase_color(timer.get().phase)
        } else {
            GREY
        }
    };
    view()
        .w(px(56.0))
        .h(px(56.0))
        .flex_shrink_0()
        .rounded(px(28.0))
        .border_2()
        .border_color(move || rgb(color()))
        .items_center()
        .justify_center()
        .role(AccessibilityRole::Button)
        .label(move || {
            if timer.get().running {
                crate::i18n::t("pause_timer_161")
            } else {
                crate::i18n::t("start_timer_162")
            }
        })
        .on_mouse_enter(move |_: &MouseEvent| hovered.set(true))
        .on_mouse_leave(move |_: &MouseEvent| hovered.set(false))
        .active(|s| s.opacity(0.5))
        .on_press(move || timer.update(|state| state.toggle(Instant::now())))
        .child(dynamic(move || {
            icon(
                if timer.get().running {
                    Icon::Pause
                } else {
                    Icon::Play
                },
                color(),
                28.0,
            )
        }))
}

fn announcement_toggle(timer: Signal<TimerState>, announcements: Signal<bool>) -> View {
    let hovered = signal(false);
    view()
        .w(px(38.0))
        .h(px(44.0))
        .items_center()
        .justify_center()
        .role(AccessibilityRole::Button)
        .label(move || {
            if announcements.get() {
                crate::i18n::t("mute_announcements_163")
            } else {
                crate::i18n::t("enable_announcements_164")
            }
        })
        .on_mouse_enter(move |_: &MouseEvent| hovered.set(true))
        .on_mouse_leave(move |_: &MouseEvent| hovered.set(false))
        .active(|s| s.opacity(0.5))
        .on_press(move || announcements.update(|value| *value = !*value))
        .child(dynamic(move || {
            icon(
                if announcements.get() {
                    Icon::Volume2
                } else {
                    Icon::VolumeX
                },
                if hovered.get() {
                    phase_color(timer.get().phase)
                } else {
                    GREY
                },
                23.0,
            )
        }))
}

fn control(
    timer: Signal<TimerState>,
    label: &'static str,
    name: Icon,
    action: impl Fn() + 'static,
) -> View {
    let hovered = signal(false);
    view()
        .w(px(38.0))
        .h(px(44.0))
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .role(AccessibilityRole::Button)
        .label(crate::i18n::tr(label))
        .on_mouse_enter(move |_: &MouseEvent| hovered.set(true))
        .on_mouse_leave(move |_: &MouseEvent| hovered.set(false))
        .active(|s| s.opacity(0.5))
        .on_press(action)
        .child(dynamic(move || {
            icon(
                name,
                if hovered.get() {
                    phase_color(timer.get().phase)
                } else {
                    GREY
                },
                23.0,
            )
        }))
}
