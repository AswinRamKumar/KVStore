# Visual Example: Why LogPointer Matters

## Scenario: User updates their name 3 times

### Log File Contents:
```
Offset  Length  Content
------  ------  -------
0       45      {"Set":{"key":"user","val":"Alice"}}
45      43      {"Set":{"key":"user","val":"Bob"}}
88      47      {"Set":{"key":"user","val":"Charlie"}}
```

Total file size: 135 bytes
Live data: Only the last entry (47 bytes)
Stale data: 88 bytes (should trigger compaction)

---

## OLD CODE (WRONG):

### Index Structure:
```rust
HashMap<String, u64>
{
    "user" => 88  // Only stores offset
}
```

### Stale Calculation During rebuild_index():
```rust
// First Set (offset 0)
index.insert("user", 0);
uncompacted = 0;

// Second Set (offset 45)
if let Some(&old) = index.get("user") {
    uncompacted += 45 - 0;  // = 45 ❌ WRONG!
}
index.insert("user", 45);

// Third Set (offset 88)
if let Some(&old) = index.get("user") {
    uncompacted += 88 - 45;  // = 43 ❌ WRONG!
}
index.insert("user", 88);

// Final: uncompacted = 45 + 43 = 88
```

**Why it's wrong:** `88 - 45 = 43` is the DISTANCE between entries, not the SIZE of the stale entry!

---

## NEW CODE (CORRECT):

### Index Structure:
```rust
HashMap<String, LogPointer>
{
    "user" => LogPointer { offset: 88, len: 47 }
}
```

### Stale Calculation During rebuild_index():
```rust
let mut total_bytes = 0;
let mut live_bytes = 0;

// First Set (offset 0, length 45)
index.insert("user", LogPointer { offset: 0, len: 45 });
live_bytes += 45;
total_bytes += 45;

// Second Set (offset 45, length 43)
if let Some(old_ptr) = index.get("user") {
    live_bytes -= old_ptr.len;  // -= 45 (first entry now stale)
}
index.insert("user", LogPointer { offset: 45, len: 43 });
live_bytes += 43;
total_bytes += 43;

// Third Set (offset 88, length 47)
if let Some(old_ptr) = index.get("user") {
    live_bytes -= old_ptr.len;  // -= 43 (second entry now stale)
}
index.insert("user", LogPointer { offset: 88, len: 47 });
live_bytes += 47;
total_bytes += 47;

// Final calculation:
total_bytes = 135
live_bytes = 47
uncompacted = 135 - 47 = 88 ✅ CORRECT!
```

---

## After Compaction:

### New Log File:
```
Offset  Length  Content
------  ------  -------
0       47      {"Set":{"key":"user","val":"Charlie"}}
```

### New Index:
```rust
{
    "user" => LogPointer { offset: 0, len: 47 }
}
```

### Stats:
- File size: 47 bytes (was 135)
- Saved: 88 bytes (65% reduction!)
- uncompacted: 0

---

## Real-World Impact

### Scenario: 1000 updates to same key

**OLD CODE:**
- Calculates stale bytes incorrectly
- Might trigger compaction too early or too late
- Wastes CPU on unnecessary compaction
- Or delays compaction causing huge log files

**NEW CODE:**
- Accurate tracking
- Compacts exactly when threshold is reached
- Optimal performance

### Example with 1MB threshold:

```
OLD: Might compact at 500KB (too early) or 2MB (too late)
NEW: Compacts at exactly 1MB of stale data
```

---

## The Key Insight

**You can't calculate entry size from offsets alone!**

```
Entry 1: offset=0,  next_offset=45  → size could be 45
Entry 2: offset=45, next_offset=88  → size could be 43

BUT what if there's corruption or variable-length encoding?
The only way to know is to MEASURE when you read it!
```

That's why we store `len` in `LogPointer` - we measure it when we read the line, then store it for accurate tracking.
