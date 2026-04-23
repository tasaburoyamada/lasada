# Lasada Prerequisites

The following environments and settings are required to successfully run and develop `lasada`.

## 1. Operating System
- **Linux** (Recommended)
- **Windows Subsystem for Linux (WSL/WSL2)**
  - This tool is designed to use the WSL environment as a safe sandbox.

## 2. Software & Tools
- **Rust Toolchain**
  - `cargo` and `rustc` must be installed (Edition 2024 recommended).
- **Bash**
  - `BashExecutor` uses `/bin/bash` on the system to execute commands.
- **Git**
  - Used for version control and obtaining source code.

## 3. Network & API
- **LLM Connection Environment**
  - Access to an in-house LLM or an OpenAI-compatible API.
  - Verification of connectivity to the Internet (or the target local network).
- **API Key**
  - A valid key must be set in the `LLM_API_KEY` environment variable.

## 4. Knowledge & Skills
- **Basic Terminal Operations**
  - Setting environment variables and executing commands.
- **Basic Security Awareness**
  - Understanding how to manage API keys and the risks of automatically executing AI-generated commands.

## 5. Recommended Directory Structure
- To manage project knowledge and design philosophy, `HV-CAD-Framework/specs/` should exist in a parent directory, with `.vlog` files being accumulated there.
