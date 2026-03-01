use std::collections::HashSet;

use anyhow::Result;

use crate::{openrouter, tokens};

const DEFAULT_MAX_TOKENS: usize = 20_000;

const REVIEW_PROMPT: &str = "\
You are a Rust code auditor focused on token efficiency. \
Analyze the given Rust file and list 3-8 DISTINCT improvements to reduce token count. \
Rules: \
1. Each suggestion must be fundamentally different (not variations of the same pattern). \
2. Order by potential impact (highest savings first). \
3. For each: describe the change, reference the function/line, estimate tokens saved with ~N. \
4. Do NOT repeat the same suggestion for multiple occurrences — mention it once. \
5. Format each as a single bullet line starting with -. \
No markdown fences, no headers.";

pub fn run(n: usize, model: &str) -> Result<()> {
    let mut stats = tokens::scan_project()?;
    stats.files.sort_by(|a, b| b.tokens.cmp(&a.tokens));

    let show = n.min(stats.files.len());

    let max_tokens = model_context_limit(model).unwrap_or(DEFAULT_MAX_TOKENS);

    println!("Scanning project... {} files, {} tokens total", stats.files.len(), stats.total_tokens);
    println!("Reviewing top {show} files via {model}...");
    println!();

    let mut total_estimated_savings = 0;

    for (i, f) in stats.files.iter().take(show).enumerate() {
        let pct_of_total = if stats.total_tokens > 0 {
            (f.tokens as f64 / stats.total_tokens as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "  #{:<2} {}  ({} lines, {} tokens, T/L: {:.1}, {pct_of_total:.1}% of total)",
            i + 1, f.path, f.lines, f.tokens, f.ratio
        );

        if f.tokens > max_tokens {
            println!("      (skipped — {} tokens exceeds {max_tokens} limit for {model})", f.tokens);
            println!("      Tip: split this file into smaller modules.");
            println!();
            continue;
        }

        eprint!("      [{}/{}] reviewing... ", i + 1, show);

        match openrouter::chat(model, REVIEW_PROMPT, &f.content) {
            Ok(analysis) => {
                eprintln!("done");
                let deduped = deduplicate_suggestions(&analysis);
                for line in &deduped {
                    println!("      {line}");
                }

                let estimated = estimate_savings(&analysis, f.tokens);
                if estimated > 0 {
                    let est_pct = (estimated as f64 / f.tokens as f64) * 100.0;
                    println!("      => est. savings: ~{estimated} tokens ({est_pct:.1}%)");
                    total_estimated_savings += estimated;
                }
            }
            Err(e) => {
                eprintln!("failed");
                println!("      (review failed: {e})");
            }
        }
        println!();
    }

    println!("{}", "─".repeat(70));

    let top_tokens: usize = stats.files.iter().take(show).map(|f| f.tokens).sum();
    println!("Reviewed {show}/{} files ({top_tokens} of {} tokens)", stats.files.len(), stats.total_tokens);

    if total_estimated_savings > 0 {
        let total_pct = (total_estimated_savings as f64 / stats.total_tokens as f64) * 100.0;
        println!("Estimated total savings: ~{total_estimated_savings} tokens ({total_pct:.1}%)");
    }

    println!();
    println!("Run `cargo syntax rewrite <file>` on any file to apply changes.");

    Ok(())
}

fn model_context_limit(model: &str) -> Option<usize> {
    let id = model.to_lowercase();
    if id.contains("gemini") || id.contains("claude-sonnet-4") || id.contains("claude-opus") {
        Some(100_000)
    } else if id.contains("gpt-4o") || id.contains("gpt-4.1") {
        Some(80_000)
    } else if id.contains("deepseek") || id.contains("qwen") {
        Some(30_000)
    } else {
        None
    }
}

fn deduplicate_suggestions(analysis: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for line in analysis.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let normalized: String = trimmed
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphabetic() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        if normalized.len() < 10 {
            continue;
        }

        if seen.iter().any(|s: &String| {
            let overlap = normalized.chars().zip(s.chars()).take_while(|(a, b)| a == b).count();
            overlap > normalized.len() / 2 && overlap > 20
        }) {
            continue;
        }

        seen.insert(normalized);
        result.push(trimmed.to_string());
    }

    result
}

fn estimate_savings(analysis: &str, file_tokens: usize) -> usize {
    let mut total = 0;

    for line in analysis.lines() {
        let lower = line.to_lowercase();
        if let Some(pos) = lower.find('~') {
            let after = &lower[pos + 1..];
            let num: String = after.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(n) = num.parse::<usize>() {
                total += n;
            }
        }
    }

    total.min(file_tokens / 2)
}
