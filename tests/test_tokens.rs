use cargo_syntax::tokens::*;

#[test]
fn test_count_tokens_empty_content() {
    assert_eq!(count_tokens("").unwrap(), 0);
}

#[test]
fn test_count_tokens_valid_content() {
    assert!(count_tokens("fn main() {}").unwrap() > 0);
}

#[test]
fn test_count_tokens_multiline() {
    let code = "fn main() {\n    println!(\"hello\");\n}";
    let tokens = count_tokens(code).unwrap();
    assert!(tokens > 5);
}

#[test]
fn test_efficiency_grade_a_plus() {
    assert_eq!(efficiency_grade(4.0), ("A%2B", "brightgreen", "A+"));
}

#[test]
fn test_efficiency_grade_a() {
    assert_eq!(efficiency_grade(6.0), ("A", "green", "A"));
}

#[test]
fn test_efficiency_grade_b() {
    assert_eq!(efficiency_grade(8.0), ("B", "blue", "B"));
}

#[test]
fn test_efficiency_grade_c() {
    assert_eq!(efficiency_grade(11.0), ("C", "orange", "C"));
}

#[test]
fn test_efficiency_grade_d() {
    assert_eq!(efficiency_grade(13.0), ("D", "red", "D"));
}

#[test]
fn test_efficiency_grade_boundary_5() {
    assert_eq!(efficiency_grade(5.0).2, "A+");
}

#[test]
fn test_efficiency_grade_boundary_7() {
    assert_eq!(efficiency_grade(7.0).2, "A");
}

#[test]
fn test_efficiency_grade_boundary_9() {
    assert_eq!(efficiency_grade(9.0).2, "B");
}

#[test]
fn test_efficiency_grade_boundary_12() {
    assert_eq!(efficiency_grade(12.0).2, "C");
}

#[test]
fn test_default_model_fallback() {
    // SAFETY: test runs single-threaded, no other threads read this env var
    unsafe { std::env::remove_var("CARGO_SYNTAX_MODEL") };
    assert_eq!(default_model(), "deepseek/deepseek-chat");
}

#[test]
fn test_default_model_from_env() {
    // SAFETY: test runs single-threaded, no other threads read this env var
    unsafe { std::env::set_var("CARGO_SYNTAX_MODEL", "test/model") };
    assert_eq!(default_model(), "test/model");
    unsafe { std::env::remove_var("CARGO_SYNTAX_MODEL") };
}

#[test]
fn test_scan_project_finds_rs_files() {
    let stats = scan_project().unwrap();
    assert!(!stats.files.is_empty());
    assert!(stats.total_tokens > 0);
    assert!(stats.total_lines > 0);
}

#[test]
fn test_scan_project_no_target_files() {
    let stats = scan_project().unwrap();
    for f in &stats.files {
        assert!(!f.path.contains("target"), "found target file: {}", f.path);
    }
}

#[test]
fn test_scan_project_ratio_positive() {
    let stats = scan_project().unwrap();
    for f in &stats.files {
        assert!(f.ratio > 0.0, "ratio should be positive for {}", f.path);
    }
}

#[test]
fn test_git_list_rs_files_valid_head() {
    let files = git_list_rs_files("HEAD").unwrap();
    assert!(!files.is_empty());
    for f in &files {
        assert!(f.ends_with(".rs"));
    }
}

#[test]
fn test_git_list_rs_files_invalid_rev() {
    assert!(git_list_rs_files("nonexistent_rev_xyz").is_err());
}

#[test]
fn test_git_show_file_valid() {
    let content = git_show_file("HEAD", "src/main.rs").unwrap();
    assert!(content.contains("fn main"));
}

#[test]
fn test_git_show_file_invalid() {
    assert!(git_show_file("HEAD", "nonexistent_file.rs").is_err());
}

#[test]
fn test_ratio() {
    assert_eq!(ratio(100, 10), 10.0);
    assert_eq!(ratio(0, 0), 0.0);
    assert_eq!(ratio(50, 0), 0.0);
}

#[test]
fn test_pct() {
    assert_eq!(pct(50, 100), 50.0);
    assert_eq!(pct(0, 100), 0.0);
    assert_eq!(pct(10, 0), 0.0);
}

#[test]
fn test_pct_delta_positive() {
    assert_eq!(pct_delta(50, 100), 50.0);
}

#[test]
fn test_pct_delta_negative() {
    assert_eq!(pct_delta(-25, 100), -25.0);
}

#[test]
fn test_pct_delta_zero_base() {
    assert_eq!(pct_delta(10, 0), 0.0);
}

#[test]
fn test_scan_project_sorted_descending() {
    let stats = scan_project_sorted().unwrap();
    for w in stats.files.windows(2) {
        assert!(w[0].tokens >= w[1].tokens, "not sorted: {} < {}", w[0].tokens, w[1].tokens);
    }
}

#[test]
fn test_build_manifest_contains_files() {
    let stats = scan_project().unwrap();
    let manifest = build_manifest(&stats);
    for f in &stats.files {
        assert!(manifest.contains(&f.path), "manifest missing {}", f.path);
    }
}

#[test]
fn test_build_manifest_format() {
    let stats = scan_project().unwrap();
    let manifest = build_manifest(&stats);
    assert!(manifest.contains("==="), "manifest missing === delimiters");
    assert!(manifest.contains("tokens"), "manifest missing token counts");
}

#[test]
fn test_read_rs_file_valid() {
    let (content, tokens, lines) = read_rs_file("src/main.rs").unwrap();
    assert!(content.contains("fn main"));
    assert!(tokens > 0);
    assert!(lines > 0);
}

#[test]
fn test_read_rs_file_not_found() {
    assert!(read_rs_file("nonexistent.rs").is_err());
}

#[test]
fn test_read_rs_file_not_rs() {
    assert!(read_rs_file("Cargo.toml").is_err());
}

#[test]
fn test_count_rev_tokens_head() {
    let rev = count_rev_tokens("HEAD").unwrap();
    assert!(rev.files > 0);
    assert!(rev.tokens > 0);
    assert!(rev.lines > 0);
}

#[test]
fn test_count_rev_tokens_invalid() {
    assert!(count_rev_tokens("nonexistent_rev_xyz").is_err());
}

#[test]
fn test_strip_markdown_fences_plain() {
    assert_eq!(strip_markdown_fences("fn main() {}"), "fn main() {}");
}

#[test]
fn test_strip_markdown_fences_rust() {
    assert_eq!(strip_markdown_fences("```rust\nfn main() {}\n```"), "fn main() {}");
}

#[test]
fn test_strip_markdown_fences_bare() {
    assert_eq!(strip_markdown_fences("```\nfn main() {}\n```"), "fn main() {}");
}

#[test]
fn test_suggestion_items_schema_has_required_fields() {
    let schema = suggestion_items_schema();
    let props = schema["properties"].as_object().unwrap();
    assert!(props.contains_key("description"));
    assert!(props.contains_key("location"));
    assert!(props.contains_key("tokens_saved"));
}
