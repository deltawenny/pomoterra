use nsh_core::prelude::*;

use super::{BLUE, GREY};
use crate::icons::{Icon, icon};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Tab {
    Config,
    Timer,
    History,
}

pub(crate) fn tab_header(selected: Signal<Tab>) -> View {
    view()
        .h(px(57.0))
        .flex_shrink_0()
        .flex_row()
        .border_b_1()
        .border_color(rgb(0xeeeeee))
        .justify_center()
        .child(tab(selected, Tab::Config, "Config", Icon::Clock))
        .child(tab(selected, Tab::Timer, "Timer", Icon::Timer))
        .child(tab(selected, Tab::History, "History", Icon::History))
}

fn tab(selected: Signal<Tab>, tab: Tab, label: &'static str, name: Icon) -> View {
    let color = move || if selected.get() == tab { BLUE } else { GREY };
    view()
        .w(px(85.0))
        .h_full()
        .items_center()
        .justify_center()
        .gap(px(3.0))
        .border_b_1()
        .border_color(move || {
            rgb(if selected.get() == tab {
                BLUE
            } else {
                0xeeeeee
            })
        })
        .role(AccessibilityRole::Button)
        .label(label)
        .on_press(move || selected.set(tab))
        .child(dynamic(move || icon(name, color(), 16.0)))
        .child(
            text(label)
                .text_size(px(13.0))
                .text_color(move || rgb(color())),
        )
}
