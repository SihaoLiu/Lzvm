use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use lzvm_artifacts::fixed::{
    read_fixed_columns_file, read_fixed_columns_file_for_setup, FixedColumn, FixedColumns,
};
use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, key_directory_catalog_digest_hex, read_key_directory_catalog,
    read_key_directory_layout, validate_key_directory_layout,
};
use lzvm_artifacts::pcs_plan::{derive_pcs_setup_plan, encode_pcs_setup_plan};
use lzvm_artifacts::proof::read_proof_artifact_file;
use lzvm_artifacts::public_values::{public_values_digest, read_public_values_file};
use lzvm_artifacts::setup_info::{
    encode_unit_setup_info, read_unit_setup_info_binary_file, read_unit_setup_info_file,
};
use lzvm_artifacts::verification_key::{read_verification_key_binary_file, VerificationKeyRoot};
use lzvm_prover::derive_prove_schedule;
use lzvm_setup::{
    build_constant_tree_from_fixed_columns_with_backend, write_base_constant_tree,
    write_base_fixed_columns, write_constant_tree_leaves_with_backend,
    write_verification_key_from_constant_tree, FixedExtensionBackend,
};
use serde_json::Value;

mod prove_inputs;
mod prove_plan;
mod prove_witness;

pub fn run_cli(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        ["prove", "inputs", rest @ ..] => prove_inputs::run(rest, stdout, stderr),
        ["prove", "plan", rest @ ..] => prove_plan::run(rest, stdout, stderr),
        ["prove", "schedule", setup_dir] => prove_schedule(setup_dir, stdout, stderr),
        ["prove", "schedule", ..] => write_prove_schedule_usage(stderr),
        ["prove", "witness", rest @ ..] => prove_witness::run(rest, stdout, stderr),
        ["verify", "setup-preflight", setup_dir, proof_bin, public_values_json] => {
            verify_setup_preflight(setup_dir, proof_bin, public_values_json, stdout, stderr)
        }
        ["verify", "setup-preflight", ..] => write_verify_setup_preflight_usage(stderr),
        ["verify", "preflight", proof_bin, public_values_json] => {
            verify_preflight(proof_bin, public_values_json, stdout, stderr)
        }
        ["verify", "preflight", ..] => write_verify_preflight_usage(stderr),
        ["setup", "fingerprint", setup_dir] => {
            fingerprint_setup_directory(setup_dir, stdout, stderr)
        }
        ["setup", "fingerprint", ..] => write_fingerprint_usage(stderr),
        ["setup", "validate", setup_dir] => validate_setup_directory(setup_dir, stdout, stderr),
        ["setup", "validate", ..] => write_validate_usage(stderr),
        ["setup", "write-info-bin", setup_info, out_setup_info_bin] => {
            write_setup_info_bin(setup_info, out_setup_info_bin, stdout, stderr)
        }
        ["setup", "write-info-bin", ..] => write_info_bin_usage(stderr),
        ["setup", "write-pcs-plan", setup_info_bin, out_pcs_plan] => {
            write_pcs_setup_plan(setup_info_bin, out_pcs_plan, stdout, stderr)
        }
        ["setup", "write-pcs-plan", ..] => write_pcs_plan_usage(stderr),
        ["setup", "write-fixed", setup_info, columns_json, out_const] => {
            write_fixed_columns(setup_info, columns_json, out_const, stdout, stderr)
        }
        ["setup", "write-fixed", ..] => write_fixed_usage(stderr),
        ["setup", "write-fixed-bin", setup_info, columns_bin, out_const] => {
            write_fixed_columns_bin(setup_info, columns_bin, out_const, stdout, stderr)
        }
        ["setup", "write-fixed-bin", ..] => write_fixed_bin_usage(stderr),
        ["setup", "write-fixed-native", setup_info_bin, columns_bin, out_const] => {
            write_fixed_columns_native(setup_info_bin, columns_bin, out_const, stdout, stderr)
        }
        ["setup", "write-fixed-native", ..] => write_fixed_native_usage(stderr),
        ["setup", "write-base-native", "--backend", backend, setup_info_bin, columns_bin, out_const, out_consttree] =>
        {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup native base write", stderr)
            else {
                return 1;
            };
            write_base_native(
                setup_info_bin,
                columns_bin,
                out_const,
                out_consttree,
                backend,
                stdout,
                stderr,
            )
        }
        ["setup", "write-base-native", setup_info_bin, columns_bin, out_const, out_consttree] => {
            write_base_native(
                setup_info_bin,
                columns_bin,
                out_const,
                out_consttree,
                FixedExtensionBackend::Cpu,
                stdout,
                stderr,
            )
        }
        ["setup", "write-base-native", ..] => write_base_native_usage(stderr),
        ["setup", "write-base-directory", "--derive-verkey", "--backend", backend, setup_dir] => {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup native base directory write", stderr)
            else {
                return 1;
            };
            write_base_directory(setup_dir, backend, true, stdout, stderr)
        }
        ["setup", "write-base-directory", "--backend", backend, "--derive-verkey", setup_dir] => {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup native base directory write", stderr)
            else {
                return 1;
            };
            write_base_directory(setup_dir, backend, true, stdout, stderr)
        }
        ["setup", "write-base-directory", "--derive-verkey", setup_dir] => {
            write_base_directory(setup_dir, FixedExtensionBackend::Cpu, true, stdout, stderr)
        }
        ["setup", "write-base-directory", "--backend", backend, setup_dir] => {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup native base directory write", stderr)
            else {
                return 1;
            };
            write_base_directory(setup_dir, backend, false, stdout, stderr)
        }
        ["setup", "write-base-directory", setup_dir] => {
            write_base_directory(setup_dir, FixedExtensionBackend::Cpu, false, stdout, stderr)
        }
        ["setup", "write-base-directory", ..] => write_base_directory_usage(stderr),
        ["setup", "write-pcs-directory", setup_dir] => {
            write_pcs_directory(setup_dir, stdout, stderr)
        }
        ["setup", "write-pcs-directory", ..] => write_pcs_directory_usage(stderr),
        ["setup", "write-verkey-native", setup_info_bin, consttree, out_verkey_json, out_verkey_bin] => {
            write_verification_key_native(
                setup_info_bin,
                consttree,
                out_verkey_json,
                out_verkey_bin,
                stdout,
                stderr,
            )
        }
        ["setup", "write-verkey-native", ..] => write_verkey_native_usage(stderr),
        ["setup", "write-const-tree", setup_info_bin, tree_bin, root_bin, out_consttree] => {
            write_constant_tree(
                setup_info_bin,
                tree_bin,
                root_bin,
                out_consttree,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-tree", ..] => write_const_tree_usage(stderr),
        ["setup", "write-const-leaves", "--backend", backend, setup_info_bin, columns_bin, out_leaves] =>
        {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup constant-tree leaf write", stderr)
            else {
                return 1;
            };
            write_constant_tree_leaves_command(
                setup_info_bin,
                columns_bin,
                out_leaves,
                backend,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-leaves", setup_info_bin, columns_bin, out_leaves] => {
            write_constant_tree_leaves_command(
                setup_info_bin,
                columns_bin,
                out_leaves,
                FixedExtensionBackend::Cpu,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-leaves", ..] => write_const_leaves_usage(stderr),
        ["setup", "write-const-native", "--backend", backend, setup_info_bin, columns_bin, root_bin, out_consttree] =>
        {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup native constant-tree write", stderr)
            else {
                return 1;
            };
            write_constant_tree_native(
                setup_info_bin,
                columns_bin,
                Some(root_bin),
                out_consttree,
                backend,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-native", "--backend", backend, setup_info_bin, columns_bin, out_consttree] =>
        {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup native constant-tree write", stderr)
            else {
                return 1;
            };
            write_constant_tree_native(
                setup_info_bin,
                columns_bin,
                None,
                out_consttree,
                backend,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-native", setup_info_bin, columns_bin, root_bin, out_consttree] => {
            write_constant_tree_native(
                setup_info_bin,
                columns_bin,
                Some(root_bin),
                out_consttree,
                FixedExtensionBackend::Cpu,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-native", setup_info_bin, columns_bin, out_consttree] => {
            write_constant_tree_native(
                setup_info_bin,
                columns_bin,
                None,
                out_consttree,
                FixedExtensionBackend::Cpu,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-native", ..] => write_const_native_usage(stderr),
        _ => write_validate_usage(stderr),
    }
}

fn prove_schedule(setup_dir: &str, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let catalog = match read_key_directory_catalog(setup_dir) {
        Ok(catalog) => catalog,
        Err(error) => {
            let _ = writeln!(stderr, "prove schedule failed: {error}");
            return 1;
        }
    };
    let schedule = match derive_prove_schedule(&catalog) {
        Ok(schedule) => schedule,
        Err(error) => {
            let _ = writeln!(stderr, "prove schedule failed: {error}");
            return 1;
        }
    };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", schedule.unit_count);
    let _ = writeln!(stdout, "fixed_bytes={}", schedule.total_fixed_bytes);
    let _ = writeln!(stdout, "queries={}", schedule.total_query_count);
    let _ = writeln!(
        stdout,
        "max_extended_domain_bits={}",
        schedule.max_extended_domain_bits
    );
    let _ = writeln!(stdout, "setup_hash={}", format_hash(&schedule.setup_hash));
    0
}

fn fingerprint_setup_directory(
    setup_dir: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match read_key_directory_catalog(setup_dir) {
        Ok(catalog) => match key_directory_catalog_digest_hex(&catalog) {
            Ok(fingerprint) => {
                let _ = writeln!(stdout, "status=ok");
                let _ = writeln!(stdout, "units={}", catalog.units.len());
                let _ = writeln!(stdout, "fingerprint={fingerprint}");
                0
            }
            Err(error) => {
                let _ = writeln!(stderr, "setup fingerprint failed: {error}");
                1
            }
        },
        Err(error) => {
            let _ = writeln!(stderr, "setup fingerprint failed: {error}");
            1
        }
    }
}

fn verify_preflight(
    proof_bin: &str,
    public_values_json: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let proof = match read_proof_artifact_file(proof_bin) {
        Ok(proof) => proof,
        Err(error) => {
            let _ = writeln!(stderr, "verify preflight failed: {error}");
            return 1;
        }
    };
    let public_values = match read_public_values_file(public_values_json) {
        Ok(public_values) => public_values,
        Err(error) => {
            let _ = writeln!(stderr, "verify preflight failed: {error}");
            return 1;
        }
    };
    if proof.setup_hash != public_values.setup_hash {
        let _ = writeln!(stderr, "verify preflight failed: setup hash mismatch");
        return 1;
    }
    let digest = match public_values_digest(&public_values) {
        Ok(digest) => digest,
        Err(error) => {
            let _ = writeln!(stderr, "verify preflight failed: {error}");
            return 1;
        }
    };
    if proof.public_values_hash != digest {
        let _ = writeln!(
            stderr,
            "verify preflight failed: public-values hash mismatch"
        );
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "segments={}", proof.segments.len());
    let _ = writeln!(stdout, "public_values={}", public_values.values.len());
    0
}

fn verify_setup_preflight(
    setup_dir: &str,
    proof_bin: &str,
    public_values_json: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let catalog = match read_key_directory_catalog(setup_dir) {
        Ok(catalog) => catalog,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };
    let setup_hash = match key_directory_catalog_digest(&catalog) {
        Ok(setup_hash) => setup_hash,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };
    let proof = match read_proof_artifact_file(proof_bin) {
        Ok(proof) => proof,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };
    let public_values = match read_public_values_file(public_values_json) {
        Ok(public_values) => public_values,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };
    if proof.setup_hash != public_values.setup_hash {
        let _ = writeln!(stderr, "verify setup-preflight failed: setup hash mismatch");
        return 1;
    }
    if proof.setup_hash != setup_hash {
        let _ = writeln!(
            stderr,
            "verify setup-preflight failed: setup catalog fingerprint mismatch"
        );
        return 1;
    }
    let digest = match public_values_digest(&public_values) {
        Ok(digest) => digest,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };
    if proof.public_values_hash != digest {
        let _ = writeln!(
            stderr,
            "verify setup-preflight failed: public-values hash mismatch"
        );
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", catalog.units.len());
    let _ = writeln!(stdout, "segments={}", proof.segments.len());
    let _ = writeln!(stdout, "public_values={}", public_values.values.len());
    0
}

fn validate_setup_directory(
    setup_dir: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match read_key_directory_catalog(setup_dir) {
        Ok(catalog) => {
            let fixed_bytes = catalog
                .units
                .iter()
                .map(|unit| unit.actual_fixed_bytes)
                .sum::<u64>();
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "units={}", catalog.units.len());
            let _ = writeln!(
                stdout,
                "global_constraints={}",
                catalog.global_constraints.entries.len()
            );
            let _ = writeln!(stdout, "fixed_bytes={fixed_bytes}");
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup validation failed: {error}");
            1
        }
    }
}

fn write_setup_info_bin(
    setup_info: &str,
    out_setup_info_bin: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_file(setup_info) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup-info binary write failed: {error}");
            return 1;
        }
    };
    let bytes = match encode_unit_setup_info(&setup) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "setup-info binary write failed: {error}");
            return 1;
        }
    };
    let output = Path::new(out_setup_info_bin);
    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        if let Err(error) = std::fs::create_dir_all(parent) {
            let _ = writeln!(stderr, "setup-info binary write failed: {error}");
            return 1;
        }
    }
    if let Err(error) = std::fs::write(output, &bytes) {
        let _ = writeln!(stderr, "setup-info binary write failed: {error}");
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "bytes_written={}", bytes.len());
    let _ = writeln!(stdout, "output={}", output.display());
    0
}

fn write_pcs_setup_plan(
    setup_info_bin: &str,
    out_pcs_plan: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS plan write failed: {error}");
            return 1;
        }
    };
    let plan = match derive_pcs_setup_plan(&setup) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS plan write failed: {error}");
            return 1;
        }
    };
    let bytes = match encode_pcs_setup_plan(&plan) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS plan write failed: {error}");
            return 1;
        }
    };
    let output = Path::new(out_pcs_plan);
    if let Some(parent) = output.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            let _ = writeln!(stderr, "setup PCS plan write failed: {error}");
            return 1;
        }
    }
    if let Err(error) = std::fs::write(output, &bytes) {
        let _ = writeln!(stderr, "setup PCS plan write failed: {error}");
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "bytes_written={}", bytes.len());
    let _ = writeln!(stdout, "output={}", output.display());
    0
}

