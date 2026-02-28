use anyhow::Result;
use tiktoken_rs::o200k_base;
use walkdir::WalkDir;

pub fn count_src_tokens() -> Result<usize> {
    let bpe = o200k_base()?;
    let mut total = 0usize;

    for entry in WalkDir::new("src")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
    {
        let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
        total += bpe.encode_with_special_tokens(&content).len();
    }

    Ok(total)
}
