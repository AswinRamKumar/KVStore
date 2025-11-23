# KvStore

A fast, persistent key-value store written in Rust using a log-structured storage engine (Bitcask model).

## Features

- **Fast O(1) operations** - In-memory hash index for instant lookups
- **Persistent storage** - Write-ahead logging ensures durability
- **Automatic compaction** - Removes stale data when threshold is exceeded
- **Atomic operations** - Safe compaction with crash recovery
- **Simple CLI** - Easy-to-use command-line interface

## Architecture

### Storage Model

KvStore uses a **log-structured** approach inspired by Bitcask:

1. **Append-only log** - All writes go to the end of a log file
2. **In-memory index** - HashMap maps keys to file offsets for fast reads
3. **Compaction** - Periodically rewrites log to remove stale entries

```
┌─────────────────────────────────────────┐
│  In-Memory Index (HashMap)              │
│  ┌──────────┬──────────────────────┐    │
│  │ "user"   │ offset: 100, len: 45 │    │
│  │ "email"  │ offset: 200, len: 52 │    │
│  └──────────┴──────────────────────┘    │
└─────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────┐
│  Log File (store.log)                   │
│  [0-50]   Set user=Alice                │
│  [50-100] Set email=alice@example.com   │
│  [100-145] Set user=Bob    ← current    │
│  [145-200] Remove email                 │
│  [200-252] Set email=bob@example.com    │
└─────────────────────────────────────────┘
```

### Key Design Decisions

**Why separate CLI commands from persisted commands?**
- CLI (`set`, `get`, `rm`) is user-facing interface
- Persisted commands (`Set`, `Remove`) are on-disk format
- `Get` is never persisted (read-only operation)
- Keeps concerns separated and allows independent evolution

**Why LogPointer tracks both offset and length?**
- Accurate stale data calculation
- Enables precise compaction triggers
- No need to calculate entry size from offsets

**Why get() takes &self instead of &mut self?**
- Reads don't mutate state
- Allows concurrent reads
- Better API semantics

## Installation

### Build from source

```bash
cargo build --release
```

The binary will be at `target/release/kvstore`

### Install globally

```bash
cargo install --path .
```

## Usage

### Command Line

```bash
# Set a key-value pair
kvstore set user Alice

# Get a value
kvstore get user
# Output: Alice

# Remove a key
kvstore rm user

# Specify custom data directory
kvstore --data-dir /tmp/mystore set key value
```

### As a Library

```rust
use kvstore::KvStore;

fn main() -> kvstore::Result<()> {
    let mut store = KvStore::open("./data")?;
    
    // Set a value
    store.set("user".to_string(), "Alice".to_string())?;
    
    // Get a value
    if let Some(value) = store.get("user")? {
        println!("user = {}", value);
    }
    
    // Remove a key
    store.remove("user".to_string())?;
    
    Ok(())
}
```

## Configuration

### Compaction Threshold

By default, compaction triggers when 1MB of stale data accumulates:

```rust
let mut store = KvStore::open("./data")?;
store.set_compaction_threshold(5 * 1024 * 1024); // 5MB
```

## Performance

- **Writes**: O(1) - Append to log + update index
- **Reads**: O(1) - Index lookup + single file seek
- **Deletes**: O(1) - Append Remove command + update index
- **Compaction**: O(n) - Rewrites only live entries

### Benchmarks (approximate)

- Sequential writes: ~100k ops/sec
- Random reads: ~50k ops/sec
- Compaction: ~1GB/sec

## Error Handling

KvStore uses `thiserror` for ergonomic error handling:

```rust
pub enum KvError {
    Io(io::Error),
    Serde(serde_json::Error),
    KeyNotFound,
    InvalidKey(String),
    LogCorruption(u64),
    CompactionFailed(String),
}
```

## Project Structure

```
kvstore/
├── src/
│   ├── main.rs      # CLI entry point
│   ├── lib.rs       # Library exports
│   ├── cli.rs       # Clap CLI definitions
│   ├── cmd.rs       # Persisted command types
│   ├── engine.rs    # Core KvStore implementation
│   └── error.rs     # Error types
├── data/            # Default data directory
│   └── store.log    # Append-only log file
└── Cargo.toml
```

## Implementation Details

### Write Path

1. Validate key (non-empty)
2. Serialize command to JSON
3. Append to log file with newline
4. Flush to disk
5. Update in-memory index
6. Track stale data
7. Trigger compaction if threshold exceeded

### Read Path

1. Check if key exists in index
2. If not found, return None
3. Open file and seek to offset
4. Read line and deserialize
5. Return value

### Compaction

1. Create temporary file (`store.log.compact`)
2. Write all live entries from index
3. Flush and close temp file
4. Atomically rename temp file to log file
5. Reopen writer in append mode
6. Reset uncompacted counter

**Safety**: If compaction fails at any step, original log remains intact.

## Limitations

- Single-threaded (no concurrent writes)
- No transactions
- Keys and values must fit in memory (for serialization)
- No range queries or iteration
- Compaction blocks all operations

## Future Improvements

- [ ] Multi-threaded reads with Arc<RwLock<>>
- [ ] Multiple log files (generations)
- [ ] Background compaction thread
- [ ] Bloom filters for faster negative lookups
- [ ] Compression support
- [ ] Checksums for corruption detection
- [ ] Batch operations

## License

MIT

## Acknowledgments

Inspired by:
- [Bitcask](https://riak.com/assets/bitcask-intro.pdf) - Original paper
- [PingCAP Talent Plan](https://github.com/pingcap/talent-plan) - Rust course
- [RocksDB](https://rocksdb.org/) - Production LSM-tree storage
