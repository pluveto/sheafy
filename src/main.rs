use anyhow::{bail, Context, Result};
use clap::{ArgAction, Parser};
use ignore::WalkBuilder; // Use the ignore crate's WalkBuilder
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
};
// No longer need walkdir directly for bundle
// use walkdir::WalkDir;

const CONFIG_FILENAME: &str = "sheafy.toml";
const DEFAULT_BUNDLE_NAME: &str = "project_bundle.md";

// --- Configuration Struct ---
#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "kebab-case")] // Allows kebab-case keys in TOML
struct SheafyConfig {
    filters: Option<Vec<String>>,
    bundle_name: Option<String>,
    use_gitignore: Option<bool>, // Add use_gitignore field
    prologue: Option<String>,
    epilogue: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
struct Config {
    #[serde(default)]
    sheafy: SheafyConfig,
}

// --- Command Line Arguments ---
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Bundles project files into a single Markdown file.
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
    /// Restores files from a Markdown bundle file, overwriting existing files.
    Restore {
        /// The Markdown file to restore from.
        input_file: String,
    },
}

// --- Language Mapping (unchanged) ---
fn get_language_hint(extension: &str) -> &str {
    match extension {
        "py" => "python",
        "js" => "javascript",
        "html" => "html",
        "css" => "css",
        "rs" => "rust",
        "go" => "go",
        "java" => "java",
        "c" => "c",
        "cpp" => "cpp",
        "sh" => "bash",
        "md" => "markdown",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "sql" => "sql",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" => "kotlin",
        "scala" => "scala",
        "ts" => "typescript",
        "txt" => "",
        _ => "",
    }
}

// --- Load Configuration (unchanged) ---
fn load_config() -> Result<Config> {
    let config_path = Path::new(CONFIG_FILENAME);
    if config_path.exists() {
        let config_content = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file: {}", CONFIG_FILENAME))?;
        toml::from_str(&config_content)
            .with_context(|| format!("Failed to parse config file: {}", CONFIG_FILENAME))
    } else {
        Ok(Config::default()) // Return default config if file doesn't exist
    }
}

