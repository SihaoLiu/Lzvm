use std::io::Write;
use std::path::PathBuf;

use lzvm_artifacts::eth_block_input::{
    eth_block_input_bytes_digest, eth_block_input_extra_field_counts,
    eth_block_input_receipt_kind_counts, eth_block_input_transaction_kind_counts,
    eth_block_input_withdrawal_count, parse_eth_block_input,
};
use lzvm_artifacts::eth_block_input_segment::{
    encode_eth_block_input_segment, ETH_BLOCK_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_public_values::validate_eth_block_public_values;
use lzvm_artifacts::program_image::read_program_image_commitment_cache_file;
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};
use lzvm_artifacts::proof::read_proof_artifact_file;
use lzvm_artifacts::public_values::read_public_values_file;
use lzvm_prover::contribution::{
    derive_global_challenge_from_contribution_proofs, derive_global_challenge_from_files,
    ContributionChallengeReport,
};
use lzvm_prover::proof_preflight::validate_proof_public_values_from_files;
use lzvm_prover::setup_preflight::validate_setup_preflight_from_files;

use crate::{eth_block_output, program_image_cache, prove_plan};

pub(super) fn verify_preflight(
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
        "public_values_hash={}",
        prove_plan::format_hash(&report.public_values_hash)
    );
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
        for (cache, hash) in report
            .program_image_caches
            .iter()
            .zip(&report.program_image_cache_hashes)
        {
            program_image_cache::write_program_image_cache_fields_with_segment_hash(
                stdout, cache, hash,
            );
        }
    }
    write_challenge_values_summary(
        stdout,
        report.challenge_values_segment_count,
        &report.challenge_values_segment_byte_counts,
        &report.challenge_values_value_counts,
    );
    if report.eth_block_input_count > 0 {
        let _ = writeln!(stdout, "eth_block_inputs={}", report.eth_block_input_count);
        for (index, hash) in report.eth_block_input_hashes.iter().enumerate() {
            let _ = writeln!(
                stdout,
                "eth_block_input_hash={}",
                prove_plan::format_hash(hash)
            );
            if let Some(block_input_bytes) = report.eth_block_input_byte_counts.get(index) {
                let _ = writeln!(stdout, "eth_block_input_bytes={block_input_bytes}");
            }
            if let Some(block_rlp_bytes) = report.eth_block_input_block_rlp_byte_counts.get(index) {
                let _ = writeln!(stdout, "eth_block_rlp_bytes={block_rlp_bytes}");
            }
            write_eth_extra_field_summary(
                stdout,
                report
                    .eth_block_input_extra_header_field_counts
                    .get(index)
                    .copied()
                    .unwrap_or(0),
                report
                    .eth_block_input_extra_body_field_counts
                    .get(index)
                    .copied()
                    .unwrap_or(0),
            );
            if let Some(block_hash) = report.eth_block_input_block_hashes.get(index) {
                let _ = writeln!(
                    stdout,
                    "eth_block_hash={}",
                    prove_plan::format_hash(block_hash)
                );
            }
            if let Some(parent_hash) = report.eth_block_input_parent_hashes.get(index) {
                let _ = writeln!(
                    stdout,
                    "eth_parent_hash={}",
                    prove_plan::format_hash(parent_hash)
                );
            }
            if let Some(ommers_hash) = report.eth_block_input_ommers_hashes.get(index) {
                let _ = writeln!(
                    stdout,
                    "eth_ommers_hash={}",
                    prove_plan::format_hash(ommers_hash)
                );
            }
            if let Some(beneficiary) = report.eth_block_input_beneficiaries.get(index) {
                let _ = writeln!(stdout, "eth_beneficiary={}", format_bytes_hex(beneficiary));
            }
            if let Some(state_root) = report.eth_block_input_state_roots.get(index) {
                let _ = writeln!(
                    stdout,
                    "eth_state_root={}",
                    prove_plan::format_hash(state_root)
                );
            }
            if let Some(receipt_root) = report.eth_block_input_receipt_roots.get(index) {
                let _ = writeln!(
                    stdout,
                    "eth_receipts_root={}",
                    prove_plan::format_hash(receipt_root)
                );
            }
            if let Some(logs_bloom) = report.eth_block_input_logs_blooms.get(index) {
                let _ = writeln!(stdout, "eth_logs_bloom={}", format_bytes_hex(logs_bloom));
            }
            if let Some(difficulty) = report.eth_block_input_difficulties.get(index) {
                let _ = writeln!(stdout, "eth_difficulty={}", format_u256(difficulty));
            }
            if let Some(block_number) = report.eth_block_input_block_numbers.get(index) {
                let _ = writeln!(stdout, "eth_block_number={block_number}");
            }
            if let Some(timestamp) = report.eth_block_input_timestamps.get(index) {
                let _ = writeln!(stdout, "eth_block_timestamp={timestamp}");
            }
            if let Some(extra_data) = report.eth_block_input_extra_data.get(index) {
                let _ = writeln!(stdout, "eth_extra_data={}", format_bytes_hex(extra_data));
            }
            if let Some(gas_limit) = report.eth_block_input_gas_limits.get(index) {
                let _ = writeln!(stdout, "eth_gas_limit={gas_limit}");
            }
            if let Some(gas_used) = report.eth_block_input_gas_used_values.get(index) {
                let _ = writeln!(stdout, "eth_gas_used={gas_used}");
            }
            if let Some(base_fee_per_gas) = report.eth_block_input_base_fees_per_gas.get(index) {
                let _ = writeln!(
                    stdout,
                    "eth_base_fee_per_gas={}",
                    format_optional_u256(base_fee_per_gas.as_ref())
                );
            }
            if let Some(mix_hash) = report.eth_block_input_mix_hashes.get(index) {
                let _ = writeln!(stdout, "eth_mix_hash={}", prove_plan::format_hash(mix_hash));
            }
            if let Some(nonce) = report.eth_block_input_nonces.get(index) {
                let _ = writeln!(stdout, "eth_nonce={}", format_bytes_hex(nonce));
            }
            if let Some(transactions_root) = report.eth_block_input_transaction_roots.get(index) {
                let _ = writeln!(
                    stdout,
                    "eth_transactions_root={}",
                    prove_plan::format_hash(transactions_root)
                );
            }
            write_eth_transaction_preimage_summary(
                stdout,
                report
                    .eth_block_input_transaction_preimage_counts
                    .get(index)
                    .copied()
                    .unwrap_or(0),
            );
            let legacy_transaction_count = report
                .eth_block_input_legacy_transaction_counts
                .get(index)
                .copied()
                .unwrap_or(0);
            let typed_transaction_count = report
                .eth_block_input_typed_transaction_counts
                .get(index)
                .copied()
                .unwrap_or(0);
            write_eth_transaction_count_summary(
                stdout,
                legacy_transaction_count + typed_transaction_count,
            );
            write_eth_transaction_kind_summary(
                stdout,
                legacy_transaction_count,
                typed_transaction_count,
            );
            write_eth_receipt_preimage_summary(
                stdout,
                report
                    .eth_block_input_receipt_preimage_counts
                    .get(index)
                    .copied()
                    .unwrap_or(None),
                report
                    .eth_block_input_receipts_rlp_byte_counts
                    .get(index)
                    .copied()
                    .unwrap_or(None),
            );
            let legacy_receipt_count = report
                .eth_block_input_legacy_receipt_counts
                .get(index)
                .copied()
                .unwrap_or(None);
            let typed_receipt_count = report
                .eth_block_input_typed_receipt_counts
                .get(index)
                .copied()
                .unwrap_or(None);
            if let (Some(legacy_count), Some(typed_count)) =
                (legacy_receipt_count, typed_receipt_count)
            {
                write_eth_receipt_count_summary(stdout, legacy_count + typed_count);
            }
            write_eth_receipt_kind_summary(stdout, legacy_receipt_count, typed_receipt_count);
            write_eth_withdrawal_summary(
                stdout,
                report
                    .eth_block_input_withdrawal_roots
                    .get(index)
                    .copied()
                    .unwrap_or(None),
                report
                    .eth_block_input_withdrawal_counts
                    .get(index)
                    .copied()
                    .unwrap_or(None),
                report
                    .eth_block_input_withdrawal_preimage_counts
                    .get(index)
                    .copied()
                    .unwrap_or(None),
            );
        }
    }
    0
}

