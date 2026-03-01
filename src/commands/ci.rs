use anyhow::Result;
use serde::Serialize;

use crate::tokens;

#[derive(Serialize)]
struct CiOutput {
    files: usize,
    total_tokens: usize,
    total_lines: usize,
    ratio: f64,
    grade: String,
    pass: bool,
    failures: Vec<String>,
}

pub fn run(
    max_tokens: Option<usize>,
    max_tl: Option<f64>,
    min_grade: Option<&str>,
    json: bool,
) -> Result<()> {
    let stats = tokens::scan_project()?;
    let avg_ratio = tokens::ratio(stats.total_tokens, stats.total_lines);

    let (_, _, grade) = tokens::efficiency_grade(avg_ratio);
    let mut failures: Vec<String> = Vec::new();

    if let Some(max) = max_tokens
        && stats.total_tokens > max
    {
        failures.push(format!("token budget exceeded: {} > {max} (max)", stats.total_tokens));
    }

    if let Some(max) = max_tl
        && avg_ratio > max
    {
        failures.push(format!("T/L ratio too high: {avg_ratio:.1} > {max:.1} (max)"));
    }

    if let Some(min) = min_grade
        && grade_rank(grade) < grade_rank(min)
    {
        failures.push(format!("grade too low: {grade} < {min} (minimum)"));
    }

    if json {
        print_json(&stats, avg_ratio, grade, &failures);
    } else {
        print_human(&stats, avg_ratio, grade, &failures);
    }

    if failures.is_empty() {
        Ok(())
    } else {
        std::process::exit(1);
    }
}

fn grade_rank(grade: &str) -> u8 {
    match grade {
        "A+" => 5,
        "A" => 4,
        "B" => 3,
        "C" => 2,
        "D" => 1,
        _ => 0,
    }
}

fn print_json(stats: &tokens::ProjectStats, avg_ratio: f64, grade: &str, failures: &[String]) {
    let output = CiOutput {
        files: stats.files.len(),
        total_tokens: stats.total_tokens,
        total_lines: stats.total_lines,
        ratio: (avg_ratio * 100.0).round() / 100.0,
        grade: grade.to_string(),
        pass: failures.is_empty(),
        failures: failures.to_vec(),
    };
    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
}

fn print_human(stats: &tokens::ProjectStats, avg_ratio: f64, grade: &str, failures: &[String]) {
    println!(
        "cargo syntax ci: {} files, {} tokens, {:.1} T/L, grade {grade}",
        stats.files.len(),
        stats.total_tokens,
        avg_ratio
    );

    if failures.is_empty() {
        println!("PASS");
    } else {
        println!();
        for f in failures {
            println!("  FAIL: {f}");
        }
        println!();
        println!("FAILED ({} check(s))", failures.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grade_rank_values() {
        for (grade, expected) in
            [("A+", 5), ("A", 4), ("B", 3), ("C", 2), ("D", 1), ("X", 0), ("", 0)]
        {
            assert_eq!(grade_rank(grade), expected, "grade_rank({grade:?})");
        }
    }

    #[test]
    fn test_grade_rank_ordering() {
        let grades = ["A+", "A", "B", "C", "D", "X"];
        for w in grades.windows(2) {
            assert!(grade_rank(w[0]) > grade_rank(w[1]), "{} should rank above {}", w[0], w[1]);
        }
    }
}