fn write_fixed_columns(
    setup_info: &str,
    columns_json: &str,
    out_const: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_file(setup_info) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            return 1;
        }
    };
    let columns = match read_fixed_columns_json(columns_json) {
        Ok(columns) => columns,
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            return 1;
        }
    };

    publish_fixed_columns(out_const, &columns, &setup, stdout, stderr)
}

fn write_fixed_columns_bin(
    setup_info: &str,
    columns_bin: &str,
    out_const: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_file(setup_info) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            return 1;
        }
    };
    let columns = match read_fixed_columns_file(columns_bin) {
        Ok(columns) => columns,
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            return 1;
        }
    };

    publish_fixed_columns(out_const, &columns, &setup, stdout, stderr)
}

fn write_fixed_columns_native(
    setup_info_bin: &str,
    columns_bin: &str,
    out_const: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            return 1;
        }
    };
    let columns = match read_fixed_columns_file(columns_bin) {
        Ok(columns) => columns,
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            return 1;
        }
    };

    publish_fixed_columns(out_const, &columns, &setup, stdout, stderr)
}

fn write_base_native(
    setup_info_bin: &str,
    columns_bin: &str,
    out_const: &str,
    out_consttree: &str,
    backend: FixedExtensionBackend,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup native base write failed: {error}");
            return 1;
        }
    };
    let columns = match read_fixed_columns_file_for_setup(columns_bin, &setup, "raw", "unit") {
        Ok(columns) => columns,
        Err(error) => {
            let _ = writeln!(stderr, "setup native base write failed: {error}");
            return 1;
        }
    };
    let tree = match build_constant_tree_from_fixed_columns_with_backend(&columns, &setup, backend)
    {
        Ok(tree) => tree,
        Err(error) => {
            let _ = writeln!(stderr, "setup native base write failed: {error}");
            return 1;
        }
    };
    let fixed_report = match write_base_fixed_columns(out_const, &columns, &setup) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "setup native base write failed: {error}");
            return 1;
        }
    };
    let tree_report = match write_base_constant_tree(out_consttree, &tree, &setup, None) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "setup native base write failed: {error}");
            return 1;
        }
    };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "fixed_bytes={}", fixed_report.bytes_written);
    let _ = writeln!(stdout, "tree_bytes={}", tree_report.bytes_written);
    let _ = writeln!(stdout, "root={}", format_root(&tree_report.root));
    let _ = writeln!(stdout, "fixed_output={}", fixed_report.path.display());
    let _ = writeln!(stdout, "tree_output={}", tree_report.path.display());
    0
}