pub(super) fn verify_setup_preflight(
    setup_dir: &str,
    proof_bin: &str,
    public_values_path: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    verify_setup_validation(
        VerifySetupValidationCommand {
            role: "verify setup-preflight",
            setup_dir,
            proof_bin,
            public_values_path,
            eth_block_input: None,
            program_image_cache: None,
        },
        stdout,
        stderr,
    )
}

struct ParsedVerifyProofArgs<'a> {
    setup_dir: &'a str,
    proof_bin: &'a str,
    public_values_path: &'a str,
    eth_block_input: Option<&'a str>,
    program_image_cache: Option<&'a str>,
}

fn parse_verify_proof_args<'a>(
    args: &'a [&'a str],
) -> Result<ParsedVerifyProofArgs<'a>, VerifyProofArgError> {
    let mut eth_block_input = None;
    let mut program_image_cache = None;
    let mut positionals = Vec::with_capacity(args.len());
    let mut index = 0;
    while index < args.len() {
        match args[index] {
            "--eth-block-input" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    VerifyProofArgError::Invalid("missing --eth-block-input value".to_owned())
                })?;
                if value.starts_with("--") {
                    return Err(VerifyProofArgError::Invalid(
                        "missing --eth-block-input value".to_owned(),
                    ));
                }
                if eth_block_input.replace(*value).is_some() {
                    return Err(VerifyProofArgError::Invalid(
                        "duplicate --eth-block-input option".to_owned(),
                    ));
                }
            }
            "--program-image-cache" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    VerifyProofArgError::Invalid("missing --program-image-cache value".to_owned())
                })?;
                if value.starts_with("--") {
                    return Err(VerifyProofArgError::Invalid(
                        "missing --program-image-cache value".to_owned(),
                    ));
                }
                if program_image_cache.replace(*value).is_some() {
                    return Err(VerifyProofArgError::Invalid(
                        "duplicate --program-image-cache option".to_owned(),
                    ));
                }
            }
            value if value.starts_with("--") => {
                return Err(VerifyProofArgError::Invalid(format!(
                    "unknown option {value}"
                )));
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
        program_image_cache,
    })
}

