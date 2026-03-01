use std::io::{self, BufRead, Write};
use std::path::Path;

use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::json;

use crate::{openrouter, tokens};

const TEST_PROMPT: &str = "\
You are a Rust test engineer. Given a Rust source file from a crate, generate integration tests. \
The tests will be placed in a separate file (tests/ directory), NOT inline in the source. \
Rules: \
1. Import the crate with `use <crate_name>::<module>::*;` — do NOT use `mod tests` or `use super::*;` \
2. Write top-level #[test] functions — no wrapping `mod tests` block \
3. Test every public function, including edge cases and error paths \
4. Use assert!, assert_eq!, assert_ne! — no external test frameworks \
5. For functions that return Result, test both Ok and Err paths \
6. Use descriptive test names: test_<function>_<scenario> \
7. Keep tests minimal and token-efficient (no unnecessary comments) \
8. If a function requires complex setup (filesystem, network), mark with #[ignore] \
9. In Rust edition 2024, std::env::set_var/remove_var are unsafe — wrap in unsafe {} \
10. Return ONLY the test functions, no markdown fences or explanations";

const EXPLAIN_PROMPT: &str = "\
Given a Rust source file and generated tests, produce a brief summary of test coverage.";

#[derive(Deserialize)]
struct TestCoverage {
    functions_tested: Vec<String>,
    functions_untestable: Vec<String>,
    test_count: u32,
    coverage_notes: String,
}

fn coverage_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "functions_tested": {
                "type": "array",
                "items": { "type": "string" },
                "description": "List of public functions that have tests"
            },
            "functions_untestable": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Functions skipped (need I/O, network, etc.)"
            },
            "test_count": {
                "type": "integer",
                "description": "Total number of #[test] functions generated"
            },
            "coverage_notes": {
                "type": "string",
                "description": "Brief notes on what's covered and what's not"
            }
        },
        "required": ["functions_tested", "functions_untestable", "test_count", "coverage_notes"],
        "additionalProperties": false
    })
}

pub fn run(file: &str, output: Option<&str>, model: &str) -> Result<()> {
    let path = Path::new(file);
    if !path.exists() {
        bail!("File not found: {file}");
    }
    if path.extension().is_none_or(|ext| ext != "rs") {
        bail!("Only .rs files are supported");
    }

    let content = std::fs::read_to_string(path)?;
    let token_count = tokens::count_tokens(&content)?;
    let lines = content.lines().count();

    let crate_name = detect_crate_name();
    let module_path = file_to_module_path(file);

    println!("Generating tests for {file} ({lines} lines, {token_count} tokens) via {model}...");
    eprint!("  analyzing... ");

    let prompt = format!(
        "Crate name: {crate_name}\nModule path: {module_path}\n\
         Import as: use {crate_name}::{module_path}::*;\n\n\
         Source file ({file}):\n{content}"
    );
    let test_code = openrouter::chat(model, TEST_PROMPT, &prompt)?;
    let test_code = strip_markdown_fences(&test_code);
    eprintln!("done");

    let test_tokens = tokens::count_tokens(&test_code)?;
    let test_lines = test_code.lines().count();

    println!("  Generated: {test_lines} lines, {test_tokens} tokens");
    println!();

    // Get coverage analysis
    eprint!("  coverage analysis... ");
    let coverage_input = format!("SOURCE:\n{content}\n\nGENERATED TESTS:\n{test_code}");
    let coverage = openrouter::chat_json::<TestCoverage>(
        model,
        EXPLAIN_PROMPT,
        &coverage_input,
        "test_coverage",
        coverage_schema(),
    );
    eprintln!("done");

    if let Ok(cov) = &coverage {
        println!("  Tests: {}", cov.test_count);
        if !cov.functions_tested.is_empty() {
            println!("  Covered: {}", cov.functions_tested.join(", "));
        }
        if !cov.functions_untestable.is_empty() {
            println!("  Skipped: {} (need I/O/network)", cov.functions_untestable.join(", "));
        }
        println!("  {}", cov.coverage_notes);
    }

    println!();
    println!("{}", "─".repeat(70));
    println!("{test_code}");
    println!("{}", "─".repeat(70));

    // Determine output path
    let target = if let Some(out) = output { out.to_string() } else { default_test_path(file) };

    println!();
    print!("Write to {target}? [y/n/append] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;

    match input.trim() {
        "y" | "Y" => {
            write_tests(&target, &test_code, false)?;
            println!("Written to {target}");
            try_compile(&target);
        }
        "a" | "append" => {
            write_tests(&target, &test_code, true)?;
            println!("Appended to {target}");
            try_compile(&target);
        }
        _ => println!("Discarded."),
    }

    Ok(())
}

fn default_test_path(source: &str) -> String {
    let p = Path::new(source);
    let stem = p.file_stem().unwrap_or_default().to_string_lossy();

    // If source is src/foo.rs → tests/test_foo.rs
    // If source is src/commands/bar.rs → tests/test_bar.rs
    if Path::new("tests").exists() || source.starts_with("src") {
        format!("tests/test_{stem}.rs")
    } else {
        format!("test_{stem}.rs")
    }
}

fn write_tests(path: &str, code: &str, append: bool) -> Result<()> {
    let p = Path::new(path);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if append && p.exists() {
        let existing = std::fs::read_to_string(p)?;
        std::fs::write(p, format!("{existing}\n\n{code}"))?;
    } else {
        std::fs::write(p, code)?;
    }
    Ok(())
}

fn try_compile(test_file: &str) {
    eprint!("  compiling tests... ");
    let output = std::process::Command::new("cargo").args(["test", "--no-run", "--quiet"]).output();

    match output {
        Ok(o) if o.status.success() => eprintln!("compiled OK"),
        Ok(o) => {
            eprintln!("COMPILE ERROR");
            let stderr = String::from_utf8_lossy(&o.stderr);
            let relevant: Vec<&str> =
                stderr.lines().filter(|l| l.contains("error") || l.contains(test_file)).collect();
            for line in relevant.iter().take(10) {
                eprintln!("    {line}");
            }
            eprintln!("  Fix errors or delete {test_file} and retry");
        }
        Err(e) => eprintln!("failed to run cargo test: {e}"),
    }
}

fn detect_crate_name() -> String {
    std::fs::read_to_string("Cargo.toml")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("name"))
                .and_then(|l| l.split('"').nth(1))
                .map(|n| n.replace('-', "_"))
        })
        .unwrap_or_else(|| "crate_name".to_string())
}

