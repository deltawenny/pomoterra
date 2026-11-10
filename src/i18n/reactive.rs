//! Reactive UI adapters. Native integration uses Language directly.
use super::Language;
use crate::persistence::Library;
use nsh_core::prelude::*;

#[derive(Clone, Copy)]
struct Locale(Signal<Library>);

impl Locale {
    fn language(self) -> Language {
        Language::from_preference(&self.0.get().settings.language)
    }
}

pub(crate) fn provide(library: Signal<Library>) {
    provide_context(Locale(library));
}

pub(crate) fn language() -> Language {
    use_context::<Locale>()
        .map(|locale| locale.language())
        .unwrap_or_default()
}

pub(crate) fn t(key: &str) -> String {
    language().translate(key)
}

/// Capture the locale in the owning view, then track its preference on each read.
pub(crate) fn tr(key: impl Into<String>) -> impl Fn() -> String + Clone {
    let locale = use_context::<Locale>();
    let key = key.into();
    move || {
        locale
            .map(|locale| locale.language())
            .unwrap_or_default()
            .translate(&key)
    }
}

/// Optional text properties used by dropdowns and sliders.
pub(crate) fn optional(key: impl Into<String>) -> impl Fn() -> Option<SharedString> + Clone {
    let translate = tr(key);
    move || Some(translate().into())
}

pub(crate) fn message(key: &str, values: &[(&str, String)]) -> String {
    language().message(key, values)
}