enum VerifyProofArgError {
    Usage,
    Invalid(String),
}

pub(super) fn verify_proof(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_verify_proof_args(args) {
        Ok(parsed) => parsed,
        Err(VerifyProofArgError::Usage) => return write_verify_proof_usage(stderr),
        Err(VerifyProofArgError::Invalid(message)) => {
            let _ = writeln!(stderr, "verify proof failed: {message}");
            return 1;
        }
    };
    verify_setup_validation(
        VerifySetupValidationCommand {
            role: "verify proof",
            setup_dir: parsed.setup_dir,
            proof_bin: parsed.proof_bin,
            public_values_path: parsed.public_values_path,
            eth_block_input: parsed.eth_block_input,
            program_image_cache: parsed.program_image_cache,
        },
        stdout,
        stderr,
    )
}

pub(super) fn verify_contribution(
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
        "public_values_hash={}",
        prove_plan::format_hash(&report.public_values_hash)
    );
    let _ = writeln!(
        stdout,
        "public_value_fields={}",
        report.public_value_field_count
    );
    write_contribution_binding_summary(stdout, &report);
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

pub(super) fn verify_contribution_set(
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
        "public_values_hash={}",
        prove_plan::format_hash(&report.public_values_hash)
    );
    let _ = writeln!(
        stdout,
        "public_value_fields={}",
        report.public_value_field_count
    );
    write_contribution_binding_summary(stdout, &report);
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

struct VerifySetupValidationCommand<'a> {
    role: &'a str,
    setup_dir: &'a str,
    proof_bin: &'a str,
    public_values_path: &'a str,
    eth_block_input: Option<&'a str>,
    program_image_cache: Option<&'a str>,
}

struct EthBlockInputBinding {
    hash: [u8; 32],
    bytes: usize,
    block_rlp_bytes: usize,
    extra_header_field_count: usize,
    extra_body_field_count: usize,
    block_hash: [u8; 32],
    parent_hash: [u8; 32],
    ommers_hash: [u8; 32],
    beneficiary: [u8; 20],
    state_root: [u8; 32],
    receipts_root: [u8; 32],
    logs_bloom: [u8; 256],
    difficulty: [u8; 32],
    block_number: u64,
    timestamp: u64,
    extra_data: Vec<u8>,
    gas_limit: u64,
    gas_used: u64,
    base_fee_per_gas: Option<[u8; 32]>,
    mix_hash: [u8; 32],
    nonce: [u8; 8],
    transactions_root: [u8; 32],
    transaction_preimage_count: usize,
    legacy_transaction_count: usize,
    typed_transaction_count: usize,
    receipts_rlp_bytes: Option<usize>,
    receipt_preimage_count: Option<usize>,
    legacy_receipt_count: Option<usize>,
    typed_receipt_count: Option<usize>,
    withdrawal_root: Option<[u8; 32]>,
    withdrawal_count: Option<usize>,
    withdrawal_preimage_count: Option<usize>,
}

