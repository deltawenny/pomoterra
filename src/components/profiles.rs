use super::{
    BLUE, GREY,
    combobox::{combobox, sorted_items},
    input::text_input,
};
use crate::{
    icons::{Icon, icon},
    persistence::{Library, color},
    store::TimerState,
};
use nsh_base::SelectItem;
use nsh_core::prelude::*;

pub(crate) fn action(label: &'static str, glyph: Icon, run: impl Fn() + 'static) -> View {
    view()
        .w(px(30.0))
        .h(px(30.0))
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(5.0))
        .role(AccessibilityRole::Button)
        .label(label)
        .on_press(run)
        .hover(|s| s.bg(rgb(0xf0f6fa)))
        .active(|s| s.bg(rgb(0xe3eff7)))
        .child(icon(glyph, GREY, 15.0))
}

pub(crate) fn choose(
    timer: Signal<TimerState>,
    library: Signal<Library>,
    active: Signal<i64>,
    id: i64,
) {
    crate::flush_completed(timer, library, active.get());
    if !timer.get().completed.is_empty() {
        return;
    }
    let data = library.get();
    let Some(profile) = data.profiles.iter().find(|p| p.id == id) else {
        return;
    };
    let result = data
        .database
        .as_ref()
        .ok_or_else(|| "Storage unavailable".to_owned())
        .and_then(|db| db.select(id));
    if let Err(error) = result {
        library.update(|data| data.error = error);
        return;
    }
    // Persist old rules before changing the active profile, including rapid edits.
    library.update(|data| data.edit(active.get(), |p| p.rules = timer.get().rules.clone()));
    active.set(id);
    timer.update(|state| {
        state.configure(|rules| *rules = profile.rules.clone());
        state.note.clear();
    });
}

pub(crate) fn profile_picker(
    timer: Signal<TimerState>,
    library: Signal<Library>,
    active: Signal<i64>,
    after_change: impl Fn() + 'static,
) -> impl IntoElement {
    combobox()
        .items(move || {
            sorted_items(
                library
                    .get()
                    .profiles
                    .iter()
                    .map(|p| SelectItem::new(p.id.to_string(), p.name.clone())),
            )
        })
        .value(move || Some(vec![active.get().to_string().into()]))
        .accessibility_label("Configuration profile")
        .placeholder("Search profiles")
        .on_change(move |value: &Vec<SharedString>| {
            if let Some(id) = value.first().and_then(|v| v.parse().ok()) {
                choose(timer, library, active, id);
                after_change();
            }
        })
}

/// A single-height shortcut strip, shared by the timer and profile settings.
/// Long names are truncated; additional profiles scroll instead of squeezing
/// the timer or overflowing the window at its minimum width.
pub(crate) fn quick_profile_switcher(
    timer: Signal<TimerState>,
    library: Signal<Library>,
    active: Signal<i64>,
) -> View {
    view()
        .w_full()
        .flex_shrink_0()
        .min_w(px(0.0))
        .child(dynamic(move || {
            let data = library.get();
            let profiles: Vec<_> = data
                .settings
                .quick_profiles
                .iter()
                .filter_map(|id| data.profiles.iter().find(|profile| profile.id == *id))
                .collect();
            if profiles.is_empty() {
                return view();
            }
            view().mt(px(8.0)).child(
                nsh_core::element::scroll().horizontal().h(px(42.0)).child(
                    view().flex_row().items_start().gap(px(6.0)).children(
                        profiles
                            .into_iter()
                            .map(|profile| {
                                let id = profile.id;
                                let name = profile.name.clone();
                                let accent = color(&profile.color).unwrap_or(BLUE);
                                view()
                                    .h(px(30.0))
                                    .max_w(px(180.0))
                                    .flex_shrink_0()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(6.0))
                                    .px(px(10.0))
                                    .rounded(px(15.0))
                                    .border_1()
                                    .border_color(move || {
                                        rgb(if active.get() == id { BLUE } else { 0xe0e6eb })
                                    })
                                    .bg(move || {
                                        rgb(if active.get() == id {
                                            0xeaf5fb
                                        } else {
                                            0xf7f9fb
                                        })
                                    })
                                    .hover(|s| s.bg(rgb(0xeaf5fb)))
                                    .active(|s| s.bg(rgb(0xdceef8)))
                                    .role(AccessibilityRole::Button)
                                    .label(move || {
                                        format!(
                                            "Switch to {name}{}",
                                            if active.get() == id {
                                                " (current profile)"
                                            } else {
                                                ""
                                            }
                                        )
                                    })
                                    .on_press(move || {
                                        if active.get() != id {
                                            choose(timer, library, active, id);
                                        }
                                    })
                                    .child(
                                        view()
                                            .w(px(7.0))
                                            .h(px(7.0))
                                            .flex_shrink_0()
                                            .rounded(px(4.0))
                                            .bg(rgb(accent)),
                                    )
                                    .child(
                                        text(profile.name.clone())
                                            .min_w(px(0.0))
                                            .text_size(px(12.0))
                                            .truncate()
                                            .text_color(move || {
                                                rgb(if active.get() == id { BLUE } else { GREY })
                                            }),
                                    )
                            })
                            .collect::<Vec<_>>(),
                    ),
                ),
            )
        }))
}

