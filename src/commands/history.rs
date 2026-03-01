use std::process::Command;

use anyhow::{Result, bail};
use tiktoken_rs::o200k_base;

struct CommitStats {
    hash: String,
    message: String,
    files: usize,
    tokens: usize,
    lines: usize,
}

pub fn run(n: usize) -> Result<()> {
    let output = Command::new("git").args(["log", "--oneline", "-n", &n.to_string()]).output()?;

    if !output.status.success() {
        bail!("git log failed — are you in a git repository?");
    }

    let log = String::from_utf8_lossy(&output.stdout);
    let commits: Vec<(&str, &str)> = log.lines().filter_map(|line| line.split_once(' ')).collect();

    if commits.is_empty() {
        bail!("No commits found");
    }

    println!("Scanning {} commits for token trends...\n", commits.len());

    let bpe = o200k_base()?;
    let mut snapshots: Vec<CommitStats> = Vec::new();

    for (hash, msg) in &commits {
        let rs_files = list_rs_files(hash)?;
        let mut total_tokens = 0;
        let mut total_lines = 0;

        for file in &rs_files {
            if let Ok(content) = show_file(hash, file) {
                total_tokens += bpe.encode_with_special_tokens(&content).len();
                total_lines += content.lines().count();
            }
        }

        snapshots.push(CommitStats {
            hash: hash.to_string(),
            message: msg.to_string(),
            files: rs_files.len(),
            tokens: total_tokens,
            lines: total_lines,
        });
    }

    // Print newest-last so the trend reads chronologically
    println!(
        "{:<10} {:>5} {:>8} {:>6} {:>6}  Message",
        "Commit", "Files", "Tokens", "Lines", "T/L"
    );
    println!("{}", "─".repeat(75));

    for s in snapshots.iter().rev() {
        let ratio = if s.lines > 0 { s.tokens as f64 / s.lines as f64 } else { 0.0 };
        println!(
            "{:<10} {:>5} {:>8} {:>6} {:>5.1}  {}",
            s.hash,
            s.files,
            s.tokens,
            s.lines,
            ratio,
            truncate(&s.message, 30)
        );
    }

    // Show delta between oldest and newest
    if snapshots.len() >= 2 {
        let newest = &snapshots[0];
        let oldest = snapshots.last().unwrap();
        let delta = newest.tokens as isize - oldest.tokens as isize;
        let sign = if delta >= 0 { "+" } else { "" };

        println!();
        println!(
            "Trend: {sign}{delta} tokens ({sign}{:.1}%) over {} commits",
            if oldest.tokens > 0 { (delta as f64 / oldest.tokens as f64) * 100.0 } else { 0.0 },
            snapshots.len()
        );
    }

    Ok(())
}

fn list_rs_files(commit: &str) -> Result<Vec<String>> {
    let output = Command::new("git").args(["ls-tree", "-r", "--name-only", commit]).output()?;

    if !output.status.success() {
        bail!("git ls-tree failed for {commit}");
    }

    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|f| f.ends_with(".rs") && !f.starts_with("target/"))
        .map(String::from)
        .collect();

    Ok(files)
}

fn show_file(commit: &str, file: &str) -> Result<String> {
    let output = Command::new("git").args(["show", &format!("{commit}:{file}")]).output()?;

    if !output.status.success() {
        bail!("git show failed for {commit}:{file}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max - 3]) }
}
