use nsh_core::prelude::*;

use super::{BLUE, GREY, phase_color};
use crate::icons::{Icon, icon};
use crate::store::TimerState;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Tab {
    Settings,
    Config,
    Timer,
    History,
}

pub(crate) fn tab_header(selected: Signal<Tab>, timer: Signal<TimerState>) -> View {
    view()
        .h(px(57.0))
        .flex_shrink_0()
        .flex_row()
        .border_b_1()
        .border_color(rgb(0xeeeeee))
        .child(
            view()
                .flex_1()
                .h_full()
                .flex_row()
                .justify_center()
                .child(tab(selected, timer, Tab::Config, "Config", Icon::Clock))
                .child(tab(selected, timer, Tab::Timer, "Timer", Icon::Timer))
                .child(tab(selected, timer, Tab::History, "History", Icon::History))
                .child(tab(
                    selected,
                    timer,
                    Tab::Settings,
                    "Settings",
                    Icon::Settings,
                )),
        )
}

fn tab(
    selected: Signal<Tab>,
    timer: Signal<TimerState>,
    tab: Tab,
    label: &'static str,
    name: Icon,
) -> View {
    let color = move || {
        if selected.get() != tab {
            GREY
        } else if tab == Tab::Timer {
            phase_color(timer.get().phase)
        } else {
            BLUE
        }
    };
    view()
        .w(px(85.0))
        .flex_shrink_0()
        .h_full()
        .items_center()
        .justify_center()
        .gap(px(3.0))
        .border_b_1()
        .border_color(move || {
            rgb(if selected.get() == tab {
                color()
            } else {
                0xeeeeee
            })
        })
        .role(AccessibilityRole::Button)
        .label(crate::i18n::tr(label))
        .on_press(move || selected.set(tab))
        .child(dynamic(move || icon(name, color(), 16.0)))
        .child(
            text(crate::i18n::tr(label))
                .text_size(move || {
                    px(
                        if crate::i18n::language() == crate::i18n::Language::Spanish {
                            11.0
                        } else {
                            13.0
                        },
                    )
                })
                .text_color(move || rgb(color())),
        )
}
