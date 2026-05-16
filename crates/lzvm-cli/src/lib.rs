use std::io::Write;
use std::path::Path;

use lzvm_artifacts::program_image::ProgramImageGpuMode;
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_prover::derive_prove_schedule_from_directory;
use lzvm_prover::proof_preflight::validate_proof_public_values_from_files;
use lzvm_prover::setup_preflight::validate_setup_preflight_from_files;
use lzvm_setup::{
    summarize_setup_directory, FixedExtensionBackend, ProgramImageCommitmentCacheFileRequest,
};

mod program_image_cache;
mod prove_inputs;
mod prove_plan;
mod prove_witness;

pub use prove_witness::{build_witness_proof_artifact, build_witness_proof_core_artifact};

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
        ["setup", "write-program-image-cache", "--backend", backend, program_bin, guest_image, constraint_digest_bin, root_bin, trace_rows, trace_columns, blowup_factor, arity, out_cache] =>
        {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup program-image cache write", stderr)
            else {
                return 1;
            };
            write_program_image_cache(
                ProgramImageCacheCommand {
                    program_bin,
                    guest_image,
                    constraint_digest_bin,
                    root_bin,
                    trace_rows,
                    trace_columns,
                    blowup_factor,
                    arity,
                    gpu_mode: program_image_gpu_mode_from_backend(backend),
                    out_cache,
                },
                stdout,
                stderr,
            )
        }
        ["setup", "write-program-image-cache", program_bin, guest_image, constraint_digest_bin, root_bin, trace_rows, trace_columns, blowup_factor, arity, out_cache] => {
            write_program_image_cache(
                ProgramImageCacheCommand {
                    program_bin,
                    guest_image,
                    constraint_digest_bin,
                    root_bin,
                    trace_rows,
                    trace_columns,
                    blowup_factor,
                    arity,
                    gpu_mode: ProgramImageGpuMode::Cpu,
                    out_cache,
                },
                stdout,
                stderr,
            )
        }
        ["setup", "write-program-image-cache", ..] => write_program_image_cache_usage(stderr),
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
        ["setup", "write-key-directory", "--backend", backend, setup_dir] => {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup key directory write", stderr)
            else {
                return 1;
            };
            write_key_directory(setup_dir, backend, stdout, stderr)
        }
        ["setup", "write-key-directory", setup_dir] => {
            write_key_directory(setup_dir, FixedExtensionBackend::Cpu, stdout, stderr)
        }
        ["setup", "write-key-directory", ..] => write_key_directory_usage(stderr),
        ["setup", "generate-key", "--backend", backend, setup_dir] => {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup key generation", stderr)
            else {
                return 1;
            };
            write_key_directory(setup_dir, backend, stdout, stderr)
        }
        ["setup", "generate-key", setup_dir] => {
            write_key_directory(setup_dir, FixedExtensionBackend::Cpu, stdout, stderr)
        }
        ["setup", "generate-key", ..] => write_generate_key_usage(stderr),
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
    let schedule = match derive_prove_schedule_from_directory(setup_dir) {
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
    match summarize_setup_directory(setup_dir) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "units={}", report.unit_count);
            let _ = writeln!(stdout, "fingerprint={}", report.fingerprint);
            0
        }
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
    let report = match validate_proof_public_values_from_files(proof_bin, public_values_path) {
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
    let public_report =
        match validate_setup_preflight_from_files(setup_dir, proof_bin, public_values_path) {
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
    match summarize_setup_directory(setup_dir) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "units={}", report.unit_count);
            let _ = writeln!(
                stdout,
                "global_constraints={}",
                report.global_constraint_count
            );
            let _ = writeln!(stdout, "fixed_bytes={}", report.fixed_bytes);
            let _ = writeln!(
                stdout,
                "pcs_material_units={}",
                report.pcs_material_unit_count
            );
            let _ = writeln!(stdout, "pcs_material_bytes={}", report.pcs_material_bytes);
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
    match lzvm_setup::write_pcs_setup_plan_file(setup_info_bin, out_pcs_plan) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            let _ = writeln!(stdout, "output={}", report.path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS plan write failed: {error}");
            1
        }
    }
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
    match lzvm_setup::write_pcs_setup_material_file(
        setup_info_bin,
        pcs_plan,
        fixed_const,
        consttree,
        out_pcs_material,
    ) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            let _ = writeln!(stdout, "output={}", report.path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            1
        }
    }
}

struct ProgramImageCacheCommand<'a> {
    program_bin: &'a str,
    guest_image: &'a str,
    constraint_digest_bin: &'a str,
    root_bin: &'a str,
    trace_rows: &'a str,
    trace_columns: &'a str,
    blowup_factor: &'a str,
    arity: &'a str,
    gpu_mode: ProgramImageGpuMode,
    out_cache: &'a str,
}

