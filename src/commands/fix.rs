use std::process::Command;

use anyhow::Result;

use crate::tokens;

pub fn run() -> Result<()> {
    let before = tokens::count_src_tokens()?;

    println!("Running clippy --fix...");
    Command::new("cargo").args(["clippy", "--fix", "--allow-dirty", "--allow-no-vcs"]).status()?;

    println!("Running fmt...");
    Command::new("cargo").args(["fmt"]).status()?;

    let after = tokens::count_src_tokens()?;
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