fn write_base_directory(
    setup_dir: &str,
    backend: FixedExtensionBackend,
    derive_verkey: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let layout = match read_key_directory_layout(setup_dir) {
        Ok(layout) => layout,
        Err(error) => {
            let _ = writeln!(stderr, "setup native base directory write failed: {error}");
            return 1;
        }
    };
    if let Err(error) = validate_base_directory_inputs(&layout, derive_verkey) {
        let _ = writeln!(stderr, "setup native base directory write failed: {error}");
        return 1;
    }

    let mut fixed_bytes = 0_u64;
    let mut tree_bytes = 0_u64;
    let mut verkey_bytes = 0_u64;
    for unit in &layout.units {
        let setup_path = match unit.setup_info() {
            Some(path) => path,
            None => {
                let _ = writeln!(
                    stderr,
                    "setup native base directory write failed: missing unit setup metadata path"
                );
                return 1;
            }
        };
        let setup = match read_unit_setup_info_file(&setup_path) {
            Ok(setup) => setup,
            Err(error) => {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        };
        let group_name = unit.group_name.as_deref().unwrap_or("raw");
        let unit_name = unit.unit_name.as_deref().unwrap_or("unit");
        let columns = match read_fixed_columns_file_for_setup(
            &unit.fixed_columns,
            &setup,
            group_name,
            unit_name,
        ) {
            Ok(columns) => columns,
            Err(error) => {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        };
        let expected_root = if derive_verkey {
            None
        } else {
            match read_verification_key_binary_file(unit.verification_key_binary()) {
                Ok(root) => Some(root),
                Err(error) => {
                    let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                    return 1;
                }
            }
        };
        let tree =
            match build_constant_tree_from_fixed_columns_with_backend(&columns, &setup, backend) {
                Ok(tree) => tree,
                Err(error) => {
                    let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                    return 1;
                }
            };
        let fixed_report = match write_base_fixed_columns(&unit.fixed_columns, &columns, &setup) {
            Ok(report) => report,
            Err(error) => {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        };
        let tree_report = match write_base_constant_tree(
            &unit.constant_tree,
            &tree,
            &setup,
            expected_root.as_ref(),
        ) {
            Ok(report) => report,
            Err(error) => {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        };
        if derive_verkey {
            let key_report = match write_verification_key_from_constant_tree(
                unit.verification_key_json(),
                unit.verification_key_binary(),
                &tree,
                &setup,
            ) {
                Ok(report) => report,
                Err(error) => {
                    let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                    return 1;
                }
            };
            verkey_bytes = verkey_bytes
                .saturating_add(key_report.json_bytes)
                .saturating_add(key_report.binary_bytes);
        }

        fixed_bytes = fixed_bytes.saturating_add(fixed_report.bytes_written);
        tree_bytes = tree_bytes.saturating_add(tree_report.bytes_written);
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", layout.units.len());
    let _ = writeln!(stdout, "fixed_bytes={fixed_bytes}");
    let _ = writeln!(stdout, "tree_bytes={tree_bytes}");
    if derive_verkey {
        let _ = writeln!(stdout, "verkey_bytes={verkey_bytes}");
    }
    0
}

fn write_pcs_directory(setup_dir: &str, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let layout = match read_key_directory_layout(setup_dir) {
        Ok(layout) => layout,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS directory write failed: {error}");
            return 1;
        }
    };

    let mut bytes_written = 0_u64;
    for unit in &layout.units {
        let setup_path = match unit.setup_info() {
            Some(path) => path,
            None => {
                let _ = writeln!(
                    stderr,
                    "setup PCS directory write failed: missing unit setup metadata path"
                );
                return 1;
            }
        };
        let output = match unit.pcs_setup_plan() {
            Some(path) => path,
            None => {
                let _ = writeln!(
                    stderr,
                    "setup PCS directory write failed: missing unit PCS plan output path"
                );
                return 1;
            }
        };
        let setup = match read_unit_setup_info_file(&setup_path) {
            Ok(setup) => setup,
            Err(error) => {
                let _ = writeln!(stderr, "setup PCS directory write failed: {error}");
                return 1;
            }
        };
        let plan = match derive_pcs_setup_plan(&setup) {
            Ok(plan) => plan,
            Err(error) => {
                let _ = writeln!(stderr, "setup PCS directory write failed: {error}");
                return 1;
            }
        };
        let bytes = match encode_pcs_setup_plan(&plan) {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = writeln!(stderr, "setup PCS directory write failed: {error}");
                return 1;
            }
        };
        if let Some(parent) = output.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                let _ = writeln!(stderr, "setup PCS directory write failed: {error}");
                return 1;
            }
        }
        if let Err(error) = std::fs::write(&output, &bytes) {
            let _ = writeln!(stderr, "setup PCS directory write failed: {error}");
            return 1;
        }
        bytes_written = bytes_written.saturating_add(bytes.len() as u64);
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", layout.units.len());
    let _ = writeln!(stdout, "bytes_written={bytes_written}");
    0
}

fn validate_base_directory_inputs(
    layout: &lzvm_artifacts::key_directory::KeyDirectoryLayout,
    derive_verkey: bool,
) -> Result<(), String> {
    if !derive_verkey {
        return validate_key_directory_layout(layout).map_err(|error| error.to_string());
    }

    let mut seen = BTreeSet::new();
    for required in layout.required_paths() {
        if matches!(
            required.role,
            "unit verification-key metadata" | "unit verification-key binary"
        ) {
            continue;
        }
        if !seen.insert(required.path.clone()) {
            continue;
        }
        if !required.path.is_file() {
            return Err(format!(
                "missing key-directory {}: {}",
                required.role,
                required.path.display()
            ));
        }
    }
    Ok(())
}

fn write_verification_key_native(
    setup_info_bin: &str,
    consttree: &str,
    out_verkey_json: &str,
    out_verkey_bin: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup verification-key write failed: {error}");
            return 1;
        }
    };
    let tree = match std::fs::read(consttree) {
        Ok(tree) => tree,
        Err(error) => {
            let _ = writeln!(stderr, "setup verification-key write failed: {error}");
            return 1;
        }
    };

    match write_verification_key_from_constant_tree(out_verkey_json, out_verkey_bin, &tree, &setup)
    {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "json_bytes={}", report.json_bytes);
            let _ = writeln!(stdout, "binary_bytes={}", report.binary_bytes);
            let _ = writeln!(stdout, "root={}", format_root(&report.root));
            let _ = writeln!(stdout, "json_output={}", report.json_path.display());
            let _ = writeln!(stdout, "binary_output={}", report.binary_path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup verification-key write failed: {error}");
            1
        }
    }
}

