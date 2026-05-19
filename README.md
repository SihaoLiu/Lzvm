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
- `crates/lzvm-crypto`: native hash primitives for proof-input plumbing and future execution-path integration.
- `crates/lzvm-artifacts`: native readers, writers, validators, and PCS setup-plan derivation for setup and proving artifacts.
- `crates/lzvm-setup`: native setup-generation primitives with validated staging, publish behavior, and optional CUDA fixed-column extension plus native tree hashing.
- `crates/lzvm-prover`: native proof scheduling, witness commitment generation, proof artifact construction, and setup-aware verification.
- `crates/lzvm-cli`: repository-owned command entry points.

## Current Commands

Validate an existing setup directory:

```sh
cargo run -p lzvm-cli -- setup validate <setup-dir>
```

The command loads the discovered setup catalog, validates companion metadata and binary artifacts, and prints a stable summary with the catalog `setup_hash`.
When key material companions are present, the summary includes the material unit count and byte total.

Fingerprint an existing setup directory catalog:

```sh
cargo run -p lzvm-cli -- setup fingerprint <setup-dir>
```

This command loads the same setup catalog as validation, hashes the parsed global metadata, global constraint program, unit metadata, expression programs, verification-key roots, fixed-column byte counts, and constant-tree companion roots, then prints a deterministic catalog fingerprint. It is a catalog preflight identifier, not a full proof verifier or a byte-for-byte setup archive hash.

Generate native setup key material for a setup directory:

```sh
cargo run -p lzvm-cli -- setup generate-key [--backend cpu|cuda] <setup-dir>
```

This is the public directory-level setup generation entry point. It reads repository-owned binary setup, expression, verifier, and global metadata plus fixed-column inputs, then writes expression programs, verifier programs, raw fixed-column artifacts, native constant trees, binary verification keys, PCS setup plans, and PCS setup-material companions for every discovered unit. The default backend is `cpu`; `cuda` is available when the CLI is built with the `cuda` feature.
On success, the report includes `setup_hash` and `setup_directory_manifest` so later proof commands can bind to the generated key material without running a separate fingerprint command.

Write source-level setup companion artifacts into a setup directory:

```sh
cargo run -p lzvm-cli -- setup write-source-companions [--include-path <dir>] [--include-path-first] [--refresh-manifest] <main-file> <setup-dir>
```

This command loads the source program, records the loaded source graph in the setup directory's source-program archive companion, and records fixed-file pragmas in the setup directory's source fixed-file manifest companion. `setup validate` and setup fingerprints include these companions when present. Pass `--refresh-manifest` to write the setup directory manifest in the same command even when it was not already present.

Check proof and public-value artifact consistency:

```sh
cargo run -p lzvm-cli -- verify preflight <proof-bin> <public-values>
```

This command parses the native proof envelope and public-values artifact, checks that setup hashes match, checks that the proof envelope references the canonical public-values hash, and prints a stable summary. The public-values input may use the repository-owned binary format or the legacy JSON shape. It is an artifact preflight check, not a full proof verifier.

Check proof, public-value, and setup catalog consistency:

```sh
cargo run -p lzvm-cli -- verify setup-preflight <setup-dir> <proof-bin> <public-values>
```

This compatibility command runs the setup-aware artifact validation path. Prefer `verify proof` for direct proof verification.

Verify a proof artifact against a setup directory:

```sh
cargo run -p lzvm-cli -- verify proof <setup-dir> <proof-bin> <public-values>
```

This command reads the setup catalog, proof artifact, and public-values artifact, then validates catalog fingerprint binding, setup-directory manifest binding, proof/public-values binding, PCS material manifests, witness commitments, query plans, constant openings, witness openings, global constraints, global hints, and optional FRI openings.

Inspect the proof schedule derived from a setup directory:

```sh
cargo run -p lzvm-cli -- prove schedule <setup-dir>
```

This command loads the setup catalog, derives the native proof schedule, and prints the setup hash, unit count, fixed-byte total, query total, and maximum extended-domain size. It is a proof-runtime preflight summary, not a full proof constructor.

Validate the full proof run plan for a setup directory:

```sh
cargo run -p lzvm-cli -- prove plan [options] <setup-dir> <output-dir>
```

This command loads the setup catalog, derives the native proof run plan, checks partition, aggregation, output, and GPU execution settings, then prints a stable plan summary. It is a proof-runtime planning check, not a full proof constructor.

Validate proof input artifacts for a setup directory:

```sh
cargo run -p lzvm-cli -- prove inputs [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]
```

This command derives the native proof run plan, validates witness library, guest image, and optional public-input paths, parses witness and guest image metadata, and prints stable input fingerprints. It is an execution-input preflight check, not a full proof constructor.

Generate native witness commitments and a proof artifact:

```sh
cargo run -p lzvm-cli -- prove witness [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]
```

This command runs the native witness execution path, commits witness stages, and writes `proof.bin` when public inputs are supplied. Use `--trace-bytes <trace-bin>` for a single-unit run backed by a precomputed trace, `--trace-bundle <bundle-bin>` for bundled traces, and `--all-units` or aggregation options when the output must cover every discovered unit.