// --- Bundle Logic (Updated) ---
fn run_bundle(
    cli_filters: Option<Vec<String>>,
    cli_output: Option<String>,
    cli_use_git: bool,
    cli_no_git: bool,
) -> Result<()> {
    let config = load_config().context("Failed to load configuration")?;
    let current_dir = std::env::current_dir().context("Failed to get current working directory")?;

    // --- Determine effective settings (CLI > Config > Default) ---

    // Filters
    let filters_str: Vec<String> = cli_filters.or(config.sheafy.filters).unwrap_or_default();
    if filters_str.is_empty() {
        bail!(
            "No file filters provided via flags or in {}. Cannot proceed.",
            CONFIG_FILENAME
        );
    }
    let filters: HashSet<String> = filters_str
        .into_iter()
        .map(|f| f.trim_start_matches('.').to_lowercase())
        .collect();
    println!("Using filters: {:?}", filters.iter().collect::<Vec<_>>());

    // Output file
    let output_filename = cli_output
        .or(config.sheafy.bundle_name)
        .unwrap_or_else(|| DEFAULT_BUNDLE_NAME.to_string());
    let output_path = PathBuf::from(&output_filename);
    // Get absolute path for exclusion check later
    let absolute_output_path = current_dir.join(&output_path).canonicalize().ok();

    println!("Output file: {}", output_filename);

    // Gitignore usage
    let config_git_setting = config.sheafy.use_gitignore.unwrap_or(true); // Default to true if not in config

    let effective_use_gitignore = match (cli_use_git, cli_no_git) {
        (true, true) => bail!("Cannot specify both --use-gitignore and --no-gitignore"),
        (true, false) => true,                // CLI explicitly enables
        (false, true) => false,               // CLI explicitly disables
        (false, false) => config_git_setting, // Use config or default
    };

    if effective_use_gitignore {
        println!("Respecting .gitignore rules.");
    } else {
        println!("Ignoring .gitignore rules.");
    }

    // --- File Collection using `ignore` crate ---
    let mut matched_files: Vec<PathBuf> = Vec::new();
    let config_path_abs = current_dir.join(CONFIG_FILENAME).canonicalize().ok();
    let executable_path_abs = std::env::current_exe().ok(); // Get path of the running executable

    let mut builder = WalkBuilder::new(&current_dir);
    builder
        .hidden(true) // Use standard hidden file filtering (respects .gitignore overrides)
        .parents(effective_use_gitignore) // Use parent ignore files if gitignore is enabled
        .ignore(effective_use_gitignore) // Use .ignore files if gitignore is enabled
        .git_ignore(effective_use_gitignore) // Use .gitignore if enabled
        .git_global(effective_use_gitignore) // Use global gitignore if enabled
        .git_exclude(effective_use_gitignore); // Use .git/info/exclude if enabled
                                               // .require_git(false) // Don't require a git repo to exist for gitignore rules

    println!("Starting file scan...");

    for entry_result in builder.build() {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("Warning: Skipping path due to error: {}", e);
                continue;
            }
        };
        let path = entry.path();

        // Skip directories, keep files (WalkBuilder yields both)
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }

        // Get absolute path for explicit exclusion checks
        // Need canonicalize for robust comparison, but ignore errors if file disappears
        let absolute_path = path.canonicalize().ok();

        // Explicitly skip the config file, the output file, and the executable itself
        // These might not be covered by gitignore rules or might be desired even if ignored.
        if Some(absolute_path.as_ref()) == config_path_abs.as_ref().map(|p| Some(p))
            || Some(absolute_path.as_ref()) == absolute_output_path.as_ref().map(|p| Some(p))
            || Some(absolute_path.as_ref()) == executable_path_abs.as_ref().map(|p| Some(p))
        {
            // println!("Skipping explicitly excluded file: {}", path.display()); // Debugging
            continue;
        }

        // Apply *our* extension filters AFTER ignore rules have been processed
        if let Some(ext) = path.extension().and_then(|os| os.to_str()) {
            if filters.contains(&ext.to_lowercase()) {
                // Use relative path from the base directory where the walk started
                // The `ignore` crate entry path is often already relative or can be easily made so.
                // Using pathdiff ensures consistency if entry.path() isn't relative.
                if let Some(relative_path) = pathdiff::diff_paths(path, &current_dir) {
                    matched_files.push(relative_path);
                } else {
                    // Fallback: use the path as given by the entry if diff fails (less ideal)
                    // This might happen for paths outside current_dir if symlinks are followed, though WalkBuilder usually handles this.
                    eprintln!(
                        "Warning: Could not determine relative path for {:?}. Using original path.",
                        path
                    );
                    matched_files.push(path.to_path_buf());
                }
            }
        }
    }

    if matched_files.is_empty() {
        println!("No files found matching the specified filters and ignore rules.");
        return Ok(());
    }

    // Sort paths for consistent output
    matched_files.sort();

    // --- Create Markdown File (mostly unchanged) ---
    println!("\nCreating Markdown file: {}", output_filename);
    let output_file = File::create(&output_path)
        .with_context(|| format!("Failed to create output file: {}", output_filename))?;
    let mut writer = BufWriter::new(output_file);
    if let Some(prologue) = config.sheafy.prologue {
        writer.write_all(prologue.as_bytes())?;
    }

    for rel_path in &matched_files {
        // Ensure forward slashes for Markdown header consistency
        let header_path = rel_path
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        println!("  Adding: {}", header_path);

        let mut file_content = String::new();
        // Construct full path to read the file relative to current dir
        let full_read_path = current_dir.join(rel_path);
        match File::open(&full_read_path) {
            Ok(mut f) => {
                if let Err(e) = f.read_to_string(&mut file_content) {
                    eprintln!(
                        "Warning: Could not read file '{}': {}. Skipping.",
                        full_read_path.display(), // Show full path in error
                        e
                    );
                    continue;
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Could not open file '{}': {}. Skipping.",
                    full_read_path.display(), // Show full path in error
                    e
                );
                continue;
            }
        }

        let lang_hint = rel_path
            .extension()
            .and_then(|os| os.to_str())
            .map(get_language_hint)
            .unwrap_or("");

        writeln!(writer, "## {}", header_path)?;
        writeln!(writer, "```{}", lang_hint)?;
        writer.write_all(file_content.as_bytes())?;
        if !file_content.ends_with('\n') {
            writeln!(writer)?;
        }
        writeln!(writer, "```\n")?;
    }

    if let Some(epilogue) = config.sheafy.epilogue {
        writer.write_all(epilogue.as_bytes())?;
    }
    writer.flush()?;
    println!(
        "\nSuccessfully created '{}' with {} file(s).",
        output_filename,
        matched_files.len()
    );

    Ok(())
}

