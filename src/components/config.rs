use super::input::{input_content, input_group};
use nsh_base::{SliderValue, slider};
use nsh_core::prelude::*;

use super::GREY;
use crate::store::{TimerRules, TimerState};

#[derive(Clone, Copy)]
enum Rule {
    Focus,
    ShortBreak,
    LongBreak,
    Rounds,
}

impl Rule {
    fn value(self, rules: &TimerRules) -> u32 {
        match self {
            Self::Focus => rules.focus_minutes,
            Self::ShortBreak => rules.short_break_minutes,
            Self::LongBreak => rules.long_break_minutes,
            Self::Rounds => rules.rounds,
        }
    }
}

pub(crate) fn config_tab(
    timer: Signal<TimerState>,
    library: Signal<crate::persistence::Library>,
    active: Signal<i64>,
) -> View {
    view()
        .flex_1()
        .px(px(20.0))
        .pt(px(23.0))
        .pb(px(24.0))
        .child(super::profiles::profile_settings(timer, library, active))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .mb(px(18.0))
                .child(section_title("RULES"))
                .child(
                    view()
                        .role(AccessibilityRole::Button)
                        .label("Restore Default")
                        .on_press(move || {
                            timer.update(|state| {
                                state.configure(|rules| *rules = TimerRules::default())
                            })
                        })
                        .child(
                            text("Restore Default")
                                .text_size(px(13.0))
                                .text_color(rgb(GREY)),
                        ),
                ),
        )
        .child(rule_slider(timer, Rule::Focus, "Stay focused", 120.0))
        .child(rule_slider(timer, Rule::ShortBreak, "Short break", 60.0))
        .child(rule_slider(timer, Rule::LongBreak, "Long break", 60.0))
        .child(rule_slider(timer, Rule::Rounds, "Session rounds", 10.0))
        .child(section_title("SPECIAL BREAKS").mt(px(9.0)).mb(px(14.0)))
        .child(
            view()
                .gap(px(12.0))
                .child(special_row(timer, 0))
                .child(special_row(timer, 2)),
        )
}

fn section_title(label: &'static str) -> Text {
    text(label).text_size(px(14.0)).text_color(rgb(0x111111))
}

fn rule_slider(timer: Signal<TimerState>, rule: Rule, label: &'static str, max: f64) -> View {
    view()
        .h(px(54.0))
        .flex_shrink_0()
        .child(
            view()
                .flex_row()
                .justify_between()
                .child(text(label).text_size(px(13.0)))
                .child(
                    text(move || {
                        let value = rule.value(&timer.get().rules);
                        format!(
                            "{value} {}",
                            if matches!(rule, Rule::Rounds) {
                                "rounds"
                            } else {
                                "mins"
                            }
                        )
                    })
                    .text_size(px(13.0)),
                ),
        )
        .child(
            slider()
                .min(1.0)
                .max(max)
                .step(1.0)
                .value(move || SliderValue::Single(f64::from(rule.value(&timer.get().rules))))
                .accessibility_label(label)
                .track_class("h-6 justify-center")
                .indicator_class("h-1.5 rounded-full bg-[#e5e5e5]")
                .range_class("h-full rounded-full bg-[#0083c9]")
                .thumb_class("w-[19px] h-3 rounded-full border border-[#0083c9] bg-white")
                .on_change(move |value: &SliderValue| {
                    timer.update(|state| {
                        state.configure(|rules| {
                            let value = value.start().round() as u32;
                            match rule {
                                Rule::Focus => rules.focus_minutes = value,
                                Rule::ShortBreak => rules.short_break_minutes = value,
                                Rule::LongBreak => rules.long_break_minutes = value,
                                Rule::Rounds => rules.rounds = value,
                            }
                        })
                    });
                }),
        )
}

fn special_row(timer: Signal<TimerState>, start: usize) -> View {
    view()
        .flex_row()
        .gap(px(12.0))
        .child(special_break(timer, start))
        .child(special_break(timer, start + 1))
}

fn special_break(timer: Signal<TimerState>, index: usize) -> View {
    let invalid = move || {
        let entry = &timer.get().rules.special_breaks[index];
        (!entry.time.is_empty() || !entry.minutes.is_empty()) && entry.schedule().is_none()
    };
    input_group(invalid)
        .flex_1()
        .child(
            input_content()
                .flex_1()
                .h_full()
                .label(format!("Special break {} start time (HH:MM)", index + 1))
                .placeholder("--:--")
                .value(move || timer.get().rules.special_breaks[index].time.clone())
                .on_change(move |value: &str| {
                    timer.update(|state| {
                        state.rules.special_breaks[index].time = value.to_owned();
                    })
                }),
        )
        .child(
            input_content()
                .w(px(54.0))
                .h_full()
                .border_l_1()
                .border_color(rgb(0xcccccc))
                .label(format!(
                    "Special break {} duration in minutes (1–180)",
                    index + 1
                ))
                .placeholder("min")
                .value(move || timer.get().rules.special_breaks[index].minutes.clone())
                .on_change(move |value: &str| {
                    timer.update(|state| {
                        state.rules.special_breaks[index].minutes = value.to_owned();
                    })
                }),
        )
}
