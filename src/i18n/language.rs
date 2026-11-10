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

    pub(crate) fn translate(self, key: &str) -> String {
        let catalog = match self {
            Self::English => return key.to_owned(),
            Self::Spanish => super::spanish::SPANISH,
            Self::Russian => super::russian::RUSSIAN,
        };
        catalog
            .iter()
            .find(|(source, _)| *source == key)
            .map(|(_, value)| *value)
            .unwrap_or(key)
            .to_owned()
    }

    /// Substitute only placeholders in the template, never in inserted values.
    /// Unknown placeholders are preserved to make missing arguments visible.
    pub(crate) fn message(self, key: &str, values: &[(&str, String)]) -> String {
        interpolate(&self.translate(key), values)
    }
}

fn interpolate(template: &str, values: &[(&str, String)]) -> String {
    let mut result = String::with_capacity(template.len());
    let mut remaining = template;
    while let Some(start) = remaining.find('{') {
        result.push_str(&remaining[..start]);
        let placeholder = &remaining[start..];
        let Some(end) = placeholder.find('}') else {
            result.push_str(placeholder);
            return result;
        };
        let name = &placeholder[1..end];
        if let Some((_, value)) = values.iter().find(|(key, _)| *key == name) {
            result.push_str(value);
        } else {
            result.push_str(&placeholder[..=end]);
        }
        remaining = &placeholder[end + 1..];
    }
    result.push_str(remaining);
    result
}
