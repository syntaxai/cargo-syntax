mod commands;
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
    }
}
