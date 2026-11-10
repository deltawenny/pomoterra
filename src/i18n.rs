//! Shared translations. User-authored profile names and notes are never translated.
use crate::persistence::Library;
use nsh_core::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Language {
    #[default]
    English,
    Spanish,
}

impl Language {
    pub(crate) fn from_preference(value: &str) -> Self {
        match value {
            "Español" | "es" | "es-ES" => Self::Spanish,
            _ => Self::English,
        }
    }

    pub(crate) fn translate(self, key: &str) -> String {
        if self == Self::Spanish {
            if let Some((_, value)) = SPANISH.iter().find(|(source, _)| *source == key) {
                return (*value).to_owned();
            }
        }
        key.to_owned()
    }
}

#[derive(Clone, Copy)]
struct Locale(Signal<Library>);

pub(crate) fn provide(library: Signal<Library>) {
    provide_context(Locale(library));
}

pub(crate) fn language() -> Language {
    use_context::<Locale>()
        .map(|locale| Language::from_preference(&locale.0.get().settings.language))
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
            .map(|locale| Language::from_preference(&locale.0.get().settings.language))
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
    let mut result = t(key);
    for (name, value) in values {
        result = result.replace(&format!("{{{name}}}"), value);
    }
    result
}

const SPANISH: &[(&str, &str)] = &[
    ("RULES", "REGLAS"),
    ("Sessions", "Sesiones"),
    ("Jan", "ene"),
    ("Feb", "feb"),
    ("Mar", "mar"),
    ("Apr", "abr"),
    ("May", "may"),
    ("Jun", "jun"),
    ("Jul", "jul"),
    ("Aug", "ago"),
    ("Sep", "sep"),
    ("Oct", "oct"),
    ("Nov", "nov"),
    ("Dec", "dic"),
    ("Config", "Config."),
    ("Timer", "Temporizador"),
    ("History", "Historial"),
    ("Settings", "Ajustes"),
    ("SETTINGS", "AJUSTES"),
    ("Restore to Default", "Restablecer valores"),
    ("Language", "Idioma"),
    (
        "Show window after focus session",
        "Mostrar ventana al terminar la concentración",
    ),
    (
        "Bring the app forward when focus ends",
        "Traer la aplicación al frente al terminar la concentración",
    ),
    (
        "Show window after break session",
        "Mostrar ventana al terminar el descanso",
    ),
    (
        "Bring the app forward when a break ends",
        "Traer la aplicación al frente al terminar un descanso",
    ),
    ("Discord Rich Presence", "Actividad de Discord"),
    (
        "Not available yet. Discord integration is not included in this build.",
        "Aún no disponible. Esta versión no incluye la integración con Discord.",
    ),
    ("Close to Tray", "Minimizar a la bandeja al cerrar"),
    (
        "Keep the timer running when the window closes",
        "Mantener el temporizador activo al cerrar la ventana",
    ),
    ("System notifications", "Notificaciones del sistema"),
    (
        "Announce completed sessions in the operating system",
        "Notificar las sesiones completadas en el sistema operativo",
    ),
    ("Continue after focus", "Continuar tras la concentración"),
    (
        "Start the next break automatically",
        "Iniciar el siguiente descanso automáticamente",
    ),
    ("Continue after break", "Continuar tras el descanso"),
    (
        "Start the next focus session automatically",
        "Iniciar la siguiente sesión de concentración automáticamente",
    ),
    (
        "Quick switch configuration profiles",
        "Cambio rápido de perfiles de configuración",
    ),
    (
        "Choose profiles for quick switching on the Timer tab and in the tray menu.",
        "Elige perfiles para cambiar rápidamente en la pestaña Temporizador y en el menú de la bandeja.",
    ),
    ("Quick switch profiles", "Perfiles de cambio rápido"),
    ("Select profiles", "Seleccionar perfiles"),
    ("No matching profiles", "No hay perfiles coincidentes"),
    ("Configuration profile", "Perfil de configuración"),
    ("Configuration Profile", "Perfil de configuración"),
    ("Search profiles", "Buscar perfiles"),
    ("Add profile", "Añadir perfil"),
    ("Rename profile", "Renombrar perfil"),
    ("Delete profile", "Eliminar perfil"),
    ("Profile name", "Nombre del perfil"),
    ("Save profile name", "Guardar nombre del perfil"),
    ("Cancel rename", "Cancelar cambio de nombre"),
    ("Delete this profile?", "¿Eliminar este perfil?"),
    ("Its history will be kept.", "Se conservará su historial."),
    ("Confirm delete", "Confirmar eliminación"),
    ("Cancel delete", "Cancelar eliminación"),
    (
        "Keep at least one profile. Add another before deleting this one.",
        "Debe quedar al menos un perfil. Añade otro antes de eliminar este.",
    ),
    ("Storage unavailable", "Almacenamiento no disponible"),
    ("Payable", "Remunerado"),
    ("Hourly rate", "Tarifa por hora"),
    ("0.00 / hour", "0.00 / hora"),
    ("Currency", "Moneda"),
    ("No matching currencies", "No hay monedas coincidentes"),
    ("/ hour", "/ hora"),
    ("Profile color", "Color del perfil"),
    ("Profile color hex", "Color hexadecimal del perfil"),
    ("Stay Focused", "Concéntrate"),
    ("Take a Break", "Tómate un descanso"),
    ("Reset timer", "Reiniciar temporizador"),
    ("Skip session", "Saltar sesión"),
    ("Toggle full screen", "Alternar pantalla completa"),
    ("Pause timer", "Pausar temporizador"),
    ("Start timer", "Iniciar temporizador"),
    ("Mute announcements", "Silenciar avisos"),
    ("Enable announcements", "Activar avisos"),
    ("Session", "Sesión"),
    ("Session note", "Nota de la sesión"),
    ("What are you focusing on?", "¿En qué te vas a concentrar?"),
    ("Save note", "Guardar nota"),
    ("Edit note", "Editar nota"),
    ("TIMER RULES", "REGLAS DEL TEMPORIZADOR"),
    ("Restore Default", "Restablecer"),
    ("Stay focused", "Concentración"),
    ("Short break", "Descanso corto"),
    ("Long break", "Descanso largo"),
    ("Session rounds", "Rondas por sesión"),
    ("SPECIAL BREAKS", "DESCANSOS ESPECIALES"),
    ("rounds", "rondas"),
    ("mins", "min"),
    (
        "Special break {number} start time (HH:MM)",
        "Hora de inicio del descanso especial {number} (HH:MM)",
    ),
    (
        "Special break {number} duration in minutes (1–180)",
        "Duración del descanso especial {number} en minutos (1–180)",
    ),
    ("Switch to {name}{current}", "Cambiar a {name}{current}"),
    (" (current profile)", " (perfil actual)"),
    ("Use color {value}", "Usar color {value}"),
    ("Focus history", "Historial de concentración"),
    ("Reverse date order", "Invertir orden de fechas"),
    ("Show real history", "Mostrar historial real"),
    ("Show sample history", "Mostrar historial de ejemplo"),
    (
        "Sample preview only · saved history is unchanged.",
        "Solo vista de ejemplo · el historial guardado no cambia.",
    ),
    ("Today", "Hoy"),
    ("Week", "Semana"),
    ("Month", "Mes"),
    ("Year", "Año"),
    ("All", "Todo"),
    ("History: {title}", "Historial: {title}"),
    ("All profiles", "Todos los perfiles"),
    ("History profile", "Perfil del historial"),
    ("From date", "Fecha inicial"),
    ("To date", "Fecha final"),
    ("YYYY-MM-DD", "AAAA-MM-DD"),
    (
        "{count} sessions · {hours}h {minutes}m",
        "{count} sesiones · {hours}h {minutes}m",
    ),
    ("{amount} {currency} earned", "{amount} {currency} ganados"),
    ("SESSIONS · OLDEST FIRST", "SESIONES · MÁS ANTIGUAS PRIMERO"),
    (
        "SESSIONS · NEWEST FIRST",
        "SESIONES · MÁS RECIENTES PRIMERO",
    ),
    (
        "No completed focus sessions in this period. Finish a focus session to start your history.",
        "No hay sesiones de concentración completadas en este período. Completa una sesión para iniciar tu historial.",
    ),
    (
        "{count} focus sessions in the last year",
        "{count} sesiones de concentración en el último año",
    ),
    (
        "{date}: {count} completed focus sessions",
        "{date}: {count} sesiones de concentración completadas",
    ),
    (
        "Less  ░ ▒ ▓ █  More · colors show profiles",
        "Menos  ░ ▒ ▓ █  Más · los colores indican perfiles",
    ),
    (
        "FOCUS TIME BY PROFILE",
        "TIEMPO DE CONCENTRACIÓN POR PERFIL",
    ),
    ("Focus session", "Sesión de concentración"),
    ("Profile", "Perfil"),
    ("Focus", "Concentración"),
    ("Break", "Descanso"),
    ("Running", "En marcha"),
    ("Paused", "En pausa"),
    ("Stop", "Detener"),
    ("Play", "Iniciar"),
    (
        "Choose quick profiles in Settings",
        "Elige perfiles rápidos en Ajustes",
    ),
    ("Switch profile", "Cambiar perfil"),
    ("Restore the app", "Mostrar la aplicación"),
    ("Quit", "Salir"),
    (
        "Focus session complete. Time for a break.",
        "Sesión de concentración completada. Es hora de descansar.",
    ),
    (
        "Break complete. Your next focus session is ready.",
        "Descanso completado. Tu siguiente sesión de concentración está lista.",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(Language::from_preference("Русский"), Language::English);
        assert_eq!(Language::Spanish.translate("unknown"), "unknown");
        assert_eq!(Language::Spanish.translate("Timer"), "Temporizador");
    }

    #[test]
    fn catalog_has_unique_keys_and_matching_placeholders() {
        let mut keys = std::collections::HashSet::new();
        for (key, value) in SPANISH {
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
