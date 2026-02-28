use anyhow::Result;
use tiktoken_rs::o200k_base;

use crate::tokens;

pub fn run(n: usize) -> Result<()> {
    let bpe = o200k_base()?;

    let mut files: Vec<(String, usize, usize, f64)> = Vec::new();

    for entry in tokens::rust_file_walker() {
        let file_path = entry.path();
        let content = std::fs::read_to_string(file_path).unwrap_or_default();
        let tok = bpe.encode_with_special_tokens(&content).len();
        let lines = content.lines().count();
        let ratio = if lines > 0 { tok as f64 / lines as f64 } else { 0.0 };

        let display = file_path.strip_prefix(".").unwrap_or(file_path);
        files.push((display.display().to_string(), lines, tok, ratio));
    }

    files.sort_by(|a, b| b.2.cmp(&a.2));

    let total_tokens: usize = files.iter().map(|f| f.2).sum();
    let show = n.min(files.len());

    println!("Top {show} most token-heavy files:");
    println!();
    println!(
        "{:<4} {:<50} {:>6} {:>8} {:>6} {:>7}",
        "#", "File", "Lines", "Tokens", "T/L", "% Tot"
    );
    println!("{}", "-".repeat(84));

    for (i, (name, lines, tok, ratio)) in files.iter().take(show).enumerate() {
        let pct = if total_tokens > 0 { (*tok as f64 / total_tokens as f64) * 100.0 } else { 0.0 };
        println!("{:<4} {:<50} {:>6} {:>8} {:>5.1} {:>6.1}%", i + 1, name, lines, tok, ratio, pct);
    }

    let top_tokens: usize = files.iter().take(show).map(|f| f.2).sum();
    let top_pct =
        if total_tokens > 0 { (top_tokens as f64 / total_tokens as f64) * 100.0 } else { 0.0 };

    println!("{}", "-".repeat(84));
    println!("Top {show} = {top_tokens} tokens ({top_pct:.1}% of {total_tokens} total)");

    Ok(())
}
