use std::process::Command;

use anyhow::Result;

pub fn run() -> Result<()> {
    println!("Running clippy --fix...");
    Command::new("cargo").args(["clippy", "--fix", "--allow-dirty", "--allow-no-vcs"]).status()?;

    println!("Running fmt...");
    Command::new("cargo").args(["fmt"]).status()?;

    println!("Done.");
    Ok(())
}
