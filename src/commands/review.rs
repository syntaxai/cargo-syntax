use anyhow::Result;
use tiktoken_rs::o200k_base;

use crate::{openrouter, tokens};

const MAX_TOKENS_PER_FILE: usize = 20_000;

const REVIEW_PROMPT: &str = "\
You are a Rust code auditor focused on token efficiency. \
Analyze the given Rust file and list concrete improvements to reduce token count. \
For each suggestion: describe the change, reference the function/line, and estimate tokens saved. \
Format each as a single bullet line starting with -. \
Be specific and actionable. No markdown fences, no headers.";

pub fn run(n: usize, model: &str) -> Result<()> {
    let bpe = o200k_base()?;

    let mut files: Vec<(String, String, usize, usize, f64)> = Vec::new();

    for entry in tokens::rust_file_walker() {
        let path = entry.path();
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let tok = bpe.encode_with_special_tokens(&content).len();
        let lines = content.lines().count();
        let ratio = if lines > 0 { tok as f64 / lines as f64 } else { 0.0 };
        let display = path.strip_prefix(".").unwrap_or(path).display().to_string();
        files.push((display, content, lines, tok, ratio));
    }

    files.sort_by(|a, b| b.3.cmp(&a.3));

    let total_tokens: usize = files.iter().map(|f| f.3).sum();
    let total_files = files.len();
    let show = n.min(files.len());

    println!("Scanning project... {total_files} files, {total_tokens} tokens total");
    println!("Reviewing top {show} files via {model}...");
    println!();

    let mut total_estimated_savings = 0usize;

    for (i, (name, content, lines, tok, ratio)) in files.iter().take(show).enumerate() {
        let pct_of_total =
            if total_tokens > 0 { (*tok as f64 / total_tokens as f64) * 100.0 } else { 0.0 };

        println!(
            "  #{:<2} {name}  ({lines} lines, {tok} tokens, T/L: {ratio:.1}, {pct_of_total:.1}% of total)",
            i + 1
        );

        if *tok > MAX_TOKENS_PER_FILE {
            println!("      (skipped — {tok} tokens exceeds {MAX_TOKENS_PER_FILE} limit, too large for most models)");
            println!("      Tip: split this file into smaller modules.");
            println!();
            continue;
        }

        match openrouter::chat(model, REVIEW_PROMPT, content) {
            Ok(analysis) => {
                for line in analysis.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        println!("      {trimmed}");
                    }
                }

                let estimated = estimate_savings(&analysis, *tok);
                if estimated > 0 {
                    let est_pct = (estimated as f64 / *tok as f64) * 100.0;
                    println!("      => est. savings: ~{estimated} tokens ({est_pct:.1}%)");
                    total_estimated_savings += estimated;
                }
            }
            Err(e) => println!("      (review failed: {e})"),
        }
        println!();
    }

    println!("{}", "─".repeat(70));

    let top_tokens: usize = files.iter().take(show).map(|f| f.3).sum();
    println!("Reviewed {show}/{total_files} files ({top_tokens} of {total_tokens} tokens)");

    if total_estimated_savings > 0 {
        let total_pct = (total_estimated_savings as f64 / total_tokens as f64) * 100.0;
        println!("Estimated total savings: ~{total_estimated_savings} tokens ({total_pct:.1}%)");
    }

    println!();
    println!("Run `cargo syntax rewrite <file>` on any file to apply changes.");

    Ok(())
}

fn estimate_savings(analysis: &str, file_tokens: usize) -> usize {
    let mut total = 0usize;

    for line in analysis.lines() {
        let lower = line.to_lowercase();
        if let Some(pos) = lower.find("~") {
            let after = &lower[pos + 1..];
            let num: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num.parse::<usize>() {
                total += n;
            }
        }
    }

    total.min(file_tokens / 2)
}
