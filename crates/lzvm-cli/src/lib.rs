use std::io::Write;
use std::path::Path;

use lzvm_artifacts::program_image::{
    read_program_image_commitment_cache_file, ProgramImageGpuMode,
};
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_prover::derive_prove_schedule_from_directory;
use lzvm_setup::{
    summarize_setup_directory, FixedExtensionBackend, ProgramImageCommitmentCacheFileRequest,
    ProgramImageCommitmentCacheForSetupDirectoryRequest, SetupDirectorySummaryReport,
};

mod contribution_challenge;
mod eth_block_input;
mod eth_block_output;
mod eth_block_prove_input;
mod eth_block_public_values;
mod eth_block_summary;
mod eth_framed_input;
mod eth_public_input;
mod eth_rpc_block;
mod pil_archive;
mod pil_archive_summary;
mod pil_fixed_file_manifest;
mod pil_graph;
mod pil_summary;
mod program_image_cache;
mod prove_inputs;
mod prove_plan;
mod prove_witness;
mod setup_fixed_source;
mod setup_generate_key;
mod setup_source_companions;
mod trace_bundle;
mod verify_commands;

pub use prove_witness::{build_witness_proof_artifact, build_witness_proof_core_artifact};

pub fn run_cli(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        ["eth", "block-summary", rest @ ..] => eth_block_summary::run(rest, stdout, stderr),
        ["eth", "block-input-summary", rest @ ..] => {
            eth_block_input::run_summary(rest, stdout, stderr)
        }
        ["eth", "framed-input-summary", rest @ ..] => {
            eth_framed_input::run_summary(rest, stdout, stderr)
        }
        ["eth", "public-input-summary", rest @ ..] => {
            eth_public_input::run_summary(rest, stdout, stderr)
        }
        ["eth", "write-public-block-rlp", rest @ ..] => {
            eth_public_input::run_write_block_rlp(rest, stdout, stderr)
        }
        ["eth", "write-public-block-input", rest @ ..] => {
            eth_public_input::run_write_block_input(rest, stdout, stderr)
        }
        ["eth", "write-framed-input-chunk", rest @ ..] => {
            eth_framed_input::run_write_chunk(rest, stdout, stderr)
        }
        ["eth", "write-block-input", rest @ ..] => eth_block_input::run(rest, stdout, stderr),
        ["eth", "write-block-public-values", rest @ ..] => {
            eth_block_public_values::run(rest, stdout, stderr)
        }
        ["pil", "archive", rest @ ..] => pil_archive::run(rest, stdout, stderr),
        ["pil", "archive-summary", rest @ ..] => pil_archive_summary::run(rest, stdout, stderr),
        ["pil", "fixed-file-manifest", rest @ ..] => {
            pil_fixed_file_manifest::run(rest, stdout, stderr)
        }
        ["pil", "graph", rest @ ..] => pil_graph::run(rest, stdout, stderr),
        ["pil", "summary", rest @ ..] => pil_summary::run(rest, stdout, stderr),
        ["prove", "inputs", rest @ ..] => prove_inputs::run(rest, stdout, stderr),
        ["prove", "plan", rest @ ..] => prove_plan::run(rest, stdout, stderr),
        ["prove", "schedule", setup_dir] => prove_schedule(setup_dir, stdout, stderr),
        ["prove", "schedule", ..] => write_prove_schedule_usage(stderr),
        ["prove", "write-contribution-challenges", setup_dir, public_values_path, out_challenge_values_segment, proof_bins @ ..]
            if !proof_bins.is_empty() =>
        {
            contribution_challenge::run(
                setup_dir,
                public_values_path,
                out_challenge_values_segment,
                proof_bins,
                stdout,
                stderr,
            )
        }
        ["prove", "write-contribution-challenges", ..] => {
            contribution_challenge::write_usage(stderr)
        }
        ["prove", "write-trace-bundle", out_bundle, unit_args @ ..] => {
            trace_bundle::run(out_bundle, unit_args, stdout, stderr)
        }
        ["prove", "witness", rest @ ..] => prove_witness::run(rest, stdout, stderr),
        ["prove", rest @ ..] => prove_witness::run(rest, stdout, stderr),
        ["verify", "setup-preflight", setup_dir, proof_bin, public_values_path] => {
            verify_commands::verify_setup_preflight(
                setup_dir,
                proof_bin,
                public_values_path,
                stdout,
                stderr,
            )
        }
        ["verify", "setup-preflight", ..] => {
            verify_commands::write_verify_setup_preflight_usage(stderr)
        }
        ["verify", "proof", rest @ ..] => verify_commands::verify_proof(rest, stdout, stderr),
        ["verify", "contribution", setup_dir, proof_bin, public_values_path] => {
            verify_commands::verify_contribution(
                setup_dir,
                proof_bin,
                public_values_path,
                stdout,
                stderr,
            )
        }
        ["verify", "contribution", ..] => verify_commands::write_verify_contribution_usage(stderr),
        ["verify", "contribution-set", setup_dir, public_values_path, proof_bins @ ..]
            if !proof_bins.is_empty() =>
        {
            verify_commands::verify_contribution_set(
                setup_dir,
                public_values_path,
                proof_bins,
                stdout,
                stderr,
            )
        }
        ["verify", "contribution-set", ..] => {
            verify_commands::write_verify_contribution_set_usage(stderr)
        }
        ["verify", "contribution-challenge", setup_dir, public_values_path, challenge_values_segment, proof_bins @ ..]
            if !proof_bins.is_empty() =>
        {
            contribution_challenge::verify(
                setup_dir,
                public_values_path,
                challenge_values_segment,
                proof_bins,
                stdout,
                stderr,
            )
        }
        ["verify", "contribution-challenge", ..] => {
            contribution_challenge::write_verify_usage(stderr)
        }
        ["verify", "preflight", proof_bin, public_values_path] => {
            verify_commands::verify_preflight(proof_bin, public_values_path, stdout, stderr)
        }
        ["verify", "preflight", ..] => verify_commands::write_verify_preflight_usage(stderr),
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
        ["setup", "write-program-image-cache", "--backend", backend, "--setup-dir", setup_dir, program_bin, guest_image, root_bin, trace_rows, trace_columns, blowup_factor, arity, out_cache] =>
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
                    digest: ProgramImageCacheDigest::SetupDirectory(setup_dir),
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
        ["setup", "write-program-image-cache", "--setup-dir", setup_dir, "--backend", backend, program_bin, guest_image, root_bin, trace_rows, trace_columns, blowup_factor, arity, out_cache] =>
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
                    digest: ProgramImageCacheDigest::SetupDirectory(setup_dir),
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
        ["setup", "write-program-image-cache", "--setup-dir", setup_dir, program_bin, guest_image, root_bin, trace_rows, trace_columns, blowup_factor, arity, out_cache] => {
            write_program_image_cache(
                ProgramImageCacheCommand {
                    program_bin,
                    guest_image,
                    digest: ProgramImageCacheDigest::SetupDirectory(setup_dir),
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
                    digest: ProgramImageCacheDigest::File(constraint_digest_bin),
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
                    digest: ProgramImageCacheDigest::File(constraint_digest_bin),
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
        ["setup", "write-fixed-source", rest @ ..] => setup_fixed_source::run(rest, stdout, stderr),
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
        ["setup", "generate-key", rest @ ..] => setup_generate_key::run(rest, stdout, stderr),
        ["setup", "write-pcs-directory", setup_dir] => {
            write_pcs_directory(setup_dir, stdout, stderr)
        }
        ["setup", "write-pcs-directory", ..] => write_pcs_directory_usage(stderr),
        ["setup", "write-pcs-material-directory", setup_dir] => {
            write_pcs_material_directory(setup_dir, stdout, stderr)
        }
        ["setup", "write-pcs-material-directory", ..] => write_pcs_material_directory_usage(stderr),
        ["setup", "write-source-program-archive", rest @ ..] => {
            pil_archive::run(rest, stdout, stderr)
        }
        ["setup", "write-source-fixed-file-manifest", rest @ ..] => {
            pil_fixed_file_manifest::run(rest, stdout, stderr)
        }
        ["setup", "write-source-companions", rest @ ..] => {
            setup_source_companions::run(rest, stdout, stderr)
        }
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
            write_setup_source_companion_status(stdout, &report);
            let _ = writeln!(stdout, "fingerprint={}", report.fingerprint);
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup fingerprint failed: {error}");
            1
        }
    }
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
            write_setup_source_companion_status(stdout, &report);
            let _ = writeln!(stdout, "setup_hash={}", report.fingerprint);
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup validation failed: {error}");
            1
        }
    }
}

