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
- `crates/lzvm-accel`: feature-gated C++/CUDA acceleration boundary with GPU field-arithmetic, NTT, shifted coset-extension, and Poseidon2 width-8/width-16 parity tests.
- `crates/lzvm-artifacts`: native readers, writers, validators, and PCS setup-plan derivation for setup and proving artifacts.
- `crates/lzvm-setup`: native setup-generation primitives with validated staging, publish behavior, and optional CUDA fixed-column extension plus native tree hashing.
- `crates/lzvm-prover`: native proof scheduling and preflight planning derived from setup catalogs.
- `crates/lzvm-cli`: repository-owned command entry points.

## Current Commands

Validate an existing setup directory:

```sh
cargo run -p lzvm-cli -- setup validate <setup-dir>
```

The command loads the discovered setup catalog, validates companion metadata and binary artifacts, and prints a stable summary.

Fingerprint an existing setup directory catalog:

```sh
cargo run -p lzvm-cli -- setup fingerprint <setup-dir>
```

This command loads the same setup catalog as validation, hashes the parsed global metadata, global constraint program, unit metadata, expression programs, verification-key roots, fixed-column byte counts, and constant-tree companion roots, then prints a deterministic catalog fingerprint. It is a catalog preflight identifier, not a full proof verifier or a byte-for-byte setup archive hash.

Check proof and public-value artifact consistency:

```sh
cargo run -p lzvm-cli -- verify preflight <proof-bin> <public-values-json>
```

This command parses the native proof envelope and public-values JSON, checks that setup hashes match, checks that the proof envelope references the canonical public-values hash, and prints a stable summary. It is an artifact preflight check, not a full proof verifier.

Check proof, public-value, and setup catalog consistency:

```sh
cargo run -p lzvm-cli -- verify setup-preflight <setup-dir> <proof-bin> <public-values-json>
```

This command runs the proof/public-values preflight and also checks that the proof setup hash matches the deterministic setup catalog fingerprint for the supplied setup directory. It is a setup-aware artifact preflight check, not a full proof verifier.

Encode setup metadata JSON into the repository-owned binary setup format:

```sh
cargo run -p lzvm-cli -- setup write-info-bin <setup-info-json> <out-setup-info-bin>
```

This command validates the JSON metadata and writes the canonical binary setup metadata used by native setup commands.

Generate a raw fixed-column artifact from native binary setup metadata and a native binary fixed-column source:

```sh
cargo run -p lzvm-cli -- setup write-fixed-native <setup-info-bin> <columns-bin> <out-const>
```

This path uses repository-owned sectioned binary codecs for setup metadata and fixed-column source data. It is preferred for generated setup flows because it avoids JSON parsing for the inputs that feed raw fixed-column output.

Generate native base fixed-column and constant-tree artifacts in one command:

```sh
cargo run -p lzvm-cli -- setup write-base-native [--backend cpu|cuda] <setup-info-bin> <columns-bin> <out-const> <out-consttree>
```

This command reads binary setup metadata plus sectioned or raw fixed-column source data, writes the raw fixed-column artifact, builds the native GL constant tree, validates both outputs through the setup crate, and publishes them through staging paths. The default backend is `cpu`; `cuda` is available when the CLI is built with the `cuda` feature.

Regenerate native base fixed-column and constant-tree artifacts for a setup directory:

```sh
cargo run -p lzvm-cli -- setup write-base-directory [--derive-verkey] [--backend cpu|cuda] <setup-dir>
```

This command derives units from the setup directory metadata, reads each unit's setup metadata and fixed columns, then rewrites the raw fixed-column artifact and publishes a validated native constant tree for every unit. By default it checks generated tree roots against existing binary verification keys. With `--derive-verkey`, it writes JSON and binary verification-key companions from the generated tree roots. The default backend is `cpu`; `cuda` is available when the CLI is built with the `cuda` feature.

Generate verification-key companions from a native constant tree:

```sh
cargo run -p lzvm-cli -- setup write-verkey-native <setup-info-bin> <consttree> <out-verkey-json> <out-verkey-bin>
```

This command reads binary setup metadata and a raw constant-tree artifact, extracts the tree root through the native artifact parser, writes JSON and binary verification-key companions through staging paths, and validates both companions before publishing them.

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

This command validates the raw tree length and root before publishing through a staging path. It does not compute the tree; use `setup write-const-native` or `setup write-base-native` when the tree should be built by this repository.

Generate row-major extended fixed-column leaves for native constant-tree construction:

```sh
cargo run -p lzvm-cli -- setup write-const-leaves [--backend cpu|cuda] <setup-info-bin> <columns-bin> <out-leaves>
```

This command validates fixed columns against setup metadata, extends them over the shifted extended domain, and writes the leaf bytes used by the constant-tree backend. The default backend is `cpu`; `cuda` is available when the CLI is built with the `cuda` feature.

Generate a native GL constant-tree artifact directly from binary setup metadata and binary fixed-column source data:

```sh
cargo run -p lzvm-cli -- setup write-const-native [--backend cpu|cuda] <setup-info-bin> <columns-bin> [root-bin] <out-consttree>
```

This command accepts either sectioned or raw fixed-column bytes, extends fixed columns, builds leaf digests and parent nodes, optionally checks the generated root against a binary expected root, validates the resulting raw tree artifact, and publishes it through a staging path. The current native path covers arity 2 and arity 4; the optional `cuda` backend accelerates fixed-column extension and native tree hashing when the CLI is built with the `cuda` feature.

## Verification

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

CUDA parity tests require the local CUDA toolchain and a matching architecture target:

```sh
source /etc/profile.d/modules.sh && module load intel/compiler cuda openmpi
cargo test -p lzvm-accel --features cuda --test cuda_field
cargo test -p lzvm-setup --features cuda --test constant_tree_native
```