// --- Restore Logic (unchanged) ---
lazy_static! {
    static ref RESTORE_REGEX: Regex =
        Regex::new(r"(?ms)^##\s*(.*?)\s*\n```[^\n]*\n(.*?)\n```\s*$").unwrap();
}

fn run_restore(input_filename: &str) -> Result<()> {
    println!("Attempting to restore files from: {}", input_filename);
    let input_path = Path::new(input_filename);
    let current_dir = std::env::current_dir().context("Failed to get current working directory")?;

    let content = fs::read_to_string(input_path)
        .with_context(|| format!("Failed to read input file: {}", input_filename))?;

    let mut restored_count = 0;
    let mut found_blocks = 0;

    for cap in RESTORE_REGEX.captures_iter(&content) {
        found_blocks += 1;
        let rel_path_str = cap.get(1).map_or("", |m| m.as_str()).trim();
        let code_content = cap.get(2).map_or("", |m| m.as_str());

        if rel_path_str.is_empty() {
            eprintln!("Warning: Found block with empty filepath. Skipping.");
            continue;
        }

        // Convert header path (always '/') to native path relative to current_dir
        let target_path =
            current_dir.join(rel_path_str.replace('/', std::path::MAIN_SEPARATOR_STR));

        println!("  Restoring: {}", target_path.display()); // Display full target path

        // Ensure target directory exists
        if let Some(parent_dir) = target_path.parent() {
            if !parent_dir.exists() {
                if !parent_dir.as_os_str().is_empty() {
                    println!("    Creating directory: {}", parent_dir.display());
                    fs::create_dir_all(parent_dir).with_context(|| {
                        format!("Failed to create directory: {}", parent_dir.display())
                    })?;
                }
            }
        }

        // Write the file (overwrite)
        match File::create(&target_path) {
            Ok(output_file) => {
                let mut writer = BufWriter::new(output_file);
                match writer.write_all(code_content.as_bytes()) {
                    Ok(_) => {
                        if let Err(e) = writer.flush() {
                            eprintln!(
                                "Error flushing buffer for file '{}': {}. File might be incomplete.",
                                target_path.display(), e
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Error writing content to file '{}': {}. Skipping file.",
                            target_path.display(),
                            e
                        );
                        continue;
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "Error creating/opening file '{}' for writing: {}. Skipping file.",
                    target_path.display(),
                    e
                );
                continue;
            }
        }
        restored_count += 1;
    }

    if found_blocks == 0 {
        println!(
            "Warning: No valid sheafy blocks found in '{}'. No files restored.",
            input_filename
        );
    } else {
        println!(
            "\nRestore complete. {} file(s) restored/overwritten.",
            restored_count
        );
    }

    Ok(())
}

// --- Main Function ---
fn main() -> Result<()> {
    let cli = Cli::parse();
    let current_dir = std::env::current_dir().context("Failed to get current working directory")?;
    println!("Working directory: {}", current_dir.display());

    match cli.command {
        Commands::Bundle {
            filters,
            output,
            use_gitignore,
            no_gitignore,
        } => {
            run_bundle(filters, output, use_gitignore, no_gitignore)
                .context("Bundle operation failed")?;
        }
        Commands::Restore { input_file } => {
            run_restore(&input_file).context("Restore operation failed")?;
        }
    }

    Ok(())
}
