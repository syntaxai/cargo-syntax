use std::process::Command;

use anyhow::{Context, Result};

use crate::tokens;

pub fn run(n: usize, validate: bool, auto: bool, model: &str) -> Result<()> {
    let mut stats = tokens::scan_project()?;
    stats.files.sort_by(|a, b| b.tokens.cmp(&a.tokens));

    let count = n.min(stats.files.len());

    println!("Batch rewriting top {count} files via {model}...");
    if validate {
        println!("  Validation: cargo check + cargo test after each rewrite");
    }
    if auto && !validate {
        println!("  WARNING: --auto without --validate accepts all rewrites blindly");
    }
    if auto {
        println!("  Auto-apply: skipping interactive prompts");
    }
    println!();

    let mut rewritten = 0;
    let mut skipped = 0;
    let mut failed = 0;
    let mut total_saved: isize = 0;

    for (i, f) in stats.files.iter().take(count).enumerate() {
        println!(
            "[{}/{}] {}  ({} tokens, {} lines, T/L: {:.1})",
            i + 1,
            count,
            f.path,
            f.tokens,
            f.lines,
            f.ratio
        );

        eprint!("  rewriting... ");
        let result = match super::rewrite::rewrite_file(&f.path, model) {
            Ok(r) => {
                eprintln!("done");
                r
            }
            Err(e) => {
                eprintln!("failed");
                println!("  Error: {e}");
                failed += 1;
                println!();
                continue;
            }
        };

        let saved = result.saved();
        let pct = if result.tokens_before > 0 {
            (saved as f64 / result.tokens_before as f64) * 100.0
        } else {
            0.0
        };

        if saved <= 0 {
            println!("  No improvement ({saved:+} tokens). Skipping.");
            skipped += 1;
            println!();
            continue;
        }

        println!(
            "  {} → {} tokens (saves {saved}, {pct:.1}%)",
            result.tokens_before, result.tokens_after
        );

        let accepted = if auto { true } else { ask_accept()? };

        if accepted {
            std::fs::write(&f.path, &result.rewritten)?;

            if validate {
                eprint!("  validating... ");
                match run_validation() {
                    Ok(()) => {
                        eprintln!("passed ✓");
                        rewritten += 1;
                        total_saved += saved;
                    }
                    Err(e) => {
                        eprintln!("failed ✗");
                        println!("  {e}");
                        println!("  Rolling back...");
                        std::fs::write(&f.path, &result.original)?;
                        failed += 1;
                    }
                }
            } else {
                println!("  Applied.");
                rewritten += 1;
                total_saved += saved;
            }
        } else {
            println!("  Skipped.");
            skipped += 1;
        }
        println!();
    }

    println!("{}", "─".repeat(70));
    println!("Batch complete: {rewritten} rewritten, {skipped} skipped, {failed} failed");
    if total_saved > 0 {
        let total_pct = if stats.total_tokens > 0 {
            (total_saved as f64 / stats.total_tokens as f64) * 100.0
        } else {
            0.0
        };
        println!("Total saved: ~{total_saved} tokens ({total_pct:.1}% of project)");
    }

    Ok(())
}

fn ask_accept() -> Result<bool> {
    use std::io::{self, BufRead, Write};
    print!("  Accept? [y/n] ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "Y"))
}

fn run_validation() -> Result<()> {
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .output()
        .context("failed to run cargo check")?;

    if !check.status.success() {
        let stderr = String::from_utf8_lossy(&check.stderr);
        anyhow::bail!("cargo check: {}", stderr.lines().next().unwrap_or("failed"));
    }

    let test = Command::new("cargo")
        .args(["test", "--quiet"])
        .output()
        .context("failed to run cargo test")?;

    if !test.status.success() {
        let stderr = String::from_utf8_lossy(&test.stderr);
        anyhow::bail!("cargo test: {}", stderr.lines().next().unwrap_or("failed"));
    }

    Ok(())
}
