
# Sheafy - Project File Bundler

## Overview

Sheafy is a command-line tool that bundles project files into a single Markdown document and can restore files from such bundles. It's particularly useful for sharing code with AI assistants or archiving projects in a readable format.

## Features

- **Smart Bundling**: Collects files from your project directory into a well-formatted Markdown file.
- **Flexible Ignoring**: Uses `.gitignore` rules by default and supports additional custom ignore patterns via `sheafy.toml`.
- **Restore Capability**: Can recreate the original file structure from a bundle.
- **Configurable**: Supports prologue/epilogue text, output filename, working directory, and ignore behavior configuration.

## Installation

### From Source

1.  Ensure you have Rust installed (via [rustup](https://rustup.rs/))
2.  Clone this repository
3.  Build and install:
    ```bash
    cargo install --path .
    ```

## Usage

### Basic Commands

**Create a bundle (using defaults and `.gitignore`):**
```bash
sheafy bundle
```

**Create a bundle with a specific output name:**
```bash
sheafy bundle -o my_project_bundle.md
```

**Restore files from a bundle:**
```bash
sheafy restore project_bundle.md
```

**Initialize a default `sheafy.toml` config file:**
```bash
sheafy init
```

### Configuration

Create a `sheafy.toml` file in your project root to customize behavior:

```toml
[sheafy]
# Output filename for bundle command, optional, default `project_bundle.md`
# bundle_name = "docs/project_bundle.md"

# Optional working directory (relative to where sheafy is run), optional, default "."
# working_dir = "src"

# Whether to respect .gitignore rules, optional, default true
# use_gitignore = true

# Optional: Add custom ignore patterns (multi-line string, gitignore syntax)
# These are applied *in addition* to .gitignore rules (if use_gitignore is true).
# Patterns are relative to the working directory.
# ignore_patterns = """
# # Ignore all log files
# *.log
#
# # Ignore specific directories
# target/
# node_modules/
#
# # But include specific file types within an ignored directory (if needed)
# # !target/*.rs
# """

# Optional prologue text to include at start of bundle
# prologue = """
# # Project Bundle
#
# This is a bundle of all files in the project directory.
# """

# Optional epilogue text to include at end of bundle
# epilogue = """
# # """
```

## Command Line Options

### Init Command
```
USAGE:
    sheafy init
```
Creates a default `sheafy.toml` file.

### Bundle Command

```
USAGE:
    sheafy bundle [OPTIONS]

OPTIONS:
    -o, --output <OUTPUT>        Output Markdown filename (overrides config)
        --use-gitignore          Force use of .gitignore rules (overrides config if set to false)
        --no-gitignore           Force disabling .gitignore rules (overrides config and --use-gitignore)
```
*Note: File inclusion/exclusion is now primarily controlled by `.gitignore` (if enabled) and the `ignore_patterns` setting in `sheafy.toml`.*

### Restore Command

```
USAGE:
    sheafy restore [INPUT_FILE]

ARGS:
    <INPUT_FILE>    The Markdown file to restore from (optional, defaults to `bundle_name` in config or `project_bundle.md`)
```

## Examples

**Bundle using default settings:**
```bash
# Creates project_bundle.md respecting .gitignore and sheafy.toml ignore_patterns
sheafy bundle
```

**Bundle ignoring `.gitignore` rules but using `sheafy.toml` patterns:**
```bash
sheafy bundle --no-gitignore
```

**Bundle to a specific file:**
```bash
sheafy bundle -o my_code.md
```

**Restore files overwriting existing ones:**
```bash
sheafy restore backup_bundle.md
```

## License

See ![LICENSE](LICENSE) for more information.
