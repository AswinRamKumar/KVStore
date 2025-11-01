use serde::{Serialize, Deserialize};

// Represents a database command that can be persisted to disk.
#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
   
    Set { key: String, val: String }, // Set a key-value pair
    Remove { key: String },// Remove a key from the store
}
