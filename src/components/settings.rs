use super::{
    BLUE, GREY,
    combobox::{combobox, sorted_items},
    profiles,
};
use crate::{
    icons::{Icon, icon},
    persistence::{AppSettings, Library},
    store::TimerState,
};
use nsh_base::SelectItem;
use nsh_core::prelude::*;

pub(crate) fn settings_tab(
    timer: Signal<TimerState>,
    library: Signal<Library>,
    active: Signal<i64>,
    announcements: Signal<bool>,
) -> View {
    let restore = move || {
        library.update(|data| data.update_settings(|settings| *settings = AppSettings::default()));
        announcements.set(true);
    };
    view()
        .flex_shrink_0()
        .px(px(20.0))
        .pt(px(20.0))
        .pb(px(24.0))
        .gap(px(12.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    text(crate::i18n::tr("settings_heading"))
                        .text_size(px(14.0))
                        .text_color(rgb(0x111111)),
                )
                .child(profiles::text_button("restore_to_default", restore)),
        )
        .child(language_setting(library))
        .child(toggle_row(
            library,
            "show_window_after_focus",
            "bring_forward_after_focus",
            |s| s.show_after_focus,
            |s, v| s.show_after_focus = v,
        ))
        .child(toggle_row(
            library,
            "show_window_after_break",
            "bring_forward_after_break",
            |s| s.show_after_break,
            |s, v| s.show_after_break = v,
        ))
        .child(info_row(
            "discord_rich_presence",
            "discord_unavailable",
            Icon::MessageCircle,
        ))
        .child(toggle_row(
            library,
            "close_to_tray",
            "keep_running_on_close",
            |s| s.close_to_tray,
            |s, v| s.close_to_tray = v,
        ))
        .child(toggle_row(
            library,
            "system_notifications",
            "announce_completed_sessions",
            |s| s.system_notifications,
            |s, v| s.system_notifications = v,
        ))
        .child(toggle_row(
            library,
            "continue_after_focus",
            "auto_start_break",
            |s| s.continue_after_focus,
            |s, v| s.continue_after_focus = v,
        ))
        .child(toggle_row(
            library,
            "continue_after_break",
            "auto_start_focus",
            |s| s.continue_after_break,
            |s, v| s.continue_after_break = v,
        ))
        .child(quick_profiles(timer, library, active))
}

fn card() -> View {
    view()
        .w_full()
        .flex_shrink_0()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .p(px(12.0))
        .rounded(px(12.0))
        .border_1()
        .border_color(rgb(0xdddddd))
        .bg(rgb(0xffffff))
}

fn language_setting(library: Signal<Library>) -> View {
    card()
        .child(
            text(crate::i18n::tr("language"))
                .flex_1()
                .text_size(px(13.0)),
        )
        .child(
            view().w(px(150.0)).flex_shrink_0().child(
                combobox()
                    .items(move || {
                        sorted_items(crate::i18n::Language::ALL.into_iter().map(|language| {
                            let name = language.preference();
                            SelectItem::new(name, name)
                        }))
                    })
                    .value(move || Some(vec![library.get().settings.language.clone().into()]))
                    .accessibility_label(crate::i18n::optional("language"))
                    .placeholder(crate::i18n::optional("language"))
                    .on_change(move |value: &Vec<SharedString>| {
                        if let Some(value) = value.first() {
                            library.update(|data| {
                                data.update_settings(|settings| {
                                    settings.language = value.to_string()
                                })
                            });
                        }
                    }),
            ),
        )
}

fn toggle_row(
    library: Signal<Library>,
    label: &'static str,
    description: &'static str,
    get: fn(&AppSettings) -> bool,
    set: fn(&mut AppSettings, bool),
) -> View {
    card()
        .role(AccessibilityRole::Checkbox)
        .label(crate::i18n::tr(label))
        .on_press(move || {
            let value = !get(&library.get().settings);
            library.update(|data| data.update_settings(|settings| set(settings, value)));
        })
        .child(
            view()
                .flex_1()
                .gap(px(3.0))
                .child(
                    text(crate::i18n::tr(label))
                        .text_size(px(13.0))
                        .text_color(rgb(0x222222)),
                )
                .child(
                    text(crate::i18n::tr(description))
                        .text_size(px(11.0))
                        .text_color(rgb(GREY)),
                ),
        )
        .child(
            view()
                .w(px(22.0))
                .h(px(22.0))
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .border_1()
                .border_color(move || {
                    rgb(if get(&library.get().settings) {
                        BLUE
                    } else {
                        0xaaaaaa
                    })
                })
                .bg(move || {
                    rgb(if get(&library.get().settings) {
                        BLUE
                    } else {
                        0xffffff
                    })
                })
                .child(when(
                    move || get(&library.get().settings),
                    move || icon(Icon::Check, 0xffffff, 16.0),
                )),
        )
}

fn info_row(label: &'static str, description: &'static str, glyph: Icon) -> View {
    card()
        .child(
            view()
                .flex_1()
                .gap(px(3.0))
                .child(
                    text(crate::i18n::tr(label))
                        .text_size(px(13.0))
                        .text_color(rgb(0x222222)),
                )
                .child(
                    text(crate::i18n::tr(description))
                        .text_size(px(11.0))
                        .text_color(rgb(GREY)),
                ),
        )
        .child(icon(glyph, GREY, 18.0))
}

fn quick_profiles(
    timer: Signal<TimerState>,
    library: Signal<Library>,
    active: Signal<i64>,
) -> View {
    card()
        .flex_col()
        .items_start()
        .gap(px(8.0))
        .child(
            text(crate::i18n::tr("quick_switch_configuration_profiles"))
                .text_size(px(13.0))
                .text_color(rgb(0x222222)),
        )
        .child(
            text(crate::i18n::tr("quick_profiles_description"))
                .text_size(px(11.0))
                .text_color(rgb(GREY)),
        )
        .child(
            combobox()
                .multiple(true)
                .items(move || {
                    sorted_items(library.get().profiles.iter().map(|profile| {
                        SelectItem::new(profile.id.to_string(), profile.name.clone())
                    }))
                })
                .value(move || {
                    Some(
                        library
                            .get()
                            .settings
                            .quick_profiles
                            .iter()
                            .map(|id| id.to_string().into())
                            .collect(),
                    )
                })
                .accessibility_label(crate::i18n::optional("quick_switch_profiles"))
                .placeholder(crate::i18n::optional("select_profiles"))
                .on_change(move |value: &Vec<SharedString>| {
                    let ids = value
                        .iter()
                        .filter_map(|entry| entry.parse::<i64>().ok())
                        .collect();
                    library.update(|data| {
                        data.update_settings(|settings| settings.quick_profiles = ids)
                    });
                }),
        )
        .child(profiles::quick_profile_switcher(timer, library, active))
}
