use std::io::Write;
use std::path::{Path, PathBuf};

use lzvm_artifacts::challenge_values_segment::{
    encode_challenge_values_segment, ChallengeValuesSegment,
};
use lzvm_artifacts::eth_block_input::parse_eth_block_input;
use lzvm_artifacts::eth_block_input_segment::{
    encode_eth_block_input_segment, ETH_BLOCK_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_public_values::validate_eth_block_public_values;
use lzvm_artifacts::program_image::{
    read_program_image_commitment_cache_file, ProgramImageGpuMode,
};
use lzvm_artifacts::proof::read_proof_artifact_file;
use lzvm_artifacts::public_values::read_public_values_file;
use lzvm_artifacts::trace_bundle::{encode_trace_bundle, TraceBundle, TraceBundleUnit};
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_prover::contribution::{
    derive_global_challenge_from_contribution_proofs, derive_global_challenge_from_files,
};
use lzvm_prover::derive_prove_schedule_from_directory;
use lzvm_prover::proof_preflight::validate_proof_public_values_from_files;
use lzvm_prover::setup_preflight::validate_setup_preflight_from_files;
use lzvm_setup::{
    summarize_setup_directory, FixedExtensionBackend, ProgramImageCommitmentCacheFileRequest,
    SetupDirectorySummaryReport,
};

mod eth_block_input;
mod eth_block_prove_input;
mod eth_block_public_values;
mod eth_block_summary;
mod pil_archive;
mod pil_archive_summary;
mod pil_fixed_file_manifest;
mod pil_graph;
mod pil_summary;
mod program_image_cache;
mod prove_inputs;
mod prove_plan;
mod prove_witness;
mod setup_source_companions;

pub use prove_witness::{build_witness_proof_artifact, build_witness_proof_core_artifact};

pub fn run_cli(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        ["eth", "block-summary", rest @ ..] => eth_block_summary::run(rest, stdout, stderr),
        ["eth", "block-input-summary", rest @ ..] => {
            eth_block_input::run_summary(rest, stdout, stderr)
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
            write_contribution_challenge_segment(
                setup_dir,
                public_values_path,
                out_challenge_values_segment,
                proof_bins,
                stdout,
                stderr,
            )
        }
        ["prove", "write-contribution-challenges", ..] => {
            write_contribution_challenge_segment_usage(stderr)
        }
        ["prove", "write-trace-bundle", out_bundle, unit_args @ ..] => {
            write_trace_bundle(out_bundle, unit_args, stdout, stderr)
        }
        ["prove", "witness", rest @ ..] => prove_witness::run(rest, stdout, stderr),
        ["prove", rest @ ..] => prove_witness::run(rest, stdout, stderr),
        ["verify", "setup-preflight", setup_dir, proof_bin, public_values_path] => {
            verify_setup_preflight(setup_dir, proof_bin, public_values_path, stdout, stderr)
        }
        ["verify", "setup-preflight", ..] => write_verify_setup_preflight_usage(stderr),
        ["verify", "proof", rest @ ..] => verify_proof(rest, stdout, stderr),
        ["verify", "contribution", setup_dir, proof_bin, public_values_path] => {
            verify_contribution(setup_dir, proof_bin, public_values_path, stdout, stderr)
        }
        ["verify", "contribution", ..] => write_verify_contribution_usage(stderr),
        ["verify", "contribution-set", setup_dir, public_values_path, proof_bins @ ..]
            if !proof_bins.is_empty() =>
        {
            verify_contribution_set(setup_dir, public_values_path, proof_bins, stdout, stderr)
        }
        ["verify", "contribution-set", ..] => write_verify_contribution_set_usage(stderr),
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

fn write_trace_bundle(
    out_bundle: &str,
    unit_args: &[&str],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    if unit_args.is_empty() || !unit_args.len().is_multiple_of(2) {
        return write_trace_bundle_usage(stderr);
    }

    let mut units = Vec::with_capacity(unit_args.len() / 2);
    for pair in unit_args.chunks_exact(2) {
        let Some(unit_index) =
            parse_u32_arg(pair[0], "unit index", "prove trace bundle write", stderr)
        else {
            return 1;
        };
        let trace_bytes = match std::fs::read(pair[1]) {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = writeln!(
                    stderr,
                    "prove trace bundle write failed: read trace bytes failed: {}: {error}",
                    pair[1]
                );
                return 1;
            }
        };
        units.push(TraceBundleUnit {
            unit_index,
            trace_bytes,
        });
    }

    let bytes = match encode_trace_bundle(&TraceBundle { units }) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "prove trace bundle write failed: {error}");
            return 1;
        }
    };
    let output_path = Path::new(out_bundle);
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                let _ = writeln!(
                    stderr,
                    "prove trace bundle write failed: create output directory failed: {}: {error}",
                    parent.display()
                );
                return 1;
            }
        }
    }
    if let Err(error) = std::fs::write(output_path, &bytes) {
        let _ = writeln!(
            stderr,
            "prove trace bundle write failed: write output failed: {}: {error}",
            output_path.display()
        );
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", unit_args.len() / 2);
    let _ = writeln!(stdout, "bytes_written={}", bytes.len());
    let _ = writeln!(stdout, "output={}", output_path.display());
    0
}

