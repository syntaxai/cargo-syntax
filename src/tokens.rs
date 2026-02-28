use anyhow::Result;
use tiktoken_rs::o200k_base;
use walkdir::WalkDir;

pub fn rust_file_walker() -> impl Iterator<Item = walkdir::DirEntry> {
    WalkDir::new(".")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.path().components().any(|c| c.as_os_str() == "target"))
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
}

pub fn count_src_tokens() -> Result<usize> {
    let bpe = o200k_base()?;
    let mut total = 0usize;

    for entry in rust_file_walker() {
        let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
        total += bpe.encode_with_special_tokens(&content).len();
    }

    Ok(total)
}

pub fn count_src_lines() -> Result<usize> {
    let mut total = 0usize;

    for entry in rust_file_walker() {
        let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
        total += content.lines().count();
    }

    Ok(total)
}
