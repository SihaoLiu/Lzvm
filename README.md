# Lzvm

Lzvm is an early-stage zero-knowledge virtual machine project.

The goal is to build a ZKVM from the ground up with a Rust-first core, C++ integration for GPU acceleration, and access to LLVM infrastructure where it is useful for compilation, analysis, or execution support.

## Goals

- Provide a clear virtual machine architecture suitable for zero-knowledge proving.
- Keep the core implementation in Rust for safety, maintainability, and tooling.
- Use C++ for performance-critical interoperability with GPU runtimes and LLVM components.
- Maintain clean boundaries between the VM, proving interfaces, compilation tooling, and acceleration backends.

## Technology

- Rust: primary implementation language.
- C++: GPU acceleration and LLVM-related infrastructure.
- LLVM: compilation and program analysis support where needed.

## Current Scope

This repository starts with project documentation only. The implementation will be added incrementally as the VM architecture, instruction model, proving interface, and acceleration boundaries are defined.
