
use crate::schema::SchemaError;

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum DbError {
    TableNotFound(String),
    TableAlreadyExists(String),
    PrimaryKeyNotUnique(String),
    LogUnavailable(String),
    WriteOutcomeUnknown(String),
    InvalidSchema(SchemaError),
    DiskFull(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbErrorStatus {
    Rejected,
    Unavailable,
    OutcomeUnknown,
}

impl DbError {
    pub fn db_error_status(&self) -> DbErrorStatus {
        match &self {
            DbError::TableNotFound(_) => DbErrorStatus::Rejected,
            DbError::TableAlreadyExists(_) => DbErrorStatus::Rejected,
            DbError::PrimaryKeyNotUnique(_) => DbErrorStatus::Rejected,
            DbError::LogUnavailable(_) => DbErrorStatus::Unavailable,
            DbError::WriteOutcomeUnknown(_) => DbErrorStatus::OutcomeUnknown,
            DbError::InvalidSchema(_) => DbErrorStatus::Rejected,
            DbError::DiskFull(_) => DbErrorStatus::Unavailable,
        }
    }
}