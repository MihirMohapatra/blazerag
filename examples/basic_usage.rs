/// Basic usage example for Blazerag
///
/// Run with: cargo run --example basic_usage
/// Make sure Qdrant is running and .env is configured.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This example shows how to use the Blazerag client API
    // In production, use the HTTP endpoints instead.

    println!("Blazerag Basic Usage Example");
    println!("===========================");
    println!();
    println!("1. Start Qdrant: docker compose up -d qdrant");
    println!("2. Start Blazerag: cargo run");
    println!("3. Ingest a document:");
    println!("   curl -X POST http://localhost:3000/ingest \\");
    println!("     -H 'Content-Type: application/json' \\");
    println!("     -d '{{\"text\": \"Blazerag is a fast RAG server in Rust.\"}}'");
    println!("4. Query:");
    println!("   curl -X POST http://localhost:3000/query \\");
    println!("     -H 'Content-Type: application/json' \\");
    println!("     -d '{{\"question\": \"What is Blazerag?\"}}'");

    Ok(())
}
