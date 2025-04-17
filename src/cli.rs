use clap::{ArgAction, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new sheafy project with default config
    Init,
    /// Bundles project files into a single Markdown file
    Bundle {
        /// Comma-separated list of file extensions to include (e.g., rs,py,txt). Overrides config.
        #[arg(short, long, value_delimiter = ',')]
        filters: Option<Vec<String>>,

        /// Output Markdown filename. Overrides config.
        #[arg(short, long)]
        output: Option<String>,

        /// Force use of .gitignore rules (overrides config if set to false).
        #[arg(long, action = ArgAction::SetTrue)]
        use_gitignore: bool,

        /// Force *disabling* .gitignore rules (overrides config and --use-gitignore).
        #[arg(long, action = ArgAction::SetTrue)]
        no_gitignore: bool,
    },
    /// Restores files from a Markdown bundle file, overwriting existing files
    Restore {
        /// The Markdown file to restore from
        input_file: Option<String>,
    },
}
