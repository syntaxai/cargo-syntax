use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::templates;

pub fn run() -> Result<()> {
    let cargo_toml = Path::new("Cargo.toml");
    if !cargo_toml.exists() {
        bail!("no Cargo.toml found — run this from a Rust project root");
    }

    let content = fs::read_to_string(cargo_toml)?;
    if content.contains("[lints.clippy]") {
        println!("Cargo.toml already has [lints.clippy] — skipping lints.");
    } else {
        let mut content = content;
        content.push_str(templates::CARGO_LINTS);
        fs::write(cargo_toml, content).context("failed to write Cargo.toml")?;
        println!("Added clippy lints to Cargo.toml");
    }

    write_if_missing("rustfmt.toml", templates::RUSTFMT_TOML)?;
    write_if_missing("clippy.toml", templates::CLIPPY_TOML)?;
    write_if_missing("rust-toolchain.toml", templates::RUST_TOOLCHAIN_TOML)?;
    write_if_missing("CLAUDE.md", templates::CLAUDE_MD)?;

    let gitignore = Path::new(".gitignore");
    if gitignore.exists() {
        let existing = fs::read_to_string(gitignore)?;
        if !existing.contains("**/target") {
            let mut merged = existing;
            merged.push_str("\n# Added by cargo-syntax\n");
            merged.push_str(templates::GITIGNORE);
            fs::write(gitignore, merged)?;
            println!("Appended to .gitignore");
        } else {
            println!(".gitignore already covers target/ — skipping.");
        }
    } else {
        fs::write(gitignore, templates::GITIGNORE)?;
        println!("Created .gitignore");
    }

    println!();
    println!("Done! Run `cargo syntax check` to verify.");

    Ok(())
}

fn write_if_missing(name: &str, content: &str) -> Result<()> {
    let path = Path::new(name);
    if path.exists() {
        println!("{name} already exists — skipping.");
    } else {
        fs::write(path, content).with_context(|| format!("failed to write {name}"))?;
        println!("Created {name}");
    }
    Ok(())
}
