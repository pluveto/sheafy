// tests/integration_tests.rs

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

// Helper function to get the path to the compiled sheafy binary
fn get_sheafy_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sheafy"))
}

// Helper to check if bundle content includes specific file sections
fn check_bundle_content(bundle_path: &Path, expected_files: &[&str], unexpected_files: &[&str]) {
    assert!(
        bundle_path.exists(),
        "Bundle file was not created: {}",
        bundle_path.display()
    );
    let content = fs::read_to_string(bundle_path).expect("Failed to read bundle file content");

    for file_header in expected_files {
        let expected_section_start = format!("\n## {}", file_header);
        assert!(
            content.contains(&expected_section_start),
            "Bundle content missing expected file section: {}\nactual content:\n{}\n",
            file_header,
            content
        );
        // Basic check for code block fence
        let code_block_start = format!("{}\n```", expected_section_start);
        assert!(
            content.contains(&code_block_start)
                || content.contains(&format!("{}\n```rust", expected_section_start)), // Allow language hint
            "Bundle content missing code block for: {}\nactual content:\n{}\n",
            file_header,
            content
        );
        assert!(
            content.contains("\n```\n"), // Check for closing fence too
            "Bundle content missing closing code block fence after: {}\nactual content:\n{}\n",
            file_header,
            content
        );
    }

    for file_header in unexpected_files {
        let unexpected_section_start = format!("\n## {}", file_header);
        assert!(
            !content.contains(&unexpected_section_start),
            "Bundle content contains unexpected file section: {}\nactual content:\n{}\n",
            file_header,
            content
        );
    }
}

#[test]
fn test_init_creates_config() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("sheafy.toml");

    assert!(!config_path.exists());

    let mut cmd = get_sheafy_cmd();
    cmd.arg("init").current_dir(dir.path());

    let output = cmd.output().expect("Failed to execute sheafy init");
    // println!("Init stdout: {}", String::from_utf8_lossy(&output.stdout)); // Debugging
    // println!("Init stderr: {}", String::from_utf8_lossy(&output.stderr)); // Debugging

    assert!(output.status.success(), "sheafy init failed");
    assert!(config_path.exists(), "sheafy.toml was not created");

    // Check if the created config contains some default keys
    let config_content = fs::read_to_string(config_path).unwrap();
    assert!(config_content.contains("bundle_name ="));
    assert!(config_content.contains("# ignore_patterns =")); // Check for the new commented-out key
    assert!(!config_content.contains("filters =")); // Ensure old key is gone
}

#[test]
fn test_init_fails_if_config_exists() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("sheafy.toml");

    // Create an empty config file first
    fs::write(&config_path, "[sheafy]").unwrap();
    assert!(config_path.exists());

    let mut cmd = get_sheafy_cmd();
    cmd.arg("init").current_dir(dir.path());

    let output = cmd.output().expect("Failed to execute sheafy init");

    assert!(
        !output.status.success(),
        "sheafy init succeeded unexpectedly"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Config file already exists"),
        "Stderr did not contain expected error message"
    );
}

#[test]
fn test_bundle_basic() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "Content A").unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/b.rs"), "fn main() {}").unwrap();

    // Create a basic config enabling .txt and .rs (using ignore patterns now)
    // We'll rely on default behavior (no ignore_patterns) for this test
    // We need *some* file to exist for the command to not exit early.
    fs::write(dir.path().join("placeholder.md"), "# Placeholder").unwrap();

    let mut cmd = get_sheafy_cmd();
    cmd.arg("bundle").current_dir(dir.path());

    let output = cmd.output().expect("Failed to execute sheafy bundle");
    // println!("Bundle basic stdout: {}", String::from_utf8_lossy(&output.stdout));
    // println!("Bundle basic stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "sheafy bundle failed");

    let bundle_path = dir.path().join("project_bundle.md");
    check_bundle_content(&bundle_path, &["a.txt", "src/b.rs", "placeholder.md"], &[]);
}