fn verify_setup_validation(
    command: VerifySetupValidationCommand<'_>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let eth_block_input_binding = if let Some(path) = command.eth_block_input {
        match verify_eth_block_input_binding(command.proof_bin, command.public_values_path, path) {
            Ok(binding) => Some(binding),
            Err(message) => {
                let _ = writeln!(stderr, "{} failed: {message}", command.role);
                return 1;
            }
        }
    } else {
        None
    };
    let program_image_cache_matched = if let Some(path) = command.program_image_cache {
        match verify_program_image_cache_binding(command.proof_bin, path) {
            Ok(()) => true,
            Err(message) => {
                let _ = writeln!(stderr, "{} failed: {message}", command.role);
                return 1;
            }
        }
    } else {
        false
    };
    let public_report = match validate_setup_preflight_from_files(
        command.setup_dir,
        command.proof_bin,
        command.public_values_path,
    ) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "{} failed: {error}", command.role);
            return 1;
        }
    };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", public_report.unit_count);
    let _ = writeln!(stdout, "segments={}", public_report.segment_count);
    let _ = writeln!(stdout, "public_values={}", public_report.public_value_count);
    let _ = writeln!(
        stdout,
        "public_values_hash={}",
        prove_plan::format_hash(&public_report.public_values_hash)
    );
    let _ = writeln!(
        stdout,
        "public_value_fields={}",
        public_report.public_value_field_count
    );
    if public_report.source_fixed_file_manifest_present
        || public_report.source_program_archive_present
    {
        let _ = writeln!(
            stdout,
            "source_fixed_file_manifest={}",
            if public_report.source_fixed_file_manifest_present {
                "present"
            } else {
                "absent"
            }
        );
        let _ = writeln!(
            stdout,
            "source_fixed_file_manifest_entries={}",
            public_report.source_fixed_file_manifest_entry_count
        );
        let _ = writeln!(
            stdout,
            "source_program_archive={}",
            if public_report.source_program_archive_present {
                "present"
            } else {
                "absent"
            }
        );
        let _ = writeln!(
            stdout,
            "source_program_archive_sources={}",
            public_report.source_program_archive_source_count
        );
        let _ = writeln!(
            stdout,
            "source_program_archive_edges={}",
            public_report.source_program_archive_edge_count
        );
    }
    if public_report.program_image_cache_count > 0 {
        let _ = writeln!(
            stdout,
            "program_image_caches={}",
            public_report.program_image_cache_count
        );
        for (cache, hash) in public_report
            .program_image_caches
            .iter()
            .zip(&public_report.program_image_cache_hashes)
        {
            program_image_cache::write_program_image_cache_fields_with_segment_hash(
                stdout, cache, hash,
            );
        }
    }
    write_challenge_values_summary(
        stdout,
        public_report.challenge_values_segment_count,
        &public_report.challenge_values_segment_byte_counts,
        &public_report.challenge_values_value_counts,
    );
    if public_report.eth_block_input_count > 0 {
        let _ = writeln!(
            stdout,
            "eth_block_inputs={}",
            public_report.eth_block_input_count
        );
        for (index, hash) in public_report.eth_block_input_hashes.iter().enumerate() {
            let _ = writeln!(
                stdout,
                "eth_block_input_hash={}",
                prove_plan::format_hash(hash)
            );
            if eth_block_input_binding.is_none() {
                if let Some(block_input_bytes) =
                    public_report.eth_block_input_byte_counts.get(index)
                {
                    let _ = writeln!(stdout, "eth_block_input_bytes={block_input_bytes}");
                }
                if let Some(block_rlp_bytes) = public_report
                    .eth_block_input_block_rlp_byte_counts
                    .get(index)
                {
                    let _ = writeln!(stdout, "eth_block_rlp_bytes={block_rlp_bytes}");
                }
                write_eth_extra_field_summary(
                    stdout,
                    public_report
                        .eth_block_input_extra_header_field_counts
                        .get(index)
                        .copied()
                        .unwrap_or(0),
                    public_report
                        .eth_block_input_extra_body_field_counts
                        .get(index)
                        .copied()
                        .unwrap_or(0),
                );
                if let Some(block_hash) = public_report.eth_block_input_block_hashes.get(index) {
                    let _ = writeln!(
                        stdout,
                        "eth_block_hash={}",
                        prove_plan::format_hash(block_hash)
                    );
                }
                if let Some(parent_hash) = public_report.eth_block_input_parent_hashes.get(index) {
                    let _ = writeln!(
                        stdout,
                        "eth_parent_hash={}",
                        prove_plan::format_hash(parent_hash)
                    );
                }
                if let Some(ommers_hash) = public_report.eth_block_input_ommers_hashes.get(index) {
                    let _ = writeln!(
                        stdout,
                        "eth_ommers_hash={}",
                        prove_plan::format_hash(ommers_hash)
                    );
                }
                if let Some(beneficiary) = public_report.eth_block_input_beneficiaries.get(index) {
                    let _ = writeln!(stdout, "eth_beneficiary={}", format_bytes_hex(beneficiary));
                }
                if let Some(state_root) = public_report.eth_block_input_state_roots.get(index) {
                    let _ = writeln!(
                        stdout,
                        "eth_state_root={}",
                        prove_plan::format_hash(state_root)
                    );
                }
                if let Some(receipt_root) = public_report.eth_block_input_receipt_roots.get(index) {
                    let _ = writeln!(
                        stdout,
                        "eth_receipts_root={}",
                        prove_plan::format_hash(receipt_root)
                    );
                }
                if let Some(logs_bloom) = public_report.eth_block_input_logs_blooms.get(index) {
                    let _ = writeln!(stdout, "eth_logs_bloom={}", format_bytes_hex(logs_bloom));
                }
                if let Some(difficulty) = public_report.eth_block_input_difficulties.get(index) {
                    let _ = writeln!(stdout, "eth_difficulty={}", format_u256(difficulty));
                }
                if let Some(block_number) = public_report.eth_block_input_block_numbers.get(index) {
                    let _ = writeln!(stdout, "eth_block_number={block_number}");
                }
                if let Some(timestamp) = public_report.eth_block_input_timestamps.get(index) {
                    let _ = writeln!(stdout, "eth_block_timestamp={timestamp}");
                }
                if let Some(extra_data) = public_report.eth_block_input_extra_data.get(index) {
                    let _ = writeln!(stdout, "eth_extra_data={}", format_bytes_hex(extra_data));
                }
                if let Some(gas_limit) = public_report.eth_block_input_gas_limits.get(index) {
                    let _ = writeln!(stdout, "eth_gas_limit={gas_limit}");
                }
                if let Some(gas_used) = public_report.eth_block_input_gas_used_values.get(index) {
                    let _ = writeln!(stdout, "eth_gas_used={gas_used}");
                }
                if let Some(base_fee_per_gas) =
                    public_report.eth_block_input_base_fees_per_gas.get(index)
                {
                    let _ = writeln!(
                        stdout,
                        "eth_base_fee_per_gas={}",
                        format_optional_u256(base_fee_per_gas.as_ref())
                    );
                }
                if let Some(mix_hash) = public_report.eth_block_input_mix_hashes.get(index) {
                    let _ = writeln!(stdout, "eth_mix_hash={}", prove_plan::format_hash(mix_hash));
                }
                if let Some(nonce) = public_report.eth_block_input_nonces.get(index) {
                    let _ = writeln!(stdout, "eth_nonce={}", format_bytes_hex(nonce));
                }
                if let Some(transactions_root) =
                    public_report.eth_block_input_transaction_roots.get(index)
                {
                    let _ = writeln!(
                        stdout,
                        "eth_transactions_root={}",
                        prove_plan::format_hash(transactions_root)
                    );
                }
                write_eth_transaction_preimage_summary(
                    stdout,
                    public_report
                        .eth_block_input_transaction_preimage_counts
                        .get(index)
                        .copied()
                        .unwrap_or(0),
                );
                let legacy_transaction_count = public_report
                    .eth_block_input_legacy_transaction_counts
                    .get(index)
                    .copied()
                    .unwrap_or(0);
                let typed_transaction_count = public_report
                    .eth_block_input_typed_transaction_counts
                    .get(index)
                    .copied()
                    .unwrap_or(0);
                write_eth_transaction_count_summary(
                    stdout,
                    legacy_transaction_count + typed_transaction_count,
                );
                write_eth_transaction_kind_summary(
                    stdout,
                    legacy_transaction_count,
                    typed_transaction_count,
                );
                write_eth_receipt_preimage_summary(
                    stdout,
                    public_report
                        .eth_block_input_receipt_preimage_counts
                        .get(index)
                        .copied()
                        .unwrap_or(None),
                    public_report
                        .eth_block_input_receipts_rlp_byte_counts
                        .get(index)
                        .copied()
                        .unwrap_or(None),
                );
                let legacy_receipt_count = public_report
                    .eth_block_input_legacy_receipt_counts
                    .get(index)
                    .copied()
                    .unwrap_or(None);
                let typed_receipt_count = public_report
                    .eth_block_input_typed_receipt_counts
                    .get(index)
                    .copied()
                    .unwrap_or(None);
                if let (Some(legacy_count), Some(typed_count)) =
                    (legacy_receipt_count, typed_receipt_count)
                {
                    write_eth_receipt_count_summary(stdout, legacy_count + typed_count);
                }
                write_eth_receipt_kind_summary(stdout, legacy_receipt_count, typed_receipt_count);
                write_eth_withdrawal_summary(
                    stdout,
                    public_report
                        .eth_block_input_withdrawal_roots
                        .get(index)
                        .copied()
                        .unwrap_or(None),
                    public_report
                        .eth_block_input_withdrawal_counts
                        .get(index)
                        .copied()
                        .unwrap_or(None),
                    public_report
                        .eth_block_input_withdrawal_preimage_counts
                        .get(index)
                        .copied()
                        .unwrap_or(None),
                );
            }
        }
    }
    if let Some(binding) = eth_block_input_binding {
        if public_report.eth_block_input_hashes.is_empty() {
            let _ = writeln!(
                stdout,
                "eth_block_input_hash={}",
                prove_plan::format_hash(&binding.hash)
            );
        }
        let _ = writeln!(stdout, "eth_block_input_match=ok");
        let _ = writeln!(stdout, "eth_block_input_bytes={}", binding.bytes);
        let _ = writeln!(stdout, "eth_block_rlp_bytes={}", binding.block_rlp_bytes);
        write_eth_extra_field_summary(
            stdout,
            binding.extra_header_field_count,
            binding.extra_body_field_count,
        );
        let _ = writeln!(
            stdout,
            "eth_block_hash={}",
            prove_plan::format_hash(&binding.block_hash)
        );
        let _ = writeln!(
            stdout,
            "eth_parent_hash={}",
            prove_plan::format_hash(&binding.parent_hash)
        );
        let _ = writeln!(
            stdout,
            "eth_ommers_hash={}",
            prove_plan::format_hash(&binding.ommers_hash)
        );
        let _ = writeln!(
            stdout,
            "eth_beneficiary={}",
            format_bytes_hex(&binding.beneficiary)
        );
        let _ = writeln!(
            stdout,
            "eth_state_root={}",
            prove_plan::format_hash(&binding.state_root)
        );
        let _ = writeln!(
            stdout,
            "eth_receipts_root={}",
            prove_plan::format_hash(&binding.receipts_root)
        );
        let _ = writeln!(
            stdout,
            "eth_logs_bloom={}",
            format_bytes_hex(&binding.logs_bloom)
        );
        let _ = writeln!(
            stdout,
            "eth_difficulty={}",
            format_u256(&binding.difficulty)
        );
        let _ = writeln!(stdout, "eth_block_number={}", binding.block_number);
        let _ = writeln!(stdout, "eth_block_timestamp={}", binding.timestamp);
        let _ = writeln!(
            stdout,
            "eth_extra_data={}",
            format_bytes_hex(&binding.extra_data)
        );
        let _ = writeln!(stdout, "eth_gas_limit={}", binding.gas_limit);
        let _ = writeln!(stdout, "eth_gas_used={}", binding.gas_used);
        let _ = writeln!(
            stdout,
            "eth_base_fee_per_gas={}",
            format_optional_u256(binding.base_fee_per_gas.as_ref())
        );
        let _ = writeln!(
            stdout,
            "eth_mix_hash={}",
            prove_plan::format_hash(&binding.mix_hash)
        );
        let _ = writeln!(stdout, "eth_nonce={}", format_bytes_hex(&binding.nonce));
        let _ = writeln!(
            stdout,
            "eth_transactions_root={}",
            prove_plan::format_hash(&binding.transactions_root)
        );
        write_eth_transaction_preimage_summary(stdout, binding.transaction_preimage_count);
        write_eth_transaction_count_summary(
            stdout,
            binding.legacy_transaction_count + binding.typed_transaction_count,
        );
        write_eth_transaction_kind_summary(
            stdout,
            binding.legacy_transaction_count,
            binding.typed_transaction_count,
        );
        write_eth_receipt_preimage_summary(
            stdout,
            binding.receipt_preimage_count,
            binding.receipts_rlp_bytes,
        );
        if let (Some(legacy_count), Some(typed_count)) =
            (binding.legacy_receipt_count, binding.typed_receipt_count)
        {
            write_eth_receipt_count_summary(stdout, legacy_count + typed_count);
        }
        write_eth_receipt_kind_summary(
            stdout,
            binding.legacy_receipt_count,
            binding.typed_receipt_count,
        );
        write_eth_withdrawal_summary(
            stdout,
            binding.withdrawal_root,
            binding.withdrawal_count,
            binding.withdrawal_preimage_count,
        );
    }
    if program_image_cache_matched {
        let _ = writeln!(stdout, "program_image_cache_match=ok");
    }
    0
}

