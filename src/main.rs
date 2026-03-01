mod commands;
mod openrouter;
mod templates;
pub mod tokens;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum Cli {
    Syntax(SyntaxArgs),
}

#[derive(Parser)]
#[command(version, about = "Token-efficient Rust tooling by syntax.ai")]
struct SyntaxArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold a new token-efficient Rust project
    Init {
        /// Project name
        name: String,
    },
    /// Run strict clippy + fmt checks
    Check,
    /// Auto-fix clippy warnings and format code
    Fix,
    /// Audit token count and lines of code per file
    Audit,
    /// Generate a token efficiency badge for your README
    Badge,
    /// Apply token-efficient configs to an existing project
    Apply,
    /// Show the N most token-heavy files
    Top {
        /// Number of files to show (default: 10)
        #[arg(default_value = "10")]
        n: usize,
    },
    /// Analyze files and suggest token-efficiency improvements
    Suggest,
    /// AI-powered rewrite of a file for token efficiency (via OpenRouter)
    Rewrite {
        /// Rust file to rewrite
        file: String,
        /// OpenRouter model (default: deepseek/deepseek-chat, override with CARGO_SYNTAX_MODEL)
        #[arg(long)]
        model: Option<String>,
    },
    /// AI-powered review of the top N most token-heavy files (via OpenRouter)
    Review {
        /// Number of files to review (default: 5)
        #[arg(default_value = "5")]
        n: usize,
        /// OpenRouter model (default: deepseek/deepseek-chat, override with CARGO_SYNTAX_MODEL)
        #[arg(long)]
        model: Option<String>,
    },
    /// AI-powered review of uncommitted changes for token efficiency
    Diff {
        /// Git range (e.g. "main..HEAD"), defaults to unstaged changes
        range: Option<String>,
        /// Review staged changes instead of unstaged
        #[arg(long)]
        staged: bool,
        /// Auto-rewrite files that have suggestions
        #[arg(long)]
        fix: bool,
        /// OpenRouter model (default: deepseek/deepseek-chat, override with CARGO_SYNTAX_MODEL)
        #[arg(long)]
        model: Option<String>,
    },
    /// Bulk AI-powered rewrite of the most token-heavy files
    Batch {
        /// Number of files to rewrite (default: 5)
        #[arg(default_value = "5")]
        n: usize,
        /// Run cargo check + cargo test after each rewrite, rollback on failure
        #[arg(long)]
        validate: bool,
        /// Auto-accept rewrites without prompting
        #[arg(long)]
        auto: bool,
        /// OpenRouter model (default: deepseek/deepseek-chat, override with CARGO_SYNTAX_MODEL)
        #[arg(long)]
        model: Option<String>,
    },
    /// List available OpenRouter models for code tasks
    Models {
        /// Filter models by name or ID (e.g. "deepseek", "claude", "gemini")
        search: Option<String>,
    },
}

fn main() -> Result<()> {
    let Cli::Syntax(args) = Cli::parse();

    match args.command {
        Command::Init { name } => commands::init::run(&name),
        Command::Check => commands::check::run(),
        Command::Fix => commands::fix::run(),
        Command::Audit => commands::audit::run(),
        Command::Badge => commands::badge::run(),
        Command::Apply => commands::apply::run(),
        Command::Top { n } => commands::top::run(n),
        Command::Suggest => commands::suggest::run(),
        Command::Rewrite { file, model } => {
            let model = model.unwrap_or_else(tokens::default_model);
            commands::rewrite::run(&file, &model)
        }
        Command::Review { n, model } => {
            let model = model.unwrap_or_else(tokens::default_model);
            commands::review::run(n, &model)
        }
        Command::Diff { range, staged, fix, model } => {
            let model = model.unwrap_or_else(tokens::default_model);
            commands::diff::run(range.as_deref(), staged, fix, &model)
        }
        Command::Batch { n, validate, auto, model } => {
            let model = model.unwrap_or_else(tokens::default_model);
            commands::batch::run(n, validate, auto, &model)
        }
        Command::Models { search } => commands::models::run(search.as_deref()),
    }
}