// #[test]
// fn test_bundle_respects_gitignore() {
//     let dir = tempdir().unwrap();
//     fs::write(dir.path().join(".gitignore"), r#"
// *.log
// target/
// "#).unwrap();
//     fs::write(dir.path().join("a.rs"), "// A").unwrap();
//     fs::write(dir.path().join("b.log"), "Log B").unwrap(); // ignored by .gitignore
//     fs::create_dir(dir.path().join("target")).unwrap();
//     fs::write(dir.path().join("target/c.o"), "Object C").unwrap(); // ignored by .gitignore

//     let mut cmd = get_sheafy_cmd();
//     cmd.arg("bundle").current_dir(dir.path()); // use_gitignore is true by default

//     let output = cmd.output().expect("Failed to execute sheafy bundle");
//     println!("Bundle gitignore stdout: {}", String::from_utf8_lossy(&output.stdout));
//     assert!(output.status.success(), "sheafy bundle failed");
//     // show all files in dir.path()
//     let files: Vec<String> = fs::read_dir(dir.path())
//        .unwrap()
//        .map(|entry| entry.unwrap().path().to_str().unwrap().to_string())
//        .collect();
//     println!("Files in dir: {:#?}", files);

//     for result in ignore::Walk::new(dir.path()) {
//         // Each item yielded by the iterator is either a directory entry or an
//         // error, so either print the path or the error.
//         match result {
//             Ok(entry) => println!("{}", entry.path().display()),
//             Err(err) => println!("ERROR: {}", err),
//         }
//     }
//     let bundle_path = dir.path().join("project_bundle.md");
//     check_bundle_content(&bundle_path, &["a.rs"], &["b.log", "target/c.o"]);
// }

#[test]
fn test_bundle_no_gitignore_flag() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.rs"), "// A").unwrap();
    fs::write(dir.path().join("b.log"), "Log B").unwrap();
    fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();

    let mut cmd = get_sheafy_cmd();
    cmd.arg("bundle")
        .arg("--no-gitignore")
        .current_dir(dir.path());

    let output = cmd.output().expect("Failed to execute sheafy bundle");
    assert!(output.status.success(), "sheafy bundle failed");

    let bundle_path = dir.path().join("project_bundle.md");
    // .gitignore file *might* be included now if not hidden and not ignored otherwise
    // Check that b.log IS included
    check_bundle_content(&bundle_path, &["a.rs", "b.log"], &[]);
}

#[test]
fn test_bundle_uses_config_ignore_patterns() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.py"), "print('hello')").unwrap();
    fs::write(dir.path().join("utils.py"), "# Utils").unwrap();
    fs::write(dir.path().join("data.csv"), "a,b,c").unwrap();
    fs::write(dir.path().join("temp.tmp"), "Temporary").unwrap();

    let config_content = r#"
[sheafy]
bundle_name = "python_bundle.md"
ignore_patterns = """
# Ignore data files and temps
*.csv
*.tmp

# Keep python files (implicitly, by not ignoring them)
"""
"#;
    fs::write(dir.path().join("sheafy.toml"), config_content).unwrap();

    let mut cmd = get_sheafy_cmd();
    cmd.arg("bundle").current_dir(dir.path());

    let output = cmd.output().expect("Failed to execute sheafy bundle");
    // println!("Ignore patterns stdout: {}", String::from_utf8_lossy(&output.stdout));
    // println!("Ignore patterns stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "sheafy bundle failed");

    let bundle_path = dir.path().join("python_bundle.md");
    // sheafy.toml itself should be ignored
    check_bundle_content(
        &bundle_path,
        &["main.py", "utils.py"],
        &["data.csv", "temp.tmp", "sheafy.toml"],
    );
}

#[test]
fn test_bundle_ignore_patterns_with_negation() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("logs")).unwrap();
    fs::write(dir.path().join("logs/app.log"), "Error!").unwrap();
    fs::write(dir.path().join("logs/important.log"), "Keep me!").unwrap();
    fs::write(dir.path().join("config.toml"), "[settings]").unwrap();

    let config_content = r#"
