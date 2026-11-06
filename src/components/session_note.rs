use nsh_core::element::input;
use nsh_core::prelude::*;

use super::GREY;
use crate::icons::{Icon, icon};

pub(crate) fn session_note() -> View {
    let note = signal(String::new());
    let editing = signal(false);

    view()
        .h(px(56.0))
        .bg(rgb(0xfafafa))
        .border_1()
        .border_color(rgb(0xefefef))
        .rounded(px(3.0))
        .flex_row()
        .items_center()
        .child(
            when(
                move || editing.get(),
                move || {
                    input()
                        .flex_1()
                        .h_full()
                        .px(px(10.0))
                        .text_size(px(14.0))
                        .label("Session note")
                        .placeholder("What are you focusing on?")
                        .value(move || note.get())
                        .on_change(move |value: &str| note.set(value.to_owned()))
                },
            )
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
                .label(move || {
                    if editing.get() {
                        "Save note"
                    } else {
                        "Edit note"
                    }
                })
                .on_press(move || editing.update(|value| *value = !*value))
                .child(icon(Icon::EllipsisVertical, GREY, 16.0)),
        )
}
