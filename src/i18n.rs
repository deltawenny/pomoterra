//! Reactive localization facade backed by rust-i18n's embedded catalogs.
//! Catalogs contain presentation text only, never user-authored names or notes.
mod language;
mod reactive;

pub(crate) use language::Language;
pub(crate) use reactive::{language, message, optional, provide, t, tr};

#[cfg(test)]
mod tests;
