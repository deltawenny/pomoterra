use nsh_core::prelude::*;

use super::phase_color;
use crate::icons::{Icon, icon};
use crate::store::TimerState;

pub(crate) fn tab_header(timer: Signal<TimerState>) -> View {
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
                .border_color(move || rgb(phase_color(timer.get().phase)))
                .child(dynamic(move || {
                    icon(Icon::Timer, phase_color(timer.get().phase), 16.0)
                }))
                .child(
                    text("Timer")
                        .text_size(px(13.0))
                        .text_color(move || rgb(phase_color(timer.get().phase))),
                ),
        )
}
