pub const CARGO_LINTS: &str = r#"
[lints.clippy]
dbg_macro = "deny"
todo = "deny"
redundant_clone = "deny"
needless_return = "warn"
needless_borrow = "warn"
needless_lifetimes = "warn"
needless_pass_by_value = "warn"
redundant_closure = "warn"
redundant_closure_for_method_calls = "warn"
redundant_field_names = "warn"
redundant_pattern_matching = "warn"
redundant_else = "warn"
let_and_return = "warn"
collapsible_if = "warn"
collapsible_else_if = "warn"
single_match = "warn"
manual_map = "warn"
manual_filter = "warn"
manual_find = "warn"
manual_flatten = "warn"
manual_is_ascii_check = "warn"
manual_let_else = "warn"
manual_ok_or = "warn"
manual_string_new = "warn"
manual_unwrap_or = "warn"
map_unwrap_or = "warn"
match_like_matches_macro = "warn"
implicit_clone = "warn"
cloned_instead_of_copied = "warn"
flat_map_option = "warn"
iter_on_single_items = "warn"
option_as_ref_deref = "warn"
bind_instead_of_map = "warn"
unnecessary_wraps = "warn"
unnecessary_unwrap = "warn"
unnecessary_lazy_evaluations = "warn"
unnested_or_patterns = "warn"
unused_self = "warn"
use_self = "warn"
semicolon_if_nothing_returned = "warn"
uninlined_format_args = "warn"
"#;

pub const RUSTFMT_TOML: &str = r#"edition = "2024"
style_edition = "2024"
max_width = 100
use_small_heuristics = "Max"
"#;

pub const CLIPPY_TOML: &str = r#"msrv = "1.93.0"
"#;

pub const RUST_TOOLCHAIN_TOML: &str = r#"[toolchain]
channel = "1.93"
profile = "minimal"
components = ["rustfmt", "clippy", "rust-analyzer"]
"#;

pub const GITIGNORE: &str = r#"**/target
.env
.env.*
.DS_Store
.vscode
.idea
*.swp
*.swo
.claude/settings.local.json
"#;

pub const CLAUDE_MD: &str = r#"# Project Instructions

## Code Style
- Write minimal, idiomatic Rust — no unnecessary verbosity
- Prefer iterator chains over manual loops
- Use `?` operator instead of manual match/unwrap on Result/Option
- Prefer `derive` macros over manual trait implementations
- No comments unless logic is non-obvious
- No doc comments on private items
- Use short but descriptive variable names
- Never use `dbg!()` or `todo!()` — they are denied by clippy

## Crates to prefer (when applicable)
- `anyhow` for application error handling (instead of custom error types)
- `thiserror` for library error types (instead of manual `impl Display + Error`)
- `serde` + `serde_json` for serialization (instead of manual parsing)
- `clap` with derive for CLI args (instead of manual parsing)
- `itertools` for complex iterator chains

## Build & Check
- `cargo clippy` — strict token-minimizing lints (see Cargo.toml)
- `cargo clippy --fix --allow-dirty` — auto-fix
- `cargo fmt` — format code (rustfmt, edition 2024)
- `cargo syntax check` / `cargo syntax fix` — all-in-one

## Toolchain
- Rust 1.93 (pinned via rust-toolchain.toml)
- Edition 2024
"#;
