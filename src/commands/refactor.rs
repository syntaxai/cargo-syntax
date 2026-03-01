use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::json;

use crate::{openrouter, tokens};

const REFACTOR_PROMPT: &str = "\
You are a Rust architect analyzing an entire project for cross-file refactoring opportunities. \
Focus on: \
1. Duplicated code patterns across files (similar functions, repeated struct definitions, copy-pasted logic) \
2. Code that should be extracted into shared modules, traits, or utility functions \
3. Patterns where a generic/trait-based approach would eliminate repetition \
Only suggest changes with clear, significant token savings. \
Each suggestion must reference the specific files and functions involved. \
Order by impact (highest savings first).";

#[derive(Deserialize)]
struct RefactorResult {
    patterns: Vec<Pattern>,
    summary: String,
}

#[derive(Deserialize)]
struct Pattern {
    description: String,
    files: Vec<String>,
    suggestion: String,
    tokens_saved: u32,
}

fn refactor_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "patterns": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "What is duplicated and where"
                        },
                        "files": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Files involved in this pattern"
                        },
                        "suggestion": {
                            "type": "string",
                            "description": "How to refactor: extract to shared fn/trait/module"
                        },
                        "tokens_saved": {
                            "type": "integer",
                            "description": "Estimated total tokens saved across all files"
                        }
                    },
                    "required": ["description", "files", "suggestion", "tokens_saved"],
                    "additionalProperties": false
                }
            },
            "summary": {
                "type": "string",
                "description": "Overall assessment of project duplication level"
            }
        },
        "required": ["patterns", "summary"],
        "additionalProperties": false
    })
}

pub fn run(model: &str) -> Result<()> {
    let stats = tokens::scan_project()?;

    if stats.files.is_empty() {
        bail!("No .rs files found in project");
    }

    println!(
        "Scanning {} files ({} tokens) for cross-file duplication via {model}...",
        stats.files.len(),
        stats.total_tokens
    );

    let manifest = tokens::build_manifest(&stats);

    eprint!("  analyzing... ");

    let result = openrouter::chat_json::<RefactorResult>(
        model,
        REFACTOR_PROMPT,
        &manifest,
        "refactor_result",
        refactor_schema(),
    )?;
    eprintln!("done");

    println!();

    if result.patterns.is_empty() {
        println!("No significant cross-file duplication found. ✓");
        return Ok(());
    }

    let mut total_saveable: u32 = 0;

    for (i, p) in result.patterns.iter().enumerate() {
        println!("  {}. {}", i + 1, p.description);
        println!("     Files: {}", p.files.join(", "));
        println!("     Fix: {}", p.suggestion);
        println!("     Saves: ~{} tokens", p.tokens_saved);
        println!();
        total_saveable += p.tokens_saved;
    }

    tokens::separator(70);
    println!("{}", result.summary);

    if total_saveable > 0 {
        let save_pct = tokens::pct(total_saveable as usize, stats.total_tokens);
        println!(
            "{} pattern(s) found, ~{total_saveable} tokens saveable ({save_pct:.1}% of project)",
            result.patterns.len()
        );
    }

    Ok(())
}
