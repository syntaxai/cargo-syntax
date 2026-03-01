use std::collections::HashMap;

use crate::tokens::{self, ProjectStats};

const WINDOW_SIZE: usize = 3;

struct Fingerprint {
    file_idx: usize,
    start_line: usize,
}

pub struct DuplicateCluster {
    pub occurrences: Vec<(usize, usize, usize)>, // (file_idx, start, end)
    pub preview: String,
    pub tokens_per_instance: usize,
}

pub struct NearDuplicate {
    pub file_idx: usize,
    pub fn_a: (String, usize),
    pub fn_b: (String, usize),
    pub savings: usize,
}

pub struct DeepResult {
    pub clusters: Vec<DuplicateCluster>,
    pub near_dupes: Vec<NearDuplicate>,
    pub total_savings: usize,
}

pub fn run(stats: &ProjectStats) -> DeepResult {
    let normalized: Vec<Vec<String>> =
        stats.files.iter().map(|f| f.content.lines().map(normalize_line).collect()).collect();

    let clusters = find_duplicate_blocks(&normalized, stats);
    let near_dupes = find_near_duplicates(stats);

    let total_savings: usize = clusters.iter().map(estimate_savings).sum::<usize>()
        + near_dupes.iter().map(|n| n.savings).sum::<usize>();

    DeepResult { clusters, near_dupes, total_savings }
}

pub fn print_results(result: &DeepResult, stats: &ProjectStats) {
    let mut idx = 0;

    if !result.clusters.is_empty() {
        println!("Cross-file duplicates:\n");
        for c in &result.clusters {
            idx += 1;
            let file_count = c.occurrences.len();
            let span = c.occurrences[0].2 - c.occurrences[0].1 + 1;
            println!("  {idx}. {span}-line block duplicated in {file_count} files");

            let preview: String = c.preview.lines().take(2).collect::<Vec<_>>().join(" | ");
            println!("     {preview}");

            let mut locs: Vec<String> = c
                .occurrences
                .iter()
                .map(|(fi, start, _)| format!("{}:{}", stats.files[*fi].path, start + 1))
                .collect();

            if locs.len() > 3 {
                let rest = locs.len() - 2;
                locs.truncate(2);
                println!("     Files: {}, (+{rest} more)", locs.join(", "));
            } else {
                println!("     Files: {}", locs.join(", "));
            }

            let savings = estimate_savings(c);
            println!("     Saves: ~{savings} tokens\n");
        }
    }

    if !result.near_dupes.is_empty() {
        println!("Near-duplicate functions:\n");
        for nd in &result.near_dupes {
            idx += 1;
            let file = &stats.files[nd.file_idx].path;
            println!("  {idx}. {} ≈ {} (differ by ~{} tokens)", nd.fn_a.0, nd.fn_b.0, nd.savings);
            println!("     File: {file}:{}, :{}", nd.fn_a.1 + 1, nd.fn_b.1 + 1);
            println!("     Saves: ~{} tokens\n", nd.savings);
        }
    }

    tokens::separator(70);
    let pattern_count = result.clusters.len() + result.near_dupes.len();
    let save_pct = tokens::pct(result.total_savings, stats.total_tokens);
    println!(
        "Deep analysis: {pattern_count} pattern(s), ~{} tokens saveable ({save_pct:.1}% of project)",
        result.total_savings
    );
}

