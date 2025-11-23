use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "kvstore")]
#[command(version, about = "A log-structured key-value store")]
pub struct Cli {
    #[arg(short, long, default_value = "./data", global = true)]
    pub data_dir: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Set a key-value pair
    Set { key: String, value: String },
    
    /// Get the value of a key
    Get { key: String },
    
    /// Remove a key
    Rm { key: String },
}
