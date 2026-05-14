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
- `crates/lzvm-setup`: native setup-generation primitives with validated staging and publish behavior.
- `crates/lzvm-cli`: repository-owned command entry points.

## Current Commands

Validate an existing setup directory:

```sh
cargo run -p lzvm-cli -- setup validate <setup-dir>
```

The command loads the discovered setup catalog, validates companion metadata and binary artifacts, and prints a stable summary.

Generate a raw fixed-column artifact from setup metadata and fixed-column source JSON:

```sh
cargo run -p lzvm-cli -- setup write-fixed <setup-info-json> <columns-json> <out-const>
```

The fixed-column source JSON has this shape:

```json
{
  "group_name": "group-a",
  "unit_name": "unit-a",
  "row_count": 4,
  "columns": [
    {
      "name": "main.left",
      "dimensions": [1],
      "values": [1, 2, 3, 4]
    }
  ]
}
```

The command writes through a staging path, validates the staged artifact against the setup metadata, and then publishes the final output path.

## Verification

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```