fn write_constant_tree(
    setup_info_bin: &str,
    tree_bin: &str,
    root_bin: &str,
    out_consttree: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree write failed: {error}");
            return 1;
        }
    };
    let tree = match std::fs::read(tree_bin) {
        Ok(tree) => tree,
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree write failed: {error}");
            return 1;
        }
    };
    let root = match read_verification_key_binary_file(root_bin) {
        Ok(root) => root,
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree write failed: {error}");
            return 1;
        }
    };

    match write_base_constant_tree(out_consttree, &tree, &setup, Some(&root)) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            let _ = writeln!(stdout, "root={}", format_root(&report.root));
            let _ = writeln!(stdout, "output={}", report.path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree write failed: {error}");
            1
        }
    }
}

fn write_constant_tree_leaves_command(
    setup_info_bin: &str,
    columns_bin: &str,
    out_leaves: &str,
    backend: FixedExtensionBackend,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree leaf write failed: {error}");
            return 1;
        }
    };
    let columns = match read_fixed_columns_file(columns_bin) {
        Ok(columns) => columns,
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree leaf write failed: {error}");
            return 1;
        }
    };

    match write_constant_tree_leaves_with_backend(out_leaves, &columns, &setup, backend) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            let _ = writeln!(stdout, "rows={}", report.row_count);
            let _ = writeln!(stdout, "columns={}", report.column_count);
            let _ = writeln!(stdout, "output={}", report.path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree leaf write failed: {error}");
            1
        }
    }
}

