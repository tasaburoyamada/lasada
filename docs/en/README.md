# Lasada

A robust and extensible AI interpreter implemented in Rust.
Inheriting the design philosophy of Open-Interpreter, it aims to eliminate dependencies on Python and provide a high-performance, secure execution environment.

## Features

- **Pure Rust Implementation**: High execution speed and memory safety.
- **Python-Free**: Directly controls the system's `bash`. No Python runtime required.
- **Plugin Architecture**:
  - `LlmBackend`: Flexibly supports OpenAI-compatible APIs, in-house LLMs, and test Mocks.
  - `ExecutionEngine`: Currently supports Bash. Extensible to Wasm or Docker in the future.
- **Interactive UI**: Color-coded output with `colored` and progress displays with `indicatif`.
- **Persistent Sessions**: `BashExecutor` allows for directory changes (`cd`) and variable persistence within the same interaction.

## Architecture

The system consists of three core components:

1. **Core**: `Interpreter` manages the overall flow, mediating between the LLM and the execution engine.
2. **Traits**: Defines `LlmBackend` and `ExecutionEngine`.
3. **Plugins**: Concrete implementations (`OpenAICompatibleLlm`, `BashExecutor`, `MockLlm`).

## Setup

### Requirements
- [Rust](https://www.rust-lang.org/) (Cargo)

### Installation
```bash
./install.sh
```
This will build the binary and place it in `~/.local/bin/lasada`, and create a config file at `~/.config/lasada/config.toml`.

## Configuration

Behavior can be customized via `config.toml` and environment variables.

### 1. config.toml
Place `config.toml` in the project root or `~/.config/lasada/`.

```toml
[llm]
type = "openai_compatible" # or "mock"
model = "your-model-name"
base_url = "https://your-api-endpoint/v1"

[system]
prompt = "You are an expert AI assisting engineers..."
```

### 2. Environment Variables
Sensitive information like API keys should be set as environment variables.

```bash
export LLM_API_KEY=your_secret_key
```

## Usage

```bash
lasada
```

After launching, enter instructions at the prompt.
Example:
- "Show me the list of files in the current directory"
- "Show the current time"

## License
Apache License 2.0
