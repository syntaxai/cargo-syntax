use std::process::Command;

use anyhow::{Result, bail};

use crate::tokens;

struct BranchStats {
    name: String,
    files: usize,
    tokens: usize,
    lines: usize,
}

pub fn run(branch: &str) -> Result<()> {
    let current = current_branch()?;

    println!("Comparing token efficiency: {current} vs {branch}\n");

    let current_stats = {
        let stats = tokens::scan_project()?;
        BranchStats {
            name: current,
            files: stats.files.len(),
            tokens: stats.total_tokens,
            lines: stats.total_lines,
        }
    };

    let target_stats = {
        let rev = tokens::count_rev_tokens(branch)?;
        BranchStats {
            name: branch.to_string(),
            files: rev.files,
            tokens: rev.tokens,
            lines: rev.lines,
        }
    };

    let cur_ratio = tokens::ratio(current_stats.tokens, current_stats.lines);
    let tgt_ratio = tokens::ratio(target_stats.tokens, target_stats.lines);

    let (_, _, cur_grade) = tokens::efficiency_grade(cur_ratio);
    let (_, _, tgt_grade) = tokens::efficiency_grade(tgt_ratio);

    println!("{:<20} {:>10} {:>10} {:>10}", "", &current_stats.name, &target_stats.name, "Delta");
    tokens::separator(52);
    print_row("Files", current_stats.files, target_stats.files);
    print_row("Lines", current_stats.lines, target_stats.lines);
    print_row("Tokens", current_stats.tokens, target_stats.tokens);
    println!(
        "{:<20} {:>10.1} {:>10.1} {:>+10.1}",
        "T/L ratio",
        cur_ratio,
        tgt_ratio,
        cur_ratio - tgt_ratio
    );
    println!("{:<20} {:>10} {:>10}", "Grade", cur_grade, tgt_grade);

    let token_delta = current_stats.tokens as isize - target_stats.tokens as isize;
    let ratio_delta = cur_ratio - tgt_ratio;

    println!();
    if ratio_delta < -0.1 {
        println!("Current branch is more token-efficient (lower T/L ratio)");
    } else if ratio_delta > 0.1 {
        println!("Current branch is less token-efficient (higher T/L ratio)");
    } else {
        println!("Both branches have similar token efficiency (T/L ratio within 0.1)");
    }

    if token_delta != 0 {
        let sign = if token_delta > 0 { "+" } else { "" };
        println!(
            "Token delta: {sign}{token_delta} ({sign}{:.1}%)",
            pct(token_delta, target_stats.tokens)
        );
    }

    Ok(())
}

fn print_row(label: &str, cur: usize, tgt: usize) {
    let delta = cur as isize - tgt as isize;
    println!("{label:<20} {cur:>10} {tgt:>10} {delta:>+10}");
}

fn pct(delta: isize, base: usize) -> f64 {
    if base > 0 { (delta as f64 / base as f64) * 100.0 } else { 0.0 }
}

fn current_branch() -> Result<String> {
    let output = Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"]).output()?;
    if !output.status.success() {
        bail!("git rev-parse failed — are you in a git repository?");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
