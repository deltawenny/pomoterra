//! Toasty-backed SQLite storage for profiles and completed-focus records.
//! Toasty owns the schema; unreleased legacy layouts are not migrated.
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

mod models;
use models::{Setting, StoredProfile, StoredSession};

use crate::store::{CompletedFocus, TimerRules};

#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub payable: bool,
    /// Integer minor currency units per hour; avoids floating point money storage.
    pub rate: i64,
    pub currency: String,
    pub rules: TimerRules,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    pub id: i64,
    pub profile_id: i64,
    /// Current profile metadata joined at read time, not stored in the session.
    pub profile_name: String,
    pub color: String,
    pub started_at: i64,
    pub finished_at: i64,
    pub seconds: u64,
    pub note: String,
    /// Billing snapshots preserve earnings when profile pricing changes.
    pub rate: i64,
    pub currency: String,
}

impl Session {
    pub fn earnings(&self) -> i64 {
        ((self.seconds as i128 * self.rate as i128 + 1800) / 3600) as i64
    }
}

pub struct Database {
    connection: RefCell<toasty::Connection>,
    _db: toasty::Db,
    runtime: tokio::runtime::Runtime,
}

type Result<T> = std::result::Result<T, String>;

impl Database {
    pub fn open() -> Result<Self> {
        let path = database_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        Self::open_path(&path)
    }