fn write_constant_tree_native(
    setup_info_bin: &str,
    columns_bin: &str,
    root_bin: Option<&str>,
    out_consttree: &str,
    backend: FixedExtensionBackend,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup native constant-tree write failed: {error}");
            return 1;
        }
    };
    let columns = match read_fixed_columns_file_for_setup(columns_bin, &setup, "raw", "unit") {
        Ok(columns) => columns,
        Err(error) => {
            let _ = writeln!(stderr, "setup native constant-tree write failed: {error}");
            return 1;
        }
    };
    let expected_root = match root_bin {
        Some(path) => match read_verification_key_binary_file(path) {
            Ok(root) => Some(root),
            Err(error) => {
                let _ = writeln!(stderr, "setup native constant-tree write failed: {error}");
                return 1;
            }
        },
        None => None,
    };

    let tree = match build_constant_tree_from_fixed_columns_with_backend(&columns, &setup, backend)
    {
        Ok(tree) => tree,
        Err(error) => {
            let _ = writeln!(stderr, "setup native constant-tree write failed: {error}");
            return 1;
        }
    };

    match write_base_constant_tree(out_consttree, &tree, &setup, expected_root.as_ref()) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            let _ = writeln!(stdout, "root={}", format_root(&report.root));
            let _ = writeln!(stdout, "output={}", report.path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup native constant-tree write failed: {error}");
            1
        }
    }
}

