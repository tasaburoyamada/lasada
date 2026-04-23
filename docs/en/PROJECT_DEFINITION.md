# Project: Lasada (Rust / Plugin Architecture)

A project to rebuild a simpler and more user-friendly interpreter based on the ideas of Open-Interpreter.

## 1. Objectives
- Completely eliminate the complex dependencies and configurations of Open-Interpreter and existing tools, adopting a robust architecture based on Rust.
- Implement plugin-based execution engines and LLM backends (trait-based) to provide high extensibility and safety.

## 2. Project Phases
### Phase 1: Basic Architecture Design
- [x] Define core traits for `Lasada` (`ExecutionEngine`, `LlmBackend`, etc.)
- [x] Determine project structure and create base directories

### Phase 2: MVP Plugin Implementation
- [x] Implement Mock LLM plugin (interface for future in-house LLM replacement)
- [x] Implement stateful Bash execution engine (asynchronous wrapper of `std::process` in Rust)
- [x] Implement the "Orchestrator" to integrate the execution engine and LLM
- [x] Implement basic CLI / interactive loop (state management)

### Phase 3: Expansion & Refinement
- [ ] Explore sandboxing (integration with Wasm/Docker for environment isolation)
- [ ] Enhance error handling and auto-correction features
- [ ] Optimize session management using asynchronous processing (`tokio`)

## 3. Technology Stack
- Language: Rust
- Architecture: Trait-based plugin-oriented
- Execution Environment: OS native or custom runtime (Python-free)

## 4. Progress Tracking
- Completed tasks are marked with [x].
