# cargo-syntax

**Token-efficient Rust tooling by [syntax.ai](https://syntax.ai)**

`cargo-syntax` is a cargo subcommand that helps you write minimal, token-efficient Rust code. It enforces strict clippy lints, compact formatting, and provides tooling to measure how many LLM tokens your codebase consumes.

## Why?

When working with AI coding agents (Claude, GPT, Copilot, etc.), every token counts — both in cost and context window usage. Verbose Rust code wastes tokens on boilerplate that clippy and rustfmt can eliminate automatically.

`cargo-syntax` bundles 40+ carefully selected clippy lints and formatting rules into a single tool, so your Rust code stays lean and your AI workflows stay fast.

## Installation

```bash
cargo install --git https://github.com/syntaxai/cargo-syntax
```

## Commands

### `cargo syntax init <name>`

Scaffold a new Rust project with token-efficient defaults baked in.

```bash
cargo syntax init my-project
cd my-project
```

This creates a standard Cargo project and adds:

| File | Purpose |
|------|---------|
| `Cargo.toml` | 40+ clippy lints (deny + warn levels) |
| `rustfmt.toml` | Compact formatting (100 char width, max heuristics) |
| `clippy.toml` | MSRV pinned to 1.93 |
| `rust-toolchain.toml` | Rust 1.93 with rustfmt, clippy, rust-analyzer |
| `.gitignore` | Standard Rust ignores |
| `CLAUDE.md` | AI agent instructions for minimal code style |

### `cargo syntax check`

Run strict clippy and format checks in a single command.

```bash
cargo syntax check
```

Runs:
1. `cargo clippy --all-targets -- -D warnings` — all clippy lints as errors
2. `cargo fmt --check` — verify formatting

Exits with code 1 if any issues are found.

### `cargo syntax fix`

Auto-fix all clippy warnings and format your code.

```bash
cargo syntax fix
```

Runs:
1. `cargo clippy --fix --allow-dirty --allow-no-vcs` — apply all auto-fixable lints
2. `cargo fmt` — format everything

### `cargo syntax audit`

Measure the token cost and size of your Rust source files.

```bash
cargo syntax audit
```

Output:

```
File                                      Lines   Tokens
--------------------------------------------------------
src\commands\audit.rs                        50      404
src\commands\check.rs                        18      135
src\commands\fix.rs                          14       91
src\commands\init.rs                         40      339
src\commands\mod.rs                           4       16
src\main.rs                                  45      236
src\templates\mod.rs                        100      812
--------------------------------------------------------
Total                                       271     2033

Code: 245 | Comments: 3 | Blanks: 23
```

Token counts use OpenAI's `o200k_base` tokenizer (used by GPT-4o and similar models), giving you an accurate measure of how much context window your code occupies.

## Clippy Lints

`cargo-syntax` enforces three tiers of lints:

### Deny (zero tolerance)

| Lint | Rationale |
|------|-----------|
| `dbg_macro` | No debug macros in production code |
| `todo` | No placeholder TODOs — finish or remove |
| `redundant_clone` | Unnecessary clones waste memory and tokens |

### Warn (reduce boilerplate)

**Eliminate unnecessary syntax:**
`needless_return`, `needless_borrow`, `needless_lifetimes`, `needless_pass_by_value`, `let_and_return`, `redundant_else`, `redundant_field_names`, `redundant_pattern_matching`, `redundant_closure`, `redundant_closure_for_method_calls`

**Use stdlib instead of manual implementations:**
`manual_map`, `manual_filter`, `manual_find`, `manual_flatten`, `manual_is_ascii_check`, `manual_let_else`, `manual_ok_or`, `manual_string_new`, `manual_unwrap_or`, `map_unwrap_or`

**Simplify patterns:**
`collapsible_if`, `collapsible_else_if`, `single_match`, `match_like_matches_macro`, `unnested_or_patterns`

**Prefer concise alternatives:**
`implicit_clone`, `cloned_instead_of_copied`, `flat_map_option`, `iter_on_single_items`, `option_as_ref_deref`, `bind_instead_of_map`, `unnecessary_wraps`, `unnecessary_unwrap`, `unnecessary_lazy_evaluations`

**Idiomatic Rust:**
`use_self`, `unused_self`, `semicolon_if_nothing_returned`, `uninlined_format_args`

## Formatting

The generated `rustfmt.toml` uses:

```toml
edition = "2024"
style_edition = "2024"
max_width = 100
use_small_heuristics = "Max"
```

`use_small_heuristics = "Max"` aggressively collapses code onto fewer lines, reducing token count while maintaining readability.

## Editor Integration

### Zed

Add this to your Zed `settings.json` for automatic clippy + format on save:

```json
{
  "lsp": {
    "rust-analyzer": {
      "initialization_options": {
        "check": {
          "command": "clippy"
        }
      }
    }
  },
  "languages": {
    "Rust": {
      "format_on_save": "on",
      "code_actions_on_format": {
        "source.fixAll": true
      }
    }
  }
}
```

### VS Code

Add to `.vscode/settings.json`:

```json
{
  "rust-analyzer.check.command": "clippy"
}
```

## AI Agent Instructions

Every project created with `cargo syntax init` includes a `CLAUDE.md` file with instructions for AI coding agents:

- Write minimal, idiomatic Rust
- Prefer iterator chains over manual loops
- Use `?` instead of manual match/unwrap
- Prefer `derive` macros over manual trait implementations
- No comments unless logic is non-obvious
- Never use `dbg!()` or `todo!()`

This file is automatically picked up by Claude Code, Cursor, and other AI-aware editors.

## Recommended Crates

When building real applications, these crates reduce boilerplate significantly:

| Crate | Replaces | Token savings |
|-------|----------|---------------|
| `anyhow` | Custom error types | ~50 lines per error type |
| `thiserror` | Manual `impl Display + Error` | ~20 lines per error enum |
| `serde` | Manual serialization | ~100+ lines per data type |
| `clap` (derive) | Manual arg parsing | ~30 lines per CLI |
| `itertools` | Verbose iterator chains | ~5-10 lines per chain |

## License

MIT
