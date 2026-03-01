use anyhow::Result;

use crate::tokens;

pub fn run(n: usize) -> Result<()> {
    let mut stats = tokens::scan_project()?;
    stats.files.sort_by(|a, b| b.tokens.cmp(&a.tokens));

    let show = n.min(stats.files.len());

    println!("Top {show} most token-heavy files:");
    println!();
    println!("{:<4} {:<50} {:>6} {:>8} {:>6} {:>7}", "#", "File", "Lines", "Tokens", "T/L", "% Tot");
    println!("{}", "-".repeat(84));

    for (i, f) in stats.files.iter().take(show).enumerate() {
        let pct = if stats.total_tokens > 0 {
            (f.tokens as f64 / stats.total_tokens as f64) * 100.0
        } else {
            0.0
        };
        println!("{:<4} {:<50} {:>6} {:>8} {:>5.1} {:>6.1}%", i + 1, f.path, f.lines, f.tokens, f.ratio, pct);
    }

    let top_tokens: usize = stats.files.iter().take(show).map(|f| f.tokens).sum();
    let top_pct = if stats.total_tokens > 0 {
        (top_tokens as f64 / stats.total_tokens as f64) * 100.0
    } else {
        0.0
    };

    println!("{}", "-".repeat(84));
    println!("Top {show} = {top_tokens} tokens ({top_pct:.1}% of {} total)", stats.total_tokens);

    Ok(())
}
