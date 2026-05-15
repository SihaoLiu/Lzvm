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

Generate a raw fixed-column artifact from native binary setup metadata and a native binary fixed-column source:

```sh
cargo run -p lzvm-cli -- setup write-fixed-native <setup-info-bin> <columns-bin> <out-const>
```

This path uses repository-owned sectioned binary codecs for setup metadata and fixed-column source data. It is preferred for generated setup flows because it avoids JSON parsing for the inputs that feed raw fixed-column output.

Generate the same raw artifact from JSON setup metadata and a native binary fixed-column source:

```sh
cargo run -p lzvm-cli -- setup write-fixed-bin <setup-info-json> <columns-bin> <out-const>
```

This bridge path keeps existing setup metadata fixtures usable while fixed-column values move through the binary artifact codec.

Generate the same raw artifact from fixed-column source JSON:

```sh
cargo run -p lzvm-cli -- setup write-fixed <setup-info-json> <columns-json> <out-const>
```

This JSON path is kept for compact fixtures and debugging. The source shape is:

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

Publish a raw constant-tree artifact after validating its setup metadata and expected root:

```sh
cargo run -p lzvm-cli -- setup write-const-tree <setup-info-bin> <tree-bin> <root-bin> <out-consttree>
```

This command validates the raw tree length and root before publishing through a staging path. It does not compute the tree; native tree construction remains a backend task with Rust/C++/CUDA parity checks.

Generate row-major extended fixed-column leaves for native constant-tree construction:

```sh
cargo run -p lzvm-cli -- setup write-const-leaves <setup-info-bin> <columns-bin> <out-leaves>
```

This command validates fixed columns against setup metadata, extends them over the shifted extended domain, and writes the leaf bytes used by the constant-tree backend.

## Verification

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```
