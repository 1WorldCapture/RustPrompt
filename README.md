# RustPrompt

A powerful command-line tool for managing and processing code snippets with XML output format, built in Rust.

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

## Features

- 🌲 Automatic project tree generation
- 📝 Interactive REPL interface with two modes:
  - Manual mode: For direct file management
  - Prompt mode: For collecting and managing prompt text
- 🔍 Smart path completion with gitignore support
- 📋 Clipboard integration for easy XML output
- 🔢 Token counting for GPT model compatibility
- 💡 Multi-line prompt editing support
- 🚫 Intelligent file filtering (respects .gitignore, hidden files)

## Installation

### Prerequisites

- Rust 1.70 or higher
- Cargo (Rust's package manager)

### Building from Source

1. Clone the repository:
```bash
git clone https://github.com/yourusername/rustprompt.git
cd rustprompt
```

2. Build the project:
```bash
cargo build --release
```

The compiled binary will be available at `target/release/rustprompt`.

## Usage

### Basic Commands

- `/add <path>` - Add files or directories to context
- `/remove <path>` - Remove files or directories from context
- `/context` - Show current context information
- `/copy` - Copy current context (with project tree) to clipboard
- `/reset` - Clear all context and prompt text
- `/mode [manual|prompt]` - View or switch modes
- `/help` - Show help information
- `/quit` - Exit program

### Mode-Specific Features

#### Manual Mode
- File management through `/add` and `/remove` commands
- Direct context manipulation
- Project tree generation

#### Prompt Mode
- Direct text input for prompt collection
- Multi-line editing support
- Automatic prompt text accumulation

### Example Usage

1. Start in manual mode:
```bash
./rustprompt
```

2. Add some files:
```bash
/add src/
```

3. Switch to prompt mode:
```bash
/mode prompt
```

4. Enter your prompt text:
```
Please analyze this code and suggest improvements.
```

5. Copy the final XML:
```bash
/copy
```

## Project Structure

```
rustprompt
├── src
│   ├── app         # Application state and management
│   ├── command     # Command parsing and execution
│   ├── core        # Core functionality (file scanning, XML generation)
│   ├── repl        # REPL engine and prompt handling
│   └── main.rs     # Entry point
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [reedline](https://docs.rs/reedline) for REPL functionality
- Uses [ignore](https://docs.rs/ignore) for .gitignore support
- Integrates [tiktoken-rs](https://docs.rs/tiktoken-rs) for token counting 