fn write_contribution_challenge_segment(
    setup_dir: &str,
    public_values_path: &str,
    output_path: &str,
    proof_bins: &[&str],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let proof_paths = proof_bins.iter().map(PathBuf::from).collect::<Vec<_>>();
    let report = match derive_global_challenge_from_contribution_proofs(
        setup_dir,
        public_values_path,
        &proof_paths,
    ) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "prove contribution challenges write failed: {error}"
            );
            return 1;
        }
    };

    let challenge_values = vec![[
        report.challenge.c0.to_u64(),
        report.challenge.c1.to_u64(),
        report.challenge.c2.to_u64(),
    ]];
    let segment = match encode_challenge_values_segment(&ChallengeValuesSegment {
        values: challenge_values.clone(),
    }) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "prove contribution challenges write failed: {error}"
            );
            return 1;
        }
    };

    let output_path = Path::new(output_path);
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                let _ = writeln!(
                    stderr,
                    "prove contribution challenges write failed: create output directory failed: {}: {error}",
                    parent.display()
                );
                return 1;
            }
        }
    }
    if let Err(error) = std::fs::write(output_path, &segment) {
        let _ = writeln!(
            stderr,
            "prove contribution challenges write failed: write output failed: {}: {error}",
            output_path.display()
        );
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "proofs={}", report.proof_count);
    let _ = writeln!(stdout, "segments={}", report.segment_count);
    let _ = writeln!(stdout, "public_values={}", report.public_value_count);
    let _ = writeln!(
        stdout,
        "public_value_fields={}",
        report.public_value_field_count
    );
    let _ = writeln!(stdout, "proof_values={}", report.proof_value_count);
    let _ = writeln!(stdout, "contributions={}", report.contribution_count);
    let _ = writeln!(stdout, "challenge_values={}", challenge_values.len());
    let _ = writeln!(
        stdout,
        "contribution_challenge={},{},{}",
        report.challenge.c0.to_u64(),
        report.challenge.c1.to_u64(),
        report.challenge.c2.to_u64()
    );
    let _ = writeln!(stdout, "bytes_written={}", segment.len());
    let _ = writeln!(stdout, "output={}", output_path.display());
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
    let _ = writeln!(
        stdout,
        "public_value_fields={}",
        report.public_value_field_count
    );
    if report.program_image_cache_count > 0 {
        let _ = writeln!(
            stdout,
            "program_image_caches={}",
            report.program_image_cache_count
        );
    }
    if report.eth_block_input_count > 0 {
        let _ = writeln!(stdout, "eth_block_inputs={}", report.eth_block_input_count);
    }
    0
}

fn verify_setup_preflight(
    setup_dir: &str,
    proof_bin: &str,
    public_values_path: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    verify_setup_validation(
        "verify setup-preflight",
        setup_dir,
        proof_bin,
        public_values_path,
        None,
        stdout,
        stderr,
    )
}

struct ParsedVerifyProofArgs<'a> {
    setup_dir: &'a str,
    proof_bin: &'a str,
    public_values_path: &'a str,
    eth_block_input: Option<&'a str>,
}

fn parse_verify_proof_args<'a>(
    args: &'a [&'a str],
) -> Result<ParsedVerifyProofArgs<'a>, VerifyProofArgError> {
    let mut eth_block_input = None;
    let mut positionals = Vec::with_capacity(args.len());
    let mut index = 0;
    while index < args.len() {
        match args[index] {
            "--eth-block-input" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    VerifyProofArgError::Invalid("missing --eth-block-input value".to_owned())
                })?;
                if eth_block_input.replace(*value).is_some() {
                    return Err(VerifyProofArgError::Invalid(
                        "duplicate --eth-block-input option".to_owned(),
                    ));
                }
            }
            value => positionals.push(value),
        }
        index += 1;
    }
    if positionals.len() != 3 {
        return Err(VerifyProofArgError::Usage);
    }
    Ok(ParsedVerifyProofArgs {
        setup_dir: positionals[0],
        proof_bin: positionals[1],
        public_values_path: positionals[2],
        eth_block_input,
    })
}

enum VerifyProofArgError {
    Usage,
    Invalid(String),
}

fn verify_proof(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_verify_proof_args(args) {
        Ok(parsed) => parsed,
        Err(VerifyProofArgError::Usage) => return write_verify_proof_usage(stderr),
        Err(VerifyProofArgError::Invalid(message)) => {
            let _ = writeln!(stderr, "verify proof failed: {message}");
            return 1;
        }
    };
    verify_setup_validation(
        "verify proof",
        parsed.setup_dir,
        parsed.proof_bin,
        parsed.public_values_path,
        parsed.eth_block_input,
        stdout,
        stderr,
    )
}