fn file_to_module_path(file: &str) -> String {
    file.strip_prefix("src/")
        .unwrap_or(file)
        .strip_suffix(".rs")
        .unwrap_or(file)
        .replace('/', "::")
        .replace("mod", "")
        .trim_end_matches("::")
        .to_string()
}

fn strip_markdown_fences(s: &str) -> String {
    // Extract code from within ```rust ... ``` blocks, even if there's text around them
    if let Some(start) = s.find("```rust").or_else(|| s.find("```rs")).or_else(|| s.find("```")) {
        let after_fence = &s[start..];
        let code_start = after_fence.find('\n').map_or(0, |p| p + 1);
        let code_section = &after_fence[code_start..];
        if let Some(end) = code_section.find("```") {
            return code_section[..end].trim().to_string();
        }
        return code_section.trim().to_string();
    }
    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markdown_fences_plain() {
        assert_eq!(strip_markdown_fences("fn main() {}"), "fn main() {}");
    }

    #[test]
    fn test_strip_markdown_fences_rust_block() {
        let input = "```rust\nfn main() {}\n```";
        assert_eq!(strip_markdown_fences(input), "fn main() {}");
    }

    #[test]
    fn test_strip_markdown_fences_with_preamble() {
        let input = "Here are the tests:\n```rust\nfn test() {}\n```\n";
        assert_eq!(strip_markdown_fences(input), "fn test() {}");
    }

    #[test]
    fn test_strip_markdown_fences_generic_block() {
        let input = "```\nfn main() {}\n```";
        assert_eq!(strip_markdown_fences(input), "fn main() {}");
    }

    #[test]
    fn test_strip_markdown_fences_rs_block() {
        let input = "```rs\nfn main() {}\n```";
        assert_eq!(strip_markdown_fences(input), "fn main() {}");
    }

    #[test]
    fn test_detect_crate_name() {
        let name = detect_crate_name();
        assert_eq!(name, "cargo_syntax");
    }

    #[test]
    fn test_file_to_module_path_simple() {
        assert_eq!(file_to_module_path("src/tokens.rs"), "tokens");
    }

    #[test]
    fn test_file_to_module_path_nested() {
        assert_eq!(file_to_module_path("src/commands/ci.rs"), "commands::ci");
    }

    #[test]
    fn test_file_to_module_path_no_src_prefix() {
        assert_eq!(file_to_module_path("lib.rs"), "lib");
    }

    #[test]
    fn test_default_test_path() {
        assert_eq!(default_test_path("src/tokens.rs"), "tests/test_tokens.rs");
    }

    #[test]
    fn test_default_test_path_nested() {
        assert_eq!(default_test_path("src/commands/ci.rs"), "tests/test_ci.rs");
    }
}