    fn open_path(path: &Path) -> Result<Self> {
        let driver = if path == Path::new(":memory:") {
            toasty_driver_sqlite::Sqlite::in_memory()
        } else {
            toasty_driver_sqlite::Sqlite::open(path)
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let (db, toasty_connection) = runtime
            .block_on(async {
                let db = toasty::Db::builder()
                    .models(toasty::models!(StoredProfile, StoredSession, Setting))
                    .max_pool_size(1)
                    .build(driver)
                    .await?;
                let mut connection = db.connection().await?;
                for pragma in ["PRAGMA foreign_keys = ON", "PRAGMA busy_timeout = 3000", "PRAGMA journal_mode = WAL"] {
                    toasty::sql::query(pragma).exec(&mut connection).await?;
                }
                // push_schema creates a fresh schema, not an incremental migration.
                // Leave existing databases untouched; unsupported development
                // layouts surface as storage errors instead of being destroyed.
                let tables = toasty::sql::query(
                    "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'"
                ).exec(&mut connection).await?;
                if tables.is_empty() {
                    connection.push_schema().await?;
                }
                Ok::<_, toasty::Error>((db, connection))
            })
            .map_err(|e| e.to_string())?;
        let database = Self {
            connection: RefCell::new(toasty_connection),
            _db: db,
            runtime,
        };
        if database.profiles()?.is_empty() {
            database.create_profile("Default")?;
        }
        Ok(database)
    }

    fn run<T>(
        &self,
        operation: impl AsyncFnOnce(&mut toasty::Connection) -> toasty::Result<T>,
    ) -> Result<T> {
        self.runtime
            .block_on(operation(&mut self.connection.borrow_mut()))
            .map_err(|e| e.to_string())
    }

    pub fn profiles(&self) -> Result<Vec<Profile>> {
        let mut rows = self.run(async |db| {
            StoredProfile::all()
                .filter(StoredProfile::fields().archived().eq(false))
                .exec(db)
                .await
        })?;
        rows.sort_by_key(|row| row.id);
        rows.into_iter()
            .map(|row| {
                Ok(Profile {
                    id: row.id,
                    name: row.name,
                    color: row.color,
                    payable: row.payable,
                    rate: row.rate,
                    currency: row.currency,
                    rules: serde_json::from_str(&row.rules).map_err(|e| e.to_string())?,
                })
            })
            .collect()
    }

    pub fn create_profile(&self, name: &str) -> Result<Profile> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Profile name cannot be empty.".into());
        }
        let rules = TimerRules::default();
        let json = serde_json::to_string(&rules).map_err(|e| e.to_string())?;
        let row = self.run(async |db| {
            StoredProfile::create()
                .name(name)
                .color("#0083c9")
                .payable(false)
                .rate(0_i64)
                .currency("USD")
                .rules(json)
                .archived(false)
                .exec(db)
                .await
        })?;
        Ok(Profile {
            id: row.id,
            name: row.name,
            color: row.color,
            payable: row.payable,
            rate: row.rate,
            currency: row.currency,
            rules,
        })
    }

    pub fn save_profile(&self, profile: &Profile) -> Result<()> {
        if profile.name.trim().is_empty() {
            return Err("Profile name cannot be empty.".into());
        }
        if profile.rate < 0 {
            return Err("Hourly rate cannot be negative.".into());
        }
        let rules = serde_json::to_string(&profile.rules).map_err(|e| e.to_string())?;
        self.run(async |db| {
            StoredProfile::filter_by_id(profile.id)
                .update()
                .name(profile.name.trim())
                .color(profile.color.as_str())
                .payable(profile.payable)
                .rate(profile.rate)
                .currency(profile.currency.as_str())
                .rules(rules)
                .exec(db)
                .await?;
            Ok(())
        })
    }

    pub fn archive(&self, id: i64) -> Result<()> {
        self.run(async |db| {
            StoredProfile::filter_by_id(id)
                .update()
                .archived(true)
                .exec(db)
                .await?;
            Ok(())
        })
    }

    pub fn select(&self, id: i64) -> Result<()> {
        self.run(async |db| {
            let mut transaction = db.transaction().await?;
            if Setting::filter_by_key("active_profile")
                .first()
                .exec(&mut transaction)
                .await?
                .is_some()
            {
                Setting::filter_by_key("active_profile")
                    .update()
                    .value(id.to_string())
                    .exec(&mut transaction)
                    .await?;
            } else {
                Setting::create()
                    .key("active_profile")
                    .value(id.to_string())
                    .exec(&mut transaction)
                    .await?;
            }
            transaction.commit().await
        })
    }

    pub fn selected(&self) -> Option<i64> {
        self.run(async |db| {
            Setting::filter_by_key("active_profile")
                .first()
                .exec(db)
                .await
        })
        .ok()??
        .value
        .parse()
        .ok()
    }

    pub fn record(&self, profile: &Profile, focus: &CompletedFocus) -> Result<()> {
        let seconds = i64::try_from(focus.seconds).map_err(|e| e.to_string())?;
        if seconds == 0 {
            return Err("A completed session must have a positive duration.".into());
        }
        self.run(async |db| {
            StoredProfile::filter_by_id(profile.id)
                .one()
                .exec(db)
                .await?;
            StoredSession::create()
                .profile_id(profile.id)
                .started_at(focus.started_at)
                .finished_at(focus.finished_at)
                .seconds(seconds)
                .note(focus.note.as_str())
                .rate(if profile.payable { profile.rate } else { 0 })
                .currency(profile.currency.as_str())
                .exec(db)
                .await?;
            Ok(())
        })
    }

    pub fn sessions(&self) -> Result<Vec<Session>> {
        // Fetch both tables in one read transaction. Include archived profiles;
        // metadata is current while billing fields remain session snapshots.
        let (profiles, mut sessions) = self.run(async |db| {
            let mut transaction = db.transaction().await?;
            let profiles = StoredProfile::all().exec(&mut transaction).await?;
            let sessions = StoredSession::all().exec(&mut transaction).await?;
            transaction.commit().await?;
            Ok((profiles, sessions))
        })?;
        let profiles: HashMap<_, _> = profiles.into_iter().map(|p| (p.id, p)).collect();
        sessions.sort_by_key(|s| std::cmp::Reverse((s.finished_at, s.id)));
        sessions
            .into_iter()
            .map(|row| {
                let profile = profiles.get(&row.profile_id).ok_or_else(|| {
                    format!("Missing profile {} for session {}", row.profile_id, row.id)
                })?;
                Ok(Session {
                    id: row.id,
                    profile_id: row.profile_id,
                    profile_name: profile.name.clone(),
                    color: profile.color.clone(),
                    started_at: row.started_at,
                    finished_at: row.finished_at,
                    seconds: u64::try_from(row.seconds).map_err(|e| e.to_string())?,
                    note: row.note,
                    rate: row.rate,
                    currency: row.currency,
                })
            })
            .collect()
    }
}

