//! SQLite storage for profiles and completed-focus records linked by profile ID.
use std::{path::PathBuf, rc::Rc};

use crate::store::{CompletedFocus, TimerRules};
use rusqlite::{Connection, params};

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

pub struct Database(Connection);

type Result<T> = std::result::Result<T, String>;

impl Database {
    pub fn open() -> Result<Self> {
        let path = database_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        Self::initialize(Connection::open(path).map_err(|e| e.to_string())?)
    }

    fn initialize(mut connection: Connection) -> Result<Self> {
        connection
            .busy_timeout(std::time::Duration::from_secs(3))
            .map_err(|e| e.to_string())?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 PRAGMA journal_mode = WAL;",
            )
            .map_err(|e| e.to_string())?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let version: i64 = transaction
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        if version > 2 {
            return Err(format!(
                "Unsupported database version {version}. Update Pomoterra."
            ));
        }
        transaction
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS profiles (
                id INTEGER PRIMARY KEY, name TEXT NOT NULL, color TEXT NOT NULL,
                payable INTEGER NOT NULL, rate INTEGER NOT NULL CHECK(rate >= 0),
                currency TEXT NOT NULL, rules TEXT NOT NULL, archived INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY, profile_id INTEGER NOT NULL REFERENCES profiles(id),
                started_at INTEGER NOT NULL, finished_at INTEGER NOT NULL,
                seconds INTEGER NOT NULL CHECK(seconds > 0), note TEXT NOT NULL,
                rate INTEGER NOT NULL, currency TEXT NOT NULL
             );",
            )
            .map_err(|e| e.to_string())?;
        // Older databases stored profile name/color snapshots alongside the ID.
        // Rebuild atomically, preserving session IDs and all historical billing data.
        let legacy: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('sessions') WHERE name='profile_name')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if legacy {
            transaction
                .execute_batch(
                    "CREATE TABLE sessions_relational (
                        id INTEGER PRIMARY KEY,
                        profile_id INTEGER NOT NULL REFERENCES profiles(id),
                        started_at INTEGER NOT NULL, finished_at INTEGER NOT NULL,
                        seconds INTEGER NOT NULL CHECK(seconds > 0), note TEXT NOT NULL,
                        rate INTEGER NOT NULL, currency TEXT NOT NULL
                     );
                     INSERT INTO sessions_relational
                        (id,profile_id,started_at,finished_at,seconds,note,rate,currency)
                     SELECT id,profile_id,started_at,finished_at,seconds,note,rate,currency
                     FROM sessions;
                     DROP TABLE sessions;
                     ALTER TABLE sessions_relational RENAME TO sessions;",
                )
                .map_err(|e| e.to_string())?;
        }
        transaction
            .execute_batch(
                "CREATE INDEX IF NOT EXISTS sessions_finished ON sessions(finished_at);
                 CREATE INDEX IF NOT EXISTS sessions_profile ON sessions(profile_id);
                 PRAGMA user_version = 2;",
            )
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())?;
        let database = Self(connection);
        if database.profiles()?.is_empty() {
            database.create_profile("Default")?;
        }
        Ok(database)
    }

    pub fn profiles(&self) -> Result<Vec<Profile>> {
        let mut query = self.0.prepare("SELECT id,name,color,payable,rate,currency,rules FROM profiles WHERE archived=0 ORDER BY id").map_err(|e| e.to_string())?;
        let rows = query
            .query_map([], |row| {
                let json: String = row.get(6)?;
                let rules = serde_json::from_str(&json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        6,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(Profile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    payable: row.get(3)?,
                    rate: row.get(4)?,
                    currency: row.get(5)?,
                    rules,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn create_profile(&self, name: &str) -> Result<Profile> {
        let mut profile = Profile {
            id: 0,
            name: name.into(),
            color: "#0083c9".into(),
            payable: false,
            rate: 0,
            currency: "USD".into(),
            rules: TimerRules::default(),
        };
        self.0.execute("INSERT INTO profiles(name,color,payable,rate,currency,rules) VALUES (?1,?2,0,0,'USD',?3)", params![profile.name, profile.color, serde_json::to_string(&profile.rules).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
        profile.id = self.0.last_insert_rowid();
        Ok(profile)
    }

    pub fn save_profile(&self, profile: &Profile) -> Result<()> {
        if profile.name.trim().is_empty() {
            return Err("Profile name cannot be empty.".into());
        }
        self.0.execute("UPDATE profiles SET name=?2,color=?3,payable=?4,rate=?5,currency=?6,rules=?7 WHERE id=?1", params![profile.id, profile.name.trim(), profile.color, profile.payable, profile.rate, profile.currency, serde_json::to_string(&profile.rules).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn archive(&self, id: i64) -> Result<()> {
        self.0
            .execute("UPDATE profiles SET archived=1 WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn select(&self, id: i64) -> Result<()> {
        self.0.execute("INSERT INTO settings(key,value) VALUES ('active_profile',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [id.to_string()]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn selected(&self) -> Option<i64> {
        self.0
            .query_row(
                "SELECT value FROM settings WHERE key='active_profile'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()?
            .parse()
            .ok()
    }

    pub fn record(&self, profile: &Profile, focus: &CompletedFocus) -> Result<()> {
        let seconds = i64::try_from(focus.seconds).map_err(|e| e.to_string())?;
        self.0.execute("INSERT INTO sessions(profile_id,started_at,finished_at,seconds,note,rate,currency) VALUES (?1,?2,?3,?4,?5,?6,?7)", params![profile.id, focus.started_at, focus.finished_at, seconds, focus.note, if profile.payable { profile.rate } else { 0 }, profile.currency]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn sessions(&self) -> Result<Vec<Session>> {
        let mut query = self.0.prepare("SELECT s.id,s.profile_id,p.name,p.color,s.started_at,s.finished_at,s.seconds,s.note,s.rate,s.currency FROM sessions s JOIN profiles p ON p.id=s.profile_id ORDER BY s.finished_at DESC,s.id DESC").map_err(|e| e.to_string())?;
        let rows = query
            .query_map([], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    profile_id: row.get(1)?,
                    profile_name: row.get(2)?,
                    color: row.get(3)?,
                    started_at: row.get(4)?,
                    finished_at: row.get(5)?,
                    seconds: row.get::<_, i64>(6)? as u64,
                    note: row.get(7)?,
                    rate: row.get(8)?,
                    currency: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
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
    fn profiles_sessions_and_archiving_round_trip() {
        let db = Database::initialize(Connection::open_in_memory().unwrap()).unwrap();
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
    fn legacy_sessions_migrate_without_losing_history() {
        let db = Database::initialize(Connection::open_in_memory().unwrap()).unwrap();
        let mut profile = db.profiles().unwrap().remove(0);
        profile.name = "Current name".into();
        profile.color = "#a855f7".into();
        db.save_profile(&profile).unwrap();
        // Recreate the version-one session layout with stale profile snapshots.
        db.0.execute_batch(
            "DROP TABLE sessions;
             CREATE TABLE sessions (
                id INTEGER PRIMARY KEY, profile_id INTEGER NOT NULL REFERENCES profiles(id),
                profile_name TEXT NOT NULL, color TEXT NOT NULL,
                started_at INTEGER NOT NULL, finished_at INTEGER NOT NULL,
                seconds INTEGER NOT NULL CHECK(seconds > 0), note TEXT NOT NULL,
                rate INTEGER NOT NULL, currency TEXT NOT NULL
             );
             CREATE INDEX sessions_finished ON sessions(finished_at);
             INSERT INTO sessions VALUES
                (42,1,'Old name','#0083c9',100,1600,1500,'Original note',12550,'USD'),
                (43,1,'Another old name','#008c69',2000,5600,3600,'Second note',0,'EUR');
             UPDATE profiles SET archived=1 WHERE id=1;
             PRAGMA user_version = 1;",
        )
        .unwrap();
        let db = Database::initialize(db.0).unwrap();
        let sessions = db.sessions().unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, 43);
        assert_eq!(sessions[1].id, 42);
        for session in &sessions {
            assert_eq!(session.profile_id, profile.id);
            assert_eq!(session.profile_name, profile.name);
            assert_eq!(session.color, profile.color);
        }
        assert_eq!(sessions[1].started_at, 100);
        assert_eq!(sessions[1].finished_at, 1600);
        assert_eq!(sessions[1].seconds, 1500);
        assert_eq!(sessions[1].note, "Original note");
        assert_eq!(sessions[1].rate, 12550);
        assert_eq!(sessions[1].currency, "USD");
        assert_eq!(sessions[1].earnings(), 5229);
        assert_eq!(sessions[0].note, "Second note");
        assert_eq!(sessions[0].rate, 0);
        assert_eq!(sessions[0].currency, "EUR");
        let columns: String =
            db.0.query_row(
                "SELECT group_concat(name, ',') FROM pragma_table_info('sessions')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            columns,
            "id,profile_id,started_at,finished_at,seconds,note,rate,currency"
        );
        assert_eq!(
            db.0.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            2
        );
        // Opening an already-migrated database is idempotent, even for archived history.
        let reopened = Database::initialize(db.0).unwrap();
        assert_eq!(reopened.sessions().unwrap(), sessions);
    }

    #[test]
    fn profile_edits_update_loaded_history_only_for_that_profile() {
        let db = Rc::new(Database::initialize(Connection::open_in_memory().unwrap()).unwrap());
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
    fn sessions_require_an_existing_profile_and_prevent_its_deletion() {
        let db = Database::initialize(Connection::open_in_memory().unwrap()).unwrap();
        let mut profile = db.profiles().unwrap().remove(0);
        let focus = CompletedFocus {
            started_at: 100,
            finished_at: 1600,
            seconds: 1500,
            note: String::new(),
        };
        db.record(&profile, &focus).unwrap();
        assert!(
            db.0.execute("DELETE FROM profiles WHERE id=?1", [profile.id])
                .is_err()
        );
        profile.id += 100;
        assert!(db.record(&profile, &focus).is_err());
        assert_eq!(db.sessions().unwrap().len(), 1);
    }

    #[test]
    fn newer_database_versions_are_not_downgraded() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA user_version = 3;")
            .unwrap();
        assert!(
            matches!(Database::initialize(connection), Err(error) if error.contains("Unsupported database version 3"))
        );
    }

    #[test]
    fn database_survives_reopening() {
        let path = std::env::temp_dir().join(format!(
            "pomoterra-test-{}-{}.sqlite3",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        {
            let db = Database::initialize(Connection::open(&path).unwrap()).unwrap();
            db.create_profile("Persistent").unwrap();
        }
        let db = Database::initialize(Connection::open(&path).unwrap()).unwrap();
        assert_eq!(db.profiles().unwrap()[1].name, "Persistent");
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
}
