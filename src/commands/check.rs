use std::process::Command;

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    println!("Running clippy...");
    let clippy =
        Command::new("cargo").args(["clippy", "--all-targets", "--", "-D", "warnings"]).status()?;

    println!("Running fmt check...");
    let fmt = Command::new("cargo").args(["fmt", "--check"]).status()?;

    if !clippy.success() || !fmt.success() {
        bail!("check failed — run `cargo syntax fix` to auto-fix");
    }

    println!("All checks passed.");
    Ok(())
}
