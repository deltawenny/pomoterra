use super::input::{input_content, input_group};
use nsh_core::prelude::*;

use super::{GREY, phase_color};
use crate::icons::{Icon, icon};
use crate::store::TimerState;

pub(crate) fn session_note(timer: Signal<TimerState>) -> View {
    let hovered = signal(false);
    let editing = signal(false);

    input_group(|| false)
        .child(
            when(
                move || editing.get(),
                move || {
                    input_content()
                        .flex_1()
                        .h_full()
                        .label(crate::i18n::tr("Session note"))
                        .placeholder(crate::i18n::tr("What are you focusing on?"))
                        .value(move || timer.get().note.clone())
                        .on_change(move |value: &str| {
                            timer.update(|state| state.note = value.to_owned())
                        })
                },
            )
            .otherwise(move || {
                text(move || timer.get().note.clone())
                    .flex_1()
                    .px(px(10.0))
                    .text_size(px(12.0))
                    .text_color(rgb(0x333333))
            }),
        )
        .child(
            view()
                .w(px(30.0))
                .h_full()
                .items_center()
                .justify_center()
                .role(AccessibilityRole::Button)
                .label(move || {
                    if editing.get() {
                        crate::i18n::t("Save note")
                    } else {
                        crate::i18n::t("Edit note")
                    }
                })
                .on_mouse_enter(move |_: &MouseEvent| hovered.set(true))
                .on_mouse_leave(move |_: &MouseEvent| hovered.set(false))
                .on_press(move || editing.update(|value| *value = !*value))
                .child(dynamic(move || {
                    icon(
                        Icon::EllipsisVertical,
                        if hovered.get() {
                            phase_color(timer.get().phase)
                        } else {
                            GREY
                        },
                        16.0,
                    )
                })),
        )
}
