use anyhow::Result;

use crate::tokens;

pub fn run() -> Result<()> {
    let stats = tokens::scan_project()?;

    println!("{:<60} {:>6} {:>8} {:>6}", "File", "Lines", "Tokens", "T/L");
    println!("{}", "-".repeat(83));

    for f in &stats.files {
        println!("{:<60} {:>6} {:>8} {:>5.1}", f.path, f.lines, f.tokens, f.ratio);
    }

    let avg_ratio = tokens::ratio(stats.total_tokens, stats.total_lines);

    println!("{}", "-".repeat(83));
    println!(
        "{:<60} {:>6} {:>8} {:>5.1}",
        "Total", stats.total_lines, stats.total_tokens, avg_ratio
    );

    println!();
    println!(
        "Code: {} | Comments: {} | Blanks: {}",
        stats.code_lines, stats.comment_lines, stats.blank_lines
    );

    println!();
    let (_, _, grade) = tokens::efficiency_grade(avg_ratio);
    let msg = match grade {
        "A+" => "Excellent — extremely token-efficient",
        "A" => "Great — lean and concise code",
        "B" => "Good — some room for improvement",
        "C" => "Fair — consider running `cargo syntax fix`",
        _ => "Verbose — run `cargo syntax fix` to reduce tokens",
    };
    println!("Token efficiency: {grade} ({avg_ratio:.1} tokens/line)");
    println!("{msg}");

    Ok(())
}
