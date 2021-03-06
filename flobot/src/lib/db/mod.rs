pub mod models;
use diesel::Connection;
use std::convert::From;

use crate::db::models as business_models;

#[cfg(feature = "sqlite")]
pub type DatabaseConnection = diesel::SqliteConnection;
#[cfg(feature = "sqlite")]
pub mod sqlite;

pub mod schema;

#[derive(Debug)]
pub enum Error {
    Database(String),
    Migration(String),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Migration(e) => write!(f, "Cannot run migrations: {}", e),
            Error::Database(e) => write!(f, "db::Error: {}", e),
        }
    }
}

impl From<diesel::result::Error> for Error {
    fn from(e: diesel::result::Error) -> Self {
        Error::Database(e.to_string())
    }
}

impl From<diesel_migrations::RunMigrationsError> for Error {
    fn from(e: diesel_migrations::RunMigrationsError) -> Self {
        Error::Migration(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Trigger {
    fn list(&self, team_id: &str) -> Result<Vec<business_models::Trigger>>;
    fn search(&self, team_id_: &str) -> Result<Vec<business_models::Trigger>>;
    fn add_text(&self, team_id: &str, trigger: &str, text: &str) -> Result<()>;
    fn add_emoji(&self, team_id: &str, trigger: &str, emoji: &str) -> Result<()>;
    fn del(&self, team_id: &str, trigger: &str) -> Result<()>;
}

pub trait Edits {
    fn list(&self, team_id: &str) -> Result<Vec<business_models::Edit>>;
    fn find(
        &self,
        user_id: &str,
        team_id: &str,
        edit: &str,
    ) -> Result<Option<business_models::Edit>>;
    fn del_team(&self, team_id: &str, edit: &str) -> Result<()>;
    fn add_team(&self, team_id: &str, edit: &str, replace: &str) -> Result<()>;
}

pub trait Joke {
    // with 0 <= relnum < count()
    fn pick(&self, team_id: &str, relnum: u64)
        -> Result<Option<business_models::Joke>>;
    fn count(&self, team_id: &str) -> Result<u64>;
    fn list(&self, team_id: &str) -> Result<Vec<business_models::Joke>>;
    fn del(&self, team_id: &str, id: i32) -> Result<()>;
    fn add(&self, team_id: &str, text: &str) -> Result<()>;
}

pub trait SMS {
    fn set_contact(
        &self,
        team_id: &str,
        name: &str,
        number: &str,
    ) -> Result<business_models::SMSContact>;
    fn set_prepare(
        &self,
        team_id: &str,
        contact_id: &i32,
        trigname: &str,
        name: &str,
        text: &str,
    ) -> Result<business_models::SMSPrepare>;
    fn get_contact(
        &self,
        team_id: &str,
        name: Option<&str>,
        id: Option<&i32>,
    ) -> Result<Option<business_models::SMSContact>>;
    fn get_prepare(
        &self,
        team_id: &str,
        trigname: &str,
    ) -> Result<Option<business_models::SMSPrepare>>;
    fn list_contacts(&self, team_id: &str) -> Result<Vec<business_models::SMSContact>>;
    fn list_prepare(
        &self,
        team_id: &str,
    ) -> Result<Vec<(business_models::SMSPrepare, business_models::SMSContact)>>;
}

pub fn conn(db_url: &str) -> DatabaseConnection {
    return DatabaseConnection::establish(db_url).expect("db connection");
}
