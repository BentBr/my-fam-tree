use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("config: {0}")]
    Config(String),
}
