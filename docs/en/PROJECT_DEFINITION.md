# Project: Lasada (Rust / HV-CAD Architecture)

A next-generation interpreter reconstruction project that fuses the overwhelming stability of Rust with the autonomous control of HV-CAD, inspired by Open-Interpreter.

## 1. Objectives
- **Complete Elimination of Python Dependency**: To eliminate complex dependencies, runtime instability, and the constraints of the Global Interpreter Lock (GIL), achieving native robustness and high performance through Rust.
- **Introduction of HV-CAD (Human-Value Centric Autonomous Development)**: Defining AI not as a "personality" but as a "target for probability distribution manipulation." Through symbolic state management (.vlog), human value judgments are directly reflected in the execution engine.
- **Plugin-Oriented Extensibility**: Abstracting execution engines and LLM backends based on traits to achieve both high security and flexible replaceability.

## 2. Architectural Principles
- **Fact Driven Orbit Determination (FDOD)**: Exhaustively investigate the "certain facts" (code, logs, environment) before inference to structurally suppress hallucinations.
- **Overcoming Statelessness**: To supplement the volatility of LLMs, past decisions and constraints are persisted in symbolic vector formats (`.vlog`) to maintain context consistency.
- **Separation of Generation and Evaluation**: AI specializes in "Generation (How)," while humans monopolize "Value Judgment (What is right)." AI output is always subject to regularization by humans.

## 3. Technical Goals
- **Core Implementation in Rust**: Asynchronous wrapping of `std::process`, highly efficient session management using `tokio`, and establishment of plugin interfaces via traits.
- **HV-CAD Integration**: High-density AI control using `@BIAS` (weighting of value criteria) and `@CONCEPT` (concept activation), without relying solely on natural language.
- **Autonomous Error Correction**: Feedback execution errors as facts, establishing a "self-healing loop" where AI autonomously generates and applies patches.

## 4. Roadmap (Phases)
### Phase 1: Basic Architecture Design [DONE]
- [x] Definition of Rust core traits (`ExecutionEngine`, `LlmBackend`, etc.)
- [x] Application of HV-CAD concepts to the project and determination of directory structure

### Phase 2: MVP & HV-CAD Implementation [DONE]
- [x] Implementation of stateful Bash execution engine (Rust)
- [x] Implementation of Orchestrator and basic CLI loop
- [x] Introduction of HV-CAD state management via `system_philosophy.vlog`, etc.

### Phase 3: Enhancement of Autonomy & Safety
- [ ] Sandboxing (separation of execution environment via Wasm/Docker integration, etc.)
- [ ] Integration of autonomous recovery protocols into error handling
- [ ] Optimization of asynchronous session management and consideration for multi-agent support

## 5. Technology Stack
- **Language**: Rust
- **State Management**: HV-CAD (.vlog)
- **Async Runtime**: tokio
- **License**: Apache License 2.0
