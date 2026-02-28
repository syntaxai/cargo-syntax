use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::templates;

pub fn run(name: &str) -> Result<()> {
    let path = Path::new(name);

    if path.exists() {
        bail!("directory '{name}' already exists");
    }

    println!("Creating project '{name}'...");

    let status =
        Command::new("cargo").args(["init", name]).status().context("failed to run cargo init")?;

    if !status.success() {
        bail!("cargo init failed");
    }

    // Append lints to Cargo.toml
    let cargo_toml = path.join("Cargo.toml");
    let mut content = fs::read_to_string(&cargo_toml)?;
    content.push_str(templates::CARGO_LINTS);
    fs::write(&cargo_toml, content)?;

    // Write config files
    fs::write(path.join("rustfmt.toml"), templates::RUSTFMT_TOML)?;
    fs::write(path.join("clippy.toml"), templates::CLIPPY_TOML)?;
    fs::write(path.join("rust-toolchain.toml"), templates::RUST_TOOLCHAIN_TOML)?;
    fs::write(path.join(".gitignore"), templates::GITIGNORE)?;
    fs::write(path.join("CLAUDE.md"), templates::CLAUDE_MD)?;

    println!("Project '{name}' created with token-efficient config.");
    println!();
    println!("  cd {name}");
    println!("  cargo syntax check");

    Ok(())
}
