use nsh_core::prelude::*;

use super::BLUE;
use crate::icons::{Icon, icon};

pub(crate) fn tab_header() -> View {
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
                .child(icon(Icon::Timer, BLUE, 16.0))
                .child(text("Timer").text_size(px(13.0)).text_color(rgb(BLUE))),
        )
}