fn read_fixed_columns_json(path: impl AsRef<Path>) -> Result<FixedColumns, String> {
    let path = path.as_ref();
    let input = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    parse_fixed_columns_json(&input)
}

fn parse_fixed_columns_json(input: &str) -> Result<FixedColumns, String> {
    let value = serde_json::from_str::<Value>(input).map_err(|error| error.to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "fixed-column source must be a JSON object".to_owned())?;
    let group_name = read_string_field(object, "group_name")?;
    let unit_name = read_string_field(object, "unit_name")?;
    let row_count = read_u64_field(object, "row_count")?;
    let columns_value = object
        .get("columns")
        .and_then(Value::as_array)
        .ok_or_else(|| "fixed-column source must contain a columns array".to_owned())?;
    let mut columns = Vec::with_capacity(columns_value.len());
    for column_value in columns_value {
        let column = column_value
            .as_object()
            .ok_or_else(|| "fixed-column entry must be a JSON object".to_owned())?;
        columns.push(FixedColumn {
            name: read_string_field(column, "name")?,
            dimensions: read_u32_array(column, "dimensions")?,
            values: read_u64_array(column, "values")?,
        });
    }

    Ok(FixedColumns {
        group_name,
        unit_name,
        row_count,
        columns,
    })
}

fn publish_fixed_columns(
    out_const: &str,
    columns: &FixedColumns,
    setup: &lzvm_artifacts::setup_info::UnitSetupInfo,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match write_base_fixed_columns(out_const, columns, setup) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            let _ = writeln!(stdout, "output={}", report.path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            1
        }
    }
}

