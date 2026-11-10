use super::{
    BLUE, GREY,
    combobox::{combobox, sorted_items},
};
use crate::persistence::{Library, Session, color};
use chrono::{Datelike, Days, Local, NaiveDate, TimeZone};
use nsh_base::SelectItem;
use nsh_core::{element::scroll, prelude::*};
use std::collections::BTreeMap;

mod sample;

#[derive(Clone, Copy, PartialEq)]
enum Period {
    Today,
    Week,
    Month,
    Year,
    All,
}
impl Period {
    fn start(self, today: NaiveDate) -> Option<NaiveDate> {
        match self {
            Self::Today => Some(today),
            Self::Week => {
                today.checked_sub_days(Days::new(today.weekday().num_days_from_monday() as u64))
            }
            Self::Month => today.with_day(1),
            Self::Year => today.with_ordinal(1),
            Self::All => None,
        }
    }
}

fn date(session: &Session) -> NaiveDate {
    Local
        .timestamp_opt(session.finished_at, 0)
        .single()
        .unwrap()
        .date_naive()
}

pub(crate) fn history_tab(library: Signal<Library>) -> impl IntoElement {
    let period = signal(Period::Week);
    let profile = signal(0_i64);
    let from_date = signal(String::new());
    let to_date = signal(String::new());
    let oldest = signal(false);
    let preview = signal(None::<Vec<Session>>);
    let displayed_sessions = move || preview.get().unwrap_or_else(|| library.get().sessions);
    scroll().flex_1().child(
        view()
            .p(px(16.0))
            .gap(px(16.0))
            .child(
                view()
                    .flex_row()
                    .justify_between()
                    .items_center()
                    .child(
                        text(crate::i18n::tr("focus_history"))
                            .text_size(px(17.0))
                            .text_color(rgb(0x222222)),
                    )
                    .child(super::profiles::text_button(
                        "reverse_date_order",
                        move || oldest.update(|v| *v = !*v),
                    )),
            )
            .child(
                view()
                    .gap(px(6.0))
                    .child(dynamic(move || {
                        super::profiles::text_button(
                            if preview.get().is_some() {
                                "show_real_history"
                            } else {
                                "show_sample_history"
                            },
                            move || {
                                preview.set(if preview.get().is_some() {
                                    None
                                } else {
                                    Some(sample::sessions(Local::now()))
                                });
                                profile.set(0);
                            },
                        )
                    }))
                    .child(when(
                        move || preview.get().is_some(),
                        || {
                            text(crate::i18n::tr("sample_preview_notice"))
                                .text_size(px(11.0))
                                .text_color(rgb(BLUE))
                        },
                    )),
            )
            .child(dynamic(move || {
                calendar(&displayed_sessions(), profile.get())
            }))
            .child(
                view()
                    .gap(px(6.0))
                    .child(
                        view().flex_row().gap(px(6.0)).children(
                            [
                                (Period::Today, "today"),
                                (Period::Week, "week"),
                                (Period::Month, "month"),
                                (Period::Year, "year"),
                                (Period::All, "all"),
                            ]
                            .into_iter()
                            .map(move |(value, title)| {
                                view()
                                    .flex_1()
                                    .py(px(7.0))
                                    .items_center()
                                    .rounded(px(4.0))
                                    .bg(move || {
                                        rgb(if period.get() == value {
                                            0xe7f4fc
                                        } else {
                                            0xf6f6f6
                                        })
                                    })
                                    .role(AccessibilityRole::Button)
                                    .label(move || {
                                        crate::i18n::message(
                                            "history_title",
                                            &[("title", crate::i18n::t(title))],
                                        )
                                    })
                                    .on_press(move || period.set(value))
                                    .child(
                                        text(crate::i18n::tr(title))
                                            .text_size(px(11.0))
                                            .text_color(rgb(BLUE)),
                                    )
                            }),
                        ),
                    )
                    .child(
                        combobox()
                            .items(move || {
                                let data = library.get();
                                let mut names: BTreeMap<i64, String> = displayed_sessions()
                                    .iter()
                                    .map(|s| (s.profile_id, s.profile_name.clone()))
                                    .collect();
                                if preview.get().is_none() {
                                    names.extend(
                                        data.profiles.iter().map(|p| (p.id, p.name.clone())),
                                    );
                                }
                                std::iter::once(SelectItem::new(
                                    "0",
                                    crate::i18n::t("all_profiles"),
                                ))
                                .chain(sorted_items(
                                    names
                                        .into_iter()
                                        .map(|(id, name)| SelectItem::new(id.to_string(), name)),
                                ))
                                .collect::<Vec<_>>()
                            })
                            .value(move || Some(vec![profile.get().to_string().into()]))
                            .accessibility_label(crate::i18n::optional("history_profile"))
                            .placeholder(crate::i18n::optional("search_profiles"))
                            .on_change(move |value: &Vec<SharedString>| {
                                if let Some(id) = value.first().and_then(|v| v.parse().ok()) {
                                    profile.set(id);
                                }
                            }),
                    )
                    .child(
                        view()
                            .flex_row()
                            .gap(px(6.0))
                            .child(
                                super::input::text_input(move || {
                                    !from_date.get().trim().is_empty()
                                        && parse_date(&from_date.get()).is_none()
                                })
                                .flex_1()
                                .label(crate::i18n::tr("from_date"))
                                .placeholder(crate::i18n::tr("yyyy_mm_dd"))
                                .value(move || from_date.get())
                                .on_change(move |value: &str| from_date.set(value.to_owned())),
                            )
                            .child(
                                super::input::text_input(move || {
                                    !to_date.get().trim().is_empty()
                                        && parse_date(&to_date.get()).is_none()
                                })
                                .flex_1()
                                .label(crate::i18n::tr("to_date"))
                                .placeholder(crate::i18n::tr("yyyy_mm_dd"))
                                .value(move || to_date.get())
                                .on_change(move |value: &str| to_date.set(value.to_owned())),
                            ),
                    ),
            )
            .child(dynamic(move || {
                let today = Local::now().date_naive();
                let start = period.get().start(today);
                let from = parse_date(&from_date.get());
                let to = parse_date(&to_date.get());
                let from_entered = !from_date.get().trim().is_empty();
                let to_entered = !to_date.get().trim().is_empty();
                let from_valid = !from_entered || from.is_some();
                let to_valid = !to_entered || to.is_some();
                let custom_range = from_entered || to_entered;
                let mut sessions: Vec<_> = displayed_sessions()
                    .into_iter()
                    .filter(|s| {
                        let session_date = date(s);
                        from_valid
                            && to_valid
                            && (profile.get() == 0 || s.profile_id == profile.get())
                            && session_date <= today
                            && if custom_range {
                                from.is_none_or(|from| session_date >= from)
                                    && to.is_none_or(|to| session_date <= to)
                            } else {
                                start.is_none_or(|start| session_date >= start)
                            }
                    })
                    .collect();
                if oldest.get() {
                    sessions.reverse();
                }
                let seconds: u64 = sessions.iter().map(|s| s.seconds).sum();
                let mut earnings: BTreeMap<String, i64> = BTreeMap::new();
                for session in &sessions {
                    if session.rate > 0 {
                        *earnings.entry(session.currency.clone()).or_default() +=
                            session.earnings();
                    }
                }
                let empty = sessions.is_empty();
                view()
                    .gap(px(14.0))
                    .child(
                        text(crate::i18n::message(
                            "session_totals",
                            &[
                                ("count", sessions.len().to_string()),
                                ("hours", (seconds / 3600).to_string()),
                                ("minutes", (seconds % 3600 / 60).to_string()),
                            ],
                        ))
                        .text_size(px(14.0))
                        .text_color(rgb(0x333333)),
                    )
                    .children(earnings.into_iter().map(|(currency, amount)| {
                        text(crate::i18n::message(
                            "amount_currency_earned",
                            &[("amount", money(amount)), ("currency", currency)],
                        ))
                        .text_size(px(12.0))
                        .text_color(rgb(0x008c69))
                    }))
                    .child(profile_chart(&sessions))
                    .child(
                        text(if oldest.get() {
                            crate::i18n::t("sessions_oldest_first")
                        } else {
                            crate::i18n::t("sessions_newest_first")
                        })
                        .text_size(px(11.0)),
                    )
                    .child(when(
                        move || empty,
                        || {
                            text(crate::i18n::tr("history_empty"))
                                .text_size(px(13.0))
                                .text_color(rgb(GREY))
                        },
                    ))
                    .children(sessions.into_iter().map(session_row))
            })),
    )
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    let value = value.trim();
    (!value.is_empty())
        .then(|| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .flatten()
}

fn calendar(sessions: &[Session], profile_id: i64) -> View {
    let today = Local::now().date_naive();
    let first = today.checked_sub_days(Days::new(364)).unwrap();
    let start = first
        .checked_sub_days(Days::new(first.weekday().num_days_from_sunday() as u64))
        .unwrap();
    let mut days: BTreeMap<NaiveDate, Vec<&Session>> = BTreeMap::new();
    for session in sessions.iter().filter(|s| {
        (profile_id == 0 || s.profile_id == profile_id) && date(s) >= first && date(s) <= today
    }) {
        days.entry(date(session)).or_default().push(session);
    }
    let count: usize = days.values().map(Vec::len).sum();
    let weeks = (today - start).num_days() / 7 + 1;
    view()
        .gap(px(8.0))
        .child(
            text(crate::i18n::message(
                "year_focus_sessions",
                &[("count", count.to_string())],
            ))
            .text_size(px(12.0)),
        )
        .child(
            scroll().horizontal().h(px(108.0)).child(
                view().w_full().flex_row().justify_center().child(
                    view()
                        .flex_row()
                        .gap(px(3.0))
                        .children((0..weeks).map(move |week| {
                            let week_start =
                                start.checked_add_days(Days::new(week as u64 * 7)).unwrap();
                            let label = if week == 0 || week_start.day() <= 7 {
                                week_start.format("%b").to_string().to_lowercase()
                            } else {
                                String::new()
                            };
                            view()
                                .gap(px(3.0))
                                .child(text(crate::i18n::tr(label)).text_size(px(8.0)).h(px(12.0)))
                                .children(
                                    (0..7)
                                        .map(|day| {
                                            let current = week_start
                                                .checked_add_days(Days::new(day))
                                                .unwrap();
                                            let entries = days.get(&current);
                                            let n = entries.map_or(0, |v| v.len());
                                            let base = entries
                                                .and_then(|v| v.last())
                                                .and_then(|s| color(&s.color))
                                                .unwrap_or(0x008c69);
                                            let fill = if n == 0 {
                                                0xebedf0
                                            } else {
                                                tint(
                                                    base,
                                                    match n {
                                                        1 => 0.35,
                                                        2..=3 => 0.55,
                                                        4..=6 => 0.78,
                                                        _ => 1.0,
                                                    },
                                                )
                                            };
                                            view()
                                                .w(px(9.0))
                                                .h(px(9.0))
                                                .flex_shrink_0()
                                                .rounded(px(2.0))
                                                .bg(rgb(fill))
                                                .style_when(
                                                    move || current > today || current < first,
                                                    |s| s.opacity(0.0),
                                                )
                                                .label(crate::i18n::message(
                                                    "day_focus_sessions",
                                                    &[
                                                        ("date", current.to_string()),
                                                        ("count", n.to_string()),
                                                    ],
                                                ))
                                        })
                                        .collect::<Vec<_>>(),
                                )
                        })),
                ),
            ),
        )
        .child(text(crate::i18n::tr("calendar_legend")).text_size(px(10.0)))
}

fn tint(color: u32, strength: f64) -> u32 {
    [16, 8, 0].into_iter().fold(0, |value, shift| {
        value
            | ((255.0 - (255 - ((color >> shift) & 255)) as f64 * strength).round() as u32) << shift
    })
}

fn profile_chart(sessions: &[Session]) -> View {
    let mut totals: BTreeMap<i64, (String, String, u64)> = BTreeMap::new();
    for session in sessions {
        let total = totals.entry(session.profile_id).or_insert((
            session.profile_name.clone(),
            session.color.clone(),
            0,
        ));
        total.2 += session.seconds;
    }
    let max = totals.values().map(|v| v.2).max().unwrap_or(1).max(1);
    view()
        .gap(px(8.0))
        .p(px(12.0))
        .bg(rgb(0xfffbf1))
        .rounded(px(6.0))
        .child(text(crate::i18n::tr("focus_time_by_profile")).text_size(px(10.0)))
        .children(totals.into_values().map(move |(name, hex, seconds)| {
            view()
                .gap(px(4.0))
                .child(
                    view()
                        .flex_row()
                        .justify_between()
                        .child(text(name).text_size(px(11.0)))
                        .child(
                            text(format!("{}h {}m", seconds / 3600, seconds % 3600 / 60))
                                .text_size(px(11.0)),
                        ),
                )
                .child(
                    view().h(px(7.0)).bg(rgb(0xede9df)).rounded(px(3.0)).child(
                        view()
                            .h(px(7.0))
                            .w(relative(seconds as f32 / max as f32))
                            .rounded(px(3.0))
                            .bg(rgb(color(&hex).unwrap_or(BLUE))),
                    ),
                )
        }))
}

fn session_row(session: Session) -> View {
    let completed = Local
        .timestamp_opt(session.finished_at, 0)
        .single()
        .unwrap();
    let started = Local.timestamp_opt(session.started_at, 0).single().unwrap();
    view()
        .gap(px(5.0))
        .p(px(10.0))
        .border_1()
        .border_color(rgb(0xe8e8e8))
        .rounded(px(5.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(7.0))
                .child(
                    view()
                        .w(px(8.0))
                        .h(px(8.0))
                        .rounded(px(4.0))
                        .bg(rgb(color(&session.color).unwrap_or(BLUE))),
                )
                .child(
                    text(session.profile_name.clone())
                        .text_size(px(13.0))
                        .text_color(rgb(0x222222)),
                ),
        )
        .child(
            text(format!(
                "{} · {}–{} · {} min",
                format!(
                    "{} {} {}",
                    completed.format("%d"),
                    crate::i18n::t(&completed.format("%b").to_string().to_lowercase()),
                    completed.format("%Y")
                ),
                started.format("%H:%M"),
                completed.format("%H:%M"),
                session.seconds / 60
            ))
            .text_size(px(10.0)),
        )
        .child(
            text(if session.note.is_empty() {
                crate::i18n::t("focus_session")
            } else {
                session.note.clone()
            })
            .text_size(px(12.0)),
        )
        .child(when(
            move || session.rate > 0,
            move || {
                text(format!(
                    "{} {}",
                    money(session.earnings()),
                    session.currency
                ))
                .text_size(px(11.0))
                .text_color(rgb(0x008c69))
            },
        ))
}

fn money(value: i64) -> String {
    format!("{}.{:02}", value / 100, value % 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn periods_use_local_calendar_boundaries() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
        assert_eq!(
            Period::Week.start(today),
            NaiveDate::from_ymd_opt(2026, 11, 2)
        );
        assert_eq!(
            Period::Month.start(today),
            NaiveDate::from_ymd_opt(2026, 11, 1)
        );
        assert_eq!(
            Period::Year.start(today),
            NaiveDate::from_ymd_opt(2026, 1, 1)
        );
        assert_eq!(Period::All.start(today), None);
    }
}
