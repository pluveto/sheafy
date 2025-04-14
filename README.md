# Sheafy - Project File Bundler

## Overview

Sheafy is a command-line tool that bundles project files into a single Markdown document and can restore files from such bundles. It's particularly useful for sharing code with AI assistants or archiving projects in a readable format.

## Features

- **Smart Bundling**: Collects files from your project directory into a well-formatted Markdown file
- **Flexible Filtering**: Supports file extension filters via CLI or config file
- **Gitignore Integration**: Respects `.gitignore` rules by default (configurable)
- **Restore Capability**: Can recreate the original file structure from a bundle
- **Configurable**: Supports prologue/epilogue text and output filename configuration

## Installation

### From Source

1. Ensure you have Rust installed (via [rustup](https://rustup.rs/))
2. Clone this repository
3. Build and install:
   ```bash
   cargo install --path .
   ```

## Usage

### Basic Commands

**Create a bundle:**
```bash
sheafy bundle -f rs,toml,md -o project_bundle.md
```

**Restore files from a bundle:**
```bash
sheafy restore project_bundle.md
```

### Configuration

Create a `sheafy.toml` file in your project root to customize behavior:

```toml
[sheafy]
filters = ["rs", "toml", "md"]  # File extensions to include
bundle_name = "docs/project_bundle.md"  # Output filename
use_gitignore = true  # Whether to respect .gitignore rules
prologue = """
# Project Bundle

This is a bundle of all files in the project directory.
"""
epilogue = """
<!-- End of Project Bundle -->
"""
```

## Command Line Options

### Bundle Command

```
USAGE:
    sheafy bundle [OPTIONS]

OPTIONS:
    -f, --filters <FILTERS>...    Comma-separated list of file extensions to include
    -o, --output <OUTPUT>         Output Markdown filename
        --use-gitignore           Force use of .gitignore rules
        --no-gitignore            Force disabling .gitignore rules
```

### Restore Command

```
USAGE:
    sheafy restore <INPUT_FILE>

ARGS:
    <INPUT_FILE>    The Markdown file to restore from
```

## Examples

**Bundle only Rust and Markdown files:**
```bash
sheafy bundle -f rs,md
```

**Bundle with custom output name:**
```bash
sheafy bundle -f py -o python_files.md
```

**Restore files overwriting existing ones:**
```bash
sheafy restore backup_2023-12-01.md
```

## License

See [LICENSE](LICENSE) for more information.
