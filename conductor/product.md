# Initial Concept
A semantic contract language for human-AI collaboration in software development. Topos serves as a checkpoint for human verification of AI understanding, capturing software intent in a structured, human-readable format.

# Product Guide: Topos

## Target Users
- **AI-assisted software developers:** Specifically designed as a checkpoint for human-AI collaboration.
- **Technical product managers:** Using the "readable over formal" principle to bridge intent and implementation.
- **Teams using LLMs for code generation:** Ensuring AI understands intent before generating complex code.

## Goals
- **Bridge the gap:** Act as a checkpoint between natural language intent and code generation.
- **Human-verifiable checkpoint:** Ensure humans can verify AI understanding *before* code is generated.
- **Traceability:** Create a clear, structured path from requirements to behaviors, tasks, and code.
- **Foundations for verifiability:** Create the preconditions for future formal verification of code.

## Core Features (V1)
- **Core Language Specification:** A CommonMark-compatible, structured prose format.
- **Context Compiler:** Solving the "context window bottleneck" by generating focused AI rules (Cursor, Windsurf, Cline).
- **LSP Server:** Providing diagnostics, hover support, and go-to-definition for a professional IDE experience.
- **CLI Tools:** Supporting `check`, `format`, `trace`, `context`, and `drift` detection.
- **Traceability Reporting:** Linking requirements to concrete evidence (tests, commits, PRs).
- **Excellent Developer Experience (DX):** Prioritizing a smooth, intuitive flow for engineers.
- **Versioning Strategy:** Managing the evolution of the language and specifications.
- **Performance Optimization:** Leveraging Tree-sitter and Salsa for sub-millisecond responsiveness.

## Success Factors
- **Minimal friction for developers:** Strict CommonMark compatibility ensures Topos files are valid Markdown and easy to adopt.
- **High-quality AI context generation:** Ensuring the Context Compiler provides high accuracy and effectiveness for LLMs.
- **Robust integration with existing IDEs:** Seamless operation within VS Code, Cursor, Windsurf, and other modern development environments.

## Collaboration Loop
- **Intent-to-Code Pipeline:** Humans write high-level intent, Topos structures it, and AI generates code based on that structured understanding.
- **Intelligent Refinement:** AI suggests "typed holes" and refinements to the specification for human review and approval.
- **Human-Prompted Drift Detection:** Topos detects drift between specification and code, prompting human intervention to resolve divergences.
- **Evidence-Based Verification:** Automatically gathering proof of completion via test coverage, PRs, and benchmarks.