Generate a native PCS setup-plan artifact from binary setup metadata:

```sh
cargo run -p lzvm-cli -- setup write-pcs-plan <setup-info-bin> <out-pcs-plan>
```

This command derives domain sizes, query counts, commitment widths, opening points, and FRI folding layers from repository-owned binary setup metadata, then writes the canonical binary PCS setup-plan artifact used by later native proof setup work.

Generate native PCS setup-plan artifacts for every unit in a setup directory:

```sh
cargo run -p lzvm-cli -- setup write-pcs-directory <setup-dir>
```

This lower-level command derives each unit from setup-directory metadata, reads the unit setup metadata, and writes a canonical `.pcs-plan` companion next to that unit's setup artifact prefix. Use `setup generate-key` for the normal full directory flow.

Generate a native PCS setup-material artifact from binary setup inputs:

```sh
cargo run -p lzvm-cli -- setup write-pcs-material <setup-info-bin> <pcs-plan> <fixed-const> <consttree> <out-pcs-material>
```

This command validates that the PCS setup plan matches the binary setup metadata, reads the raw fixed-column artifact and native constant tree, then writes a canonical binary material descriptor with digests, tree root, and byte counts for later setup generation.

Generate native PCS setup-material artifacts for every unit in a setup directory:

```sh
cargo run -p lzvm-cli -- setup write-pcs-material-directory <setup-dir>
```

This lower-level command requires each unit's `.pcs-plan`, raw fixed-column artifact, and native constant tree to be present, then writes a `.pcs-material` companion next to that unit's setup artifact prefix. Use `setup generate-key` for the normal full directory flow.

Generate a program-image commitment cache for proof input binding:

```sh
cargo run -p lzvm-cli -- setup write-program-image-cache [--backend cpu|cuda] <program-bin> <guest-image> <constraint-digest-bin> <root-bin> <trace-rows> <trace-columns> <blowup-factor> <arity> <out-cache>
cargo run -p lzvm-cli -- setup write-program-image-cache [--backend cpu|cuda] --setup-dir <setup-dir> <program-bin> <guest-image> <root-bin> <trace-rows> <trace-columns> <blowup-factor> <arity> <out-cache>
```

The first form accepts an explicit 32-byte constraint digest. The second form derives that digest from the setup catalog and should be preferred when the cache is built next to a generated key directory. In both forms the digest must match the `setup_hash` used for proving. Proof input and witness commands reject cache files whose guest-image digest or setup hash no longer matches the current run.

Generate a raw fixed-column artifact from native binary setup metadata and a native binary fixed-column source:

```sh
cargo run -p lzvm-cli -- setup write-fixed-native <setup-info-bin> <columns-bin> <out-const>
```

This path uses repository-owned sectioned binary codecs for setup metadata and fixed-column source data. It is preferred for generated setup flows because it avoids JSON parsing for the inputs that feed raw fixed-column output.

Generate a sectioned fixed-column source artifact from literal fixed-column declarations:

```sh
cargo run -p lzvm-cli -- setup write-fixed-source [--include-path <dir>] [--include-path-first] <setup-info-bin> <main-file> <group-name> <unit-name> <out-columns-bin>
```

This command reads native binary setup metadata and source declarations such as `col fixed name = [1, 2];`, validates the resulting columns against the setup metadata, and publishes a sectioned fixed-column artifact. It intentionally accepts only literal decimal or hexadecimal sequence initializers; non-literal expressions should be lowered by an explicit setup compiler stage before this command runs.

Generate native base fixed-column and constant-tree artifacts in one command:

```sh
cargo run -p lzvm-cli -- setup write-base-native [--backend cpu|cuda] <setup-info-bin> <columns-bin> <out-const> <out-consttree>
```

This command reads binary setup metadata plus sectioned or raw fixed-column source data, writes the raw fixed-column artifact, builds the native GL constant tree, validates both outputs through the setup crate, and publishes them through staging paths. The default backend is `cpu`; `cuda` is available when the CLI is built with the `cuda` feature.

Regenerate native base fixed-column and constant-tree artifacts for a setup directory:

```sh
cargo run -p lzvm-cli -- setup write-base-directory [--derive-verkey] [--backend cpu|cuda] <setup-dir>
```

This command derives units from the setup directory metadata, reads each unit's setup metadata and fixed columns, then rewrites the raw fixed-column artifact and publishes a validated native constant tree for every unit. By default it checks generated tree roots against existing binary verification keys. With `--derive-verkey`, it writes binary verification-key artifacts from the generated tree roots. The default backend is `cpu`; `cuda` is available when the CLI is built with the `cuda` feature.

Generate a binary verification-key artifact from a native constant tree:

```sh
cargo run -p lzvm-cli -- setup write-verkey-native <setup-info-bin> <consttree> <out-verkey-bin>
```

This command reads binary setup metadata and a raw constant-tree artifact, extracts the tree root through the native artifact parser, writes the binary verification-key artifact through a staging path, and validates it before publishing.

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
cargo test -p lzvm-setup --features cuda
cargo test -p lzvm-cli --features cuda --test setup_write_base_directory
```