fn normalize_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn find_duplicate_blocks(
    normalized: &[Vec<String>],
    stats: &ProjectStats,
) -> Vec<DuplicateCluster> {
    // Build fingerprints: hash of WINDOW_SIZE consecutive non-blank normalized lines
    let mut map: HashMap<u64, Vec<Fingerprint>> = HashMap::new();

    for (file_idx, lines) in normalized.iter().enumerate() {
        let non_blank: Vec<(usize, &str)> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.is_empty())
            .map(|(i, l)| (i, l.as_str()))
            .collect();

        if non_blank.len() < WINDOW_SIZE {
            continue;
        }

        for window in non_blank.windows(WINDOW_SIZE) {
            let combined: String = window.iter().map(|(_, l)| *l).collect::<Vec<_>>().join("\n");
            // Skip trivial windows (single braces, use statements, etc.)
            if combined.len() < 20 {
                continue;
            }
            let hash = hash_str(&combined);
            map.entry(hash).or_default().push(Fingerprint { file_idx, start_line: window[0].0 });
        }
    }

    // Keep only hashes that appear in 2+ different files
    let mut clusters: Vec<DuplicateCluster> = Vec::new();

    for fps in map.values() {
        let unique_files: Vec<usize> = fps
            .iter()
            .map(|f| f.file_idx)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if unique_files.len() < 2 {
            continue;
        }

        // Verify actual text matches (not just hash collision)
        let first_text = get_window_text(normalized, fps[0].file_idx, fps[0].start_line);
        let all_match = fps
            .iter()
            .all(|fp| get_window_text(normalized, fp.file_idx, fp.start_line) == first_text);

        if !all_match {
            continue;
        }

        // Get original (non-normalized) preview from first occurrence
        let preview = get_original_window(&stats.files[fps[0].file_idx].content, fps[0].start_line);

        let tokens_per_instance = tokens::count_tokens(&preview).unwrap_or(0);

        let occurrences: Vec<(usize, usize, usize)> = fps
            .iter()
            .map(|fp| {
                let end = find_window_end(normalized, fp.file_idx, fp.start_line);
                (fp.file_idx, fp.start_line, end)
            })
            .collect();

        clusters.push(DuplicateCluster { occurrences, preview, tokens_per_instance });
    }

    // Deduplicate overlapping clusters: keep the one with more occurrences or more lines
    clusters.sort_by(|a, b| {
        b.occurrences
            .len()
            .cmp(&a.occurrences.len())
            .then(estimate_savings(b).cmp(&estimate_savings(a)))
    });

    // Remove clusters whose occurrences are subsets of a larger cluster
    let mut kept: Vec<DuplicateCluster> = Vec::new();
    for cluster in clusters {
        let dominated = kept.iter().any(|existing| {
            cluster.occurrences.iter().all(|(fi, s, _)| {
                existing.occurrences.iter().any(|(efi, es, ee)| fi == efi && *s >= *es && *s <= *ee)
            })
        });
        if !dominated {
            kept.push(cluster);
        }
    }

    kept
}

fn find_near_duplicates(stats: &ProjectStats) -> Vec<NearDuplicate> {
    let mut results = Vec::new();

    for (file_idx, file) in stats.files.iter().enumerate() {
        let fns = extract_functions(&file.content);

        for i in 0..fns.len() {
            for j in (i + 1)..fns.len() {
                let norm_a = normalize_line(&fns[i].body);
                let norm_b = normalize_line(&fns[j].body);

                // Skip very short functions
                if norm_a.len() < 30 || norm_b.len() < 30 {
                    continue;
                }

                let similarity = string_similarity(&norm_a, &norm_b);
                if similarity > 0.75 && similarity < 1.0 {
                    let tokens_a = tokens::count_tokens(&fns[i].body).unwrap_or(0);
                    let tokens_b = tokens::count_tokens(&fns[j].body).unwrap_or(0);
                    let savings = tokens_a.min(tokens_b).saturating_mul(60) / 100;

                    if savings >= 5 {
                        results.push(NearDuplicate {
                            file_idx,
                            fn_a: (fns[i].name.clone(), fns[i].line),
                            fn_b: (fns[j].name.clone(), fns[j].line),
                            savings,
                        });
                    }
                }
            }
        }
    }

    results.sort_by(|a, b| b.savings.cmp(&a.savings));
    results
}

struct FnInfo {
    name: String,
    line: usize,
    body: String,
}

