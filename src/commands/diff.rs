use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::json;

use crate::{openrouter, tokens};

const DIFF_PROMPT: &str = "\
You are a Rust code auditor focused on token efficiency. \
You are reviewing ONLY the changed/new code in a file. \
Analyze the full file content but focus your suggestions on the recently changed parts. \
List 1-5 concrete improvements to make the changes more token-efficient. \
Only suggest changes that are clearly beneficial — if the code is already efficient, return an empty list.";

#[derive(Deserialize)]
struct DiffResult {
    suggestions: Vec<DiffSuggestion>,
    verdict: String,
}

#[derive(Deserialize)]
struct DiffSuggestion {
    description: String,
    location: String,
    tokens_saved: u32,
}

fn diff_schema() -> serde_json::Value {
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
                            "description": "Function name or line reference"
                        },
                        "tokens_saved": {
                            "type": "integer",
                            "description": "Estimated tokens saved"
                        }
                    },
                    "required": ["description", "location", "tokens_saved"],
                    "additionalProperties": false
                }
            },
            "verdict": {
                "type": "string",
                "description": "One of: efficient, minor_issues, needs_work"
            }
        },
        "required": ["suggestions", "verdict"],
        "additionalProperties": false
    })
}

pub fn run(range: Option<&str>, staged: bool, fix: bool, model: &str) -> Result<()> {
    let diff_args = build_diff_args(range, staged);
    let diff_output = run_git_diff(&diff_args)?;

    if diff_output.trim().is_empty() {
        println!("No changes to review.");
        return Ok(());
    }

    let changed_files = parse_changed_rs_files(&diff_output);

    if changed_files.is_empty() {
        println!("No .rs file changes found.");
        return Ok(());
    }

    let label = if staged {
        "staged"
    } else if let Some(r) = range {
        r
    } else {
        "unstaged"
    };

    println!("Analyzing {label} changes via {model}...");
    println!();

    let mut total_files = 0;
    let mut total_added_tokens = 0;
    let mut total_suggestions = 0;
    let mut total_saveable = 0;
    let mut efficient_files = 0;
    let mut files_to_fix = Vec::new();

    for file in &changed_files {
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let file_tokens = match tokens::count_tokens(&content) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let lines = content.lines().count();
        let ratio = if lines > 0 { file_tokens as f64 / lines as f64 } else { 0.0 };

        let file_diff = extract_file_diff(&diff_output, file);
        let added_lines = file_diff.lines().filter(|l| l.starts_with('+')).count();
        let added_tokens_est = added_lines * 8;

        total_files += 1;
        total_added_tokens += added_tokens_est;

        let is_new = file_diff.contains("new file mode");
        let status = if is_new { "new file" } else { "modified" };

        println!(
            "{file}  ({status}, +{added_lines} lines, ~+{added_tokens_est} tokens, T/L: {ratio:.1})"
        );

        let prompt = format!(
            "GIT DIFF for this file:\n{file_diff}\n\nFULL FILE CONTENT:\n{content}"
        );

        eprint!("  reviewing... ");

        match openrouter::chat_json::<DiffResult>(
            model, DIFF_PROMPT, &prompt, "diff_result", diff_schema(),
        ) {
            Ok(result) => {
                eprintln!("done");

                if result.suggestions.is_empty() || result.verdict == "efficient" {
                    println!("  ✓ Changes look token-efficient");
                    efficient_files += 1;
                } else {
                    for s in &result.suggestions {
                        println!("  - {} [{}] (~{} tokens)", s.description, s.location, s.tokens_saved);
                        total_saveable += s.tokens_saved as usize;
                    }
                    total_suggestions += result.suggestions.len();
                    files_to_fix.push(file.clone());
                }
            }
            Err(e) => {
                eprintln!("failed");
                println!("  (review failed: {e})");
            }
        }
        println!();
    }

    println!("{}", "─".repeat(70));
    println!(
        "Summary: {total_files} file(s) changed, ~+{total_added_tokens} tokens added"
    );

    if efficient_files == total_files {
        println!("All changes look token-efficient. ✓");
    } else if total_saveable > 0 {
        let pct = if total_added_tokens > 0 {
            (total_saveable as f64 / total_added_tokens as f64) * 100.0
        } else {
            0.0
        };
        println!("{total_suggestions} suggestion(s) could save ~{total_saveable} tokens ({pct:.0}%)");
    }

    if fix && !files_to_fix.is_empty() {
        println!();
        println!("Rewriting {} file(s) with suggestions...", files_to_fix.len());
        println!();

        for file in &files_to_fix {
            super::rewrite::run(file, model)?;
            println!();
        }
    } else if !fix && !files_to_fix.is_empty() {
        println!();
        println!("Run `cargo syntax diff --fix` to rewrite, or `cargo syntax rewrite <file>` individually.");
    }

    Ok(())
}

fn build_diff_args(range: Option<&str>, staged: bool) -> Vec<String> {
    let mut args = vec!["diff".to_string()];
    if staged {
        args.push("--staged".to_string());
    }
    if let Some(r) = range {
        args.push(r.to_string());
    }
    args.push("--".to_string());
    args.push("*.rs".to_string());
    args
}

fn run_git_diff(args: &[String]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("failed to run git diff")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff failed: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_changed_rs_files(diff: &str) -> Vec<String> {
    let mut files = Vec::new();

    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            if path.ends_with(".rs") {
                files.push(path.to_string());
            }
        }
    }

    files.dedup();
    files
}

fn extract_file_diff<'a>(full_diff: &'a str, file: &str) -> &'a str {
    let marker = format!("diff --git a/{file}");
    let Some(start) = full_diff.find(&marker) else {
        return "";
    };

    let rest = &full_diff[start..];
    let end = rest[1..]
        .find("diff --git ")
        .map(|p| p + 1)
        .unwrap_or(rest.len());

    &rest[..end]
}
