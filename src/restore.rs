use crate::config::{Config, DEFAULT_BUNDLE_NAME}; // Keep Config import
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    borrow::Cow,
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf, // Add PathBuf import
};

lazy_static! {
    static ref RESTORE_REGEX: Regex =
        Regex::new(r"(?ms)^##\s*(.*?)\s*\n```[^\n]*\n(.*?)\n```\s*$").unwrap();
}

pub fn get_language_hint(extension: &str) -> &str {
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
        "txt" => "", // Treat txt as plain text without hint
        _ => "",     // Default to no hint
    }
}

fn ensure_eof_newline(slice: &str) -> Cow<str> {
    if slice.ends_with('\n') {
        Cow::Borrowed(slice)
    } else {
        let mut owned_string = String::with_capacity(slice.len() + 1);
        owned_string.push_str(slice);
        owned_string.push('\n');
        Cow::Owned(owned_string)
    }
}

// Update function signature
pub fn run_restore(config: Config, input_filename: Option<String>) -> Result<()> {
    println!("Attempting to restore files");
    // Use working_dir already determined in main.rs
    let working_dir = config
        .get_working_dir()
        .context("Failed to get working directory for restore")?;

    // Determine input file path relative to the *initial* directory sheafy was run from,
    // or use the bundle_name from config (which is usually relative to working_dir)
    let input_path_str = input_filename
        .as_deref()
        .or(config.sheafy.bundle_name.as_deref())
        .unwrap_or(DEFAULT_BUNDLE_NAME);

    // Resolve input path: if absolute, use it; otherwise, assume relative to initial run dir OR working_dir?
    // Let's assume relative to working_dir for consistency with bundle output default.
    let input_path = PathBuf::from(input_path_str);
    let absolute_input_path = if input_path.is_absolute() {
        input_path
    } else {
        working_dir.join(input_path)
    };

    println!("Reading bundle file: {}", absolute_input_path.display());
    let content = fs::read_to_string(&absolute_input_path).with_context(|| {
        format!(
            "Failed to read input file: {}",
            absolute_input_path.display()
        )
    })?;

    let mut restored_count = 0;
    let mut found_blocks = 0;

    for cap in RESTORE_REGEX.captures_iter(&content) {
        found_blocks += 1;
        let rel_path_str = cap.get(1).map_or("", |m| m.as_str()).trim();
        let code_content = ensure_eof_newline(cap.get(2).map_or("", |m| m.as_str()));

        if rel_path_str.is_empty() {
            eprintln!("Warning: Found block with empty filepath. Skipping.");
            continue;
        }

        // Construct target path relative to the determined working_dir
        let target_path =
            working_dir.join(rel_path_str.replace('/', std::path::MAIN_SEPARATOR_STR));

        println!("  Restoring: {}", target_path.display());

        // Ensure parent directory exists
        if let Some(parent_dir) = target_path.parent() {
            if !parent_dir.exists() && !parent_dir.as_os_str().is_empty() {
                println!("    Creating directory: {}", parent_dir.display());
                fs::create_dir_all(parent_dir).with_context(|| {
                    format!("Failed to create directory: {}", parent_dir.display())
                })?;
            }
        }

        // Write the file content
        match File::create(&target_path) {
            Ok(output_file) => {
                let mut writer = BufWriter::new(output_file);
                match writer.write_all(code_content.as_bytes()) {
                    Ok(_) => {
                        // Explicitly flush before dropping to catch potential errors
                        if let Err(e) = writer.flush() {
                            eprintln!(
                                "Error flushing buffer for file '{}': {}. File might be incomplete.",
                                target_path.display(), e
                            );
                            // Optionally continue, or return Err(e.into()) ? Continuing seems reasonable.
                        }
                        // Buffer flushed implicitly on drop if flush() wasn't called or succeeded
                    }
                    Err(e) => {
                        eprintln!(
                            "Error writing content to file '{}': {}. Skipping file.",
                            target_path.display(),
                            e
                        );
                        continue; // Skip this file
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "Error creating/opening file '{}' for writing: {}. Skipping file.",
                    target_path.display(),
                    e
                );
                continue; // Skip this file
            }
        }
        restored_count += 1;
    }

    if found_blocks == 0 {
        println!(
            "Warning: No valid sheafy blocks found in '{}'. No files restored.",
            absolute_input_path.display()
        );
    } else {
        println!(
            "\nRestore complete. {} file(s) restored/overwritten in {}.",
            restored_count,
            working_dir.display()
        );
    }

    Ok(())
}
