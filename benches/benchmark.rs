use std::time::Instant;

fn main() {
    println!("Blazerag Benchmarks");
    println!("==================");
    println!();

    // Chunking benchmark
    let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(1000);
    let config = blazerag::chunker::ChunkerConfig {
        chunk_size: 256,
        chunk_overlap: 32,
    };

    let start = Instant::now();
    let iterations = 1000;
    for _ in 0..iterations {
        let _ = blazerag::chunker::chunk_text(&text, &config);
    }
    let elapsed = start.elapsed();
    println!(
        "Chunking {:.1} ops/sec",
        iterations as f64 / elapsed.as_secs_f64()
    );

    // Cold start benchmark (if model is present)
    println!();
    println!("Note: For actual ONNX embedding and Qdrant benchmarks,");
    println!("run the full integration test suite.");
}
