use std::io::Write;
use std::path::Path;

use lzvm_artifacts::constant_tree::read_constant_tree_file;
use lzvm_artifacts::fixed::{
    read_fixed_columns_file, read_fixed_columns_file_for_setup, FixedColumns,
};
use lzvm_artifacts::key_directory::{key_directory_catalog_digest_hex, read_key_directory_catalog};
use lzvm_artifacts::pcs_material::{build_pcs_setup_material, encode_pcs_setup_material};
use lzvm_artifacts::pcs_plan::{
    derive_pcs_setup_plan, encode_pcs_setup_plan, read_pcs_setup_plan_file,
};
use lzvm_artifacts::proof::read_proof_artifact_file;
use lzvm_artifacts::public_values::read_public_values_file;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_artifacts::verification_key::{read_verification_key_binary_file, VerificationKeyRoot};
use lzvm_prover::derive_prove_schedule;
use lzvm_prover::proof_preflight::validate_proof_public_values;
use lzvm_prover::setup_preflight::validate_setup_preflight;
use lzvm_setup::{
    build_constant_tree_from_fixed_columns_with_backend, write_base_constant_tree,
    write_base_fixed_columns, write_constant_tree_leaves_with_backend,
    write_verification_key_from_constant_tree, FixedExtensionBackend,
};

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
        ["verify", "setup-preflight", setup_dir, proof_bin, public_values_path] => {
            verify_setup_preflight(setup_dir, proof_bin, public_values_path, stdout, stderr)
        }
        ["verify", "setup-preflight", ..] => write_verify_setup_preflight_usage(stderr),
        ["verify", "preflight", proof_bin, public_values_path] => {
            verify_preflight(proof_bin, public_values_path, stdout, stderr)
        }
        ["verify", "preflight", ..] => write_verify_preflight_usage(stderr),
        ["setup", "fingerprint", setup_dir] => {
            fingerprint_setup_directory(setup_dir, stdout, stderr)
        }
        ["setup", "fingerprint", ..] => write_fingerprint_usage(stderr),
        ["setup", "validate", setup_dir] => validate_setup_directory(setup_dir, stdout, stderr),
        ["setup", "validate", ..] => write_validate_usage(stderr),
        ["setup", "write-pcs-plan", setup_info_bin, out_pcs_plan] => {
            write_pcs_setup_plan(setup_info_bin, out_pcs_plan, stdout, stderr)
        }
        ["setup", "write-pcs-plan", ..] => write_pcs_plan_usage(stderr),
        ["setup", "write-pcs-material", setup_info_bin, pcs_plan, fixed_const, consttree, out_pcs_material] => {
            write_pcs_setup_material(
                setup_info_bin,
                pcs_plan,
                fixed_const,
                consttree,
                out_pcs_material,
                stdout,
                stderr,
            )
        }
        ["setup", "write-pcs-material", ..] => write_pcs_material_usage(stderr),
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
        ["setup", "write-pcs-material-directory", setup_dir] => {
            write_pcs_material_directory(setup_dir, stdout, stderr)
        }
        ["setup", "write-pcs-material-directory", ..] => write_pcs_material_directory_usage(stderr),
        ["setup", "write-verkey-native", setup_info_bin, consttree, out_verkey_bin] => {
            write_verification_key_native(setup_info_bin, consttree, out_verkey_bin, stdout, stderr)
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
    let _ = writeln!(
        stdout,
        "pcs_material_units={}",
        schedule.pcs_material_unit_count
    );
    let _ = writeln!(
        stdout,
        "pcs_material_bytes={}",
        schedule.total_pcs_material_bytes
    );
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
    public_values_path: &str,
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
    let public_values = match read_public_values_file(public_values_path) {
        Ok(public_values) => public_values,
        Err(error) => {
            let _ = writeln!(stderr, "verify preflight failed: {error}");
            return 1;
        }
    };
    let report = match validate_proof_public_values(&proof, &public_values) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "verify preflight failed: {error}");
            return 1;
        }
    };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "segments={}", report.segment_count);
    let _ = writeln!(stdout, "public_values={}", report.public_value_count);
    0
}

