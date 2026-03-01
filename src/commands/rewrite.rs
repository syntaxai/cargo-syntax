use std::io::{self, BufRead, Write};
use std::path::Path;

use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::json;
use tiktoken_rs::o200k_base;

use crate::openrouter;

const REWRITE_PROMPT: &str = "\
You are a Rust code optimizer focused on token efficiency. \
Rewrite the given Rust code to minimize token count while preserving identical behavior. \
Apply these rules: \
- Prefer iterator chains over manual loops \
- Use ? operator instead of match/unwrap on Result/Option \
- Inline format args (write `\"{x}\"` not `\"{}\", x`) \
- Remove redundant closures, borrows, lifetimes, clone calls \
- Use manual_let_else, matches!, and other idiomatic patterns \
- Collapse collapsible if/else blocks \
- Remove unnecessary type annotations \
- Remove comments that restate the code \
Return ONLY the rewritten Rust code. No markdown fences, no explanations.";

const EXPLAIN_PROMPT: &str = "\
You are a Rust code auditor. Given an ORIGINAL and REWRITTEN version of the same file, \
list each change: what was changed and how many tokens it saves. \
Be specific (mention function names, patterns).";

#[derive(Deserialize)]
struct ExplainResult {
    changes: Vec<Change>,
}

#[derive(Deserialize)]
struct Change {
    description: String,
    tokens_saved: u32,
}

fn explain_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "changes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "What was changed and where"
                        },
                        "tokens_saved": {
                            "type": "integer",
                            "description": "Number of tokens saved by this change"
                        }
                    },
                    "required": ["description", "tokens_saved"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["changes"],
        "additionalProperties": false
    })
}

pub fn run(file: &str, model: &str) -> Result<()> {
    let path = Path::new(file);
    if !path.exists() {
        bail!("File not found: {file}");
    }
    if path.extension().is_none_or(|ext| ext != "rs") {
        bail!("Only .rs files are supported");
    }

    let original = std::fs::read_to_string(path)?;
    let bpe = o200k_base()?;

    let tokens_before = bpe.encode_with_special_tokens(&original).len();
    let lines_before = original.lines().count();

    println!("Sending {file} to {model} via OpenRouter...");
    println!("  {lines_before} lines, {tokens_before} tokens");
    println!();

    let rewritten = openrouter::chat(model, REWRITE_PROMPT, &original)?;

    let clean = strip_markdown_fences(&rewritten);
    let tokens_after = bpe.encode_with_special_tokens(&clean).len();
    let lines_after = clean.lines().count();

    let diff = tokens_before as isize - tokens_after as isize;
    let pct = if tokens_before > 0 { (diff as f64 / tokens_before as f64) * 100.0 } else { 0.0 };

    println!("Result:");
    println!("  Lines:  {lines_before} → {lines_after}");
    println!("  Tokens: {tokens_before} → {tokens_after}");

    if diff > 0 {
        println!("  Saved:  {diff} tokens ({pct:.1}%)");
    } else if diff < 0 {
        println!("  Added:  {} tokens ({:.1}%)", diff.unsigned_abs(), pct.abs());
    } else {
        println!("  No token change.");
    }

    println!();
    println!("Changes:");
    let explain_input = format!("ORIGINAL:\n{original}\n\nREWRITTEN:\n{clean}");
    match openrouter::chat_json::<ExplainResult>(
        model,
        EXPLAIN_PROMPT,
        &explain_input,
        "explain_result",
        explain_schema(),
    ) {
        Ok(result) => {
            for c in &result.changes {
                println!("  - {} (~{} tokens)", c.description, c.tokens_saved);
            }
        }
        Err(_) => println!("  (could not generate explanation)"),
    }

    println!();
    print!("Accept? [y/n/diff] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;

    match input.trim() {
        "y" | "Y" => {
            std::fs::write(path, &clean)?;
            println!("Written to {file}");
        }
        "diff" | "d" => {
            print_diff(&original, &clean);
            println!();
            print!("Accept? [y/n] ");
            io::stdout().flush()?;
            input.clear();
            io::stdin().lock().read_line(&mut input)?;
            if input.trim() == "y" || input.trim() == "Y" {
                std::fs::write(path, &clean)?;
                println!("Written to {file}");
            } else {
                println!("Discarded.");
            }
        }
        _ => println!("Discarded."),
    }

    Ok(())
}

fn strip_markdown_fences(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.starts_with("```") {
        let without_opening = trimmed
            .strip_prefix("```rust")
            .or_else(|| trimmed.strip_prefix("```rs"))
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        without_opening.strip_suffix("```").unwrap_or(without_opening).trim().to_string()
    } else {
        trimmed.to_string()
    }
}

fn print_diff(original: &str, rewritten: &str) {
    let old_lines: Vec<&str> = original.lines().collect();
    let new_lines: Vec<&str> = rewritten.lines().collect();
    let max = old_lines.len().max(new_lines.len());

    println!("{}", "─".repeat(70));
    for i in 0..max {
        let old = old_lines.get(i).copied().unwrap_or("");
        let new = new_lines.get(i).copied().unwrap_or("");
        if old != new {
            if !old.is_empty() {
                println!("- {old}");
            }
            if !new.is_empty() {
                println!("+ {new}");
            }
        }
    }
    println!("{}", "─".repeat(70));
}
