use std::collections::HashMap;
use std::io::BufRead;
use std::process::{Command, Stdio};

use anyhow::{Result, bail};
use serde::Deserialize;
use tiktoken_rs::o200k_base;

use crate::tokens;

const WARN_LINTS: &[&str] = &[
    "needless_return",
    "needless_borrow",
    "needless_lifetimes",
    "needless_pass_by_value",
    "let_and_return",
    "redundant_else",
    "redundant_field_names",
    "redundant_pattern_matching",
    "redundant_closure",
    "redundant_closure_for_method_calls",
    "manual_map",
    "manual_filter",
    "manual_find",
    "manual_flatten",
    "manual_is_ascii_check",
    "manual_let_else",
    "manual_ok_or",
    "manual_string_new",
    "manual_unwrap_or",
    "map_unwrap_or",
    "collapsible_if",
    "collapsible_else_if",
    "single_match",
    "match_like_matches_macro",
    "unnested_or_patterns",
    "implicit_clone",
    "cloned_instead_of_copied",
    "flat_map_option",
    "iter_on_single_items",
    "option_as_ref_deref",
    "bind_instead_of_map",
    "unnecessary_wraps",
    "unnecessary_unwrap",
    "unnecessary_lazy_evaluations",
    "use_self",
    "unused_self",
    "semicolon_if_nothing_returned",
    "uninlined_format_args",
    "dbg_macro",
    "redundant_clone",
];

#[derive(Deserialize)]
struct ClippyMsg {
    reason: String,
    message: Option<Diagnostic>,
}

#[derive(Deserialize)]
struct Diagnostic {
    message: String,
    level: String,
    code: Option<Code>,
    spans: Vec<Span>,
}

#[derive(Deserialize)]
struct Code {
    code: String,
}

#[derive(Deserialize)]
struct Span {
    file_name: String,
    line_start: u32,
    is_primary: bool,
}

struct Hint {
    line: u32,
    lint: String,
    message: String,
}

pub fn run() -> Result<()> {
    println!("Analyzing code for token-efficiency improvements...");
    println!();

    let mut args = vec![
        "clippy".to_string(),
        "--all-targets".to_string(),
        "--message-format=json".to_string(),
        "--".to_string(),
    ];

    for lint in WARN_LINTS {
        args.push("-W".to_string());
        args.push(format!("clippy::{lint}"));
    }

    let output =
        Command::new("cargo").args(&args).stdout(Stdio::piped()).stderr(Stdio::null()).output()?;

    if !output.status.success() && output.stdout.is_empty() {
        bail!("clippy failed to run — make sure the project compiles first (`cargo build`)");
    }

    let mut suggestions: HashMap<String, Vec<Hint>> = HashMap::new();
    let mut seen: std::collections::HashSet<(String, u32, String)> =
        std::collections::HashSet::new();

    for line in output.stdout.lines() {
        let line = line?;
        let Ok(msg) = serde_json::from_str::<ClippyMsg>(&line) else {
            continue;
        };
        if msg.reason != "compiler-message" {
            continue;
        }
        let Some(diag) = msg.message else {
            continue;
        };
        if diag.level != "warning" && diag.level != "error" {
            continue;
        }
        let Some(code) = diag.code else {
            continue;
        };
        if !code.code.starts_with("clippy::") {
            continue;
        }

        let lint = code.code.trim_start_matches("clippy::").to_string();

        let Some(span) = diag.spans.iter().find(|s| s.is_primary) else {
            continue;
        };

        let file = normalize(&span.file_name);
        if file.contains("target/") {
            continue;
        }

        let key = (file.clone(), span.line_start, lint.clone());
        if !seen.insert(key) {
            continue;
        }

        suggestions.entry(file).or_default().push(Hint {
            line: span.line_start,
            lint,
            message: diag.message,
        });
    }

    if suggestions.is_empty() {
        println!("No suggestions — code already follows token-efficient patterns.");
        return Ok(());
    }

    let file_ratios = build_ratio_map()?;

    let mut files: Vec<(String, Vec<Hint>)> = suggestions.into_iter().collect();
    files.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    let total: usize = files.iter().map(|(_, v)| v.len()).sum();
    let file_count = files.len();

    for (file, hints) in &mut files {
        hints.sort_by_key(|h| h.line);
        let count = hints.len();
        let label = if count == 1 { "suggestion" } else { "suggestions" };

        let ratio = file_ratios
            .iter()
            .find(|(k, _)| normalize(k) == *file || normalize(k).ends_with(&format!("/{file}")))
            .map(|(_, v)| *v);

        match ratio {
            Some(r) => println!("{file}  ({count} {label}, T/L: {r:.1})"),
            None => println!("{file}  ({count} {label})"),
        }

        for hint in hints {
            println!("  line {:>4}  {:<38}  {}", hint.line, hint.lint, hint.message);
        }
        println!();
    }

    println!("{}", "─".repeat(70));
    println!("{total} suggestion(s) across {file_count} file(s)");
    println!("Run `cargo syntax fix` to auto-apply all fixable suggestions.");

    Ok(())
}

fn normalize(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

fn build_ratio_map() -> Result<HashMap<String, f64>> {
    let bpe = o200k_base()?;
    let mut map = HashMap::new();

    for entry in tokens::rust_file_walker() {
        let path = entry.path();
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let lines = content.lines().count();
        let toks = bpe.encode_with_special_tokens(&content).len();
        let ratio = if lines > 0 { toks as f64 / lines as f64 } else { 0.0 };
        let key = normalize(&path.display().to_string());
        map.insert(key, ratio);
    }

    Ok(map)
}
