use anyhow::Result;

use crate::tokens;

pub fn run() -> Result<()> {
    let total_tokens = tokens::count_src_tokens()?;
    let total_lines = tokens::count_src_lines()?;
    let ratio = if total_lines > 0 { total_tokens as f64 / total_lines as f64 } else { 0.0 };

    let (grade, color) = match ratio {
        r if r <= 5.0 => ("A%2B", "brightgreen"),
        r if r <= 7.0 => ("A", "green"),
        r if r <= 9.0 => ("B", "blue"),
        r if r <= 12.0 => ("C", "orange"),
        _ => ("D", "red"),
    };

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
