use super::*;
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
        let label = tr("timer");
        let dropdown = optional("language");
        assert_eq!(label(), "Timer");
        library.update(|data| data.settings.language = "Español".into());
        assert_eq!(label(), "Temporizador");
        assert_eq!(dropdown().unwrap().as_ref(), "Idioma");
        assert_eq!(
            message(
                "switch_to_name_current",
                &[
                    ("name", "My work".into()),
                    ("current", t("current_profile")),
                ]
            ),
            "Cambiar a My work (perfil actual)"
        );
        library.update(|data| data.settings.language = "Русский".into());
        assert_eq!(label(), "Таймер");
        assert_eq!(dropdown().unwrap().as_ref(), "Язык");
        assert_eq!(
            message(
                "switch_to_name_current",
                &[
                    ("name", "My work".into()),
                    ("current", t("current_profile")),
                ]
            ),
            "Переключиться на My work (текущий профиль)"
        );
        assert_eq!(t("jan"), "янв");
        assert_eq!(
            t("focus_complete"),
            "Рабочий сеанс завершён. Пора отдохнуть."
        );
        library.update(|data| data.settings.language = "English".into());
        assert_eq!(label(), "Timer");
        assert_eq!(t("stay_focused"), "Stay Focused");
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
    assert_eq!(Language::Spanish.translate("timer"), "Temporizador");
}

fn catalogs() -> [(
    Language,
    std::collections::BTreeMap<String, serde_json::Value>,
); 3] {
    [
        (Language::English, include_str!("../../locales/en.json")),
        (Language::Spanish, include_str!("../../locales/es.json")),
        (Language::Russian, include_str!("../../locales/ru.json")),
    ]
    .map(|(language, source)| (language, serde_json::from_str(source).unwrap()))
}

#[test]
fn embedded_catalogs_match_resources_and_placeholders() {
    let catalogs = catalogs();
    let english = &catalogs[0].1;
    let placeholders = |text: &str| {
        let mut names: Vec<_> = text
            .split("%{")
            .skip(1)
            .map(|part| part.split('}').next().unwrap().to_owned())
            .collect();
        names.sort();
        names
    };
    let mut available = rust_i18n::available_locales!();
    available.sort();
    assert_eq!(available, ["en", "es", "ru"]);
    for (language, catalog) in &catalogs {
        assert_eq!(
            catalog.keys().collect::<Vec<_>>(),
            english.keys().collect::<Vec<_>>(),
            "{language:?}: supported catalogs must translate every English message"
        );
        for (key, value) in catalog {
            if key == "_version" {
                continue;
            }
            let text = value.as_str().expect("translation must be text");
            assert!(!text.is_empty(), "{language:?}: {key}");
            let source = english
                .get(key)
                .expect("translation needs an English source");
            assert_eq!(
                placeholders(text),
                placeholders(source.as_str().unwrap()),
                "{key}"
            );
            assert_eq!(language.translate(key), text, "{language:?}: {key}");
        }
        // Every supported locale must resolve its own translation for every message.
        for key in english.keys() {
            if key != "_version" {
                let expected = catalog.get(key).unwrap().as_str().unwrap();
                assert_eq!(language.translate(key), expected, "{language:?}: {key}");
            }
        }
    }
}

#[test]
fn explicit_locales_are_independent_across_threads() {
    let threads: Vec<_> = Language::ALL
        .into_iter()
        .map(|language| {
            std::thread::spawn(move || {
                for _ in 0..100 {
                    let expected = match language {
                        Language::English => "Timer",
                        Language::Spanish => "Temporizador",
                        Language::Russian => "Таймер",
                    };
                    assert_eq!(language.translate("timer"), expected);
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
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
            ("name", "Work %{current}".to_owned()),
            ("current", "!".to_owned()),
        ];
        let result = language.message("switch_to_name_current", &values);
        assert!(result.ends_with("Work %{current}!"), "{result}");
    }
    assert_eq!(
        Language::English.message(
            "%{name} / %{name} / %{missing}",
            &[("name", "notes".into())]
        ),
        "notes / notes / %{missing}"
    );
    assert_eq!(
        Language::English.message("Unclosed %{name", &[]),
        "Unclosed %{name"
    );
    assert_eq!(Language::English.message("Plain text", &[]), "Plain text");
}
