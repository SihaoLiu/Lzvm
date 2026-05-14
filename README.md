# Lzvm

Lzvm is an early-stage zero-knowledge virtual machine and proving stack.

The goal is to build a native stack with a Rust-first core, C++ integration for performance-critical runtime boundaries, and CUDA acceleration for proof construction.

## Goals

- Provide a clear virtual machine architecture suitable for zero-knowledge proving.
- Keep artifact formats, validation, orchestration, and command surfaces in Rust.
- Use C++ and CUDA for backend work where native acceleration matters.
- Maintain clean boundaries between artifact loading, setup generation, proving, verification, compilation tooling, and acceleration backends.

## Workspace

- `crates/lzvm-field`: CPU reference field arithmetic used by artifact validation and backend parity tests.
- `crates/lzvm-artifacts`: native readers, writers, and validators for setup and proving artifacts.
- `crates/lzvm-cli`: repository-owned command entry points.

## Current Commands

Validate an existing setup directory:

```sh
cargo run -p lzvm-cli -- setup validate <setup-dir>
```

The command loads the discovered setup catalog, validates companion metadata and binary artifacts, and prints a stable summary.

## Verification

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```