fn verify_contribution(
    setup_dir: &str,
    proof_bin: &str,
    public_values_path: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let report = match derive_global_challenge_from_files(setup_dir, proof_bin, public_values_path)
    {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "verify contribution failed: {error}");
            return 1;
        }
    };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "segments={}", report.segment_count);
    let _ = writeln!(stdout, "public_values={}", report.public_value_count);
    let _ = writeln!(
        stdout,
        "public_value_fields={}",
        report.public_value_field_count
    );
    let _ = writeln!(stdout, "proof_values={}", report.proof_value_count);
    let _ = writeln!(stdout, "contributions={}", report.contribution_count);
    let _ = writeln!(
        stdout,
        "contribution_challenge={},{},{}",
        report.challenge.c0.to_u64(),
        report.challenge.c1.to_u64(),
        report.challenge.c2.to_u64()
    );
    0
}

fn verify_contribution_set(
    setup_dir: &str,
    public_values_path: &str,
    proof_bins: &[&str],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let proof_paths = proof_bins.iter().map(PathBuf::from).collect::<Vec<_>>();
    let report = match derive_global_challenge_from_contribution_proofs(
        setup_dir,
        public_values_path,
        &proof_paths,
    ) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "verify contribution-set failed: {error}");
            return 1;
        }
    };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "proofs={}", report.proof_count);
    let _ = writeln!(stdout, "segments={}", report.segment_count);
    let _ = writeln!(stdout, "public_values={}", report.public_value_count);
    let _ = writeln!(
        stdout,
        "public_value_fields={}",
        report.public_value_field_count
    );
    let _ = writeln!(stdout, "proof_values={}", report.proof_value_count);
    let _ = writeln!(stdout, "contributions={}", report.contribution_count);
    let _ = writeln!(
        stdout,
        "contribution_challenge={},{},{}",
        report.challenge.c0.to_u64(),
        report.challenge.c1.to_u64(),
        report.challenge.c2.to_u64()
    );
    0
}

fn verify_setup_validation(
    role: &str,
    setup_dir: &str,
    proof_bin: &str,
    public_values_path: &str,
    eth_block_input: Option<&str>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    if let Some(path) = eth_block_input {
        match verify_eth_block_input_binding(proof_bin, public_values_path, path) {
            Ok(()) => {}
            Err(message) => {
                let _ = writeln!(stderr, "{role} failed: {message}");
                return 1;
            }
        }
    }
    let public_report =
        match validate_setup_preflight_from_files(setup_dir, proof_bin, public_values_path) {
            Ok(report) => report,
            Err(error) => {
                let _ = writeln!(stderr, "{role} failed: {error}");
                return 1;
            }
        };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", public_report.unit_count);
    let _ = writeln!(stdout, "segments={}", public_report.segment_count);
    let _ = writeln!(stdout, "public_values={}", public_report.public_value_count);
    let _ = writeln!(
        stdout,
        "public_value_fields={}",
        public_report.public_value_field_count
    );
    if public_report.program_image_cache_count > 0 {
        let _ = writeln!(
            stdout,
            "program_image_caches={}",
            public_report.program_image_cache_count
        );
    }
    if public_report.eth_block_input_count > 0 {
        let _ = writeln!(
            stdout,
            "eth_block_inputs={}",
            public_report.eth_block_input_count
        );
    }
    if eth_block_input.is_some() {
        let _ = writeln!(stdout, "eth_block_input_match=ok");
    }
    0
}

fn verify_eth_block_input_binding(
    proof_bin: &str,
    public_values_path: &str,
    input_path: &str,
) -> Result<(), String> {
    let proof = read_proof_artifact_file(proof_bin)
        .map_err(|error| format!("read proof artifact failed: {proof_bin}: {error}"))?;
    let input_bytes = std::fs::read(input_path)
        .map_err(|error| format!("read ETH block input failed: {input_path}: {error}"))?;
    let input = parse_eth_block_input(&input_bytes)
        .map_err(|error| format!("ETH block input failed: {input_path}: {error}"))?;
    let expected = encode_eth_block_input_segment(&input)
        .map_err(|error| format!("encode ETH block input segment failed: {error}"))?;
    let segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
        .ok_or_else(|| "missing ETH block input proof segment".to_owned())?;
    if segment.data != expected {
        return Err("ETH block input proof segment mismatch".to_owned());
    }
    let public_values = read_public_values_file(public_values_path)
        .map_err(|error| format!("read public-values failed: {public_values_path}: {error}"))?;
    validate_eth_block_public_values(&input, &public_values).map_err(|error| error.to_string())?;
    Ok(())
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

fn write_verify_proof_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify proof [--eth-block-input <block-input>] <setup-dir> <proof-bin> <public-values>"
    );
    2
}

fn write_verify_contribution_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify contribution <setup-dir> <proof-bin> <public-values>"
    );
    2
}

fn write_verify_contribution_set_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify contribution-set <setup-dir> <public-values> <proof-bin> [proof-bin ...]"
    );
    2
}

fn write_prove_schedule_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm prove schedule <setup-dir>");
    2
}

fn write_trace_bundle_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm prove write-trace-bundle <out-bundle> <unit-index> <trace-bin>..."
    );
    2
}

fn write_contribution_challenge_segment_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm prove write-contribution-challenges <setup-dir> <public-values> <out-challenge-values-segment> <proof-bin> [proof-bin ...]"
    );
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
