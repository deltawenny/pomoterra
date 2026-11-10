//! Framework-independent language metadata, lookup, and interpolation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Language {
    #[default]
    English,
    Spanish,
    Russian,
}

impl Language {
    /// All supported languages, shared by settings and catalog validation.
    pub(crate) const ALL: [Self; 3] = [Self::English, Self::Spanish, Self::Russian];

    /// Native display name, also the stable value stored in existing settings.
    pub(crate) fn preference(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Spanish => "Español",
            Self::Russian => "Русский",
        }
    }

    pub(crate) fn from_preference(value: &str) -> Self {
        match value {
            "Español" | "es" | "es-ES" => Self::Spanish,
            "Русский" | "ru" | "ru-RU" => Self::Russian,
            _ => Self::English,
        }
    }

    pub(crate) fn locale(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Spanish => "es",
            Self::Russian => "ru",
        }
    }

    pub(crate) fn translate(self, key: &str) -> String {
        // Explicit locales keep UI roots and native background threads independent.
        rust_i18n::t!(key, locale = self.locale()).into_owned()
    }

    /// Substitute only placeholders in the template, never in inserted values.
    /// Unknown placeholders are preserved to make missing arguments visible.
    pub(crate) fn message(self, key: &str, values: &[(&str, String)]) -> String {
        let (patterns, values): (Vec<_>, Vec<_>) = values.iter().cloned().unzip();
        rust_i18n::replace_patterns(&self.translate(key), &patterns, &values)
    }
}