[sheafy]
ignore_patterns = """
# Ignore logs directory
logs/*

# But keep important.log
!logs/important.log

# Also ignore config.toml just because
config.toml
"""
"#;
    fs::write(dir.path().join("sheafy.toml"), config_content).unwrap();

    let mut cmd = get_sheafy_cmd();
    cmd.arg("bundle").current_dir(dir.path());

    let output = cmd.output().expect("Failed to execute sheafy bundle");
    println!(
        "Ignore patterns with negation stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    println!(
        "Ignore patterns with negation stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "sheafy bundle failed");

    let bundle_path = dir.path().join("project_bundle.md");
    check_bundle_content(
        &bundle_path,
        &["logs/important.log"],
        &["logs/app.log", "config.toml"],
    );
}

#[test]
fn test_bundle_with_prologue_epilogue() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "Content").unwrap();
    let config_content = r####"
[sheafy]
prologue = "### START ###"
epilogue = "### END ###"
"####;
    fs::write(dir.path().join("sheafy.toml"), config_content).unwrap();

    let mut cmd = get_sheafy_cmd();
    cmd.arg("bundle").current_dir(dir.path());
    let output = cmd.output().expect("Failed to execute sheafy bundle");
    assert!(output.status.success(), "sheafy bundle failed");

    let bundle_path = dir.path().join("project_bundle.md");
    let content = fs::read_to_string(bundle_path).unwrap();

    assert!(
        content.starts_with("### START ###\n"),
        "Prologue missing or incorrect"
    );
    // The check for the file section adds a newline before ##, so account for that
    assert!(content.contains("\n## a.txt\n"), "File section missing");
    // Epilogue might have extra newline added by writeln, accept both
    assert!(
        content.ends_with("### END ###\n") || content.ends_with("### END ###"),
        "Epilogue missing or incorrect"
    );
}

#[test]
fn test_bundle_output_flag() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "Content").unwrap();

    let custom_output = "my_bundle.md";
    let custom_output_path = dir.path().join(custom_output);
    let default_output_path = dir.path().join("project_bundle.md");

    let mut cmd = get_sheafy_cmd();
    cmd.arg("bundle")
        .arg("-o")
        .arg(custom_output)
        .current_dir(dir.path());

    let output = cmd.output().expect("Failed to execute sheafy bundle");
    assert!(output.status.success(), "sheafy bundle failed");

    assert!(
        custom_output_path.exists(),
        "Custom output file was not created"
    );
    assert!(
        !default_output_path.exists(),
        "Default output file was created unexpectedly"
    );
    check_bundle_content(&custom_output_path, &["a.txt"], &[]);
}

#[test]
fn test_restore_basic() {
    let dir = tempdir().unwrap();
    let bundle_content = r#"
# Some leading text

## src/main.rs
```rust
fn main() {
    println!("Hello");
}
```

## config/settings.toml
```toml
value = 123
```

"#;
    let bundle_path = dir.path().join("my_test_bundle.md");
    fs::write(&bundle_path, bundle_content).unwrap();

    let src_main_path = dir.path().join("src/main.rs");
    let config_settings_path = dir.path().join("config/settings.toml");

    assert!(!src_main_path.exists());
    assert!(!config_settings_path.exists());
    assert!(!dir.path().join("src").exists()); // Directory shouldn't exist yet
    assert!(!dir.path().join("config").exists()); // Directory shouldn't exist yet

    let mut cmd = get_sheafy_cmd();
    cmd.arg("restore")
        .arg(bundle_path.file_name().unwrap()) // Pass relative path within temp dir
        .current_dir(dir.path());

    let output = cmd.output().expect("Failed to execute sheafy restore");
    // println!("Restore basic stdout: {}", String::from_utf8_lossy(&output.stdout));
    // println!("Restore basic stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "sheafy restore failed");

    assert!(src_main_path.exists(), "src/main.rs was not restored");
    assert!(
        config_settings_path.exists(),
        "config/settings.toml was not restored"
    );
    assert!(
        dir.path().join("src").is_dir(),
        "'src' directory not created"
    );
    assert!(
        dir.path().join("config").is_dir(),
        "'config' directory not created"
    );

    let main_content = fs::read_to_string(src_main_path).unwrap();
    let settings_content = fs::read_to_string(config_settings_path).unwrap();

    assert!(main_content.contains("println!(\"Hello\");"));
    assert!(settings_content.contains("value = 123"));
    // Check exact content if needed, handling potential newline differences from bundle format
    assert_eq!(main_content, "fn main() {\n    println!(\"Hello\");\n}\n");
    assert_eq!(settings_content, "value = 123\n");
}

