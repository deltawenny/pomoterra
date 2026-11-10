//! Localization facade. English interface messages are the fallback keys.
//! Catalogs contain presentation text only, never user-authored names or notes.
mod language;
mod reactive;
mod russian;
mod spanish;

pub(crate) use language::Language;
pub(crate) use reactive::{language, message, optional, provide, t, tr};

#[cfg(test)]
mod tests;
