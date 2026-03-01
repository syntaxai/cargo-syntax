use std::process::Command;

use anyhow::{Context, Result};

use crate::tokens;

pub fn run() -> Result<()> {
    let before = {
        let stats = tokens::scan_project()?;
        stats.total_tokens
    };

    println!("Running clippy --fix...");
    Command::new("cargo")
        .args(["clippy", "--fix", "--allow-dirty", "--allow-no-vcs"])
        .status()
        .context("failed to run cargo clippy --fix")?;

    println!("Running fmt...");
    Command::new("cargo").args(["fmt"]).status().context("failed to run cargo fmt")?;

    let after = {
        let stats = tokens::scan_project()?;
        stats.total_tokens
    };
    let diff = before as isize - after as isize;

    println!();
    println!("Tokens before: {before}");
    println!("Tokens after:  {after}");

    if diff > 0 {
        let pct = (diff as f64 / before as f64) * 100.0;
        println!("Saved:         {diff} tokens ({pct:.1}%)");
    } else if diff < 0 {
        println!("Added:         {} tokens (formatting may add whitespace)", diff.unsigned_abs());
    } else {
        println!("No token change — code was already optimal.");
    }

    Ok(())
}