fn verify_setup_preflight(
    setup_dir: &str,
    proof_bin: &str,
    public_values_path: &str,
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
    let proof = match read_proof_artifact_file(proof_bin) {
        Ok(proof) => proof,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };
    let public_values = match read_public_values_file(public_values_path) {
        Ok(public_values) => public_values,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };
    let public_report = match validate_setup_preflight(&catalog, &proof, &public_values) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", public_report.unit_count);
    let _ = writeln!(stdout, "segments={}", public_report.segment_count);
    let _ = writeln!(stdout, "public_values={}", public_report.public_value_count);
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

fn write_pcs_setup_material(
    setup_info_bin: &str,
    pcs_plan: &str,
    fixed_const: &str,
    consttree: &str,
    out_pcs_material: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    let plan = match read_pcs_setup_plan_file(pcs_plan) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    let expected_plan = match derive_pcs_setup_plan(&setup) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    if plan != expected_plan {
        let _ = writeln!(
            stderr,
            "setup PCS material write failed: PCS setup plan does not match setup metadata"
        );
        return 1;
    }
    let fixed_bytes = match std::fs::read(fixed_const) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    let tree = match read_constant_tree_file(consttree, &setup) {
        Ok(tree) => tree,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    let material = match build_pcs_setup_material(&plan, &fixed_bytes, &tree) {
        Ok(material) => material,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    let bytes = match encode_pcs_setup_material(&material) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    let output = Path::new(out_pcs_material);
    if let Some(parent) = output.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    }
    if let Err(error) = std::fs::write(output, &bytes) {
        let _ = writeln!(stderr, "setup PCS material write failed: {error}");
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "bytes_written={}", bytes.len());
    let _ = writeln!(stdout, "output={}", output.display());
    0
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
    match lzvm_setup::write_base_directory(setup_dir, backend, derive_verkey) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "units={}", report.unit_count);
            let _ = writeln!(stdout, "fixed_bytes={}", report.fixed_bytes);
            let _ = writeln!(stdout, "tree_bytes={}", report.tree_bytes);
            if let Some(verkey_bytes) = report.verkey_bytes {
                let _ = writeln!(stdout, "verkey_bytes={verkey_bytes}");
            }
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup native base directory write failed: {error}");
            1
        }
    }
}

fn write_pcs_directory(setup_dir: &str, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match lzvm_setup::write_pcs_directory(setup_dir) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "units={}", report.unit_count);
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS directory write failed: {error}");
            1
        }
    }
}

fn write_pcs_material_directory(
    setup_dir: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match lzvm_setup::write_pcs_material_directory(setup_dir) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "units={}", report.unit_count);
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material directory write failed: {error}");
            1
        }
    }
}

fn write_verification_key_native(
    setup_info_bin: &str,
    consttree: &str,
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

    match write_verification_key_from_constant_tree(out_verkey_bin, &tree, &setup) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "binary_bytes={}", report.binary_bytes);
            let _ = writeln!(stdout, "root={}", format_root(&report.root));
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

fn write_validate_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm setup validate <setup-dir>");
    2
}

fn write_verify_preflight_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify preflight <proof-bin> <public-values>"
    );
    2
}

fn write_verify_setup_preflight_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify setup-preflight <setup-dir> <proof-bin> <public-values>"
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

fn write_pcs_plan_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-pcs-plan <setup-info-bin> <out-pcs-plan>"
    );
    2
}

fn write_pcs_material_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-pcs-material <setup-info-bin> <pcs-plan> <fixed-const> <consttree> <out-pcs-material>"
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

fn write_pcs_material_directory_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-pcs-material-directory <setup-dir>"
    );
    2
}

fn write_verkey_native_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-verkey-native <setup-info-bin> <consttree> <out-verkey-bin>"
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
    }
}
