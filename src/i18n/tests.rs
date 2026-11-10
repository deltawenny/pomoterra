use super::*;
use super::{russian::RUSSIAN, spanish::SPANISH};
use crate::persistence::Library;
use nsh_core::prelude::*;

#[test]
fn translations_follow_live_preference_changes() {
    let runtime = nsh_core::element::Runtime::default();
    let _entered = runtime.enter();
    let (owner, ()) = nsh_core::element::create_root(|| {
        let library = signal(Library {
            database: None,
            profiles: Vec::new(),
            sessions: Vec::new(),
            settings: Default::default(),
            error: String::new(),
        });
        provide(library);
        let label = tr("Timer");
        let dropdown = optional("Language");
        assert_eq!(label(), "Timer");
        library.update(|data| data.settings.language = "Español".into());
        assert_eq!(label(), "Temporizador");
        assert_eq!(dropdown().unwrap().as_ref(), "Idioma");
        assert_eq!(
            message(
                "Switch to {name}{current}",
                &[
                    ("name", "My work".into()),
                    ("current", t(" (current profile)")),
                ]
            ),
            "Cambiar a My work (perfil actual)"
        );
        library.update(|data| data.settings.language = "Русский".into());
        assert_eq!(label(), "Таймер");
        assert_eq!(dropdown().unwrap().as_ref(), "Язык");
        assert_eq!(
            message(
                "Switch to {name}{current}",
                &[
                    ("name", "My work".into()),
                    ("current", t(" (current profile)")),
                ]
            ),
            "Переключиться на My work (текущий профиль)"
        );
        assert_eq!(t("Jan"), "янв");
        assert_eq!(
            t("Focus session complete. Time for a break."),
            "Рабочий сеанс завершён. Пора отдохнуть."
        );
        library.update(|data| data.settings.language = "English".into());
        assert_eq!(label(), "Timer");
        assert_eq!(t("Stay Focused"), "Stay Focused");
    });
    owner.dispose();
}

#[test]
fn preferences_and_fallback_are_compatible() {
    assert_eq!(Language::from_preference("Español"), Language::Spanish);
    assert_eq!(Language::from_preference("es"), Language::Spanish);
    assert_eq!(Language::from_preference("English"), Language::English);
    assert_eq!(Language::from_preference("Русский"), Language::Russian);
    assert_eq!(Language::from_preference("ru"), Language::Russian);
    assert_eq!(Language::from_preference("ru-RU"), Language::Russian);
    assert_eq!(Language::from_preference("unsupported"), Language::English);
    assert_eq!(Language::Russian.translate("unknown"), "unknown");
    assert_eq!(Language::Spanish.translate("unknown"), "unknown");
    assert_eq!(Language::Spanish.translate("Timer"), "Temporizador");
}

#[test]
fn catalog_has_unique_keys_and_matching_placeholders() {
    let spanish_keys: std::collections::HashSet<_> = SPANISH.iter().map(|(key, _)| key).collect();
    let russian_keys: std::collections::HashSet<_> = RUSSIAN.iter().map(|(key, _)| key).collect();
    assert_eq!(
        spanish_keys, russian_keys,
        "catalogs must cover the same messages"
    );
    for catalog in [SPANISH, RUSSIAN] {
        let mut keys = std::collections::HashSet::new();
        for (key, value) in catalog {
            assert!(keys.insert(key), "duplicate translation: {key}");
            assert!(!value.is_empty());
            let placeholders = |text: &str| {
                let mut names: Vec<String> = text
                    .split('{')
                    .skip(1)
                    .map(|part| part.split('}').next().unwrap().to_owned())
                    .collect();
                names.sort();
                names
            };
            assert_eq!(placeholders(key), placeholders(value), "{key}");
            assert_eq!(Language::English.translate(key), *key);
        }
    }
}

#[test]
fn supported_languages_round_trip_saved_preferences() {
    let mut names = std::collections::HashSet::new();
    for language in Language::ALL {
        assert!(names.insert(language.preference()));
        assert_eq!(Language::from_preference(language.preference()), language);
    }
}

#[test]
fn interpolation_does_not_reinterpret_user_text() {
    for language in Language::ALL {
        let values = [
            ("name", "Work {current}".to_owned()),
            ("current", "!".to_owned()),
        ];
        let result = language.message("Switch to {name}{current}", &values);
        assert!(result.ends_with("Work {current}!"), "{result}");
    }
    assert_eq!(
        Language::English.message("{name} / {name} / {missing}", &[("name", "notes".into())]),
        "notes / notes / {missing}"
    );
    assert_eq!(
        Language::English.message("Unclosed {name", &[]),
        "Unclosed {name"
    );
    assert_eq!(Language::English.message("Plain text", &[]), "Plain text");
}
