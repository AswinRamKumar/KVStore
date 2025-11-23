use clap::Parser;
use kvstore::{cli::*, KvStore, Result};
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let mut store = KvStore::open(&cli.data_dir)?;

    match cli.command {
        Commands::Set { key, value } => {
            store.set(key, value)?;
            // Silent success (matches Redis/memcached behavior)
        }
        
        Commands::Get { key } => {
            match store.get(&key)? {
                Some(value) => println!("{}", value),
                None => {
                    eprintln!("Key not found");
                    process::exit(1);
                }
            }
        }
        
        Commands::Rm { key } => {
            store.remove(key)?;
            // Silent success
        }
    }

    Ok(())
}
