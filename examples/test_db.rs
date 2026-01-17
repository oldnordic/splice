use splice::graph::CodeGraph;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = Path::new("/tmp/test_splice.db");
    let _ = std::fs::remove_file(db_path); // Clean up

    let mut graph = CodeGraph::open(db_path)?;
    println!("Database created successfully at {:?}", db_path);

    // Check file size
    let metadata = std::fs::metadata(db_path)?;
    println!("Database file size: {} bytes", metadata.len());

    // Test symbol insertion
    let node_id = graph.store_symbol_with_file_and_language(
        Path::new("test.rs"),
        "test_function",
        "function",
        splice::symbol::Language::Rust,
        0,
        100,
    )?;
    println!("Symbol inserted: {:?}", node_id);

    // Test retrieval
    let span = graph.get_span(node_id)?;
    println!("Span retrieved: {:?}", span);

    println!("\n✅ All Native V2 backend operations successful!");

    Ok(())
}
