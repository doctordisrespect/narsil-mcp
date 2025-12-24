//! Test program for remote repository functionality

use narsil_mcp::remote::{RemoteRepo, RemoteRepoManager};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("Testing Remote Repository Functionality\n");

    // Test URL parsing
    println!("=== Testing URL Parsing ===");
    let repo1 = RemoteRepo::from_url("github.com/rust-lang/rust")?;
    println!("Parsed: {} -> {}/{}", repo1.url, repo1.owner, repo1.repo);

    let repo2 = RemoteRepo::from_url("https://github.com/microsoft/vscode/tree/main")?;
    println!(
        "Parsed with branch: {} -> {}/{} (branch: {:?})",
        repo2.url, repo2.owner, repo2.repo, repo2.branch
    );

    // Test remote manager initialization
    println!("\n=== Testing Remote Manager ===");
    let manager = RemoteRepoManager::new()?;
    println!("Remote manager created successfully");
    println!(
        "GITHUB_TOKEN: {}",
        if std::env::var("GITHUB_TOKEN").is_ok() {
            "Set"
        } else {
            "Not set"
        }
    );

    // Test listing files (requires GitHub API access)
    if std::env::var("GITHUB_TOKEN").is_ok() {
        println!("\n=== Testing File Listing ===");
        let test_repo = RemoteRepo::from_url("github.com/rust-lang/cargo")?;

        match manager.list_files(&test_repo, Some("src")).await {
            Ok(files) => {
                println!("Found {} files in src/:", files.len());
                for file in files.iter().take(5) {
                    println!("  - {}", file);
                }
                if files.len() > 5 {
                    println!("  ... and {} more", files.len() - 5);
                }
            }
            Err(e) => {
                println!("Failed to list files: {}", e);
            }
        }
    } else {
        println!("\n=== Skipping API tests (no GITHUB_TOKEN) ===");
        println!("Set GITHUB_TOKEN to test GitHub API functionality");
    }

    println!("\n=== All tests completed ===");

    Ok(())
}