fn extract_functions(content: &str) -> Vec<FnInfo> {
    let mut fns = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Match fn declarations (pub fn, fn, pub(crate) fn, etc.)
        if let Some(fn_pos) = trimmed.find("fn ") {
            let before = &trimmed[..fn_pos];
            if before.is_empty()
                || before.trim_end().ends_with("pub")
                || before.contains("pub(")
                || before.trim_end().ends_with("async")
                || before.trim_end().ends_with("unsafe")
            {
                let name_start = fn_pos + 3;
                let name_end = trimmed[name_start..]
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .map_or(trimmed.len(), |p| p + name_start);
                let name = trimmed[name_start..name_end].to_string();

                if !name.is_empty() {
                    // Find the opening brace
                    let mut brace_line = i;
                    while brace_line < lines.len() && !lines[brace_line].contains('{') {
                        brace_line += 1;
                    }

                    if brace_line < lines.len() {
                        // Count braces to find end
                        let mut depth = 0;
                        let mut end = brace_line;
                        for (li, line) in lines.iter().enumerate().skip(brace_line) {
                            for ch in line.chars() {
                                if ch == '{' {
                                    depth += 1;
                                }
                                if ch == '}' {
                                    depth -= 1;
                                }
                            }
                            if depth == 0 {
                                end = li;
                                break;
                            }
                        }

                        let body: String = lines[i..=end].join("\n");
                        fns.push(FnInfo { name, line: i, body });
                        i = end + 1;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    fns
}

fn hash_str(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

fn get_window_text(normalized: &[Vec<String>], file_idx: usize, start: usize) -> String {
    let lines = &normalized[file_idx];
    let non_blank: Vec<&str> = lines[start..]
        .iter()
        .filter(|l| !l.is_empty())
        .take(WINDOW_SIZE)
        .map(String::as_str)
        .collect();
    non_blank.join("\n")
}

fn get_original_window(content: &str, start_line: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut collected = 0;
    let mut end = start_line;

    for (i, line) in lines.iter().enumerate().skip(start_line) {
        if !line.trim().is_empty() {
            collected += 1;
        }
        end = i;
        if collected >= WINDOW_SIZE {
            break;
        }
    }

    lines[start_line..=end.min(lines.len() - 1)].join("\n")
}

fn find_window_end(normalized: &[Vec<String>], file_idx: usize, start: usize) -> usize {
    let lines = &normalized[file_idx];
    let mut collected = 0;
    let mut end = start;

    for (i, line) in lines.iter().enumerate().skip(start) {
        if !line.is_empty() {
            collected += 1;
        }
        end = i;
        if collected >= WINDOW_SIZE {
            break;
        }
    }

    end
}

fn estimate_savings(cluster: &DuplicateCluster) -> usize {
    let instances = cluster.occurrences.len();
    if instances <= 1 {
        return 0;
    }
    // Save tokens_per_instance * (instances - 1) * 80%
    cluster.tokens_per_instance * (instances - 1) * 80 / 100
}

fn string_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let words_a: Vec<&str> = a.split_whitespace().collect();
    let words_b: Vec<&str> = b.split_whitespace().collect();

    let matching = words_a.iter().filter(|w| words_b.contains(w)).count();
    let total = words_a.len().max(words_b.len());

    if total == 0 { 0.0 } else { matching as f64 / total as f64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_line() {
        assert_eq!(normalize_line("  fn   main()  { }  "), "fn main() { }");
        assert_eq!(normalize_line(""), "");
        assert_eq!(normalize_line("  "), "");
    }

    #[test]
    fn test_hash_str_consistency() {
        let h1 = hash_str("hello world");
        let h2 = hash_str("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_str_differs() {
        assert_ne!(hash_str("hello"), hash_str("world"));
    }

    #[test]
    fn test_string_similarity_identical() {
        assert_eq!(string_similarity("fn main() {}", "fn main() {}"), 1.0);
    }

    #[test]
    fn test_string_similarity_empty() {
        assert_eq!(string_similarity("", ""), 1.0);
        assert_eq!(string_similarity("hello", ""), 0.0);
    }

    #[test]
    fn test_string_similarity_partial() {
        let sim = string_similarity("fn format_input cost price", "fn format_output cost price");
        assert!(sim > 0.5, "similar strings should score > 0.5, got {sim}");
    }

    #[test]
    fn test_extract_functions_basic() {
        let code = "fn foo() {\n    42\n}\n\npub fn bar(x: i32) -> i32 {\n    x + 1\n}\n";
        let fns = extract_functions(code);
        assert_eq!(fns.len(), 2);
        assert_eq!(fns[0].name, "foo");
        assert_eq!(fns[1].name, "bar");
    }

    #[test]
    fn test_extract_functions_empty() {
        let fns = extract_functions("// no functions\nlet x = 1;\n");
        assert!(fns.is_empty());
    }

    #[test]
    fn test_estimate_savings_single() {
        let c = DuplicateCluster {
            occurrences: vec![(0, 0, 2)],
            preview: String::new(),
            tokens_per_instance: 10,
        };
        assert_eq!(estimate_savings(&c), 0);
    }

    #[test]
    fn test_estimate_savings_multiple() {
        let c = DuplicateCluster {
            occurrences: vec![(0, 0, 2), (1, 5, 7), (2, 10, 12)],
            preview: String::new(),
            tokens_per_instance: 10,
        };
        // 10 * (3 - 1) * 80% = 16
        assert_eq!(estimate_savings(&c), 16);
    }
}
