//! Shared text-field presentation matching the combobox trigger.
use super::{BLUE, GREY};
use nsh_core::{
    element::{Input, input},
    prelude::*,
};

/// A field without a border, for use inside a grouped control.
pub(super) fn input_content() -> Input {
    input()
        .min_w(px(0.0))
        .h(px(36.0))
        .px(px(10.0))
        .text_size(px(12.0))
        .text_color(rgb(0x333333))
        .placeholder_color(rgb(GREY))
}

/// A standalone text field with the same dimensions and states as a combobox.
pub(super) fn text_input(invalid: impl Fn() -> bool + Copy + 'static) -> Input {
    input_content()
        .bg(rgb(0xffffff))
        .border_1()
        .border_color(move || rgb(if invalid() { 0xd66854 } else { 0xcccccc }))
        .rounded(px(5.0))
        .hover(move |s| s.border_color(rgb(if invalid() { 0xd66854 } else { 0x999999 })))
        .focus(move |s| s.border_color(rgb(if invalid() { 0xd66854 } else { BLUE })))
}

/// Border and interaction states for controls containing multiple inputs or actions.
pub(super) fn input_group(invalid: impl Fn() -> bool + Copy + 'static) -> View {
    view()
        .min_w(px(0.0))
        .h(px(36.0))
        .flex_row()
        .items_center()
        .bg(rgb(0xffffff))
        .border_1()
        .border_color(move || rgb(if invalid() { 0xd66854 } else { 0xcccccc }))
        .rounded(px(5.0))
        .hover(move |s| s.border_color(rgb(if invalid() { 0xd66854 } else { 0x999999 })))
        .focus_within(move |s| s.border_color(rgb(if invalid() { 0xd66854 } else { BLUE })))
}