fn parse_fixed_extension_backend(
    value: &str,
    role: &str,
    stderr: &mut dyn Write,
) -> Option<FixedExtensionBackend> {
    match value {
        "cpu" => Some(FixedExtensionBackend::Cpu),
        "cuda" => Some(FixedExtensionBackend::Cuda),
        _ => {
            let _ = writeln!(stderr, "{role} failed: unsupported backend {value}");
            None
        }
    }
}

fn read_string_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("missing string field {field}"))
}

fn read_u64_field(object: &serde_json::Map<String, Value>, field: &str) -> Result<u64, String> {
    object
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing integer field {field}"))
}

fn read_u32_array(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Vec<u32>, String> {
    read_u64_array(object, field)?
        .into_iter()
        .map(|value| u32::try_from(value).map_err(|_| format!("{field} entry is too large")))
        .collect()
}

fn read_u64_array(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Vec<u64>, String> {
    object
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing integer array field {field}"))?
        .iter()
        .map(|value| {
            value
                .as_u64()
                .ok_or_else(|| format!("{field} entry must be an unsigned integer"))
        })
        .collect()
}

fn write_validate_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm setup validate <setup-dir>");
    2
}

fn write_verify_preflight_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify preflight <proof-bin> <public-values-json>"
    );
    2
}

