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
        let ratio = if lines > 0 { tokens as f64 / lines as f64 } else { 0.0 };

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

pub fn default_model() -> String {
    std::env::var("CARGO_SYNTAX_MODEL").unwrap_or_else(|_| "deepseek/deepseek-chat".to_string())
}