pub fn database_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("POMOTERRA_DB_PATH") {
        return Ok(path.into());
    }
    dirs::data_local_dir()
        .map(|path| path.join("pomoterra").join("pomoterra.sqlite3"))
        .ok_or_else(|| {
            "Could not locate your application data directory. Set POMOTERRA_DB_PATH.".into()
        })
}

#[derive(Clone)]
pub struct Library {
    pub database: Option<Rc<Database>>,
    pub profiles: Vec<Profile>,
    pub sessions: Vec<Session>,
    pub error: String,
}

impl PartialEq for Library {
    fn eq(&self, other: &Self) -> bool {
        self.profiles == other.profiles
            && self.sessions == other.sessions
            && self.error == other.error
    }
}

impl Library {
    pub fn load() -> Self {
        match Database::open().and_then(|database| {
            Ok((
                database.profiles()?,
                database.sessions()?,
                Rc::new(database),
            ))
        }) {
            Ok((profiles, sessions, database)) => Self {
                database: Some(database),
                profiles,
                sessions,
                error: String::new(),
            },
            Err(error) => Self {
                database: None,
                profiles: vec![],
                sessions: vec![],
                error: format!("Storage unavailable: {error}"),
            },
        }
    }

    pub fn edit(&mut self, id: i64, edit: impl FnOnce(&mut Profile)) {
        let Some(index) = self.profiles.iter().position(|p| p.id == id) else {
            return;
        };
        let mut profile = self.profiles[index].clone();
        edit(&mut profile);
        profile.name = profile.name.trim().to_owned();
        let result = self
            .database
            .as_ref()
            .ok_or_else(|| "Storage unavailable".to_owned())
            .and_then(|db| db.save_profile(&profile));
        match result {
            Ok(()) => {
                // Keep the loaded history in sync without waiting for a restart.
                for session in self.sessions.iter_mut().filter(|s| s.profile_id == id) {
                    session.profile_name = profile.name.clone();
                    session.color = profile.color.clone();
                }
                self.profiles[index] = profile;
                self.error.clear();
            }
            Err(error) => self.error = error,
        }
    }
}

