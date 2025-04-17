//! Sheafy is a tool to bundle project files into a Markdown document and restore them.
//!
//! # Examples
//! ```bash
//! # Bundle Rust files
//! sheafy bundle -f rs
//!
//! # Restore files
//! sheafy restore bundle.md
//! ```
//!
mod bundle;
mod cli;
mod config;
mod restore;

use anyhow::{Context, Result};
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let current_dir = std::env::current_dir().context("Failed to get current working directory")?;
    println!("Working directory: {}", current_dir.display());

    match cli.command {
        cli::Commands::Init => config::Config::init(),
        cli::Commands::Bundle {
            filters,
            output,
            use_gitignore,
            no_gitignore,
        } => bundle::run_bundle(filters, output, use_gitignore, no_gitignore),
        cli::Commands::Restore { input_file } => restore::run_restore(input_file),
    }
}
