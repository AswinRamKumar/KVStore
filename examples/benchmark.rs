use kvstore::KvStore;
use std::time::Instant;

fn main() -> kvstore::Result<()> {
    println!("=== KvStore Performance Benchmark ===\n");
    
    // Clean start
    let _ = std::fs::remove_dir_all("./bench_data");
    let mut store = KvStore::open("./bench_data")?;
    store.set_compaction_threshold(100 * 1024 * 1024); // 100MB
    
    // Benchmark 1: Sequential Writes
    println!("1. Sequential Writes (10,000 operations)");
    let start = Instant::now();
    for i in 0..10_000 {
        store.set(format!("key_{}", i), format!("value_{}", i))?;
    }
    let duration = start.elapsed();
    let ops_per_sec = 10_000.0 / duration.as_secs_f64();
    println!("   Time: {:?}", duration);
    println!("   Throughput: {:.0} ops/sec\n", ops_per_sec);
    
    // Benchmark 2: Random Reads
    println!("2. Random Reads (10,000 operations)");
    let start = Instant::now();
    for i in 0..10_000 {
        let _ = store.get(&format!("key_{}", i))?;
    }
    let duration = start.elapsed();
    let ops_per_sec = 10_000.0 / duration.as_secs_f64();
    println!("   Time: {:?}", duration);
    println!("   Throughput: {:.0} ops/sec\n", ops_per_sec);
    
    // Benchmark 3: Overwrites (creates stale data)
    println!("3. Overwrites - Same Key (5,000 operations)");
    let start = Instant::now();
    for i in 0..5_000 {
        store.set("hot_key".to_string(), format!("value_{}", i))?;
    }
    let duration = start.elapsed();
    let ops_per_sec = 5_000.0 / duration.as_secs_f64();
    println!("   Time: {:?}", duration);
    println!("   Throughput: {:.0} ops/sec\n", ops_per_sec);
    
    // Benchmark 4: Compaction
    println!("4. Compaction");
    // Create lots of stale data
    for round in 0..5 {
        for i in 0..1_000 {
            store.set(format!("key_{}", i), format!("value_{}_{}", round, i))?;
        }
    }
    
    let log_size_before = std::fs::metadata("./bench_data/store.log")?.len();
    println!("   Log size before: {} bytes", log_size_before);
    
    let start = Instant::now();
    store.set_compaction_threshold(1); // Force compaction
    store.set("trigger".to_string(), "compaction".to_string())?;
    let duration = start.elapsed();
    
    let log_size_after = std::fs::metadata("./bench_data/store.log")?.len();
    println!("   Log size after: {} bytes", log_size_after);
    println!("   Saved: {} bytes ({:.1}%)", 
        log_size_before - log_size_after,
        (1.0 - log_size_after as f64 / log_size_before as f64) * 100.0
    );
    println!("   Time: {:?}", duration);
    
    if duration.as_secs_f64() > 0.0 {
        let throughput = log_size_before as f64 / duration.as_secs_f64() / 1_000_000.0;
        println!("   Throughput: {:.2} MB/sec\n", throughput);
    }
    
    // Benchmark 5: Mixed Workload
    println!("5. Mixed Workload - 50% Reads, 50% Writes (10,000 operations)");
    store.set_compaction_threshold(100 * 1024 * 1024); // Disable compaction
    let start = Instant::now();
    for i in 0..10_000 {
        if i % 2 == 0 {
            store.set(format!("key_{}", i % 1000), format!("value_{}", i))?;
        } else {
            let _ = store.get(&format!("key_{}", i % 1000))?;
        }
    }
    let duration = start.elapsed();
    let ops_per_sec = 10_000.0 / duration.as_secs_f64();
    println!("   Time: {:?}", duration);
    println!("   Throughput: {:.0} ops/sec\n", ops_per_sec);
    
    // Cleanup
    std::fs::remove_dir_all("./bench_data")?;
    
    println!("=== Benchmark Complete ===");
    
    Ok(())
}
