use crate::config::{Config, DEFAULT_BUNDLE_NAME};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
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
        "txt" => "",
        _ => "",
    }
}

pub fn run_restore(input_filename: Option<String>) -> Result<()> {
    println!("Attempting to restore files");
    let config = Config::load().context("Failed to load configuration")?;
    let current_dir = config.get_working_dir()?;

    let input_path = Path::new(
        input_filename
            .as_deref()
            .or(config.sheafy.bundle_name.as_deref())
            .unwrap_or(DEFAULT_BUNDLE_NAME),
    );

    let content = fs::read_to_string(input_path)
        .with_context(|| format!("Failed to read input file: {}", input_path.display()))?;

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

        let target_path =
            current_dir.join(rel_path_str.replace('/', std::path::MAIN_SEPARATOR_STR));

        println!("  Restoring: {}", target_path.display());

        if let Some(parent_dir) = target_path.parent() {
            if !parent_dir.exists() && !parent_dir.as_os_str().is_empty() {
                println!("    Creating directory: {}", parent_dir.display());
                fs::create_dir_all(parent_dir).with_context(|| {
                    format!("Failed to create directory: {}", parent_dir.display())
                })?;
            }
        }

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
            input_path.display()
        );
    } else {
        println!(
            "\nRestore complete. {} file(s) restored/overwritten.",
            restored_count
        );
    }

    Ok(())
}
