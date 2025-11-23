use serde::{Deserialize, Serialize};

/// Write-ahead log command persisted to disk.
/// Only mutating operations (Set/Remove) are logged. Get is NOT persisted.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command {
    Set { key: String, val: String },
    Remove { key: String },
}

impl Command {
    pub fn key(&self) -> &str {
        match self {
            Command::Set { key, .. } => key,
            Command::Remove { key } => key,
        }
    }
}
