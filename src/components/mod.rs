//! Layout sections composed by the application root.
mod combobox;
mod config;
mod controls;
mod header;
mod history;
mod input;
mod profiles;
mod session_note;
mod timer_display;

pub(crate) use config::config_tab;
pub(crate) use controls::timer_controls;
pub(crate) use header::{Tab, tab_header};
pub(crate) use history::history_tab;
pub(crate) use profiles::profile_picker;
pub(crate) use session_note::session_note;
pub(crate) use timer_display::timer_display;

const BLUE: u32 = 0x0083c9;
const GREEN: u32 = 0x008c69;

fn phase_color(phase: crate::store::Phase) -> u32 {
    match phase {
        crate::store::Phase::Focus => BLUE,
        crate::store::Phase::Break => GREEN,
    }
}
pub(crate) const GREY: u32 = 0x686868;
