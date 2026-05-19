use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("smtp: {0}")]
    Smtp(String),
    #[error("config: {0}")]
    Config(String),
    #[error("build: {0}")]
    Build(String),
}
