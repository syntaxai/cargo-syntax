use std::path::Path;

use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::json;

use crate::{openrouter, tokens};

const FILE_PROMPT: &str = "\
You are a Rust code explainer for developer onboarding. \
Given a Rust source file, explain what it does clearly and concisely. \
Focus on purpose, key types/functions, and how it fits into a project. \
Be brief — developers want to understand quickly, not read an essay.";

const PROJECT_PROMPT: &str = "\
You are a Rust project explainer for developer onboarding. \
Given a list of all source files with their sizes and contents, \
explain the project architecture: what it does, how modules connect, \
and where a new developer should start reading. Be concise.";

#[derive(Deserialize)]
struct FileExplanation {
    purpose: String,
    key_items: Vec<KeyItem>,
    depends_on: Vec<String>,
}

#[derive(Deserialize)]
struct KeyItem {
    name: String,
    kind: String,
    description: String,
}

#[derive(Deserialize)]
struct ProjectExplanation {
    summary: String,
    modules: Vec<ModuleInfo>,
    start_here: String,
}

#[derive(Deserialize)]
struct ModuleInfo {
    path: String,
    purpose: String,
}

fn file_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "purpose": {
                "type": "string",
                "description": "One-sentence summary of what this file does"
            },
            "key_items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Name of the function/struct/enum/trait" },
                        "kind": { "type": "string", "description": "One of: function, struct, enum, trait, const, macro" },
                        "description": { "type": "string", "description": "What it does in one sentence" }
                    },
                    "required": ["name", "kind", "description"],
                    "additionalProperties": false
                }
            },
            "depends_on": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Key crate or module dependencies"
            }
        },
        "required": ["purpose", "key_items", "depends_on"],
        "additionalProperties": false
    })
}

fn project_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "summary": {
                "type": "string",
                "description": "2-3 sentence project summary"
            },
            "modules": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "purpose": { "type": "string", "description": "One sentence" }
                    },
                    "required": ["path", "purpose"],
                    "additionalProperties": false
                }
            },
            "start_here": {
                "type": "string",
                "description": "Which file(s) to read first and why"
            }
        },
        "required": ["summary", "modules", "start_here"],
        "additionalProperties": false
    })
}

pub fn run(path: &str, model: &str) -> Result<()> {
    let p = Path::new(path);

    if p.is_file() {
        explain_file(path, model)
    } else if p.is_dir() {
        explain_project(model)
    } else {
        bail!("Path not found: {path}")
    }
}

fn explain_file(file: &str, model: &str) -> Result<()> {
    let (content, token_count, lines) = tokens::read_rs_file(file)?;

    println!("Explaining {file} ({lines} lines, {token_count} tokens) via {model}...");
    eprint!("  analyzing... ");

    let result = openrouter::chat_json::<FileExplanation>(
        model,
        FILE_PROMPT,
        &content,
        "file_explanation",
        file_schema(),
    )?;
    eprintln!("done");

    println!();
    println!("  {}", result.purpose);
    println!();

    if !result.key_items.is_empty() {
        println!("  Key items:");
        for item in &result.key_items {
            println!("    {} ({}) — {}", item.name, item.kind, item.description);
        }
        println!();
    }

    if !result.depends_on.is_empty() {
        println!("  Dependencies: {}", result.depends_on.join(", "));
    }

    Ok(())
}

fn explain_project(model: &str) -> Result<()> {
    let stats = tokens::scan_project()?;

    if stats.files.is_empty() {
        bail!("No .rs files found in project");
    }

    println!(
        "Explaining project ({} files, {} tokens) via {model}...",
        stats.files.len(),
        stats.total_tokens
    );

    let mut manifest = String::new();
    for f in &stats.files {
        manifest
            .push_str(&format!("--- {} ({} lines, {} tokens) ---\n", f.path, f.lines, f.tokens));
        let preview: String = f.content.lines().take(30).collect::<Vec<_>>().join("\n");
        manifest.push_str(&preview);
        manifest.push_str("\n\n");
    }

    eprint!("  analyzing... ");

    let result = openrouter::chat_json::<ProjectExplanation>(
        model,
        PROJECT_PROMPT,
        &manifest,
        "project_explanation",
        project_schema(),
    )?;
    eprintln!("done");

    println!();
    println!("  {}", result.summary);
    println!();

    println!("  Modules:");
    for m in &result.modules {
        println!("    {:<40} {}", m.path, m.purpose);
    }

    println!();
    println!("  Start here: {}", result.start_here);

    Ok(())
}