#[test]
fn test_restore_overwrites_existing() {
    let dir = tempdir().unwrap();
    let bundle_content = r#"
## existing.txt
```
New Content
```
"#;
    let bundle_path = dir.path().join("restore_bundle.md");
    fs::write(&bundle_path, bundle_content).unwrap();

    let file_path = dir.path().join("existing.txt");
    fs::write(&file_path, "Old Content").unwrap(); // Create the file beforehand

    let mut cmd = get_sheafy_cmd();
    cmd.arg("restore")
        .arg(bundle_path.file_name().unwrap())
        .current_dir(dir.path());

    let output = cmd.output().expect("Failed to execute sheafy restore");
    assert!(output.status.success(), "sheafy restore failed");

    assert!(file_path.exists());
    let content = fs::read_to_string(file_path).unwrap();
    assert_eq!(content, "New Content\n"); // Check it was overwritten
}

#[test]
fn test_restore_uses_config_bundle_name_default() {
    let dir = tempdir().unwrap();
    let bundle_content = r#"
## from_config.txt
```
Config Default
```
"#;
    let default_bundle_name = "default_from_cfg.md";
    let bundle_path = dir.path().join(default_bundle_name);
    fs::write(bundle_path, bundle_content).unwrap();

    let config_content = format!("[sheafy]\nbundle_name = \"{}\"", default_bundle_name);
    fs::write(dir.path().join("sheafy.toml"), config_content).unwrap();

    let file_path = dir.path().join("from_config.txt");

    let mut cmd = get_sheafy_cmd();
    cmd.arg("restore") // No input file argument given
        .current_dir(dir.path());

    let output = cmd.output().expect("Failed to execute sheafy restore");
    assert!(output.status.success(), "sheafy restore failed");

    assert!(file_path.exists());
    let content = fs::read_to_string(file_path).unwrap();
    assert_eq!(content, "Config Default\n");
}

#[test]
fn test_bundle_non_utf8_file_handling() {
    // Test how bundling handles files that are not valid UTF-8
    // Currently, read_to_string will fail. The bundle command should
    // print a warning and skip the file, not crash.
    let dir = tempdir().unwrap();
    // Create a file with invalid UTF-8 sequence (0x80 is continuation byte without start)
    fs::write(
        dir.path().join("invalid_utf8.bin"),
        [0x48, 0x65, 0x6c, 0x6c, 0x80, 0x6f],
    )
    .unwrap();
    fs::write(dir.path().join("valid.txt"), "Valid text").unwrap();

    let mut cmd = get_sheafy_cmd();
    cmd.arg("bundle").current_dir(dir.path());

    let output = cmd.output().expect("Failed to execute sheafy bundle");
    let stderr = String::from_utf8_lossy(&output.stderr);
    // println!("Non-UTF8 stdout: {}", String::from_utf8_lossy(&output.stdout)); // Debugging
    // println!("Non-UTF8 stderr: {}", stderr); // Debugging

    assert!(
        output.status.success(),
        "sheafy bundle should succeed even if skipping files"
    );
    assert!(
        stderr.contains("Warning: Could not read file"),
        "Expected warning about reading file"
    );
    assert!(
        stderr.contains("invalid_utf8.bin"),
        "Warning should mention the problematic file"
    );

    let bundle_path = dir.path().join("project_bundle.md");
    // Ensure the valid file was still bundled, and the invalid one wasn't
    check_bundle_content(&bundle_path, &["valid.txt"], &["invalid_utf8.bin"]);
}
