use anyhow::Result;

use crate::tokens;

pub fn run() -> Result<()> {
    let stats = tokens::scan_project()?;
    let ratio = if stats.total_lines > 0 {
        stats.total_tokens as f64 / stats.total_lines as f64
    } else {
        0.0
    };

    let (grade, color, _) = tokens::efficiency_grade(ratio);

    let badge_url = format!(
        "https://img.shields.io/badge/token_efficiency-{grade}%20({ratio:.1}%20T/L)-{color}"
    );
    let link = "https://github.com/syntaxai/cargo-syntax";

    println!("Markdown:");
    println!("[![Token Efficiency]({badge_url})]({link})");
    println!();
    println!("HTML:");
    println!("<a href=\"{link}\"><img src=\"{badge_url}\" alt=\"Token Efficiency\"></a>");
    println!();
    println!("reStructuredText:");
    println!(".. image:: {badge_url}");
    println!("   :target: {link}");
    println!("   :alt: Token Efficiency");

    Ok(())
}
