use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write},
    path::PathBuf,
};

use crate::cmd::Command;
use crate::error::{KvError, Result};

#[derive(Debug, Clone)]
struct LogPointer {
    offset: u64,
    len: u64,
}

/// Log-structured key-value store (Bitcask model).
/// Provides O(1) reads/writes with automatic compaction.
pub struct KvStore {
    index: HashMap<String, LogPointer>,
    writer: BufWriter<File>,
    log_path: PathBuf,
    dir_path: PathBuf,
    uncompacted: u64,
    threshold: u64,
}

impl KvStore {
    /// Opens or creates a KvStore at the given directory path.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let dir_path = path.into();
        std::fs::create_dir_all(&dir_path)?;
        
        let log_path = dir_path.join("store.log");

        let writer = BufWriter::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)?
        );

        let mut store = KvStore {
            index: HashMap::new(),
            writer,
            log_path: log_path.clone(),
            dir_path,
            uncompacted: 0,
            threshold: 1024 * 1024,
        };

        store.rebuild_index()?;
        Ok(store)
    }

    pub fn set_compaction_threshold(&mut self, threshold: u64) {
        self.threshold = threshold;
    }

    fn rebuild_index(&mut self) -> Result<()> {
        let mut reader = BufReader::new(File::open(&self.log_path)?);
        let mut pos = 0u64;
        let mut line = String::new();
        let mut total_bytes = 0u64;
        let mut live_bytes = 0u64;

        while reader.read_line(&mut line)? > 0 {
            let len = line.len() as u64;
            
            match serde_json::from_str::<Command>(line.trim()) {
                Ok(cmd) => {
                    match cmd {
                        Command::Set { key, .. } => {
                            if let Some(old_ptr) = self.index.get(&key) {
                                live_bytes -= old_ptr.len;
                            }
                            self.index.insert(key, LogPointer { offset: pos, len });
                            live_bytes += len;
                        }
                        Command::Remove { key } => {
                            if let Some(old_ptr) = self.index.remove(&key) {
                                live_bytes -= old_ptr.len;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: skipping corrupted log entry at offset {}: {}", pos, e);
                }
            }

            total_bytes += len;
            pos += len;
            line.clear();
        }

        self.uncompacted = total_bytes.saturating_sub(live_bytes);
        Ok(())
    }

    pub fn set(&mut self, key: String, val: String) -> Result<()> {
        Self::validate_key(&key)?;
        
        let cmd = Command::Set { key: key.clone(), val };
        let offset = self.append_command(&cmd)?;
        
        if let Some(old_ptr) = self.index.get(&key) {
            self.uncompacted += old_ptr.len;
        }
        
        self.index.insert(key, LogPointer { offset: offset.0, len: offset.1 });
        self.maybe_compact()?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        if let Some(ptr) = self.index.get(key) {
            let mut reader = BufReader::new(File::open(&self.log_path)?);
            reader.seek(SeekFrom::Start(ptr.offset))?;
            
            let mut line = String::new();
            reader.read_line(&mut line)?;
            
            match serde_json::from_str::<Command>(line.trim()) {
                Ok(Command::Set { val, .. }) => Ok(Some(val)),
                Ok(Command::Remove { .. }) => Err(KvError::LogCorruption(ptr.offset)),
                Err(_) => Err(KvError::LogCorruption(ptr.offset)),
            }
        } else {
            Ok(None)
        }
    }

    pub fn remove(&mut self, key: String) -> Result<()> {
        if !self.index.contains_key(&key) {
            return Err(KvError::KeyNotFound);
        }

        let cmd = Command::Remove { key: key.clone() };
        let offset = self.append_command(&cmd)?;
        
        if let Some(old_ptr) = self.index.remove(&key) {
            self.uncompacted += old_ptr.len + offset.1;
        }

        self.maybe_compact()?;
        Ok(())
    }

    fn validate_key(key: &str) -> Result<()> {
        if key.is_empty() {
            return Err(KvError::InvalidKey("Key cannot be empty".to_string()));
        }
        Ok(())
    }

    fn append_command(&mut self, cmd: &Command) -> Result<(u64, u64)> {
        let offset = self.writer.stream_position()?;
        let mut json = serde_json::to_vec(cmd)?;
        json.push(b'\n');
        
        let len = json.len() as u64;
        self.writer.write_all(&json)?;
        self.writer.flush()?;
        
        Ok((offset, len))
    }

    fn maybe_compact(&mut self) -> Result<()> {
        if self.uncompacted > self.threshold {
            self.compact()?;
        }
        Ok(())
    }

    fn compact(&mut self) -> Result<()> {
        let compact_path = self.dir_path.join("store.log.compact");
        
        let mut tmp_writer = BufWriter::new(
            File::create(&compact_path)
                .map_err(|e| KvError::CompactionFailed(e.to_string()))?
        );
        
        let mut new_index = HashMap::new();
        let mut reader = BufReader::new(File::open(&self.log_path)?);
        let mut pos = 0u64;

        for (key, ptr) in &self.index {
            reader.seek(SeekFrom::Start(ptr.offset))?;
            let mut line = String::new();
            reader.read_line(&mut line)?;
            
            let len = line.len() as u64;
            tmp_writer.write_all(line.as_bytes())?;
            new_index.insert(key.clone(), LogPointer { offset: pos, len });
            pos += len;
        }

        tmp_writer.flush()?;
        drop(tmp_writer);

        std::fs::rename(&compact_path, &self.log_path)
            .map_err(|e| KvError::CompactionFailed(e.to_string()))?;

        self.writer = BufWriter::new(
            OpenOptions::new()
                .append(true)
                .open(&self.log_path)?
        );
        
        self.index = new_index;
        self.uncompacted = 0;

        Ok(())
    }
}