pub fn color(value: &str) -> Option<u32> {
    let hex = value.strip_prefix('#')?;
    (hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| u32::from_str_radix(hex, 16).ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_databases_are_isolated_and_selection_can_be_replaced() {
        let db = Database::open_path(Path::new(":memory:")).unwrap();
        let other = Database::open_path(Path::new(":memory:")).unwrap();
        assert_eq!(db.selected(), None);
        let profile = db.create_profile("Second").unwrap();
        db.select(1).unwrap();
        db.select(profile.id).unwrap();
        assert_eq!(db.selected(), Some(profile.id));
        assert_eq!(other.selected(), None);
        assert_eq!(other.profiles().unwrap().len(), 1);
        assert!(db.create_profile("  ").is_err());
    }

    #[test]
    fn database_constraints_reject_invalid_writes() {
        let db = Database::open_path(Path::new(":memory:")).unwrap();
        let mut profile = db.profiles().unwrap().remove(0);
        profile.rate = -1;
        assert!(db.save_profile(&profile).is_err());
        assert_eq!(db.profiles().unwrap()[0].rate, 0);
        for seconds in [0, u64::MAX] {
            assert!(
                db.record(
                    &profile,
                    &CompletedFocus {
                        started_at: 100,
                        finished_at: 100,
                        seconds,
                        note: String::new(),
                    }
                )
                .is_err()
            );
        }
        assert!(db.sessions().unwrap().is_empty());
    }

    #[test]
    fn profiles_sessions_and_archiving_round_trip() {
        let db = Database::open_path(Path::new(":memory:")).unwrap();
        let mut profile = db.profiles().unwrap().remove(0);
        profile.name = "Client work".into();
        profile.payable = true;
        profile.rate = 12550;
        profile.rules.focus_minutes = 50;
        profile.rules.special_breaks[0].time = "12:30".into();
        db.save_profile(&profile).unwrap();
        assert_eq!(db.profiles().unwrap()[0], profile);
        db.select(profile.id).unwrap();
        assert_eq!(db.selected(), Some(profile.id));
        let focus = CompletedFocus {
            started_at: 100,
            finished_at: 1600,
            seconds: 1500,
            note: "Implement history".into(),
        };
        db.record(&profile, &focus).unwrap();
        profile.rate = 999;
        profile.name = "Renamed".into();
        profile.color = "#a855f7".into();
        profile.currency = "EUR".into();
        db.save_profile(&profile).unwrap();
        db.archive(profile.id).unwrap();
        assert!(db.profiles().unwrap().is_empty());
        let sessions = db.sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].profile_id, profile.id);
        assert_eq!(sessions[0].profile_name, "Renamed");
        assert_eq!(sessions[0].color, "#a855f7");
        assert_eq!(sessions[0].currency, "USD");
        assert_eq!(sessions[0].earnings(), 5229);
        assert_eq!(sessions[0].note, focus.note);
        assert_eq!(sessions[0].rate, 12550);
    }

    #[test]
    fn profile_edits_update_loaded_history_only_for_that_profile() {
        let db = Rc::new(Database::open_path(Path::new(":memory:")).unwrap());
        let first = db.profiles().unwrap().remove(0);
        let second = db.create_profile("Other").unwrap();
        let focus = CompletedFocus {
            started_at: 100,
            finished_at: 3700,
            seconds: 3600,
            note: "Keep this note".into(),
        };
        db.record(&first, &focus).unwrap();
        db.record(&second, &focus).unwrap();
        let mut library = Library {
            profiles: db.profiles().unwrap(),
            sessions: db.sessions().unwrap(),
            database: Some(db.clone()),
            error: String::new(),
        };
        let original = library.sessions.clone();
        library.edit(first.id, |p| {
            p.name = "  Renamed  ".into();
            p.color = "#008c69".into();
            p.payable = true;
            p.rate = 999;
            p.currency = "EUR".into();
        });
        assert!(library.error.is_empty());
        assert_eq!(library.profiles[0].name, "Renamed");
        assert_eq!(library.sessions, db.sessions().unwrap());
        assert_eq!(library.sessions[0], original[0]);
        assert_eq!(library.sessions[1].profile_name, "Renamed");
        assert_eq!(library.sessions[1].color, "#008c69");
        assert_eq!(library.sessions[1].note, focus.note);
        assert_eq!(library.sessions[1].rate, 0);
        assert_eq!(library.sessions[1].currency, "USD");
        let saved = library.sessions.clone();
        library.edit(first.id, |p| p.name = "  ".into());
        assert!(!library.error.is_empty());
        assert_eq!(library.sessions, saved);
        assert_eq!(library.profiles[0].name, "Renamed");
    }

    #[test]
    fn sessions_require_an_existing_profile() {
        let db = Database::open_path(Path::new(":memory:")).unwrap();
        let mut profile = db.profiles().unwrap().remove(0);
        let focus = CompletedFocus {
            started_at: 100,
            finished_at: 1600,
            seconds: 1500,
            note: String::new(),
        };
        db.record(&profile, &focus).unwrap();
        profile.id += 100;
        assert!(db.record(&profile, &focus).is_err());
        assert_eq!(db.sessions().unwrap().len(), 1);
    }

    #[test]
    fn database_survives_reopening() {
        let path = std::env::temp_dir().join(format!(
            "pomoterra-test-{}-{}.sqlite3",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        {
            let db = Database::open_path(&path).unwrap();
            let profile = db.create_profile("Persistent").unwrap();
            db.select(profile.id).unwrap();
            db.record(
                &profile,
                &CompletedFocus {
                    started_at: 100,
                    finished_at: 1600,
                    seconds: 1500,
                    note: "Persistent history".into(),
                },
            )
            .unwrap();
        }
        let db = Database::open_path(&path).unwrap();
        let profile = &db.profiles().unwrap()[1];
        assert_eq!(profile.name, "Persistent");
        assert_eq!(db.selected(), Some(profile.id));
        let sessions = db.sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].profile_id, profile.id);
        assert_eq!(sessions[0].note, "Persistent history");
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
}
