# iz CLI 🚀

A powerful CLI tool for testing Git commits in temporary directories without changing your active branch.

## What is it?

`iz` allows you to test your past commits safely. It extracts files from any commit to a temporary directory, runs your desired command in that directory, and cleans up automatically when finished (unless you specify otherwise).

## Features

- ✅ **Safe testing** - Never changes your active branch
- ✅ **Flexible temporary directories** - Configure via CLI, environment, or config file
- ✅ **Variable substitution** - Use `#{variable}` syntax in commands
- ✅ **Signal handling** - Proper cleanup on Ctrl+C interruption
- ✅ **Keep option** - Preserve temporary directories for inspection
- ✅ **Cross-platform** - Works on Windows, macOS, Linux
- ✅ **Comprehensive testing** - Unit and integration tests included

## Installation

```bash
# Build the project
cargo build --release

# Copy to system PATH (optional)
sudo cp target/release/iz /usr/local/bin/

# Or use directly
./target/release/iz --help
```

## Quick Start

1. **Create configuration file** in your project root:

```json
{
    "commands": {
        "run": "dotnet run",
        "build": "dotnet build", 
        "test": "dotnet test",
        "serve": "python -m http.server #{port}"
    },
    "temp_dir": ".iztemp",
    "keep": false
}
```

2. **Run commands against any commit**:

```bash
iz run 30b5302 run
iz run abc1234 build  
iz run HEAD~2 test
```

3. **Clean up temporary directories**:

```bash
iz cleanup                    # Interactive cleanup
iz cleanup --force            # Force cleanup without confirmation
iz cleanup --temp-dir /custom # Clean specific directory
```

## Configuration

### izconfig.json Format

```json
{
    "commands": {
        "run": "dotnet run",
        "build": "dotnet build",
        "test": "dotnet test", 
        "serve": "python -m http.server #{port}",
        "greet": "echo 'Hello #{name}, you are #{age} years old!'"
    },
    "temp_dir": ".iztemp",  
    "keep": false
}
```

### Configuration Fields

- **`commands`** (required): Command definitions with variable support
- **`temp_dir`** (optional): Base temporary directory path
- **`keep`** (optional): Whether to preserve temporary directories

### Variable Substitution

Use `#{variable}` syntax in commands:

```bash
# With variables
iz 30b5302 serve --param port=8080
iz abc1234 greet --param name=Alice --param age=25
```

## Usage

### Run Commands

```bash
# Basic usage
iz run <commit-id> <command>

# Examples
iz run HEAD run
iz run 30b5302 build
iz run abc1234 test
```

### With Parameters

```bash
# Single parameter
iz run 30b5302 serve --param port=3000

# Multiple parameters  
iz run abc1234 greet --param name=Bob --param age=30
```

### Temporary Directory Control

```bash
# Custom temporary directory
iz run 30b5302 run --temp-dir /tmp/my-test

# Keep temporary directory after execution
iz run 30b5302 run --keep

# Both options
iz run 30b5302 run --temp-dir /tmp/my-test --keep
```

### Cleanup Commands

```bash
# Interactive cleanup (asks for confirmation)
iz cleanup

# Force cleanup (no confirmation)
iz cleanup --force

# Clean specific temporary directory
iz cleanup --temp-dir /custom/temp

# Clean custom directory with force
iz cleanup --temp-dir /tmp/my-iz-temp --force
```

## Configuration Priority

Settings are applied in this order (highest to lowest priority):

1. **CLI parameters**: `--temp-dir`, `--keep`
2. **Environment variables**: `IZTEMP`
3. **Config file**: `temp_dir`, `keep` in `izconfig.json`
4. **Defaults**: `.iztemp` directory, `keep=false`

### Examples

```bash
# Environment variable
IZTEMP=/tmp/iz-custom iz run 30b5302 run

# CLI override (highest priority)
iz run 30b5302 run --temp-dir /tmp/override --keep
```

## Signal Handling

iz CLI properly handles interruption signals:

- **Ctrl+C (SIGINT)**: Gracefully stops and cleans up temporary directory
- **SIGTERM**: Also triggers cleanup and exit
- **Automatic cleanup**: Only when `keep=false` (default)

## Testing

### Run Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests only  
cargo test --test integration_tests
```

### Test Coverage

- **11 Unit Tests**: Core functionality (parsing, substitution, config)
- **9 Integration Tests**: Real CLI scenarios including cleanup feature
- **Error Handling**: Missing files, invalid parameters, command failures

## Project Structure

```
iz/
├── src/
│   ├── main.rs                   # Main CLI application
│   └── lib.rs                    # Core functions + unit tests
├── tests/
│   └── integration_tests.rs      # Integration tests
├── .gitignore                    # Git ignore rules
├── Cargo.toml                    # Rust dependencies
├── Cargo.lock                    # Dependency lock file
├── README.md                     # This file
└── target/                       # Build artifacts
    ├── debug/iz                  # Debug binary
    └── release/iz                # Optimized binary
```

## Requirements

- **Rust** (1.70+ recommended)
- **Git repository** (for the project you want to test)
- **izconfig.json** file in your project root

## Help

```bash
iz --help
```

Output:
```
CLI tool for testing Git commits in temporary directories

Usage: iz <COMMAND>

Commands:
  run      Run a command in a temporary directory with files from a specific commit
  cleanup  Clean up temporary directories created by iz
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues.

---

**Happy testing!** 🎯 