fn verify_program_image_cache_binding(proof_bin: &str, cache_path: &str) -> Result<(), String> {
    let proof = read_proof_artifact_file(proof_bin)
        .map_err(|error| format!("read proof artifact failed: {proof_bin}: {error}"))?;
    let cache = read_program_image_commitment_cache_file(cache_path)
        .map_err(|error| format!("read program-image cache failed: {cache_path}: {error}"))?;
    let expected = encode_program_image_cache_segment(&cache)
        .map_err(|error| format!("encode program-image cache segment failed: {error}"))?;
    let segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PROGRAM_IMAGE_CACHE_SEGMENT_ID)
        .ok_or_else(|| "missing program image cache proof segment".to_owned())?;
    if segment.data != expected {
        return Err("program image cache proof segment mismatch".to_owned());
    }
    Ok(())
}

fn verify_eth_block_input_binding(
    proof_bin: &str,
    public_values_path: &str,
    input_path: &str,
) -> Result<EthBlockInputBinding, String> {
    let proof = read_proof_artifact_file(proof_bin)
        .map_err(|error| format!("read proof artifact failed: {proof_bin}: {error}"))?;
    let input_bytes = std::fs::read(input_path)
        .map_err(|error| format!("read ETH block input failed: {input_path}: {error}"))?;
    let input_hash = eth_block_input_bytes_digest(&input_bytes);
    let input = parse_eth_block_input(&input_bytes)
        .map_err(|error| format!("ETH block input failed: {input_path}: {error}"))?;
    let transaction_preimage_count = input.transactions.hash_preimages.len();
    let (legacy_transaction_count, typed_transaction_count) =
        eth_block_input_transaction_kind_counts(&input)
            .map_err(|error| format!("ETH block input transaction count failed: {error}"))?;
    let (extra_header_field_count, extra_body_field_count) =
        eth_block_input_extra_field_counts(&input)
            .map_err(|error| format!("ETH block input extra field count failed: {error}"))?;
    let receipt_preimage_count = input
        .receipts
        .as_ref()
        .map(|receipts| receipts.hash_preimages.len());
    let receipts_rlp_bytes = input
        .receipts_rlp
        .as_ref()
        .map(|receipts_rlp| receipts_rlp.len());
    let receipt_kind_counts = eth_block_input_receipt_kind_counts(&input)
        .map_err(|error| format!("ETH block input receipt count failed: {error}"))?;
    let withdrawal_count = eth_block_input_withdrawal_count(&input)
        .map_err(|error| format!("ETH block input withdrawal count failed: {error}"))?;
    let withdrawal_root = input.withdrawals_root;
    let withdrawal_preimage_count = input
        .withdrawals
        .as_ref()
        .map(|withdrawals| withdrawals.hash_preimages.len());
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
    Ok(EthBlockInputBinding {
        hash: input_hash,
        bytes: input_bytes.len(),
        block_rlp_bytes: input.block_rlp.len(),
        extra_header_field_count,
        extra_body_field_count,
        block_hash: input.block_hash,
        parent_hash: input.parent_hash,
        ommers_hash: input.ommers_hash,
        beneficiary: input.beneficiary,
        state_root: input.state_root,
        receipts_root: input.receipts_root,
        logs_bloom: input.logs_bloom,
        difficulty: input.difficulty,
        block_number: input.block_number,
        timestamp: input.timestamp,
        extra_data: input.extra_data,
        gas_limit: input.gas_limit,
        gas_used: input.gas_used,
        base_fee_per_gas: input.base_fee_per_gas,
        mix_hash: input.mix_hash,
        nonce: input.nonce,
        transactions_root: input.transactions_root,
        transaction_preimage_count,
        legacy_transaction_count,
        typed_transaction_count,
        receipts_rlp_bytes,
        receipt_preimage_count,
        legacy_receipt_count: receipt_kind_counts.map(|(legacy_count, _)| legacy_count),
        typed_receipt_count: receipt_kind_counts.map(|(_, typed_count)| typed_count),
        withdrawal_root,
        withdrawal_count,
        withdrawal_preimage_count,
    })
}

