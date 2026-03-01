use anyhow::Result;

use crate::tokens;

pub fn run(
    max_tokens: Option<usize>,
    max_tl: Option<f64>,
    min_grade: Option<&str>,
    json: bool,
) -> Result<()> {
    let stats = tokens::scan_project()?;

    let avg_ratio = if stats.total_lines > 0 {
        stats.total_tokens as f64 / stats.total_lines as f64
    } else {
        0.0
    };

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
    println!("{{");
    println!("  \"files\": {},", stats.files.len());
    println!("  \"total_tokens\": {},", stats.total_tokens);
    println!("  \"total_lines\": {},", stats.total_lines);
    println!("  \"ratio\": {avg_ratio:.2},");
    println!("  \"grade\": \"{grade}\",");
    println!("  \"pass\": {},", failures.is_empty());
    if !failures.is_empty() {
        println!("  \"failures\": [");
        for (i, f) in failures.iter().enumerate() {
            let comma = if i + 1 < failures.len() { "," } else { "" };
            println!("    \"{f}\"{comma}");
        }
        println!("  ]");
    } else {
        println!("  \"failures\": []");
    }
    println!("}}");
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
