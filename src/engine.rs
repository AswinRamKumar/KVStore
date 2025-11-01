use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write, BufReader, BufWriter},
    path::PathBuf,
};

use crate::cmd::Command;
use crate::error::{Result, KvError};

// The main key-value store engine
pub struct KvStore {
    index: HashMap<String, u64>, // key → offset in log file
    writer: BufWriter<File>,     // buffered file writer
    reader: BufReader<File>,     // reader for log replay
    path: PathBuf,               // log file path
}

impl KvStore {
    // Opens an existing store or creates a new one
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let path = path.into();
        let log_path = path.join("store.log");

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&log_path)?;

        let mut store = KvStore {
            index: HashMap::new(),
            writer: BufWriter::new(file.try_clone()?),
            reader: BufReader::new(file),
            path: log_path,
        };

        store.load()?; // rebuild index
        Ok(store)
    }

    // Load log and rebuild in-memory index
    fn load(&mut self) -> Result<()> {
        let mut pos = self.reader.seek(SeekFrom::Start(0))?;
        let stream = serde_json::Deserializer::from_reader(&mut self.reader).into_iter::<Command>();

        for cmd in stream {
            let new_pos = self.reader.stream_position()?;
            match cmd? {
                Command::Set { key, .. } => {
                    self.index.insert(key, pos);
                }
                Command::Remove { key } => {
                    self.index.remove(&key);
                }
            }
            pos = new_pos;
        }
        Ok(())
    }

    // Set a key to a value
    pub fn set(&mut self, key: String, val: String) -> Result<()> {
        let cmd = Command::Set { key: key.clone(), val };
        let json = serde_json::to_vec(&cmd)?;
        let offset = self.writer.stream_position()?;
        self.writer.write_all(&json)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        self.index.insert(key, offset);
        Ok(())
    }

    // Get the value for a key
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        if let Some(&offset) = self.index.get(&key) {
            self.reader.seek(SeekFrom::Start(offset))?;
            let mut line = String::new();
            use std::io::BufRead;
            self.reader.read_line(&mut line)?;
            if let Ok(Command::Set { val, .. }) = serde_json::from_str::<Command>(&line) {
                return Ok(Some(val));
            }
        }
        Ok(None)
    }

    // Remove a key
    pub fn remove(&mut self, key: String) -> Result<()> {
        if self.index.contains_key(&key) {
            let cmd = Command::Remove { key: key.clone() };
            let json = serde_json::to_vec(&cmd)?;
            self.writer.write_all(&json)?;
            self.writer.write_all(b"\n")?;
            self.writer.flush()?;
            self.index.remove(&key);
            Ok(())
        } else {
            Err(KvError::KeyNotFound)
        }
    }
}
