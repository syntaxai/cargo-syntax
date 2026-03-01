use anyhow::Result;

use crate::tokens;

pub fn run(n: usize) -> Result<()> {
    let stats = tokens::scan_project_sorted()?;

    let show = n.min(stats.files.len());

    println!("Top {show} most token-heavy files:");
    println!();
    println!(
        "{:<4} {:<50} {:>6} {:>8} {:>6} {:>7}",
        "#", "File", "Lines", "Tokens", "T/L", "% Tot"
    );
    println!("{}", "-".repeat(84));

    for (i, f) in stats.files.iter().take(show).enumerate() {
        let pct = tokens::pct(f.tokens, stats.total_tokens);
        println!(
            "{:<4} {:<50} {:>6} {:>8} {:>5.1} {:>6.1}%",
            i + 1,
            f.path,
            f.lines,
            f.tokens,
            f.ratio,
            pct
        );
    }

    let top_tokens: usize = stats.files.iter().take(show).map(|f| f.tokens).sum();
    let top_pct = tokens::pct(top_tokens, stats.total_tokens);

    println!("{}", "-".repeat(84));
    println!("Top {show} = {top_tokens} tokens ({top_pct:.1}% of {} total)", stats.total_tokens);

    Ok(())
}
