//! Thread-safe menu snapshots; native callbacks never access reactive signals.
use std::sync::{LazyLock, Mutex};

use crate::{
    persistence::Library,
    store::{Phase, TimerState},
};

static SNAPSHOT: LazyLock<Mutex<TrayMenu>> = LazyLock::new(|| Mutex::new(TrayMenu::default()));
static PROFILE_REQUEST: Mutex<Option<i64>> = Mutex::new(None);
static PLAYBACK_REQUEST: Mutex<Option<bool>> = Mutex::new(None);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct QuickProfile {
    pub id: i64,
    pub name: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TrayMenu {
    pub language: crate::i18n::Language,
    pub profile: String,
    pub phase: String,
    pub time: String,
    pub running: bool,
    pub profiles: Vec<QuickProfile>,
}

impl Default for TrayMenu {
    fn default() -> Self {
        Self {
            language: Default::default(),
            profile: "Profile: —".into(),
            phase: "Focus · Paused".into(),
            time: "25:00 / 25:00".into(),
            running: false,
            profiles: Vec::new(),
        }
    }
}

impl TrayMenu {
    fn snapshot(timer: &TimerState, library: &Library, active: i64) -> Self {
        let language = crate::i18n::Language::from_preference(&library.settings.language);
        let name = library
            .profiles
            .iter()
            .find(|p| p.id == active)
            .map(|p| p.name.as_str())
            .unwrap_or("—");
        let phase = match timer.phase {
            Phase::Focus => "focus",
            Phase::Break => "break",
        };
        let status = if timer.running { "running" } else { "paused" };
        let duration = timer.duration();
        let mut profiles: Vec<_> = library
            .settings
            .quick_profiles
            .iter()
            .filter_map(|id| library.profiles.iter().find(|p| p.id == *id))
            .map(|p| QuickProfile {
                id: p.id,
                name: p.name.clone(),
                selected: p.id == active,
            })
            .collect();
        profiles.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then(a.id.cmp(&b.id))
        });
        profiles.dedup_by_key(|p| p.id);
        Self {
            language,
            running: timer.running,
            profile: format!("{}: {name}", language.translate("profile")),
            phase: format!(
                "{} · {}",
                language.translate(phase),
                language.translate(status)
            ),
            time: format!(
                "{:02}:{:02} / {:02}:{:02}",
                timer.remaining / 60,
                timer.remaining % 60,
                duration / 60,
                duration % 60
            ),
            profiles,
        }
    }

    pub(super) fn playback_label(&self) -> String {
        self.translate(if self.running { "stop" } else { "play" })
    }

    pub(super) fn translate(&self, key: &str) -> String {
        self.language.translate(key)
    }

    pub(super) fn current() -> Self {
        SNAPSHOT.lock().unwrap().clone()
    }
}

pub(super) fn publish(timer: &TimerState, library: &Library, active: i64) {
    *SNAPSHOT.lock().unwrap() = TrayMenu::snapshot(timer, library, active);
}

pub(super) fn request_profile(id: i64) {
    *PROFILE_REQUEST.lock().unwrap() = Some(id);
}

pub(super) fn take_profile_request() -> Option<i64> {
    PROFILE_REQUEST.lock().unwrap().take()
}

pub(super) fn request_playback(running: bool) {
    *PLAYBACK_REQUEST.lock().unwrap() = Some(running);
}

pub(super) fn take_playback_request() -> Option<bool> {
    PLAYBACK_REQUEST.lock().unwrap().take()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> Library {
        Library {
            database: None,
            profiles: Vec::new(),
            sessions: Vec::new(),
            settings: Default::default(),
            error: String::new(),
        }
    }

    #[test]
    fn quick_profiles_are_filtered_sorted_and_mark_selected() {
        let mut library = library();
        for (id, name) in [(1, "Work"), (2, "Personal"), (3, "Hidden")] {
            library.profiles.push(crate::persistence::Profile {
                id,
                name: name.into(),
                color: "#0083c9".into(),
                payable: false,
                rate: 0,
                currency: "USD".into(),
                rules: Default::default(),
            });
        }
        library.settings.quick_profiles = vec![1, 999, 2, 1];
        let menu = TrayMenu::snapshot(&TimerState::default(), &library, 1);
        assert_eq!(menu.profile, "Profile: Work");
        assert_eq!(
            menu.profiles
                .iter()
                .map(|p| (p.id, p.selected))
                .collect::<Vec<_>>(),
            [(2, false), (1, true)]
        );
        library.profiles[0].name = "Renamed".into();
        assert_eq!(
            TrayMenu::snapshot(&TimerState::default(), &library, 1).profile,
            "Profile: Renamed"
        );
    }

    #[test]
    fn spanish_snapshot_translates_status_and_actions() {
        let mut library = library();
        library.settings.language = "Español".into();
        let mut timer = TimerState::default();
        let menu = TrayMenu::snapshot(&timer, &library, 0);
        assert_eq!(menu.profile, "Perfil: —");
        assert_eq!(menu.phase, "Concentración · En pausa");
        assert_eq!(menu.playback_label(), "Iniciar");
        assert_eq!(menu.translate("quit"), "Salir");
        timer.skip();
        timer.toggle(std::time::Instant::now());
        let menu = TrayMenu::snapshot(&timer, &library, 0);
        assert_eq!(menu.phase, "Descanso · En marcha");
        assert_eq!(menu.playback_label(), "Detener");
        assert_eq!(menu.time, "05:00 / 05:00");
    }

    #[test]
    fn russian_snapshot_translates_status_and_actions() {
        let mut library = library();
        library.settings.language = "Русский".into();
        let mut timer = TimerState::default();
        let menu = TrayMenu::snapshot(&timer, &library, 0);
        assert_eq!(menu.profile, "Профиль: —");
        assert_eq!(menu.phase, "Работа · Пауза");
        assert_eq!(menu.playback_label(), "Запустить");
        assert_eq!(menu.translate("quit"), "Выйти");
        assert_eq!(menu.translate("switch_profile"), "Сменить профиль");
        timer.skip();
        timer.toggle(std::time::Instant::now());
        let menu = TrayMenu::snapshot(&timer, &library, 0);
        assert_eq!(menu.phase, "Перерыв · Идёт");
        assert_eq!(menu.playback_label(), "Остановить");
        assert_eq!(menu.time, "05:00 / 05:00");
    }

    #[test]
    fn formats_remaining_and_actual_phase_duration() {
        let mut timer = TimerState::default();
        timer.remaining = 24 * 60 + 12;
        let library = library();
        let menu = TrayMenu::snapshot(&timer, &library, i64::MIN);
        assert_eq!(menu.profile, "Profile: —");
        assert_eq!(menu.phase, "Focus · Paused");
        assert_eq!(menu.playback_label(), "Play");
        assert_eq!(menu.time, "24:12 / 25:00");
        timer.skip();
        let menu = TrayMenu::snapshot(&timer, &library, i64::MIN);
        assert_eq!(menu.phase, "Break · Paused");
        assert_eq!(menu.time, "05:00 / 05:00");
        timer.toggle(std::time::Instant::now());
        assert_eq!(
            TrayMenu::snapshot(&timer, &library, 0).phase,
            "Break · Running"
        );
        assert_eq!(
            TrayMenu::snapshot(&timer, &library, 0).playback_label(),
            "Stop"
        );
        timer.configure(|rules| rules.rounds = 1);
        timer.skip();
        assert_eq!(
            TrayMenu::snapshot(&timer, &library, 0).time,
            "15:00 / 15:00"
        );
    }
}
