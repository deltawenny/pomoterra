//! SQLite storage for profiles and immutable completed-focus records.
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
    pub profile_name: String,
    pub color: String,
    pub started_at: i64,
    pub finished_at: i64,
    pub seconds: u64,
    pub note: String,
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

    fn initialize(connection: Connection) -> Result<Self> {
        connection
            .busy_timeout(std::time::Duration::from_secs(3))
            .map_err(|e| e.to_string())?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS profiles (
                id INTEGER PRIMARY KEY, name TEXT NOT NULL, color TEXT NOT NULL,
                payable INTEGER NOT NULL, rate INTEGER NOT NULL CHECK(rate >= 0),
                currency TEXT NOT NULL, rules TEXT NOT NULL, archived INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY, profile_id INTEGER NOT NULL REFERENCES profiles(id),
                profile_name TEXT NOT NULL, color TEXT NOT NULL,
                started_at INTEGER NOT NULL, finished_at INTEGER NOT NULL,
                seconds INTEGER NOT NULL CHECK(seconds > 0), note TEXT NOT NULL,
                rate INTEGER NOT NULL, currency TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS sessions_finished ON sessions(finished_at);
             PRAGMA user_version = 1;",
            )
            .map_err(|e| e.to_string())?;
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
        self.0.execute("INSERT INTO sessions(profile_id,profile_name,color,started_at,finished_at,seconds,note,rate,currency) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![profile.id, profile.name, profile.color, focus.started_at, focus.finished_at, seconds, focus.note, if profile.payable { profile.rate } else { 0 }, profile.currency]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn sessions(&self) -> Result<Vec<Session>> {
        let mut query = self.0.prepare("SELECT id,profile_id,profile_name,color,started_at,finished_at,seconds,note,rate,currency FROM sessions ORDER BY finished_at DESC,id DESC").map_err(|e| e.to_string())?;
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
        let result = self
            .database
            .as_ref()
            .ok_or_else(|| "Storage unavailable".to_owned())
            .and_then(|db| db.save_profile(&profile));
        match result {
            Ok(()) => {
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
        db.save_profile(&profile).unwrap();
        db.archive(profile.id).unwrap();
        assert!(db.profiles().unwrap().is_empty());
        let sessions = db.sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].profile_name, "Client work");
        assert_eq!(sessions[0].earnings(), 5229);
        assert_eq!(sessions[0].note, focus.note);
        assert_eq!(sessions[0].rate, 12550);
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
