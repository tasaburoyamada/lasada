# Lasada User Guide

This document explains how to install, configure, and extend `lasada`.

## 1. Quick Start

### Build
First, compile the project:
```bash
cargo build --release
```

### Run
```bash
./target/release/lasada
```
*Note: By default, it starts with `MockLlm`. To integrate with a real LLM, follow the configuration steps below.*

## 2. Configuration Guide

### Config File Location
Running the installer creates a configuration file at:
`~/.config/lasada/config.toml`

### Switching LLMs
Edit `config.toml` to select the LLM backend to use.

```toml
[llm]
# Specify "mock" or "openai_compatible"
type = "openai_compatible"
model = "your-model-name"
base_url = "https://your-api-endpoint/v1"
```

### Setting the API Key
The API key is read through the environment variable `LLM_API_KEY`.

**Recommended setup:**
Set the environment variable either temporarily or permanently in your shell configuration (.bashrc, .zshrc, etc.):
```bash
export LLM_API_KEY="your_secret_key"
```

> [!CAUTION]
> **Security Warning: Regarding the use of .env files**
> While it is possible to create and use a `.env` file within the project, it is **not recommended**. Accidentally committing a `.env` file to a version control system (like Git) creates a **risk of your API key being leaked and misused**. Whenever possible, set it directly as an environment variable or use a secret management tool.

## 3. Basic Operations
Once launched, you will see the `User >` prompt.

- **Natural Language Instructions**: Enter requests such as "List the sizes of files in the current directory."
- **Automatic Execution**: When the AI generates a `bash` command, it is executed automatically, and the result is fed back to the AI.
- **Exit**: Type `exit` or `quit`.

## 4. For Developers: Extending Functionality
`lasada` uses a plugin architecture. You can freely extend functionality by implementing the `LlmBackend` or `ExecutionEngine` traits defined in `src/core/traits.rs`.
Refer to `README.md` for more details.

## 5. Core Philosophy
The ultimate goal of this tool is **"to let the AI learn the judgment criteria of a human (you)."**
Based on `HV-CAD-Framework/specs/lasada_development.vlog`, it aims for a development process centered on high-density dialogue and human evaluation (regularization), eventually acting as a "Digital Twin" that completes complex tasks with minimal oversight.
