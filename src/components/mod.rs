//! Layout sections composed by the application root.
mod controls;
mod header;
mod session_note;
mod timer_display;

pub(crate) use controls::timer_controls;
pub(crate) use header::tab_header;
pub(crate) use session_note::session_note;
pub(crate) use timer_display::timer_display;

const BLUE: u32 = 0x0083c9;
pub(crate) const GREY: u32 = 0x686868;
