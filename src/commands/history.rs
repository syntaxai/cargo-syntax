use std::process::Command;

use anyhow::{Result, bail};

use crate::tokens;

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

    let mut snapshots: Vec<CommitStats> = Vec::new();

    for (hash, msg) in &commits {
        let rev = tokens::count_rev_tokens(hash)?;
        snapshots.push(CommitStats {
            hash: hash.to_string(),
            message: msg.to_string(),
            files: rev.files,
            tokens: rev.tokens,
            lines: rev.lines,
        });
    }

    println!(
        "{:<10} {:>5} {:>8} {:>6} {:>6}  Message",
        "Commit", "Files", "Tokens", "Lines", "T/L"
    );
    tokens::separator(75);

    for s in snapshots.iter().rev() {
        let ratio = tokens::ratio(s.tokens, s.lines);
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

    if snapshots.len() >= 2 {
        let newest = &snapshots[0];
        let oldest = snapshots.last().unwrap();
        let delta = newest.tokens as isize - oldest.tokens as isize;
        let sign = if delta >= 0 { "+" } else { "" };

        println!();
        println!(
            "Trend: {sign}{delta} tokens ({sign}{:.1}%) over {} commits",
            tokens::pct_delta(delta, oldest.tokens),
            snapshots.len()
        );
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max - 3]) }
}
