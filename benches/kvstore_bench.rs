use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use kvstore::KvStore;
use rand::{Rng, distributions::Alphanumeric};
use tempfile::TempDir;

fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn bench_sequential_writes(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_writes");
    
    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let temp_dir = TempDir::new().unwrap();
                    let mut store = KvStore::open(temp_dir.path()).unwrap();
                    store.set_compaction_threshold(100 * 1024 * 1024); // 100MB to avoid compaction
                    (store, temp_dir)
                },
                |(mut store, _temp_dir)| {
                    for i in 0..size {
                        let key = format!("key_{}", i);
                        let value = format!("value_{}", i);
                        store.set(key, value).unwrap();
                    }
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_random_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_reads");
    
    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let temp_dir = TempDir::new().unwrap();
                    let mut store = KvStore::open(temp_dir.path()).unwrap();
                    
                    // Populate store
                    for i in 0..size {
                        let key = format!("key_{}", i);
                        let value = format!("value_{}", i);
                        store.set(key, value).unwrap();
                    }
                    
                    (store, temp_dir)
                },
                |(store, _temp_dir)| {
                    for i in 0..size {
                        let key = format!("key_{}", i);
                        black_box(store.get(&key).unwrap());
                    }
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_overwrites(c: &mut Criterion) {
    let mut group = c.benchmark_group("overwrites");
    
    for iterations in [100, 1000] {
        group.throughput(Throughput::Elements(iterations as u64));
        group.bench_with_input(BenchmarkId::from_parameter(iterations), &iterations, |b, &iterations| {
            b.iter_batched(
                || {
                    let temp_dir = TempDir::new().unwrap();
                    let mut store = KvStore::open(temp_dir.path()).unwrap();
                    store.set_compaction_threshold(100 * 1024 * 1024);
                    (store, temp_dir)
                },
                |(mut store, _temp_dir)| {
                    // Overwrite same key multiple times
                    for i in 0..iterations {
                        store.set("same_key".to_string(), format!("value_{}", i)).unwrap();
                    }
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_compaction(c: &mut Criterion) {
    let mut group = c.benchmark_group("compaction");
    group.sample_size(10); // Compaction is slow, fewer samples
    
    for size in [1000, 5000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let temp_dir = TempDir::new().unwrap();
                    let mut store = KvStore::open(temp_dir.path()).unwrap();
                    store.set_compaction_threshold(100 * 1024 * 1024); // Disable auto-compaction
                    
                    // Create lots of stale data by overwriting
                    for _ in 0..10 {
                        for i in 0..size {
                            let key = format!("key_{}", i);
                            let value = random_string(100);
                            store.set(key, value).unwrap();
                        }
                    }
                    
                    (store, temp_dir)
                },
                |(mut store, _temp_dir)| {
                    // Trigger manual compaction
                    store.set_compaction_threshold(1); // Force compaction
                    store.set("trigger".to_string(), "compaction".to_string()).unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");
    
    group.bench_function("50_write_50_read", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let mut store = KvStore::open(temp_dir.path()).unwrap();
                store.set_compaction_threshold(100 * 1024 * 1024);
                
                // Pre-populate
                for i in 0..1000 {
                    store.set(format!("key_{}", i), format!("value_{}", i)).unwrap();
                }
                
                (store, temp_dir)
            },
            |(mut store, _temp_dir)| {
                for i in 0..1000 {
                    if i % 2 == 0 {
                        // Write
                        store.set(format!("key_{}", i), format!("new_value_{}", i)).unwrap();
                    } else {
                        // Read
                        black_box(store.get(&format!("key_{}", i)).unwrap());
                    }
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_sequential_writes,
    bench_random_reads,
    bench_overwrites,
    bench_compaction,
    bench_mixed_workload
);
criterion_main!(benches);
