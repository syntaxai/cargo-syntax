use anyhow::Result;
use serde::Deserialize;
use serde_json::json;

use crate::{openrouter, tokens};

const DEFAULT_MAX_TOKENS: usize = 20_000;

const REVIEW_PROMPT: &str = "\
You are a Rust code auditor focused on token efficiency. \
Analyze the given Rust file and list 3-8 DISTINCT improvements to reduce token count. \
Each suggestion must be fundamentally different. Order by impact (highest savings first). \
Do NOT repeat the same suggestion for multiple occurrences — mention it once.";

#[derive(Deserialize)]
struct ReviewResult {
    suggestions: Vec<Suggestion>,
}

#[derive(Deserialize)]
struct Suggestion {
    description: String,
    location: String,
    tokens_saved: u32,
}

fn review_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "suggestions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "What to change and why it saves tokens"
                        },
                        "location": {
                            "type": "string",
                            "description": "Function name, line number, or code pattern"
                        },
                        "tokens_saved": {
                            "type": "integer",
                            "description": "Estimated number of tokens saved"
                        }
                    },
                    "required": ["description", "location", "tokens_saved"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["suggestions"],
        "additionalProperties": false
    })
}

pub fn run(n: usize, model: &str) -> Result<()> {
    let mut stats = tokens::scan_project()?;
    stats.files.sort_by(|a, b| b.tokens.cmp(&a.tokens));

    let show = n.min(stats.files.len());
    let max_tokens = model_context_limit(model).unwrap_or(DEFAULT_MAX_TOKENS);

    println!(
        "Scanning project... {} files, {} tokens total",
        stats.files.len(),
        stats.total_tokens
    );
    println!("Reviewing top {show} files via {model}...");
    println!();

    let mut total_estimated_savings = 0;

    for (i, f) in stats.files.iter().take(show).enumerate() {
        let pct_of_total = tokens::pct(f.tokens, stats.total_tokens);

        println!(
            "  #{:<2} {}  ({} lines, {} tokens, T/L: {:.1}, {pct_of_total:.1}% of total)",
            i + 1,
            f.path,
            f.lines,
            f.tokens,
            f.ratio
        );

        if f.tokens > max_tokens {
            println!(
                "      (skipped — {} tokens exceeds {max_tokens} limit for {model})",
                f.tokens
            );
            println!("      Tip: split this file into smaller modules.");
            println!();
            continue;
        }

        eprint!("      [{}/{}] reviewing... ", i + 1, show);

        match openrouter::chat_json::<ReviewResult>(
            model,
            REVIEW_PROMPT,
            &f.content,
            "review_result",
            review_schema(),
        ) {
            Ok(result) => {
                eprintln!("done");
                let estimated: u32 = result.suggestions.iter().map(|s| s.tokens_saved).sum();

                for s in &result.suggestions {
                    println!(
                        "      - {} [{}] (~{} tokens)",
                        s.description, s.location, s.tokens_saved
                    );
                }

                if estimated > 0 {
                    let capped = (estimated as usize).min(f.tokens / 2);
                    let est_pct = tokens::pct(capped, f.tokens);
                    println!("      => est. savings: ~{capped} tokens ({est_pct:.1}%)");
                    total_estimated_savings += capped;
                }
            }
            Err(e) => {
                eprintln!("failed");
                println!("      (review failed: {e})");
            }
        }
        println!();
    }

    tokens::separator(70);

    let top_tokens: usize = stats.files.iter().take(show).map(|f| f.tokens).sum();
    println!(
        "Reviewed {show}/{} files ({top_tokens} of {} tokens)",
        stats.files.len(),
        stats.total_tokens
    );

    if total_estimated_savings > 0 {
        let total_pct = tokens::pct(total_estimated_savings, stats.total_tokens);
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