fn write_program_image_cache(
    command: ProgramImageCacheCommand<'_>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let Some(trace_rows) = parse_u64_arg(
        command.trace_rows,
        "trace rows",
        "setup program-image cache write",
        stderr,
    ) else {
        return 1;
    };
    let Some(trace_columns) = parse_u32_arg(
        command.trace_columns,
        "trace columns",
        "setup program-image cache write",
        stderr,
    ) else {
        return 1;
    };
    let Some(blowup_factor) = parse_u32_arg(
        command.blowup_factor,
        "blowup factor",
        "setup program-image cache write",
        stderr,
    ) else {
        return 1;
    };
    let Some(arity) = parse_u32_arg(
        command.arity,
        "arity",
        "setup program-image cache write",
        stderr,
    ) else {
        return 1;
    };

    match lzvm_setup::write_program_image_commitment_cache_file(
        ProgramImageCommitmentCacheFileRequest {
            program_path: Path::new(command.program_bin),
            guest_image_path: Path::new(command.guest_image),
            constraint_digest_path: Path::new(command.constraint_digest_bin),
            root_path: Path::new(command.root_bin),
            trace_row_count: trace_rows,
            trace_column_count: trace_columns,
            blowup_factor,
            merkle_tree_arity: arity,
            gpu_mode: command.gpu_mode,
            output_path: Path::new(command.out_cache),
        },
    ) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            let _ = writeln!(stdout, "output={}", report.path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup program-image cache write failed: {error}");
            1
        }
    }
}

fn write_fixed_columns_native(
    setup_info_bin: &str,
    columns_bin: &str,
    out_const: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match lzvm_setup::write_fixed_columns_native_file(setup_info_bin, columns_bin, out_const) {
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

fn write_base_native(
    setup_info_bin: &str,
    columns_bin: &str,
    out_const: &str,
    out_consttree: &str,
    backend: FixedExtensionBackend,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match lzvm_setup::write_base_native_files(
        setup_info_bin,
        columns_bin,
        out_const,
        out_consttree,
        backend,
    ) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "fixed_bytes={}", report.fixed.bytes_written);
            let _ = writeln!(stdout, "tree_bytes={}", report.tree.bytes_written);
            let _ = writeln!(stdout, "root={}", format_root(&report.tree.root));
            let _ = writeln!(stdout, "fixed_output={}", report.fixed.path.display());
            let _ = writeln!(stdout, "tree_output={}", report.tree.path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup native base write failed: {error}");
            1
        }
    }
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

fn write_key_directory(
    setup_dir: &str,
    backend: FixedExtensionBackend,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match lzvm_setup::write_key_directory(setup_dir, backend) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "units={}", report.base.unit_count);
            let _ = writeln!(stdout, "fixed_bytes={}", report.base.fixed_bytes);
            let _ = writeln!(stdout, "tree_bytes={}", report.base.tree_bytes);
            if let Some(verkey_bytes) = report.base.verkey_bytes {
                let _ = writeln!(stdout, "verkey_bytes={verkey_bytes}");
            }
            let _ = writeln!(stdout, "pcs_plan_bytes={}", report.pcs_plan.bytes_written);
            let _ = writeln!(
                stdout,
                "pcs_material_bytes={}",
                report.pcs_material.bytes_written
            );
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup key directory write failed: {error}");
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
    match lzvm_setup::write_verification_key_native_file(setup_info_bin, consttree, out_verkey_bin)
    {
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
    match lzvm_setup::write_constant_tree_file(setup_info_bin, tree_bin, root_bin, out_consttree) {
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
    match lzvm_setup::write_constant_tree_leaves_file(
        setup_info_bin,
        columns_bin,
        out_leaves,
        backend,
    ) {
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
    match lzvm_setup::write_constant_tree_native_file(
        setup_info_bin,
        columns_bin,
        root_bin,
        out_consttree,
        backend,
    ) {
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

fn program_image_gpu_mode_from_backend(backend: FixedExtensionBackend) -> ProgramImageGpuMode {
    match backend {
        FixedExtensionBackend::Cpu => ProgramImageGpuMode::Cpu,
        FixedExtensionBackend::Cuda => ProgramImageGpuMode::Cuda,
    }
}

fn parse_u64_arg(value: &str, name: &str, role: &str, stderr: &mut dyn Write) -> Option<u64> {
    value.parse().map_or_else(
        |_| {
            let _ = writeln!(stderr, "{role} failed: invalid {name}: {value}");
            None
        },
        Some,
    )
}

fn parse_u32_arg(value: &str, name: &str, role: &str, stderr: &mut dyn Write) -> Option<u32> {
    value.parse().map_or_else(
        |_| {
            let _ = writeln!(stderr, "{role} failed: invalid {name}: {value}");
            None
        },
        Some,
    )
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

fn write_program_image_cache_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-program-image-cache [--backend cpu|cuda] <program-bin> <guest-image> <constraint-digest-bin> <root-bin> <trace-rows> <trace-columns> <blowup-factor> <arity> <out-cache>"
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

fn write_key_directory_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-key-directory [--backend cpu|cuda] <setup-dir>"
    );
    2
}

fn write_generate_key_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup generate-key [--backend cpu|cuda] <setup-dir>"
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
