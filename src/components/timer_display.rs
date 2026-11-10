use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use super::{GREY, phase_color};
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
                            "<svg xmlns='http://www.w3.org/2000/svg' width='220' height='220' viewBox='0 0 220 220'><circle cx='110' cy='110' r='107' fill='none' stroke='#edf3f7' stroke-width='6'/><circle cx='110' cy='110' r='107' fill='none' stroke='#{:06x}' stroke-width='6' stroke-dasharray='{circumference}' stroke-dashoffset='{}' transform='rotate(-90 110 110)'/></svg>",
                            phase_color(state.phase),
                            circumference * (1.0 - progress)
                        ))
                    })
                    .absolute()
                    .w(px(220.0))
                    .h(px(220.0)),
                )
                .child(dynamic(move || {
                    icon(
                        match timer.get().phase {
                            Phase::Focus => Icon::LaptopMinimal,
                            Phase::Break => Icon::Coffee,
                        },
                        GREY,
                        40.0,
                    )
                }))
                .child(
                    text(move || timer.get().display())
                        .text_size(px(40.0))
                        .font_weight(FontWeight::NORMAL)
                        .text_color(move || rgb(phase_color(timer.get().phase)))
                        .mt(px(0.0)),
                )
                .child(
                    text(move || match timer.get().phase {
                        Phase::Focus => crate::i18n::t("stay_focused"),
                        Phase::Break => crate::i18n::t("take_a_break"),
                    })
                    .text_size(px(18.0))
                    .mt(px(3.0)),
                ),
        )
}
