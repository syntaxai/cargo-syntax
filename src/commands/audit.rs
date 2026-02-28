use std::path::Path;

use anyhow::Result;
use tiktoken_rs::o200k_base;
use tokei::{Config, Languages};
use walkdir::WalkDir;

pub fn run() -> Result<()> {
    let bpe = o200k_base()?;
    let path = Path::new(".");

    // Line counts via tokei
    let config = Config::default();
    let mut languages = Languages::new();
    languages.get_statistics(&[path], &[], &config);

    let rust_stats = languages.get(&tokei::LanguageType::Rust);

    println!("{:<40} {:>6} {:>8}", "File", "Lines", "Tokens");
    println!("{}", "-".repeat(56));

    let mut total_lines = 0usize;
    let mut total_tokens = 0usize;

    for entry in WalkDir::new("src")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
    {
        let file_path = entry.path();
        let content = std::fs::read_to_string(file_path).unwrap_or_default();
        let tokens = bpe.encode_with_special_tokens(&content).len();
        let lines = content.lines().count();

        let display = file_path.strip_prefix(path).unwrap_or(file_path);
        println!("{:<40} {:>6} {:>8}", display.display(), lines, tokens);

        total_lines += lines;
        total_tokens += tokens;
    }

    println!("{}", "-".repeat(56));
    println!("{:<40} {:>6} {:>8}", "Total", total_lines, total_tokens);

    if let Some(stats) = rust_stats {
        println!();
        println!("Code: {} | Comments: {} | Blanks: {}", stats.code, stats.comments, stats.blanks);
    }

    Ok(())
}
