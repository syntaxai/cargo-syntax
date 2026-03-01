use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::process::{Command, Stdio};

use anyhow::{Result, bail};
use serde::Deserialize;

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

pub fn run(deep: bool) -> Result<()> {
    let stats = tokens::scan_project()?;

    println!("Analyzing code for token-efficiency improvements...\n");

    let mut args = vec![
        "clippy".to_string(),
        "--all-targets".to_string(),
        "--message-format=json".to_string(),
        "--".to_string(),
    ];

    args.extend(WARN_LINTS.iter().flat_map(|&lint| ["-W".to_string(), format!("clippy::{lint}")]));

    let output =
        Command::new("cargo").args(&args).stdout(Stdio::piped()).stderr(Stdio::null()).output()?;

    if !output.status.success() && output.stdout.is_empty() {
        bail!("clippy failed to run — make sure the project compiles first (`cargo build`)");
    }

    let mut suggestions: HashMap<String, Vec<Hint>> = HashMap::new();
    let mut seen = HashSet::new();

    for line in output.stdout.lines() {
        let line = line?;
        let Ok(msg) = serde_json::from_str::<ClippyMsg>(&line) else { continue };
        if msg.reason != "compiler-message" {
            continue;
        }
        let Some(diag) = msg.message else { continue };
        if diag.level != "warning" && diag.level != "error" {
            continue;
        }
        let Some(code) = diag.code else { continue };
        if !code.code.starts_with("clippy::") {
            continue;
        }

        let lint = code.code.trim_start_matches("clippy::").to_string();

        let Some(span) = diag.spans.iter().find(|s| s.is_primary) else { continue };

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
        if deep {
            println!();
            let result = super::deep::run(&stats);
            if result.total_savings > 0 {
                super::deep::print_results(&result, &stats);
            } else {
                println!("Deep analysis: no cross-file duplicates found.");
            }
        }
        return Ok(());
    }

    let ratio_map: HashMap<String, f64> =
        stats.files.iter().map(|f| (normalize(&f.path), f.ratio)).collect();

    let mut files: Vec<(String, Vec<Hint>)> = suggestions.into_iter().collect();
    files.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    let total: usize = files.iter().map(|(_, v)| v.len()).sum();
    let file_count = files.len();

    for (file, hints) in &mut files {
        hints.sort_by_key(|h| h.line);
        let count = hints.len();
        let label = if count == 1 { "suggestion" } else { "suggestions" };

        let ratio = ratio_map
            .iter()
            .find(|(k, _)| *k == file || k.ends_with(&format!("/{file}")))
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

    tokens::separator(70);
    println!(
        "{total} suggestion(s) across {file_count} file(s)\nRun `cargo syntax fix` to auto-apply all fixable suggestions."
    );

    if deep {
        println!();
        let result = super::deep::run(&stats);
        if result.total_savings > 0 {
            super::deep::print_results(&result, &stats);
        } else {
            println!("Deep analysis: no cross-file duplicates found.");
        }
    }

    Ok(())
}

fn normalize(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_unix_path() {
        assert_eq!(normalize("src/main.rs"), "src/main.rs");
    }

    #[test]
    fn test_normalize_windows_path() {
        assert_eq!(normalize("src\\commands\\audit.rs"), "src/commands/audit.rs");
    }

    #[test]
    fn test_normalize_dotslash_prefix() {
        assert_eq!(normalize("./src/main.rs"), "src/main.rs");
    }

    #[test]
    fn test_normalize_dotslash_windows() {
        assert_eq!(normalize(".\\src\\main.rs"), "src/main.rs");
    }
}
