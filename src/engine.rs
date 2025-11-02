use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, BufRead, Seek, SeekFrom, Write},
    path::PathBuf,
};

use crate::cmd::Command;
use crate::error::{Result, KvError};

pub struct KvStore {
    index: HashMap<String, u64>, // key → offset in log file
    writer: BufWriter<File>,     // buffered writer
    reader: BufReader<File>,     // buffered reader
    path: PathBuf,               // log file path
    uncompacted: u64,            // bytes that can be compacted
    threshold: u64,              // trigger limit
}

impl KvStore {
    /// Opens or creates a key-value store at given path
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let path = path.into();
        let log_path = path.join("store.log");

        // Create/open file with read + append
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&log_path)?;

        let mut store = KvStore {
            index: HashMap::new(),
            writer: BufWriter::new(file.try_clone()?),
            reader: BufReader::new(File::open(&log_path)?),
            path: log_path,
            uncompacted: 0,
            threshold: 1024 * 1024, // 1MB compaction threshold
        };

        store.load()?;
        Ok(store)
    }

    /// Rebuilds index from existing log
    fn load(&mut self) -> Result<()> {
        self.reader.seek(SeekFrom::Start(0))?;
        let mut pos = 0;

        let mut line = String::new();
        while self.reader.read_line(&mut line)? > 0 {
            let new_pos = self.reader.stream_position()?;
            let cmd: Command = serde_json::from_str(&line.trim())?;

            match cmd {
                Command::Set { key, .. } => {
                    if let Some(&old) = self.index.get(&key) {
                        self.uncompacted += new_pos - old;
                    }
                    self.index.insert(key, pos);
                }
                Command::Remove { key } => {
                    if let Some(&old) = self.index.get(&key) {
                        self.uncompacted += new_pos - old;
                    }
                    self.index.remove(&key);
                }
            }

            line.clear();
            pos = new_pos;
        }
        Ok(())
    }

    /// Write new command to log and update index
    pub fn set(&mut self, key: String, val: String) -> Result<()> {
        let cmd = Command::Set { key: key.clone(), val };
        let json = serde_json::to_vec(&cmd)?;
        let offset = self.writer.stream_position()?;
        self.writer.write_all(&json)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;

        if let Some(&old) = self.index.get(&key) {
            self.uncompacted += offset - old;
        }
        self.index.insert(key, offset);

        if self.uncompacted > self.threshold {
            self.compact()?;
        }

        Ok(())
    }

    /// Retrieve value for key (if present)
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        if let Some(&offset) = self.index.get(&key) {
            self.reader.seek(SeekFrom::Start(offset))?;
            let mut line = String::new();
            self.reader.read_line(&mut line)?;
            if let Ok(Command::Set { val, .. }) = serde_json::from_str::<Command>(&line.trim()) {
                return Ok(Some(val));
            }
        }
        Ok(None)
    }

    /// Remove a key (if exists)
    pub fn remove(&mut self, key: String) -> Result<()> {
        if !self.index.contains_key(&key) {
            return Err(KvError::KeyNotFound);
        }

        let cmd = Command::Remove { key: key.clone() };
        let json = serde_json::to_vec(&cmd)?;
        self.writer.write_all(&json)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;

        if let Some(&old) = self.index.get(&key) {
            self.uncompacted += old;
        }

        self.index.remove(&key);

        if self.uncompacted > self.threshold {
            self.compact()?;
        }

        Ok(())
    }

    /// Compact log: keep only current valid entries
    fn compact(&mut self) -> Result<()> {
        let tmp_path = self.path.with_extension("compact");
        let mut tmp_writer = BufWriter::new(File::create(&tmp_path)?);
        let mut new_index = HashMap::new();
        let mut pos = 0;

        for (key, &offset) in &self.index {
            self.reader.seek(SeekFrom::Start(offset))?;
            let mut line = String::new();
            self.reader.read_line(&mut line)?;
            let len = line.len() as u64;
            tmp_writer.write_all(line.as_bytes())?;
            new_index.insert(key.clone(), pos);
            pos += len;
        }

        tmp_writer.flush()?;
        std::fs::rename(&tmp_path, &self.path)?;

        self.writer = BufWriter::new(OpenOptions::new().append(true).open(&self.path)?);
        self.reader = BufReader::new(File::open(&self.path)?);
        self.index = new_index;
        self.uncompacted = 0;

        Ok(())
    }
}
