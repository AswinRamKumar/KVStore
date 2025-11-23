use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KvError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Key not found")]
    KeyNotFound,

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Log corruption detected at offset {0}")]
    LogCorruption(u64),

    #[error("Compaction failed: {0}")]
    CompactionFailed(String),
}

pub type Result<T> = std::result::Result<T, KvError>;
