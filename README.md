# cargo-syntax

[![Token Efficiency](https://img.shields.io/badge/token_efficiency-B%20(7.7%20T/L)-blue)](https://github.com/syntaxai/cargo-syntax)

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

Shows per-file stats (lines, tokens, T/L ratio) with a project efficiency grade (A+ through D). Token counts use OpenAI's `o200k_base` tokenizer (used by GPT-4o and similar models).

### `cargo syntax top [n]`

Show the N most token-heavy files, ranked by token count.

```bash
cargo syntax top        # top 10 files
cargo syntax top 3      # top 3 files
```

### `cargo syntax suggest`

Analyze your code against 39 token-efficiency clippy lints and show grouped suggestions per file.

```bash
cargo syntax suggest
```

### `cargo syntax badge`

Generate a token-efficiency badge for your README in Markdown, HTML, and reStructuredText.

```bash
cargo syntax badge
```

### `cargo syntax apply`

Apply token-efficient configs to an existing project (adds clippy lints, rustfmt.toml, clippy.toml, rust-toolchain.toml, CLAUDE.md).

```bash
cargo syntax apply
```

### `cargo syntax rewrite <file>`

AI-powered rewrite of a single file for token efficiency, using [OpenRouter](https://openrouter.ai).

```bash
export OPENROUTER_API_KEY="sk-or-v1-..."
cargo syntax rewrite src/commands/audit.rs
```

Sends the file to an LLM, which rewrites it to be more token-efficient. Shows before/after stats, explains each change, and asks for confirmation before overwriting.

```
Sending src/commands/audit.rs to deepseek/deepseek-chat via OpenRouter...
  67 lines, 504 tokens

Result:
  Lines:  67 → 52
  Tokens: 504 → 421
  Saved:  83 tokens (16.5%)

Changes:
  - Replaced manual loop with iterator chain in run() (~15 tokens)
  - Inlined format args across 4 println! calls (~8 tokens)
  - Collapsed if/else to single expression (~6 tokens)

Accept? [y/n/diff]
```

Use `--model` to pick a different model:

```bash
cargo syntax rewrite src/main.rs --model google/gemini-2.5-flash
```

### `cargo syntax review [n]`

AI-powered project-wide review that scans your top N most token-heavy files and gives a prioritized action plan.

```bash
cargo syntax review        # review top 5 files
cargo syntax review 3      # review top 3 files
```

```
Scanning project... 15 files, 8409 tokens total
Reviewing top 3 files via deepseek/deepseek-chat...

  #1  src/commands/suggest.rs  (217 lines, 1455 tokens, T/L: 6.7, 17.3% of total)
      - Combine multiple continue conditions into single if with && (~15 tokens)
      - Inline normalize() calls where used only once (~10 tokens)
      - Replace WARN_LINTS array with joined string (~50 tokens)
      => est. savings: ~75 tokens (5.2%)

  #2  src/commands/rewrite.rs  (152 lines, 1236 tokens, T/L: 8.1, 14.7% of total)
      - Simplify strip_markdown_fences with chained operations (~5 tokens)
      - Replace Vec collections in print_diff with iterator chains (~4 tokens)
      => est. savings: ~76 tokens (6.1%)

  ──────────────────────────────────────────────────────────────────
  Reviewed 3/15 files (3620 of 8409 tokens)
  Estimated total savings: ~151 tokens (1.8%)

  Run `cargo syntax rewrite <file>` on any file to apply changes.
```

### `cargo syntax diff [range]`

AI-powered review of your uncommitted changes before you commit. Analyzes only modified `.rs` files and suggests token-efficient alternatives.

```bash
cargo syntax diff              # review unstaged changes
cargo syntax diff --staged     # review staged changes
cargo syntax diff main..HEAD   # review branch changes
cargo syntax diff --fix        # review + auto-rewrite files with suggestions
```

```
Analyzing unstaged changes via deepseek/deepseek-chat...

src/parser.rs  (modified, +45 lines, ~+380 tokens, T/L: 8.2)
  - Manual loop could be iterator chain [parse_items()] (~12 tokens)
  - Redundant clone on Copy type [line 31] (~4 tokens)
  - Verbose match could be if-let [process()] (~8 tokens)

src/optimizer.rs  (new file, 89 lines, 650 tokens, T/L: 7.3)
  ✓ Changes look token-efficient

──────────────────────────────────────────────────────────────────
Summary: 2 file(s) changed, ~+1030 tokens added
3 suggestion(s) could save ~24 tokens (2%)
```

Set `CARGO_SYNTAX_MODEL` to use a different model, or pass `--model`:

```bash
export CARGO_SYNTAX_MODEL=anthropic/claude-sonnet-4
cargo syntax diff --staged
```

### `cargo syntax batch [n]`

Bulk AI-powered rewrite of the most token-heavy files in one run.

```bash
cargo syntax batch              # rewrite top 5 files (interactive)
cargo syntax batch 3            # rewrite top 3 files
cargo syntax batch --auto       # auto-accept all rewrites
cargo syntax batch --validate   # run cargo check + cargo test after each rewrite
cargo syntax batch 5 --auto --validate  # full CI/CD mode: auto-accept, rollback on test failure
```

```
Batch rewriting top 3 files via deepseek/deepseek-chat...
  Validation: cargo check + cargo test after each rewrite
  Auto-apply: skipping interactive prompts

[1/3] src/commands/diff.rs  (1657 tokens, 235 lines, T/L: 7.1)
  rewriting... done
  1657 → 1562 tokens (saves 95, 5.7%)
  validating... passed ✓

[2/3] src/commands/rewrite.rs  (1627 tokens, 228 lines, T/L: 7.1)
  rewriting... done
  1627 → 1534 tokens (saves 93, 5.7%)
  validating... passed ✓

──────────────────────────────────────────────────────────────────────
Batch complete: 3 rewritten, 0 skipped, 0 failed
Total saved: ~244 tokens (1.8% of project)
```

### `cargo syntax explain [path]`

AI-powered code explanation for onboarding and understanding.

```bash
cargo syntax explain                     # explain entire project architecture
cargo syntax explain src/tokens.rs       # explain a single file
```

**Single file** — shows purpose, key items (functions/structs/enums), and dependencies:

```
Explaining src/commands/batch.rs (153 lines, 1019 tokens)...
  analyzing... done

  This module orchestrates bulk file rewriting using a specified AI model.

  Key items:
    run (function) — Batch rewrite top N files with optional validation
    ask_accept (function) — Interactive y/n prompt for accepting rewrites
    run_validation (function) — Runs cargo check + cargo test

  Dependencies: tokens, rewrite
```

**Project overview** — shows architecture, all modules, and where to start reading:

```
Explaining project (19 files, 15131 tokens)...
  analyzing... done

  A CLI tool for improving token efficiency in Rust projects.

  Modules:
    src/main.rs              Entry point, defines CLI commands
    src/tokens.rs            Token counting and project scanning
    src/openrouter.rs        OpenRouter API client
    src/commands/batch.rs    Bulk AI rewrite with validation
    src/commands/rewrite.rs  Single file AI rewrite
    ...

  Start here: src/main.rs
```

### `cargo syntax models [search]`

List available OpenRouter models, sorted by price. Without arguments, shows popular code models. Pass a search term to filter.

```bash
cargo syntax models             # show code-focused models
cargo syntax models free        # show only free models
cargo syntax models claude      # show Claude models
```

```
Model ID                                    Context      Input/M     Output/M
──────────────────────────────────────────────────────────────────────────────
qwen/qwen3-coder:free                       262000      $0.0000      $0.0000
deepseek/deepseek-chat                      163840      $0.3200      $0.8900
google/gemini-2.5-flash                    1048576      $0.3000      $2.5000
anthropic/claude-sonnet-4                  1000000      $3.0000     $15.0000
...
```

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