fn write_eth_transaction_preimage_summary(
    stdout: &mut dyn Write,
    transaction_preimage_count: usize,
) {
    let _ = writeln!(
        stdout,
        "eth_transaction_trie_preimages={transaction_preimage_count}"
    );
}

fn write_eth_extra_field_summary(
    stdout: &mut dyn Write,
    extra_header_field_count: usize,
    extra_body_field_count: usize,
) {
    if extra_header_field_count == 0 && extra_body_field_count == 0 {
        return;
    }
    let _ = writeln!(stdout, "eth_extra_header_fields={extra_header_field_count}");
    let _ = writeln!(stdout, "eth_extra_body_fields={extra_body_field_count}");
}

fn write_challenge_values_summary(
    stdout: &mut dyn Write,
    segment_count: usize,
    segment_byte_counts: &[usize],
    value_counts: &[usize],
) {
    if segment_count == 0 {
        return;
    }
    let _ = writeln!(stdout, "challenge_values_segments={segment_count}");
    for (byte_count, value_count) in segment_byte_counts.iter().zip(value_counts) {
        let _ = writeln!(stdout, "challenge_values_segment_bytes={byte_count}");
        let _ = writeln!(stdout, "challenge_values_count={value_count}");
    }
}

fn write_contribution_binding_summary(
    stdout: &mut dyn Write,
    report: &ContributionChallengeReport,
) {
    if report.program_image_cache_count > 0 {
        let _ = writeln!(
            stdout,
            "program_image_caches={}",
            report.program_image_cache_count
        );
        for (cache, hash) in report
            .program_image_caches
            .iter()
            .zip(&report.program_image_cache_hashes)
        {
            program_image_cache::write_program_image_cache_fields_with_segment_hash(
                stdout, cache, hash,
            );
        }
    }
    if report.eth_block_input_count > 0 {
        let _ = writeln!(stdout, "eth_block_inputs={}", report.eth_block_input_count);
        for input in &report.eth_block_inputs {
            eth_block_output::write_contribution_eth_block_input(stdout, input);
        }
    }
}

