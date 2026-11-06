//! Toasty models defining the application database schema.

#[derive(Debug, toasty::Model)]
#[table = "profiles"]
pub(super) struct StoredProfile {
    #[key]
    #[auto]
    pub id: i64,
    pub name: String,
    pub color: String,
    pub payable: bool,
    pub rate: i64,
    pub currency: String,
    pub rules: String,
    pub archived: bool,
}

#[derive(Debug, toasty::Model)]
#[table = "sessions"]
pub(super) struct StoredSession {
    #[key]
    #[auto]
    pub id: i64,
    pub profile_id: i64,
    pub started_at: i64,
    pub finished_at: i64,
    pub seconds: i64,
    pub note: String,
    pub rate: i64,
    pub currency: String,
}

#[derive(Debug, toasty::Model)]
#[table = "settings"]
pub(super) struct Setting {
    #[key]
    pub key: String,
    pub value: String,
}
