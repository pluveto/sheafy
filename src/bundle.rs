use crate::config::{Config, DEFAULT_BUNDLE_NAME};
use anyhow::{bail, Context, Result};
use ignore::WalkBuilder;
use std::{
    collections::HashSet,
    fs::File,
    io::{BufWriter, Read, Write},
    path::{PathBuf},
};

pub fn run_bundle(
    cli_filters: Option<Vec<String>>,
    cli_output: Option<String>,
    cli_use_git: bool,
    cli_no_git: bool,
) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;
    let current_dir = config.get_working_dir()?;

    // Determine effective settings (CLI > Config > Default)
    let filters_str: Vec<String> = cli_filters.or(config.sheafy.filters).unwrap_or_default();
    if filters_str.is_empty() {
        bail!(
            "No file filters provided via flags or in config. Cannot proceed."
        );
    }
    let filters: HashSet<String> = filters_str
        .into_iter()
        .map(|f| f.trim_start_matches('.').to_lowercase())
        .collect();
    println!("Using filters: {:?}", filters.iter().collect::<Vec<_>>());

    let output_filename = cli_output
        .or(config.sheafy.bundle_name)
        .unwrap_or_else(|| DEFAULT_BUNDLE_NAME.to_string());
    let output_path = PathBuf::from(&output_filename);
    let absolute_output_path = current_dir.join(&output_path).canonicalize().ok();

    println!("Output file: {}", output_filename);

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

    let mut matched_files: Vec<PathBuf> = Vec::new();
    let config_path_abs = current_dir.join("sheafy.toml").canonicalize().ok();
    let executable_path_abs = std::env::current_exe().ok();

    let mut builder = WalkBuilder::new(&current_dir);
    builder
        .hidden(true)
        .parents(effective_use_gitignore)
        .ignore(effective_use_gitignore)
        .git_ignore(effective_use_gitignore)
        .git_global(effective_use_gitignore)
        .git_exclude(effective_use_gitignore);

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

        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }

        let absolute_path = path.canonicalize().ok();

        if Some(absolute_path.as_ref()) == config_path_abs.as_ref().map(Some)
            || Some(absolute_path.as_ref()) == absolute_output_path.as_ref().map(Some)
            || Some(absolute_path.as_ref()) == executable_path_abs.as_ref().map(Some)
        {
            continue;
        }

        if let Some(ext) = path.extension().and_then(|os| os.to_str()) {
            if filters.contains(&ext.to_lowercase()) {
                if let Some(relative_path) = pathdiff::diff_paths(path, &current_dir) {
                    matched_files.push(relative_path);
                } else {
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

    matched_files.sort();

    println!("\nCreating Markdown file: {}", output_filename);
    let output_file = File::create(&output_path)
        .with_context(|| format!("Failed to create output file: {}", output_filename))?;
    let mut writer = BufWriter::new(output_file);
    if let Some(prologue) = config.sheafy.prologue {
        writer.write_all(prologue.as_bytes())?;
    }

    for rel_path in &matched_files {
        let header_path = rel_path
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        println!("  Adding: {}", header_path);

        let mut file_content = String::new();
        let full_read_path = current_dir.join(rel_path);
        match File::open(&full_read_path) {
            Ok(mut f) => {
                if let Err(e) = f.read_to_string(&mut file_content) {
                    eprintln!(
                        "Warning: Could not read file '{}': {}. Skipping.",
                        full_read_path.display(),
                        e
                    );
                    continue;
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Could not open file '{}': {}. Skipping.",
                    full_read_path.display(),
                    e
                );
                continue;
            }
        }

        let lang_hint = rel_path
            .extension()
            .and_then(|os| os.to_str())
            .map(crate::restore::get_language_hint)
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
