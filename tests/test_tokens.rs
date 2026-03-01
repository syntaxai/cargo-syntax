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
