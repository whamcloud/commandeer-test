# Commandeer Test

A CLI test binary substitute with record and replay.

## Overview

Commandeer allows you to record command-line invocations and their outputs during testing, then replay them later for deterministic test execution. This is particularly useful for:

- Testing CLI applications that invoke external commands
- Creating reproducible test environments
- Mocking system commands in integration tests
- Recording and replaying complex command interactions

## Workspace Structure

This project is organized as a Cargo workspace with two crates:

- **`commandeer-test`** - Core library providing record/replay functionality
- **`commandeer-macros`** - Procedural macros for test setup automation

## Installation

Add commandeer to your `Cargo.toml`:

```console
cargo add commandeer --dev
```

## Usage

### CLI Binary

The `commandeer` binary provides standalone record and replay functionality:

#### Recording Commands

```console
# Record a simple command
commandeer record --command echo hello world

# Record with custom storage file
commandeer record --file my-recordings.json --command ls -la
```

#### Replaying Commands

```bash
# Replay a recorded command
commandeer replay --command echo hello world

# Replay from custom storage file
commandeer replay --file my-recordings.json --command ls -la
```

### Library Usage

#### Test Environment with Mocking

```rust
use commandeer_test::{Commandeer, Mode};
use serial_test::serial;

#[test]
#[serial]
fn test_with_mocked_commands() {
    let commandeer = Commandeer::new("my-test.json", Mode::Record);

    // Mock specific commands to intercept them during test execution
    commandeer.mock_command("git");

    // Your test code that calls git/npm will now be recorded
    let output = std::process::Command::new("git")
        .args(&["status"])
        .output()
        .unwrap();

    assert!(output.status.success());
}
```

### Procedural Macro

The `#[commandeer]` macro provides automatic test setup:

```rust
use commandeer_test::commandeer;
use serial_test::serial;

#[test]
#[commandeer(Record, "git", "npm", "curl")]
#[serial]
fn with_macro_test() {
    // Macro roughly_expands_to:
    // let commandeer = Commandeer::new("test_with_macro_test.json", Mode::Record);
    // commandeer.mock_command("git");
    // commandeer.mock_command("npm");
    // commandeer.mock_command("curl");

    let output = std::process::Command::new("git")
        .args(&["--version"])
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[tokio::test]
#[commandeer(Replay, "git")]
#[serial]
async fn test_replay_with_macro() {
    // Uses replay mode with the same automatic setup
    let output = tokio::process::Command::new("git")
        .arg("--version")
        .output()
        .await
        .unwrap();

    assert!(output.status.success());
}
```

#### Macro Features

- **Automatic file naming**: Test file names are generated as `test_{function_name}.json`
- **Mode selection**: Supports both `Record` and `Replay` modes
- **Command mocking**: Automatically sets up mocks for specified commands

## How It Works

### Recording Mode

1. Commandeer intercepts specified command invocations
2. Executes the real commands and captures:
   - Command name and arguments
   - Standard output (stdout)
   - Standard error (stderr)
   - Exit code
3. Stores results in JSON format for later replay

### Replay Mode

1. Commandeer intercepts specified command invocations
2. Looks up previous recordings based on command name and arguments
3. Returns the stored stdout, stderr, and exit code
4. Provides deterministic test execution without external dependencies

### Mock System

The library uses a sophisticated PATH manipulation system:

- Creates temporary mock binaries that intercept command calls
- Mock binaries delegate to the commandeer CLI for record/replay logic
- Original PATH is preserved and restored
- Works across different shell environments

## Storage Format

Recordings are stored in JSON format:

```json
{
  "commands": {
    "git:--version": [
      {
        "binary_name": "git",
        "args": ["--version"],
        "stdout": "git version 2.39.0\n",
        "stderr": "",
        "exit_code": 0
      }
    ]
  }
}
```

## Development

### Building the Workspace

```bash
# Build all crates
cargo build --workspace

# Build specific crate
cargo build -p commandeer-test
```

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p commandeer
```

### Testing the CLI

```bash
# Test record functionality
cargo run record --command echo "Hello, Commandeer"

# Test replay functionality
cargo run replay --command echo "Hello, Commandeer"
```

## Use Cases

### Integration Testing

```rust
#[test]
#[commandeer(Record, "docker", "kubectl")]
fn test_deployment_pipeline() {
    // Test your deployment scripts that call docker and kubectl
    deploy_application();

    // Commands are recorded for later replay in CI
}
```

### CLI Application Testing

```rust
#[test]
#[commandeer(Replay, "git", "ssh")]
fn test_git_operations() {
    // Test git workflows without requiring actual git repository
    run_git_workflow();
}
```

### External Service Mocking

```rust
#[test]
#[commandeer(Record, "curl", "wget")]
fn test_api_interactions() {
    // Record API calls during development
    // Replay them in tests for consistent behavior
    fetch_external_data();
}
```

## License

Licensed under the MIT License. See `LICENSE` file for details.

## Contributing

Contributions are welcome! Please ensure tests pass and follow the existing code style.

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request