fn write_verify_setup_preflight_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify setup-preflight <setup-dir> <proof-bin> <public-values-json>"
    );
    2
}

fn write_prove_schedule_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm prove schedule <setup-dir>");
    2
}

fn write_fingerprint_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm setup fingerprint <setup-dir>");
    2
}

fn write_info_bin_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-info-bin <setup-info-json> <out-setup-info-bin>"
    );
    2
}

fn write_pcs_plan_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-pcs-plan <setup-info-bin> <out-pcs-plan>"
    );
    2
}

fn write_fixed_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-fixed <setup-info-json> <columns-json> <out-const>"
    );
    2
}

fn write_fixed_bin_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-fixed-bin <setup-info-json> <columns-bin> <out-const>"
    );
    2
}

fn write_fixed_native_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-fixed-native <setup-info-bin> <columns-bin> <out-const>"
    );
    2
}

fn write_base_native_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-base-native [--backend cpu|cuda] <setup-info-bin> <columns-bin> <out-const> <out-consttree>"
    );
    2
}

fn write_base_directory_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-base-directory [--derive-verkey] [--backend cpu|cuda] <setup-dir>"
    );
    2
}

fn write_pcs_directory_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm setup write-pcs-directory <setup-dir>");
    2
}

fn write_verkey_native_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-verkey-native <setup-info-bin> <consttree> <out-verkey-json> <out-verkey-bin>"
    );
    2
}

fn write_const_tree_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-const-tree <setup-info-bin> <tree-bin> <root-bin> <out-consttree>"
    );
    2
}

fn write_const_leaves_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-const-leaves [--backend cpu|cuda] <setup-info-bin> <columns-bin> <out-leaves>"
    );
    2
}

fn write_const_native_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-const-native [--backend cpu|cuda] <setup-info-bin> <columns-bin> [root-bin] <out-consttree>"
    );
    2
}

fn format_hash(hash: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in hash {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn format_root(root: &VerificationKeyRoot) -> String {
    match root {
        VerificationKeyRoot::FieldElements(values) => values
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(","),
        VerificationKeyRoot::DecimalScalar(value) => value.clone(),
    }
}