fn write_setup_source_companion_status(
    stdout: &mut dyn Write,
    report: &SetupDirectorySummaryReport,
) {
    let _ = writeln!(
        stdout,
        "source_fixed_file_manifest={}",
        if report.source_fixed_file_manifest_present {
            "present"
        } else {
            "absent"
        }
    );
    let _ = writeln!(
        stdout,
        "source_fixed_file_manifest_entries={}",
        report.source_fixed_file_manifest_entry_count
    );
    let _ = writeln!(
        stdout,
        "source_fixed_file_manifest_bytes={}",
        report.source_fixed_file_manifest_bytes
    );
    let _ = writeln!(
        stdout,
        "source_program_archive={}",
        if report.source_program_archive_present {
            "present"
        } else {
            "absent"
        }
    );
    let _ = writeln!(
        stdout,
        "source_program_archive_sources={}",
        report.source_program_archive_source_count
    );
    let _ = writeln!(
        stdout,
        "source_program_archive_edges={}",
        report.source_program_archive_edge_count
    );
    let _ = writeln!(
        stdout,
        "source_program_archive_bytes={}",
        report.source_program_archive_bytes
    );
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

enum ProgramImageCacheDigest<'a> {
    File(&'a str),
    SetupDirectory(&'a str),
}

struct ProgramImageCacheCommand<'a> {
    program_bin: &'a str,
    guest_image: &'a str,
    digest: ProgramImageCacheDigest<'a>,
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

    let result = match command.digest {
        ProgramImageCacheDigest::File(constraint_digest_bin) => {
            lzvm_setup::write_program_image_commitment_cache_file(
                ProgramImageCommitmentCacheFileRequest {
                    program_path: Path::new(command.program_bin),
                    guest_image_path: Path::new(command.guest_image),
                    constraint_digest_path: Path::new(constraint_digest_bin),
                    root_path: Path::new(command.root_bin),
                    trace_row_count: trace_rows,
                    trace_column_count: trace_columns,
                    blowup_factor,
                    merkle_tree_arity: arity,
                    gpu_mode: command.gpu_mode,
                    output_path: Path::new(command.out_cache),
                },
            )
        }
        ProgramImageCacheDigest::SetupDirectory(setup_dir) => {
            lzvm_setup::write_program_image_commitment_cache_file_for_setup_directory(
                ProgramImageCommitmentCacheForSetupDirectoryRequest {
                    setup_dir: Path::new(setup_dir),
                    program_path: Path::new(command.program_bin),
                    guest_image_path: Path::new(command.guest_image),
                    root_path: Path::new(command.root_bin),
                    trace_row_count: trace_rows,
                    trace_column_count: trace_columns,
                    blowup_factor,
                    merkle_tree_arity: arity,
                    gpu_mode: command.gpu_mode,
                    output_path: Path::new(command.out_cache),
                },
            )
        }
    };

    match result {
        Ok(report) => {
            let cache = match read_program_image_commitment_cache_file(&report.path) {
                Ok(cache) => cache,
                Err(error) => {
                    let _ = writeln!(stderr, "setup program-image cache write failed: {error}");
                    return 1;
                }
            };
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            let _ = writeln!(stdout, "output={}", report.path.display());
            program_image_cache::write_program_image_cache_summary(
                stdout,
                &lzvm_prover::ProveProgramImageCache {
                    path: report.path,
                    cache,
                },
            );
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
            let _ = writeln!(stdout, "manifest_bytes={}", report.manifest.bytes_written);
            let _ = writeln!(stdout, "setup_hash={}", report.manifest.fingerprint);
            let _ = writeln!(
                stdout,
                "setup_directory_manifest={}",
                report.manifest.path.display()
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
        "usage: lzvm setup write-program-image-cache [--backend cpu|cuda] <program-bin> <guest-image> <constraint-digest-bin> <root-bin> <trace-rows> <trace-columns> <blowup-factor> <arity> <out-cache>\n       lzvm setup write-program-image-cache [--backend cpu|cuda] --setup-dir <setup-dir> <program-bin> <guest-image> <root-bin> <trace-rows> <trace-columns> <blowup-factor> <arity> <out-cache>"
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
