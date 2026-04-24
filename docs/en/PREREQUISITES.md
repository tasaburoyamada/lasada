# Lasada Prerequisites

The following environments and settings are required to successfully run and develop `lasada`.

## 1. Operating System
- **Linux** (Recommended)
- **Windows Subsystem for Linux (WSL/WSL2)**
  - This tool is designed to use the WSL environment as a safe sandbox.
  - For GUI operations (Computer Use), WSLg or an X server configuration is required.

## 2. Software & Tools
The following tools must be installed and available in your PATH.

- **Rust Toolchain**
  - `cargo` and `rustc` must be installed (Edition 2024 recommended).
- **GUI Automation & Capture**
  - `xdotool`: Used for simulating mouse and keyboard operations.
  - `scrot` or `gnome-screenshot`: Used for taking screen captures.
- **Document Analysis**
  - `poppler-utils` (`pdftotext`): Required for PDF text extraction and analysis.
- **Bash**
  - `BashExecutor` uses `/bin/bash` on the system to execute commands.
- **Git**
  - Used for version control and obtaining source code.

## 3. Network & API
- **LLM Connection Environment**
  - Access to an OpenAI-compatible API.
  - Verification of connectivity to the Internet (or the target local network).
- **API Key**
  - A valid key must be set in the `LLM_API_KEY` environment variable.

## 4. Knowledge & Skills
- **Basic Terminal Operations**
  - Setting environment variables and executing commands.
- **Basic Security Awareness**
  - Understanding the risks of automatically executing AI-generated commands (especially with `--auto-run`).

## 5. Recommended Directory Structure
- To manage project knowledge and design philosophy, `HV-CAD-Framework/specs/` should exist in a parent directory, with `.vlog` files being accumulated there.
