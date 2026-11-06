use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use super::{BLUE, GREY};
use crate::icons::{Icon, icon};
use crate::store::{Phase, TimerState};

pub(crate) fn timer_display(timer: Signal<TimerState>) -> View {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .pt(px(32.0))
        .pb(px(24.0))
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
                .child(icon(Icon::LaptopMinimal, GREY, 40.0))
                .child(
                    text(move || timer.get().display())
                        .text_size(px(40.0))
                        .font_weight(FontWeight::NORMAL)
                        .text_color(rgb(BLUE))
                        .mt(px(0.0)),
                )
                .child(
                    text(move || match timer.get().phase {
                        Phase::Focus => "Stay Focused",
                        Phase::Break => "Take a Break",
                    })
                    .text_size(px(18.0))
                    .mt(px(3.0)),
                ),
        )
}
