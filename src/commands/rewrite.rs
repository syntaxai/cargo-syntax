use crate::openrouter;
use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::json;
use std::io::{self, BufRead, Write};
use std::path::Path;
use tiktoken_rs::o200k_base;

const REWRITE_PROMPT: &str = "You are a Rust code optimizer focused on token efficiency. Rewrite the given Rust code to minimize token count while preserving identical behavior. Apply these rules: - Prefer iterator chains over manual loops - Use ? operator instead of match/unwrap on Result/Option - Inline format args (write `\"{x}\"` not `\"{}\", x`) - Remove redundant closures, borrows, lifetimes, clone calls - Use manual_let_else, matches!, and other idiomatic patterns - Collapse collapsible if/else blocks - Remove unnecessary type annotations - Remove comments that restate the code Return ONLY the rewritten Rust code. No markdown fences, no explanations.";
const EXPLAIN_PROMPT: &str = "You are a Rust code auditor. Given an ORIGINAL and REWRITTEN version of the same file, list each change: what was changed and how many tokens it saves. Be specific (mention function names, patterns).";

#[derive(Deserialize)]
struct ExplainResult {
    changes: Vec<Change>,
}
#[derive(Deserialize)]
struct Change {
    description: String,
    tokens_saved: u32,
}

pub struct RewriteResult {
    pub original: String,
    pub rewritten: String,
    pub tokens_before: usize,
    pub tokens_after: usize,
    pub lines_before: usize,
    pub lines_after: usize,
}

impl RewriteResult {
    pub fn saved(&self) -> isize {
        self.tokens_before as isize - self.tokens_after as isize
    }
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
                        "description": { "type": "string", "description": "What was changed and where" },
                        "tokens_saved": { "type": "integer", "description": "Number of tokens saved by this change" }
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

pub fn rewrite_file(file: &str, model: &str) -> Result<RewriteResult> {
    let path = Path::new(file);
    if !path.exists() {
        bail!("File not found: {file}")
    }
    if path.extension().is_none_or(|ext| ext != "rs") {
        bail!("Only .rs files are supported")
    }

    let original = std::fs::read_to_string(path)?;
    let bpe = o200k_base()?;
    let tokens_before = bpe.encode_with_special_tokens(&original).len();
    let lines_before = original.lines().count();

    let raw = openrouter::chat(model, REWRITE_PROMPT, &original)?;
    let rewritten = strip_markdown_fences(&raw);
    let tokens_after = bpe.encode_with_special_tokens(&rewritten).len();
    let lines_after = rewritten.lines().count();

    Ok(RewriteResult {
        original,
        rewritten,
        tokens_before,
        tokens_after,
        lines_before,
        lines_after,
    })
}

pub fn run(file: &str, model: &str) -> Result<()> {
    println!("Sending {file} to {model} via OpenRouter...");
    eprint!("  rewriting... ");
    let result = rewrite_file(file, model)?;
    eprintln!("done");
    println!("  {} lines, {} tokens", result.lines_before, result.tokens_before);

    let diff = result.saved();
    let pct = if result.tokens_before > 0 {
        (diff as f64 / result.tokens_before as f64) * 100.0
    } else {
        0.0
    };

    println!();
    println!("Result:");
    println!("  Lines:  {} → {}", result.lines_before, result.lines_after);
    println!("  Tokens: {} → {}", result.tokens_before, result.tokens_after);

    if diff > 0 {
        println!("  Saved:  {diff} tokens ({pct:.1}%)");
    } else if diff < 0 {
        println!("  Added:  {} tokens ({:.1}%)", diff.unsigned_abs(), pct.abs());
    } else {
        println!("  No token change.");
    }

    println!();
    println!("Changes:");
    let explain_input =
        format!("ORIGINAL:\n{}\n\nREWRITTEN:\n{}", result.original, result.rewritten);
    match openrouter::chat_json::<ExplainResult>(
        model,
        EXPLAIN_PROMPT,
        &explain_input,
        "explain_result",
        explain_schema(),
    ) {
        Ok(explain) => {
            for c in &explain.changes {
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

    let path = Path::new(file);
    match input.trim() {
        "y" | "Y" => {
            std::fs::write(path, &result.rewritten)?;
            println!("Written to {file}");
        }
        "diff" | "d" => {
            print_diff(&result.original, &result.rewritten);
            println!();
            print!("Accept? [y/n] ");
            io::stdout().flush()?;
            input.clear();
            io::stdin().lock().read_line(&mut input)?;
            if input.trim() == "y" || input.trim() == "Y" {
                std::fs::write(path, &result.rewritten)?;
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

    crate::tokens::separator(70);
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
    crate::tokens::separator(70);
}