pub(crate) fn profile_settings(
    timer: Signal<TimerState>,
    library: Signal<Library>,
    active: Signal<i64>,
) -> View {
    let name = signal(String::new());
    let editing = signal(false);
    let deleting = signal(false);
    let rate = signal(String::new());
    let color_text = signal(String::new());
    effect(move || {
        let data = library.get_untracked();
        if let Some(p) = data.profiles.iter().find(|p| p.id == active.get()) {
            rate.set(format!("{:.2}", p.rate as f64 / 100.0));
            color_text.set(p.color.clone());
        }
    });
    view().gap(px(10.0)).mb(px(26.0))
        .child(text("Configuration Profile").text_size(px(12.0)).text_color(rgb(0x333333)))
        .child(view().flex_row().items_center().gap(px(4.0))
            .child(view().flex_1().min_w(px(0.0)).child(profile_picker(timer, library, active, move || {
                editing.set(false);
                deleting.set(false);
            })))
            .child(action("Add profile", Icon::Plus, move || {
                let data = library.get();
                let result = data.database.as_ref().ok_or_else(|| "Storage unavailable".to_owned())
                    .and_then(|db| db.create_profile(&format!("Profile {}", data.profiles.iter().map(|p| p.id).max().unwrap_or(0) + 1)));
                match result {
                    Ok(profile) => {
                        let id = profile.id;
                        library.update(|data| data.profiles.push(profile));
                        choose(timer, library, active, id);
                        name.set(library.get().profiles.iter().find(|p| p.id == id).unwrap().name.clone());
                        editing.set(true); deleting.set(false);
                    }
                    Err(error) => library.update(|data| data.error = error),
                }
            }))
            .child(action("Rename profile", Icon::Pencil, move || {
                name.set(library.get().profiles.iter().find(|p| p.id == active.get()).map(|p| p.name.clone()).unwrap_or_default());
                editing.set(true);
            }))
            .child(action("Delete profile", Icon::Trash2, move || deleting.set(true))))
        .child(when(move || editing.get(), move || view().flex_row().gap(px(6.0))
            .child(text_input(|| false).flex_1()
                .label("Profile name").value(move || name.get())
                .on_change(move |value: &str| name.set(value.to_owned())))
            .child(action("Save profile name", Icon::Check, move || {
                library.update(|data| data.edit(active.get(), |p| p.name = name.get()));
                if library.get().error.is_empty() { editing.set(false); }
            }))
            .child(action("Cancel rename", Icon::X, move || editing.set(false)))))
        .child(when(move || deleting.get(), move || view()
            .gap(px(8.0))
            .p(px(10.0))
            .border_1()
            .border_color(rgb(0xe0b4b4))
            .rounded(px(6.0))
            .bg(rgb(0xfff8f8))
            .child(text("Delete this profile?").text_size(px(12.0)).text_color(rgb(0x6f2222)))
            .child(text("Its history will be kept.").text_size(px(12.0)).text_color(rgb(0x555555)))
            .child(view().flex_row().gap(px(8.0))
                .child(text_button("Confirm delete", move || {
                    let data = library.get();
                    if data.profiles.len() <= 1 {
                        library.update(|data| data.error = "Keep at least one profile. Add another before deleting this one.".into());
                        deleting.set(false); return;
                    }
                    let id = active.get();
                    let next = data.profiles.iter().find(|p| p.id != id).unwrap().id;
                    choose(timer, library, active, next);
                    if active.get() != next { return; }
                    match data.database.as_ref().unwrap().archive(id) {
                        Ok(()) => library.update(|data| { data.profiles.retain(|p| p.id != id); data.error.clear(); }),
                        Err(error) => library.update(|data| data.error = error),
                    }
                    deleting.set(false);
                }))
                .child(text_button("Cancel delete", move || deleting.set(false))))))
        .child(view().flex_row().items_center().gap(px(8.0))
            .role(AccessibilityRole::Checkbox).label("Payable")
            .on_press(move || library.update(|data| data.edit(active.get(), |p| p.payable = !p.payable)))
            .child(view().w(px(20.0)).h(px(20.0)).border_1().border_color(rgb(BLUE)).rounded(px(3.0))
                .child(when(move || library.get().profiles.iter().find(|p| p.id == active.get()).is_some_and(|p| p.payable), move || icon(Icon::Check, BLUE, 18.0))))
            .child(text("Payable").text_size(px(13.0))))
        .child(when(move || library.get().profiles.iter().find(|p| p.id == active.get()).is_some_and(|p| p.payable), move || {
            view().flex_row().gap(px(8.0)).items_center()
                .child(text_input(move || parse_rate(&rate.get()).is_none()).flex_1()
                    .label("Hourly rate").placeholder("0.00 / hour")
                    .value(move || rate.get()).on_change(move |value: &str| {
                        rate.set(value.to_owned());
                        if let Some(value) = parse_rate(value) { library.update(|data| data.edit(active.get(), |p| p.rate = value)); }
                    }))
                .child(view().w(px(88.0)).flex_shrink_0().child(combobox().items(sorted_items(["USD", "EUR", "GBP", "RUB", "JPY", "CAD", "AUD", "CHF", "INR"].into_iter().map(|v| SelectItem::new(v, v))))
                    .value(move || Some(vec![library.get().profiles.iter().find(|p| p.id == active.get()).map(|p| p.currency.clone()).unwrap_or_else(|| "USD".into()).into()]))
                    .accessibility_label("Currency")
                    .placeholder("Currency")
                    .empty(text("No matching currencies").text_size(px(12.0)).text_color(rgb(GREY)).p(px(12.0)))
                    .popup_class("min-w-[88px] bg-white border border-[#dddddd] rounded-md p-1 shadow-lg")
                    .on_change(move |value: &Vec<SharedString>| { if let Some(value) = value.first() { library.update(|data| data.edit(active.get(), |p| p.currency = value.to_string())); } })))
                .child(text("/ hour").text_size(px(11.0)))
        }))
        .child(text("Profile color").text_size(px(12.0)))
        .child(view().flex_row().gap(px(8.0)).items_center()
            .children(["#0083c9", "#008c69", "#a855f7", "#e05274", "#e5a100", "#e76f35"].into_iter().map(move |value| {
                view().w(px(22.0)).h(px(22.0)).rounded(px(11.0)).bg(rgb(color(value).unwrap())).border_2()
                    .border_color(move || rgb(if library.get().profiles.iter().find(|p| p.id == active.get()).is_some_and(|p| p.color == value) { 0x222222 } else { 0xffffff }))
                    .role(AccessibilityRole::Button).label(format!("Use color {value}"))
                    .on_press(move || {
                        library.update(|data| data.edit(active.get(), |p| p.color = value.into()));
                        color_text.set(value.into());
                    })
            }))
            .child(text_input(move || color(&color_text.get()).is_none()).flex_1()
                .label("Profile color hex").placeholder("#0083c9")
                .value(move || color_text.get()).on_change(move |value: &str| {
                    color_text.set(value.to_owned());
                    if color(value).is_some() { library.update(|data| data.edit(active.get(), |p| p.color = value.to_owned())); }
                })))
}

pub(crate) fn text_button(label: &'static str, run: impl Fn() + 'static) -> View {
    dynamic_text_button(label.to_owned(), run)
}

pub(crate) fn dynamic_text_button(label: String, run: impl Fn() + 'static) -> View {
    view()
        .role(AccessibilityRole::Button)
        .label(label.clone())
        .on_press(run)
        .px(px(8.0))
        .py(px(5.0))
        .rounded(px(4.0))
        .hover(|s| s.bg(rgb(0xeaf5fb)))
        .active(|s| s.bg(rgb(0xdceef8)))
        .child(text(label).text_size(px(12.0)).text_color(rgb(BLUE)))
}

fn parse_rate(value: &str) -> Option<i64> {
    let (whole, fraction) = value.trim().split_once('.').unwrap_or((value.trim(), ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 2
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let fraction = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>().ok()? * 10,
        _ => fraction.parse().ok()?,
    };
    whole
        .checked_mul(100)?
        .checked_add(fraction)
        .filter(|v| *v <= 100_000_000)
}
