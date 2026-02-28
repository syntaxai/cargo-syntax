use std::path::Path;

use anyhow::Result;
use tiktoken_rs::o200k_base;
use tokei::{Config, Languages};

use crate::tokens;

pub fn run() -> Result<()> {
    let bpe = o200k_base()?;
    let path = Path::new(".");

    let config = Config::default();
    let mut languages = Languages::new();
    languages.get_statistics(&[path], &[], &config);

    let rust_stats = languages.get(&tokei::LanguageType::Rust);

    println!("{:<60} {:>6} {:>8} {:>6}", "File", "Lines", "Tokens", "T/L");
    println!("{}", "-".repeat(83));

    let mut total_lines = 0usize;
    let mut total_tokens = 0usize;

    for entry in tokens::rust_file_walker() {
        let file_path = entry.path();
        let content = std::fs::read_to_string(file_path).unwrap_or_default();
        let tokens = bpe.encode_with_special_tokens(&content).len();
        let lines = content.lines().count();
        let ratio = if lines > 0 { tokens as f64 / lines as f64 } else { 0.0 };

        let display = file_path.strip_prefix(path).unwrap_or(file_path);
        println!("{:<60} {:>6} {:>8} {:>5.1}", display.display(), lines, tokens, ratio);

        total_lines += lines;
        total_tokens += tokens;
    }

    let avg_ratio = if total_lines > 0 { total_tokens as f64 / total_lines as f64 } else { 0.0 };

    println!("{}", "-".repeat(83));
    println!("{:<60} {:>6} {:>8} {:>5.1}", "Total", total_lines, total_tokens, avg_ratio);

    if let Some(stats) = rust_stats {
        println!();
        println!("Code: {} | Comments: {} | Blanks: {}", stats.code, stats.comments, stats.blanks);
    }

    println!();
    print_score(avg_ratio);

    Ok(())
}

fn print_score(tokens_per_line: f64) {
    let (grade, msg) = match tokens_per_line {
        r if r <= 5.0 => ("A+", "Excellent — extremely token-efficient"),
        r if r <= 7.0 => ("A", "Great — lean and concise code"),
        r if r <= 9.0 => ("B", "Good — some room for improvement"),
        r if r <= 12.0 => ("C", "Fair — consider running `cargo syntax fix`"),
        _ => ("D", "Verbose — run `cargo syntax fix` to reduce tokens"),
    };

    println!("Token efficiency: {grade} ({tokens_per_line:.1} tokens/line)");
    println!("{msg}");
}
