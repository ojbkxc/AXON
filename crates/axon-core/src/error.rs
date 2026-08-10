use std::fmt;

#[derive(Debug)]
pub enum AxonError {
    Config(String),
    NotFound(String),
    Validation(String),
    Gateway(String),
    Upstream(String),
    Storage(String),
    Tool(String),
    Http(String),
    Internal(String),
}

impl fmt::Display for AxonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AxonError::Config(msg) => write!(f, "config error: {msg}"),
            AxonError::NotFound(msg) => write!(f, "not found: {msg}"),
            AxonError::Validation(msg) => write!(f, "validation error: {msg}"),
            AxonError::Gateway(msg) => write!(f, "gateway error: {msg}"),
            AxonError::Upstream(msg) => write!(f, "upstream error: {msg}"),
            AxonError::Storage(msg) => write!(f, "storage error: {msg}"),
            AxonError::Tool(msg) => write!(f, "tool error: {msg}"),
            AxonError::Http(msg) => write!(f, "http error: {msg}"),
            AxonError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for AxonError {}

impl From<serde_json::Error> for AxonError {
    fn from(e: serde_json::Error) -> Self {
        AxonError::Config(format!("json: {e}"))
    }
}

impl From<serde_yaml::Error> for AxonError {
    fn from(e: serde_yaml::Error) -> Self {
        AxonError::Config(format!("yaml: {e}"))
    }
}

impl From<std::io::Error> for AxonError {
    fn from(e: std::io::Error) -> Self {
        AxonError::Internal(format!("io: {e}"))
    }
}

#[cfg(feature = "sqlite")]
impl From<rusqlite::Error> for AxonError {
    fn from(e: rusqlite::Error) -> Self {
        AxonError::Storage(format!("sqlite: {e}"))
    }
}

pub type Result<T> = std::result::Result<T, AxonError>;
