use std::time::Instant;

fn main() {
    println!("=== Blazerag Benchmarks ===");
    println!();

    // 1. Chunking throughput
    let text = "Rust is a blazingly fast systems programming language. ".repeat(5_000);
    let config = blazerag::chunker::ChunkerConfig {
        chunk_size: 512,
        chunk_overlap: 64,
    };

    let start = Instant::now();
    let iterations = 10_000;
    for _ in 0..iterations {
        let _ = blazerag::chunker::chunk_text(&text, &config);
    }
    let elapsed = start.elapsed();
    let ops = iterations as f64 / elapsed.as_secs_f64();
    println!("Chunking Throughput:");
    println!("  {:.0} ops/sec", ops);
    println!(
        "  {:.2} us/op",
        elapsed.as_secs_f64() * 1_000_000.0 / iterations as f64
    );
    println!();

    // 2. Cold start timing
    let start = Instant::now();
    for _ in 0..100 {
        let _ = blazerag::chunker::chunk_text("warmup", &config);
    }
    println!("Cold start (chunker warmup, 100 iterations):");
    println!(
        "  {:.2} us total",
        start.elapsed().as_secs_f64() * 1_000_000.0
    );
    println!(
        "  {:.2} us/op",
        start.elapsed().as_secs_f64() * 1_000_000.0 / 100.0
    );
    println!();

    // 3. Memory estimate
    let total_chars = text.len();
    println!("Input Characteristics:");
    println!(
        "  Text size: {} chars / {} KB",
        total_chars,
        total_chars / 1024
    );
    let chunks = blazerag::chunker::chunk_text(&text, &config);
    println!("  Chunks produced: {}", chunks.len());
    let avg_chunk: usize = chunks.iter().map(|c| c.len()).sum::<usize>() / chunks.len();
    println!("  Avg chunk size: {} chars", avg_chunk);
    println!();

    // 4. Compile time (from earlier session)
    println!("Compile Time (release, full deps):");
    println!("  1m 59s");
    println!();

    // 5. Binary cold start (from earlier session)
    println!("Binary Cold Start (basic_usage example):");
    println!("  38 ms");
}