fn format_bytes_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn format_u256(bytes: &[u8; 32]) -> String {
    match bytes.iter().position(|byte| *byte != 0) {
        Some(index) => format_bytes_hex(&bytes[index..]),
        None => "0".to_owned(),
    }
}

fn format_optional_u256(value: Option<&[u8; 32]>) -> String {
    match value {
        Some(bytes) => format_u256(bytes),
        None => "absent".to_owned(),
    }
}

fn write_eth_transaction_kind_summary(
    stdout: &mut dyn Write,
    legacy_transaction_count: usize,
    typed_transaction_count: usize,
) {
    let _ = writeln!(stdout, "eth_legacy_transactions={legacy_transaction_count}");
    let _ = writeln!(stdout, "eth_typed_transactions={typed_transaction_count}");
}

fn write_eth_transaction_count_summary(stdout: &mut dyn Write, transaction_count: usize) {
    let _ = writeln!(stdout, "eth_transaction_count={transaction_count}");
}

fn write_eth_receipt_count_summary(stdout: &mut dyn Write, receipt_count: usize) {
    let _ = writeln!(stdout, "eth_receipt_count={receipt_count}");
}

fn write_eth_receipt_preimage_summary(
    stdout: &mut dyn Write,
    receipt_preimage_count: Option<usize>,
    receipts_rlp_bytes: Option<usize>,
) {
    match receipt_preimage_count {
        Some(count) => {
            let _ = writeln!(stdout, "eth_receipts=present");
            if let Some(bytes) = receipts_rlp_bytes {
                let _ = writeln!(stdout, "eth_receipts_rlp_bytes={bytes}");
            }
            let _ = writeln!(stdout, "eth_receipt_trie_preimages={count}");
        }
        None => {
            let _ = writeln!(stdout, "eth_receipts=absent");
        }
    }
}

