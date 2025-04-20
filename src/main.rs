//! Sheafy is a tool to bundle project files into a Markdown document and restore them.
//!
//! # Examples
//! ```bash
//! # Bundle files (respecting .gitignore and sheafy.toml ignore_patterns)
//! sheafy bundle
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

#[macro_use(defer)]
extern crate scopeguard;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    // Get current dir early, before potential working_dir change in config
    let initial_dir = std::env::current_dir().context("Failed to get initial working directory")?;
    println!("Running from directory: {}", initial_dir.display());


    match cli.command {
        cli::Commands::Init => config::Config::init(),
        cli::Commands::Bundle {
            // REMOVED: filters
            output,
            use_gitignore,
            no_gitignore,
        } => {
             // Load config *after* knowing the command might need it
             let config = config::Config::load().context("Failed to load configuration")?;
             let working_dir = config.get_working_dir()?;
             println!("Effective working directory: {}", working_dir.display());
             bundle::run_bundle(config, output, use_gitignore, no_gitignore)
        },
        cli::Commands::Restore { input_file } => {
            // Load config *after* knowing the command might need it
            let config = config::Config::load().context("Failed to load configuration")?;
            let working_dir = config.get_working_dir()?;
            println!("Effective working directory: {}", working_dir.display());
            restore::run_restore(config, input_file)
        },
    }
}
