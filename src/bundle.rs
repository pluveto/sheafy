use crate::config::{Config, DEFAULT_BUNDLE_NAME};
use anyhow::{bail, Context, Result};
use ignore::{gitignore::GitignoreBuilder, overrides::OverrideBuilder, WalkBuilder}; // Import OverrideBuilder
use std::{
    fs::{self, File},
    io::{BufWriter, Read, Write},
    path::PathBuf,
};

fn invert_patern(pattern: &str) -> String {
    // if starts with !, remove it, otherwise add!
    if pattern.starts_with('!') {
        pattern[1..].to_string()
    } else {
        format!("!{}", pattern)
    }
}

pub fn run_bundle(
    config: Config, // Pass loaded config
    // REMOVED: cli_filters: Option<Vec<String>>,
    cli_output: Option<String>,
    cli_use_git: bool,
    cli_no_git: bool,
) -> Result<()> {
    // Use working_dir already determined in main.rs
    let working_dir = config
        .get_working_dir()
        .context("Failed to get working directory for bundling")?;
    let output_filename = cli_output
        .or(config.sheafy.bundle_name)
        .unwrap_or_else(|| DEFAULT_BUNDLE_NAME.to_string());
    let output_path = PathBuf::from(&output_filename);
    let env_wd = std::env::current_dir()?;
    std::env::set_current_dir(working_dir.clone())?;
    defer! {
        std::env::set_current_dir(env_wd).unwrap();
    }
    // Ensure output path is absolute for comparison, handle potential creation errors
    let absolute_output_path = if output_path.is_absolute() {
        output_path.clone()
    } else {
        working_dir.join(&output_path)
    }
    .canonicalize() // Try to canonicalize *before* creating the file
    .or_else(|_| -> anyhow::Result<PathBuf> {
        // If canonicalize fails (e.g., file doesn't exist yet), keep the joined path
        if output_path.is_absolute() {
            Ok(output_path.clone())
        } else {
            Ok(working_dir.join(&output_path))
        }
    })?;

    println!("Output file will be: {}", absolute_output_path.display());

    let config_git_setting = config.sheafy.use_gitignore.unwrap_or(true);
    let effective_use_gitignore = match (cli_use_git, cli_no_git) {
        (true, true) => bail!("Cannot specify both --use-gitignore and --no-gitignore"),
        (true, false) => true,
        (false, true) => false,
        (false, false) => config_git_setting,
    };

    if effective_use_gitignore {
        println!("Respecting .gitignore rules.");
    } else {
        println!("Ignoring .gitignore rules.");
    }
    // --- End Custom Ignore Pattern Handling ---

    let mut matched_files: Vec<PathBuf> = Vec::new();
    // Ensure config path is absolute for comparison
    let config_path_abs = working_dir
        .join(crate::config::CONFIG_FILENAME)
        .canonicalize()
        .ok();
    let executable_path_abs = std::env::current_exe().ok();

    let mut builder = WalkBuilder::new(&working_dir);
    builder.standard_filters(effective_use_gitignore);

    // Apply custom ignore patterns
    let tmp_ignore_file = tempfile::NamedTempFile::new().unwrap();
    if let Some(patterns) = &config.sheafy.ignore_patterns {
        if !patterns.trim().is_empty() {
            tmp_ignore_file
                .as_file()
                .write_all(patterns.as_bytes())
                .unwrap();
            builder.add_custom_ignore_filename(tmp_ignore_file.path().to_str().unwrap());
        }
    }

    println!("Starting file scan in {}...", working_dir.display());

    for entry_result in builder.build() {
        println!("ENTRY: {:?}",entry_result);
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("Warning: Skipping path due to error: {}", e);
                continue;
            }
        };
        let path = entry.path();

        // Skip directories
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }

        // Attempt to get absolute path for comparison
        let absolute_path = path.canonicalize().ok();

        // Skip the config file itself
        if config_path_abs.as_ref().map_or(false, |config_abs| {
            absolute_path.as_ref() == Some(config_abs)
        }) {
            // println!("Skipping config file: {:?}", path); // Debugging
            continue;
        }

        // Skip the output file itself
        if absolute_path.as_ref() == Some(&absolute_output_path) {
            // println!("Skipping output file: {:?}", path); // Debugging
            continue;
        }

        // Skip the executable itself
        if executable_path_abs
            .as_ref()
            .map_or(false, |exec_abs| absolute_path.as_ref() == Some(exec_abs))
        {
            // println!("Skipping executable file: {:?}", path); // Debugging
            continue;
        }

        if let Some(relative_path) = pathdiff::diff_paths(path, &working_dir) {
            matched_files.push(relative_path);
        } else {
            // Fallback, though diff_paths should ideally work for files found by WalkBuilder within working_dir
            eprintln!(
                "Warning: Could not determine relative path for {:?}. Using absolute path.",
                path
            );
            matched_files.push(path.to_path_buf());
        }
    }

    if matched_files.is_empty() {
        println!(
            "No files found matching the ignore rules (including .gitignore and custom patterns)."
        );
        // Attempt to create an empty output file anyway? Or just exit? Exiting seems fine.
        return Ok(());
    }

    matched_files.sort(); // Keep sorting for consistent output

    println!(
        "\nCreating Markdown bundle: {}",
        absolute_output_path.display()
    );
    // Create parent directory if it doesn't exist
    if let Some(parent_dir) = absolute_output_path.parent() {
        if !parent_dir.exists() {
            println!("Creating output directory: {}", parent_dir.display());
            fs::create_dir_all(parent_dir).with_context(|| {
                format!(
                    "Failed to create output directory: {}",
                    parent_dir.display()
                )
            })?;
        }
    }

    let output_file = File::create(&absolute_output_path).with_context(|| {
        format!(
            "Failed to create output file: {}",
            absolute_output_path.display()
        )
    })?;
    let mut writer = BufWriter::new(output_file);

    if let Some(prologue) = config.sheafy.prologue {
        writer.write_all(prologue.as_bytes())?;
        if !prologue.ends_with('\n') {
            // Ensure newline after prologue
            writeln!(writer)?;
        }
    }

    for rel_path in &matched_files {
        let header_path = rel_path
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/"); // Use consistent / separator in header
        println!("  Adding: {}", header_path);

        let mut file_content = String::new();
        // Read from the original absolute path constructed relative to working_dir
        let full_read_path = working_dir.join(rel_path);
        match File::open(&full_read_path) {
            Ok(mut f) => {
                if let Err(e) = f.read_to_string(&mut file_content) {
                    eprintln!(
                        "Warning: Could not read file '{}': {}. Skipping.",
                        full_read_path.display(),
                        e
                    );
                    continue; // Skip this file
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Could not open file '{}': {}. Skipping.",
                    full_read_path.display(),
                    e
                );
                continue; // Skip this file
            }
        }

        // Determine language hint for ``` block
        let lang_hint = rel_path
            .extension()
            .and_then(|os| os.to_str())
            .map(crate::restore::get_language_hint) // Use existing helper
            .unwrap_or("");

        // Write file block to Markdown
        writeln!(writer, "\n## {}", header_path)?; // Add a newline before header for better separation
        writeln!(writer, "```{}", lang_hint)?;
        writer.write_all(file_content.as_bytes())?;
        if !file_content.ends_with('\n') {
            // Ensure code block ends with newline
            writeln!(writer)?;
        }
        writeln!(writer, "```")?; // Removed extra newline after ```
    }

    if let Some(epilogue) = config.sheafy.epilogue {
        if !epilogue.starts_with('\n') {
            // Ensure newline before epilogue
            writeln!(writer)?;
        }
        writer.write_all(epilogue.as_bytes())?;
        if !epilogue.ends_with('\n') {
            // Ensure newline after epilogue
            writeln!(writer)?;
        }
    }

    writer.flush()?; // Ensure buffer is written
    println!(
        "\nSuccessfully created '{}' with {} file(s).",
        absolute_output_path.display(),
        matched_files.len()
    );

    Ok(())
}