fn write_eth_receipt_kind_summary(
    stdout: &mut dyn Write,
    legacy_receipt_count: Option<usize>,
    typed_receipt_count: Option<usize>,
) {
    if let (Some(legacy_count), Some(typed_count)) = (legacy_receipt_count, typed_receipt_count) {
        let _ = writeln!(stdout, "eth_legacy_receipts={legacy_count}");
        let _ = writeln!(stdout, "eth_typed_receipts={typed_count}");
    }
}

fn write_eth_withdrawal_summary(
    stdout: &mut dyn Write,
    withdrawal_root: Option<[u8; 32]>,
    withdrawal_count: Option<usize>,
    withdrawal_preimage_count: Option<usize>,
) {
    match withdrawal_preimage_count {
        Some(count) => {
            let _ = writeln!(stdout, "eth_withdrawals=present");
            if let Some(root) = withdrawal_root {
                let _ = writeln!(
                    stdout,
                    "eth_withdrawals_root={}",
                    prove_plan::format_hash(&root)
                );
            }
            if let Some(withdrawal_count) = withdrawal_count {
                let _ = writeln!(stdout, "eth_withdrawal_count={withdrawal_count}");
            }
            let _ = writeln!(stdout, "eth_withdrawal_trie_preimages={count}");
        }
        None => {
            let _ = writeln!(stdout, "eth_withdrawals=absent");
        }
    }
}

pub(super) fn write_verify_preflight_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify preflight <proof-bin> <public-values>"
    );
    2
}

pub(super) fn write_verify_setup_preflight_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify setup-preflight <setup-dir> <proof-bin> <public-values>"
    );
    2
}

fn write_verify_proof_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify proof [--eth-block-input <block-input>] [--program-image-cache <cache-bin>] <setup-dir> <proof-bin> <public-values>"
    );
    2
}

pub(super) fn write_verify_contribution_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify contribution <setup-dir> <proof-bin> <public-values>"
    );
    2
}

pub(super) fn write_verify_contribution_set_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify contribution-set <setup-dir> <public-values> <proof-bin> [proof-bin ...]"
    );
    2
}
