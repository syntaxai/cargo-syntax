use anyhow::Result;
use tiktoken_rs::o200k_base;
use walkdir::WalkDir;

pub struct FileStats {
    pub path: String,
    pub content: String,
    pub lines: usize,
    pub tokens: usize,
    pub ratio: f64,
}

pub struct ProjectStats {
    pub files: Vec<FileStats>,
    pub total_lines: usize,
    pub total_tokens: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
}

pub fn rust_file_walker() -> impl Iterator<Item = walkdir::DirEntry> {
    WalkDir::new(".")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.path().components().any(|c| c.as_os_str() == "target"))
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
}

pub fn scan_project() -> Result<ProjectStats> {
    let bpe = o200k_base()?;
    let mut files = Vec::new();
    let mut total_lines = 0;
    let mut total_tokens = 0;
    let mut code_lines = 0;
    let mut comment_lines = 0;
    let mut blank_lines = 0;

    for entry in rust_file_walker() {
        let file_path = entry.path();
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: skipping {} ({})", file_path.display(), e);
                continue;
            }
        };

        let tokens = bpe.encode_with_special_tokens(&content).len();
        let lines = content.lines().count();
        let ratio = ratio(tokens, lines);

        let (code, comments, blanks) = count_line_types(&content);
        code_lines += code;
        comment_lines += comments;
        blank_lines += blanks;
        total_lines += lines;
        total_tokens += tokens;

        let display = file_path.strip_prefix(".").unwrap_or(file_path).display().to_string();
        files.push(FileStats { path: display, content, lines, tokens, ratio });
    }

    Ok(ProjectStats { files, total_lines, total_tokens, code_lines, comment_lines, blank_lines })
}

pub fn count_tokens(content: &str) -> Result<usize> {
    let bpe = o200k_base()?;
    Ok(bpe.encode_with_special_tokens(content).len())
}

fn count_line_types(content: &str) -> (usize, usize, usize) {
    let mut code = 0;
    let mut comments = 0;
    let mut blanks = 0;
    let mut in_block_comment = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blanks += 1;
        } else if in_block_comment {
            comments += 1;
            if trimmed.contains("*/") {
                in_block_comment = false;
            }
        } else if trimmed.starts_with("//") {
            comments += 1;
        } else if trimmed.starts_with("/*") {
            comments += 1;
            if !trimmed.contains("*/") {
                in_block_comment = true;
            }
        } else {
            code += 1;
        }
    }

    (code, comments, blanks)
}

pub fn efficiency_grade(ratio: f64) -> (&'static str, &'static str, &'static str) {
    match ratio {
        r if r <= 5.0 => ("A%2B", "brightgreen", "A+"),
        r if r <= 7.0 => ("A", "green", "A"),
        r if r <= 9.0 => ("B", "blue", "B"),
        r if r <= 12.0 => ("C", "orange", "C"),
        _ => ("D", "red", "D"),
    }
}

pub fn ratio(tokens: usize, lines: usize) -> f64 {
    if lines > 0 { tokens as f64 / lines as f64 } else { 0.0 }
}

pub fn pct(part: usize, total: usize) -> f64 {
    if total > 0 { (part as f64 / total as f64) * 100.0 } else { 0.0 }
}

pub fn separator(width: usize) {
    println!("{}", "─".repeat(width));
}

pub fn default_model() -> String {
    std::env::var("CARGO_SYNTAX_MODEL").unwrap_or_else(|_| "deepseek/deepseek-chat".to_string())
}

pub fn ask_accept(prompt: &str) -> Result<String> {
    use std::io::{self, BufRead, Write};
    print!("{prompt} ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn strip_markdown_fences(s: &str) -> String {
    if let Some(start) = s.find("```rust").or_else(|| s.find("```rs")).or_else(|| s.find("```")) {
        let after_fence = &s[start..];
        let code_start = after_fence.find('\n').map_or(0, |p| p + 1);
        let code_section = &after_fence[code_start..];
        if let Some(end) = code_section.find("```") {
            return code_section[..end].trim().to_string();
        }
        return code_section.trim().to_string();
    }
    s.trim().to_string()
}

pub fn build_manifest(stats: &ProjectStats) -> String {
    let mut manifest = String::new();
    for f in &stats.files {
        manifest.push_str(&format!("=== {} ({} tokens) ===\n", f.path, f.tokens));
        manifest.push_str(&f.content);
        manifest.push_str("\n\n");
    }
    manifest
}

pub fn read_rs_file(file: &str) -> Result<(String, usize, usize)> {
    use std::path::Path;
    let path = Path::new(file);
    if !path.exists() {
        anyhow::bail!("File not found: {file}");
    }
    if path.extension().is_none_or(|ext| ext != "rs") {
        anyhow::bail!("Only .rs files are supported");
    }
    let content = std::fs::read_to_string(path)?;
    let tokens = count_tokens(&content)?;
    let lines = content.lines().count();
    Ok((content, tokens, lines))
}

pub struct RevStats {
    pub files: usize,
    pub tokens: usize,
    pub lines: usize,
}

pub fn count_rev_tokens(rev: &str) -> Result<RevStats> {
    let bpe = o200k_base()?;
    let rs_files = git_list_rs_files(rev)?;
    let mut total_tokens = 0;
    let mut total_lines = 0;

    for file in &rs_files {
        if let Ok(content) = git_show_file(rev, file) {
            total_tokens += bpe.encode_with_special_tokens(&content).len();
            total_lines += content.lines().count();
        }
    }

    Ok(RevStats { files: rs_files.len(), tokens: total_tokens, lines: total_lines })
}

/// List .rs files at a specific git ref (commit/branch/tag), excluding target/
pub fn git_list_rs_files(rev: &str) -> Result<Vec<String>> {
    use std::process::Command;
    let output = Command::new("git").args(["ls-tree", "-r", "--name-only", rev]).output()?;
    if !output.status.success() {
        anyhow::bail!("git ls-tree failed for {rev}");
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|f| f.ends_with(".rs") && !f.starts_with("target/"))
        .map(String::from)
        .collect())
}

/// Read a file's content at a specific git ref
pub fn git_show_file(rev: &str, file: &str) -> Result<String> {
    use std::process::Command;
    let output = Command::new("git").args(["show", &format!("{rev}:{file}")]).output()?;
    if !output.status.success() {
        anyhow::bail!("git show failed for {rev}:{file}");
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
