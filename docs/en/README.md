# Lasada

**Lasada** is a next-generation AI agent and interpreter developed in Rust. 
Inheriting the design philosophy of `open-interpreter`, it delivers superior performance, memory safety, and a robust symbolic state management system to execute complex tasks with speed and reliability.

## Overview

Lasada is not just a "chatbot" but a high-performance **execution engine** that understands user philosophy and intent to directly manipulate OS resources. By moving away from Python-heavy environments, Lasada provides a lightweight, single-binary Rust architecture designed for professional engineering workflows.

## Key Features

- **Rust-Powered High Performance**:
  A pure Rust implementation serving as a high-performance, memory-safe replacement for `open-interpreter`.
- **Advanced Plugin Architecture**:
  A trait-based design that allows seamless integration of various execution engines:
  - **Bash**: State-persistent shell execution for direct system control.
  - **Python**: Script execution within isolated environments.
  - **Web**: High-speed automated web search, scraping, and real-time information extraction.
  - **Computer (Computer Use)**: GUI interaction and screen analysis capabilities.
- **Vision Support**:
  Native screen analysis support. Features a Visual Grid overlay for precise coordinate recognition and intuitive GUI interaction.
- **Local RAG (Retrieval-Augmented Generation)**:
  Embedded vector database powered by `fastembed`. Automatically indexes conversation history and external documentation to retrieve contextually relevant information.
- **Symbolic Context (.vlog)**:
  Implements the high-density `.vlog` format for state management. Defines AI behavior through "constraints and state transitions" rather than simple instructions, ensuring consistent and advanced reasoning.

## Architecture

1.  **Core Interpreter**: Orchestrates the overall workflow, integrating dialogue management, RAG, and state persistence.
2.  **Plugin Dispatcher**: Delegates tasks to specialized executors (Bash, Python, Web, Computer) based on the command type.
3.  **Context Manager**: Synchronizes L1 (short-term memory), L2 (long-term memory via vector DB), and `.vlog` (symbolic state).
4.  **LLM Connector**: Supports multiple backends, including OpenAI-compatible APIs and local models.

## Setup

### Prerequisites
- [Rust](https://www.rust-lang.org/) (Cargo, Edition 2024 or later)
- Optional: `xdotool`, `scrot` (for Computer Use features)

### Installation
```bash
git clone https://github.com/kubodad/lasada.git
cd lasada
./install.sh
```

## Usage

```bash
# Basic startup
lasada

# Debug mode (verbose logging)
lasada --debug

# Auto-run mode (skip command confirmation)
lasada --auto-run
```

## System Philosophy (HV-CAD)

Lasada is designed according to the principles of **HV-CAD (Human-Value Centric Autonomous Development)**. 
It defines AI as a "target for probability distribution manipulation" where humans remain the "sole arbiters of value." This approach aims to build a "Digital Twin" capable of achieving maximum results with minimal oversight by aligning AI behavior with human philosophical benchmarks.

## License
Apache License 2.0
