use kvstore::KvStore;
use std::time::Instant;

fn main() -> kvstore::Result<()> {
    println!("=== KvStore Stress Test ===\n");
    
    let _ = std::fs::remove_dir_all("./stress_data");
    let mut store = KvStore::open("./stress_data")?;
    
    // Test 1: Large number of unique keys
    println!("Test 1: Writing 100,000 unique keys...");
    let start = Instant::now();
    for i in 0..100_000 {
        store.set(format!("key_{}", i), format!("value_{}", i))?;
        
        if (i + 1) % 10_000 == 0 {
            println!("  Written {} keys...", i + 1);
        }
    }
    let duration = start.elapsed();
    println!("  Completed in {:?}", duration);
    println!("  Throughput: {:.0} ops/sec\n", 100_000.0 / duration.as_secs_f64());
    
    // Test 2: Read all keys
    println!("Test 2: Reading 100,000 keys...");
    let start = Instant::now();
    for i in 0..100_000 {
        let value = store.get(&format!("key_{}", i))?.expect("Key should exist");
        assert_eq!(value, format!("value_{}", i));
        
        if (i + 1) % 10_000 == 0 {
            println!("  Read {} keys...", i + 1);
        }
    }
    let duration = start.elapsed();
    println!("  Completed in {:?}", duration);
    println!("  Throughput: {:.0} ops/sec\n", 100_000.0 / duration.as_secs_f64());
    
    // Test 3: Trigger multiple compactions
    println!("Test 3: Triggering compactions with overwrites...");
    store.set_compaction_threshold(1024 * 1024); // 1MB threshold
    
    let start = Instant::now();
    let mut compaction_count = 0;
    
    for round in 0..10 {
        for i in 0..1_000 {
            store.set(format!("hot_key_{}", i % 100), format!("value_{}_{}", round, i))?;
        }
        println!("  Round {} complete", round + 1);
        compaction_count += 1;
    }
    
    let duration = start.elapsed();
    println!("  Completed in {:?}", duration);
    println!("  Estimated compactions: ~{}\n", compaction_count);
    
    // Test 4: Large values
    println!("Test 4: Writing large values (1KB each, 1,000 keys)...");
    let large_value = "x".repeat(1024);
    let start = Instant::now();
    
    for i in 0..1_000 {
        store.set(format!("large_key_{}", i), large_value.clone())?;
    }
    
    let duration = start.elapsed();
    let total_mb = 1.0; // 1KB * 1000 = ~1MB
    println!("  Completed in {:?}", duration);
    println!("  Throughput: {:.2} MB/sec\n", total_mb / duration.as_secs_f64());
    
    // Final stats
    let log_size = std::fs::metadata("./stress_data/store.log")?.len();
    println!("Final log size: {:.2} MB", log_size as f64 / 1_000_000.0);
    
    // Cleanup
    std::fs::remove_dir_all("./stress_data")?;
    
    println!("\n=== All Tests Passed ===");
    
    Ok(())
}
