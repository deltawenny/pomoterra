use super::{BLUE, GREY};
use crate::icons::{Icon, icon};
use nsh_base::{ComboboxElement, SelectEntry, SelectItem, combobox as base_combobox};
use nsh_core::prelude::*;

/// An editable trigger with its filtered list anchored below it.
pub(super) fn combobox() -> ComboboxElement {
    base_combobox()
        .placement(Placement::Bottom)
        .align(Align::Start)
        .offset(px(4.0))
        .margin(px(8.0))
        .w_full()
        .min_w(px(0.0))
        .h(px(36.0))
        .items_center()
        .bg(rgb(0xffffff))
        .border_1()
        .border_color(rgb(0xcccccc))
        .rounded(px(5.0))
        .hover(|s| s.border_color(rgb(0x999999)))
        .focus_within(|s| s.border_color(rgb(BLUE)))
        .input_class("h-full flex-1 min-w-0 px-[10px] text-xs text-[#333333]")
        .trigger_button(
            view()
                .h_full()
                .px(px(10.0))
                .items_center()
                .justify_center()
                .child(icon(Icon::ChevronDown, GREY, 13.0)),
        )
        .popup_class(
            "min-w-[200px] bg-white border border-[#dddddd] rounded-md p-1 shadow-lg",
        )
        // Keep the popup's available-space max-height intact. Cap the scroll
        // view instead so a tall dropdown cannot be shifted over its input.
        .list_class("min-h-0 max-h-[270px]")
        .item_class(
            "group h-9 flex-col items-stretch justify-center px-3 rounded text-[#333333] focus:bg-[#edf6fc]",
        )
        .render_item(option_content)
        .empty(
            text(crate::i18n::tr("no_matching_profiles_133"))
                .text_size(px(12.0))
                .text_color(rgb(GREY))
                .p(px(12.0)),
        )
}

/// Selection is indicated by a check, not a persistent row highlight.
fn option_content(entry: &SelectEntry) -> View {
    view()
        .w_full()
        .min_w(px(0.0))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            text(entry.title.clone())
                .flex_1()
                .min_w(px(0.0))
                .text_size(px(12.0))
                .truncate(),
        )
        .child(icon(Icon::Check, 0x333333, 14.0).app_class("opacity-0 group-selected:opacity-100"))
}

/// Sort display titles without changing selection values or persistence order.
pub(super) fn sorted_items(items: impl IntoIterator<Item = SelectItem>) -> Vec<SelectItem> {
    let mut items: Vec<_> = items.into_iter().collect();
    items.sort_by_cached_key(|item| item.title.to_lowercase());
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_sort_by_title_ignoring_case_and_keep_values() {
        let items = sorted_items([
            SelectItem::new("1", "zebra"),
            SelectItem::new("2", "Beta"),
            SelectItem::new("3", "alpha"),
            SelectItem::new("4", "ALPHA"),
        ]);
        let values: Vec<_> = items.iter().map(|item| item.value.as_str()).collect();
        assert_eq!(values, ["3", "4", "2", "1"]);
    }

    #[test]
    fn empty_options_stay_empty() {
        assert!(sorted_items([]).is_empty());
    }
}
