use std::process::Command;

use anyhow::{Result, bail};
use tiktoken_rs::o200k_base;

use crate::tokens;

struct BranchStats {
    name: String,
    files: usize,
    tokens: usize,
    lines: usize,
}

pub fn run(branch: &str) -> Result<()> {
    let current = current_branch()?;
    let bpe = o200k_base()?;

    println!("Comparing token efficiency: {current} vs {branch}\n");

    // Scan current branch (use filesystem directly)
    let current_stats = {
        let stats = tokens::scan_project()?;
        BranchStats {
            name: current,
            files: stats.files.len(),
            tokens: stats.total_tokens,
            lines: stats.total_lines,
        }
    };

    // Scan target branch via git
    let target_stats = {
        let rs_files = list_rs_files(branch)?;
        let mut total_tokens = 0;
        let mut total_lines = 0;

        for file in &rs_files {
            if let Ok(content) = show_file(branch, file) {
                total_tokens += bpe.encode_with_special_tokens(&content).len();
                total_lines += content.lines().count();
            }
        }

        BranchStats {
            name: branch.to_string(),
            files: rs_files.len(),
            tokens: total_tokens,
            lines: total_lines,
        }
    };

    let cur_ratio = if current_stats.lines > 0 {
        current_stats.tokens as f64 / current_stats.lines as f64
    } else {
        0.0
    };
    let tgt_ratio = if target_stats.lines > 0 {
        target_stats.tokens as f64 / target_stats.lines as f64
    } else {
        0.0
    };

    let (_, _, cur_grade) = tokens::efficiency_grade(cur_ratio);
    let (_, _, tgt_grade) = tokens::efficiency_grade(tgt_ratio);

    println!("{:<20} {:>10} {:>10}", "", &current_stats.name, &target_stats.name);
    println!("{}", "─".repeat(42));
    println!("{:<20} {:>10} {:>10}", "Files", current_stats.files, target_stats.files);
    println!("{:<20} {:>10} {:>10}", "Lines", current_stats.lines, target_stats.lines);
    println!("{:<20} {:>10} {:>10}", "Tokens", current_stats.tokens, target_stats.tokens);
    println!("{:<20} {:>10.1} {:>10.1}", "T/L ratio", cur_ratio, tgt_ratio);
    println!("{:<20} {:>10} {:>10}", "Grade", cur_grade, tgt_grade);

    let delta = current_stats.tokens as isize - target_stats.tokens as isize;
    let sign = if delta >= 0 { "+" } else { "" };

    println!();
    if delta < 0 {
        println!(
            "Current branch uses {} fewer tokens ({:.1}% more efficient)",
            -delta,
            if target_stats.tokens > 0 {
                (-delta as f64 / target_stats.tokens as f64) * 100.0
            } else {
                0.0
            }
        );
    } else if delta > 0 {
        println!(
            "Current branch uses {sign}{delta} more tokens ({:.1}% less efficient)",
            if target_stats.tokens > 0 {
                (delta as f64 / target_stats.tokens as f64) * 100.0
            } else {
                0.0
            }
        );
    } else {
        println!("Both branches have identical token counts");
    }

    Ok(())
}

fn current_branch() -> Result<String> {
    let output = Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"]).output()?;

    if !output.status.success() {
        bail!("git rev-parse failed — are you in a git repository?");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn list_rs_files(branch: &str) -> Result<Vec<String>> {
    let output = Command::new("git").args(["ls-tree", "-r", "--name-only", branch]).output()?;

    if !output.status.success() {
        bail!("Branch not found: {branch}");
    }

    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|f| f.ends_with(".rs") && !f.starts_with("target/"))
        .map(String::from)
        .collect();

    Ok(files)
}

fn show_file(branch: &str, file: &str) -> Result<String> {
    let output = Command::new("git").args(["show", &format!("{branch}:{file}")]).output()?;

    if !output.status.success() {
        bail!("git show failed for {branch}:{file}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
