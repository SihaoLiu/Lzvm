use std::io::Write;
use std::path::PathBuf;

use lzvm_artifacts::program_image::read_program_image_commitment_cache_file;
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};
use lzvm_artifacts::proof::read_proof_artifact_file;
use lzvm_prover::contribution::{
    derive_global_challenge_from_embedded_contribution_proofs, derive_global_challenge_from_files,
    ContributionChallengeReport,
};
use lzvm_prover::proof_preflight::validate_proof_public_values_from_files;
use lzvm_prover::setup_preflight::validate_setup_preflight_from_files;

use crate::{
    eth_block_output, eth_block_prove_input::EthPublicInputMode, program_image_cache, prove_plan,
};

mod eth_block_input;
mod eth_block_summary;
use eth_block_input::{
    verify_eth_block_input_binding, verify_eth_public_input_binding_with_mode, EthBlockInputBinding,
};
use eth_block_summary::{
    format_bytes_hex, format_optional_u256, format_u256, write_eth_block_input_binding_summary,
    write_eth_extra_field_summary, write_eth_receipt_count_summary, write_eth_receipt_kind_summary,
    write_eth_receipt_preimage_summary, write_eth_transaction_count_summary,
    write_eth_transaction_kind_summary, write_eth_transaction_preimage_summary,
    write_eth_withdrawal_summary, write_report_eth_block_input_summary,
};

pub(super) fn verify_preflight(
    args: &[&str],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let parsed = match parse_verify_preflight_args(args) {
        Ok(parsed) => parsed,
        Err(SetupValidationArgError::Usage) => return write_verify_preflight_usage(stderr),
        Err(SetupValidationArgError::Invalid(message)) => {
            let _ = writeln!(stderr, "verify preflight failed: {message}");
            return 1;
        }
    };
    let eth_block_input_binding = match verify_requested_eth_block_binding(
        "verify preflight",
        parsed.proof_bin,
        parsed.public_values_path,
        parsed.eth_block_input,
        parsed.eth_public_input,
        parsed.eth_public_input_allow_trailing,
        stderr,
    ) {
        Some(binding) => binding,
        None => return 1,
    };
    let program_image_cache_matched = match verify_requested_program_image_cache_binding(
        "verify preflight",
        parsed.proof_bin,
        parsed.program_image_cache,
        stderr,
    ) {
        Some(matched) => matched,
        None => return 1,
    };
    let report = match validate_proof_public_values_from_files(
        parsed.proof_bin,
        parsed.public_values_path,
    ) {
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
            if eth_block_input_binding.is_none() {
                write_report_eth_block_input_summary(stdout, &report, index);
            }
        }
    }
    if let Some(binding) = eth_block_input_binding {
        write_eth_block_input_binding_summary(stdout, &report.eth_block_input_hashes, &binding);
    }
    if program_image_cache_matched {
        let _ = writeln!(stdout, "program_image_cache_match=ok");
    }
    0
}

pub(super) fn verify_setup_preflight(
    args: &[&str],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let parsed = match parse_verify_setup_preflight_args(args) {
        Ok(parsed) => parsed,
        Err(SetupValidationArgError::Usage) => return write_verify_setup_preflight_usage(stderr),
        Err(SetupValidationArgError::Invalid(message)) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {message}");
            return 1;
        }
    };
    verify_setup_validation(
        VerifySetupValidationCommand {
            role: "verify setup-preflight",
            setup_dir: parsed.setup_dir,
            proof_bin: parsed.proof_bin,
            public_values_path: parsed.public_values_path,
            eth_block_input: parsed.eth_block_input,
            eth_public_input: parsed.eth_public_input,
            eth_public_input_allow_trailing: parsed.eth_public_input_allow_trailing,
            program_image_cache: parsed.program_image_cache,
        },
        stdout,
        stderr,
    )
}

struct ParsedSetupValidationArgs<'a> {
    setup_dir: &'a str,
    proof_bin: &'a str,
    public_values_path: &'a str,
    eth_block_input: Option<&'a str>,
    eth_public_input: Option<&'a str>,
    eth_public_input_allow_trailing: bool,
    program_image_cache: Option<&'a str>,
}

struct ParsedPreflightArgs<'a> {
    proof_bin: &'a str,
    public_values_path: &'a str,
    eth_block_input: Option<&'a str>,
    eth_public_input: Option<&'a str>,
    eth_public_input_allow_trailing: bool,
    program_image_cache: Option<&'a str>,
}

struct ParsedContributionSetArgs<'a> {
    setup_dir: &'a str,
    public_values_path: &'a str,
    proof_bins: Vec<&'a str>,
    eth_block_input: Option<&'a str>,
    eth_public_input: Option<&'a str>,
    eth_public_input_allow_trailing: bool,
    program_image_cache: Option<&'a str>,
}

pub(crate) struct ParsedBindingArgs<'a> {
    pub(crate) positionals: Vec<&'a str>,
    pub(crate) eth_block_input: Option<&'a str>,
    pub(crate) eth_public_input: Option<&'a str>,
    pub(crate) eth_public_input_allow_trailing: bool,
    pub(crate) program_image_cache: Option<&'a str>,
}

pub(crate) struct VerifiedContributionBindings {
    eth_block_input_binding: Option<EthBlockInputBinding>,
    program_image_cache_matched: bool,
}

pub(crate) struct ContributionBindingRequest<'a> {
    pub(crate) role: &'a str,
    pub(crate) proof_bins: &'a [&'a str],
    pub(crate) public_values_path: &'a str,
    pub(crate) eth_block_input: Option<&'a str>,
    pub(crate) eth_public_input: Option<&'a str>,
    pub(crate) eth_public_input_allow_trailing: bool,
    pub(crate) program_image_cache: Option<&'a str>,
}

fn parse_verify_preflight_args<'a>(
    args: &'a [&'a str],
) -> Result<ParsedPreflightArgs<'a>, SetupValidationArgError> {
    let parsed = parse_binding_args(args)?;
    if parsed.positionals.len() != 2 {
        return Err(SetupValidationArgError::Usage);
    }
    validate_binding_args(&parsed)?;
    Ok(ParsedPreflightArgs {
        proof_bin: parsed.positionals[0],
        public_values_path: parsed.positionals[1],
        eth_block_input: parsed.eth_block_input,
        eth_public_input: parsed.eth_public_input,
        eth_public_input_allow_trailing: parsed.eth_public_input_allow_trailing,
        program_image_cache: parsed.program_image_cache,
    })
}

fn parse_verify_proof_args<'a>(
    args: &'a [&'a str],
) -> Result<ParsedSetupValidationArgs<'a>, SetupValidationArgError> {
    parse_setup_validation_args(args)
}

fn parse_verify_setup_preflight_args<'a>(
    args: &'a [&'a str],
) -> Result<ParsedSetupValidationArgs<'a>, SetupValidationArgError> {
    parse_setup_validation_args(args)
}

fn parse_verify_contribution_args<'a>(
    args: &'a [&'a str],
) -> Result<ParsedSetupValidationArgs<'a>, SetupValidationArgError> {
    parse_setup_validation_args(args)
}

fn parse_verify_contribution_set_args<'a>(
    args: &'a [&'a str],
) -> Result<ParsedContributionSetArgs<'a>, SetupValidationArgError> {
    let parsed = parse_binding_args(args)?;
    if parsed.positionals.len() < 3 {
        return Err(SetupValidationArgError::Usage);
    }
    validate_binding_args(&parsed)?;
    Ok(ParsedContributionSetArgs {
        setup_dir: parsed.positionals[0],
        public_values_path: parsed.positionals[1],
        proof_bins: parsed.positionals[2..].to_vec(),
        eth_block_input: parsed.eth_block_input,
        eth_public_input: parsed.eth_public_input,
        eth_public_input_allow_trailing: parsed.eth_public_input_allow_trailing,
        program_image_cache: parsed.program_image_cache,
    })
}

fn parse_setup_validation_args<'a>(
    args: &'a [&'a str],
) -> Result<ParsedSetupValidationArgs<'a>, SetupValidationArgError> {
    let parsed = parse_binding_args(args)?;
    if parsed.positionals.len() != 3 {
        return Err(SetupValidationArgError::Usage);
    }
    validate_binding_args(&parsed)?;
    Ok(ParsedSetupValidationArgs {
        setup_dir: parsed.positionals[0],
        proof_bin: parsed.positionals[1],
        public_values_path: parsed.positionals[2],
        eth_block_input: parsed.eth_block_input,
        eth_public_input: parsed.eth_public_input,
        eth_public_input_allow_trailing: parsed.eth_public_input_allow_trailing,
        program_image_cache: parsed.program_image_cache,
    })
}

pub(crate) fn parse_binding_args<'a>(
    args: &'a [&'a str],
) -> Result<ParsedBindingArgs<'a>, SetupValidationArgError> {
    let mut eth_block_input = None;
    let mut eth_public_input = None;
    let mut eth_public_input_allow_trailing = false;
    let mut program_image_cache = None;
    let mut positionals = Vec::with_capacity(args.len());
    let mut index = 0;
    while index < args.len() {
        match args[index] {
            "--eth-block-input" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    SetupValidationArgError::Invalid("missing --eth-block-input value".to_owned())
                })?;
                if value.starts_with("--") {
                    return Err(SetupValidationArgError::Invalid(
                        "missing --eth-block-input value".to_owned(),
                    ));
                }
                if eth_block_input.replace(*value).is_some() {
                    return Err(SetupValidationArgError::Invalid(
                        "duplicate --eth-block-input option".to_owned(),
                    ));
                }
            }
            "--program-image-cache" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    SetupValidationArgError::Invalid(
                        "missing --program-image-cache value".to_owned(),
                    )
                })?;
                if value.starts_with("--") {
                    return Err(SetupValidationArgError::Invalid(
                        "missing --program-image-cache value".to_owned(),
                    ));
                }
                if program_image_cache.replace(*value).is_some() {
                    return Err(SetupValidationArgError::Invalid(
                        "duplicate --program-image-cache option".to_owned(),
                    ));
                }
            }
            "--eth-public-input" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    SetupValidationArgError::Invalid("missing --eth-public-input value".to_owned())
                })?;
                if value.starts_with("--") {
                    return Err(SetupValidationArgError::Invalid(
                        "missing --eth-public-input value".to_owned(),
                    ));
                }
                if eth_public_input.replace(*value).is_some() {
                    return Err(SetupValidationArgError::Invalid(
                        "duplicate --eth-public-input option".to_owned(),
                    ));
                }
            }
            "--eth-public-input-allow-trailing" => {
                if eth_public_input_allow_trailing {
                    return Err(SetupValidationArgError::Invalid(
                        "duplicate --eth-public-input-allow-trailing option".to_owned(),
                    ));
                }
                eth_public_input_allow_trailing = true;
            }
            value if value.starts_with("--") => {
                return Err(SetupValidationArgError::Invalid(format!(
                    "unknown option {value}"
                )));
            }
            value => positionals.push(value),
        }
        index += 1;
    }
    Ok(ParsedBindingArgs {
        positionals,
        eth_block_input,
        eth_public_input,
        eth_public_input_allow_trailing,
        program_image_cache,
    })
}

pub(crate) fn validate_binding_args(
    parsed: &ParsedBindingArgs<'_>,
) -> Result<(), SetupValidationArgError> {
    if parsed.eth_block_input.is_some() && parsed.eth_public_input.is_some() {
        return Err(SetupValidationArgError::Invalid(
            "cannot combine --eth-block-input and --eth-public-input".to_owned(),
        ));
    }
    if parsed.eth_public_input_allow_trailing && parsed.eth_public_input.is_none() {
        return Err(SetupValidationArgError::Invalid(
            "cannot use --eth-public-input-allow-trailing without --eth-public-input".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) enum SetupValidationArgError {
    Usage,
    Invalid(String),
}

pub(super) fn verify_proof(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_verify_proof_args(args) {
        Ok(parsed) => parsed,
        Err(SetupValidationArgError::Usage) => return write_verify_proof_usage(stderr),
        Err(SetupValidationArgError::Invalid(message)) => {
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
            eth_public_input: parsed.eth_public_input,
            eth_public_input_allow_trailing: parsed.eth_public_input_allow_trailing,
            program_image_cache: parsed.program_image_cache,
        },
        stdout,
        stderr,
    )
}

pub(super) fn verify_contribution(
    args: &[&str],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let parsed = match parse_verify_contribution_args(args) {
        Ok(parsed) => parsed,
        Err(SetupValidationArgError::Usage) => return write_verify_contribution_usage(stderr),
        Err(SetupValidationArgError::Invalid(message)) => {
            let _ = writeln!(stderr, "verify contribution failed: {message}");
            return 1;
        }
    };
    let eth_block_input_binding = match verify_requested_eth_block_binding(
        "verify contribution",
        parsed.proof_bin,
        parsed.public_values_path,
        parsed.eth_block_input,
        parsed.eth_public_input,
        parsed.eth_public_input_allow_trailing,
        stderr,
    ) {
        Some(binding) => binding,
        None => return 1,
    };
    let program_image_cache_matched = match verify_requested_program_image_cache_binding(
        "verify contribution",
        parsed.proof_bin,
        parsed.program_image_cache,
        stderr,
    ) {
        Some(matched) => matched,
        None => return 1,
    };
    let report = match derive_global_challenge_from_files(
        parsed.setup_dir,
        parsed.proof_bin,
        parsed.public_values_path,
    ) {
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
    let bindings = VerifiedContributionBindings {
        eth_block_input_binding,
        program_image_cache_matched,
    };
    write_contribution_binding_summary(stdout, &report, &bindings);
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
    args: &[&str],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let parsed = match parse_verify_contribution_set_args(args) {
        Ok(parsed) => parsed,
        Err(SetupValidationArgError::Usage) => return write_verify_contribution_set_usage(stderr),
        Err(SetupValidationArgError::Invalid(message)) => {
            let _ = writeln!(stderr, "verify contribution-set failed: {message}");
            return 1;
        }
    };
    let bindings = match verify_requested_contribution_bindings(
        ContributionBindingRequest {
            role: "verify contribution-set",
            proof_bins: &parsed.proof_bins,
            public_values_path: parsed.public_values_path,
            eth_block_input: parsed.eth_block_input,
            eth_public_input: parsed.eth_public_input,
            eth_public_input_allow_trailing: parsed.eth_public_input_allow_trailing,
            program_image_cache: parsed.program_image_cache,
        },
        stderr,
    ) {
        Some(bindings) => bindings,
        None => return 1,
    };
    let proof_paths = parsed
        .proof_bins
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let report = match derive_global_challenge_from_embedded_contribution_proofs(
        parsed.setup_dir,
        parsed.public_values_path,
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
    write_contribution_binding_summary(stdout, &report, &bindings);
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
    eth_public_input: Option<&'a str>,
    eth_public_input_allow_trailing: bool,
    program_image_cache: Option<&'a str>,
}

fn verify_setup_validation(
    command: VerifySetupValidationCommand<'_>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let eth_block_input_binding = match verify_requested_eth_block_binding(
        command.role,
        command.proof_bin,
        command.public_values_path,
        command.eth_block_input,
        command.eth_public_input,
        command.eth_public_input_allow_trailing,
        stderr,
    ) {
        Some(binding) => binding,
        None => return 1,
    };
    let program_image_cache_matched = match verify_requested_program_image_cache_binding(
        command.role,
        command.proof_bin,
        command.program_image_cache,
        stderr,
    ) {
        Some(matched) => matched,
        None => return 1,
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
        write_eth_block_input_binding_summary(
            stdout,
            &public_report.eth_block_input_hashes,
            &binding,
        );
    }
    if program_image_cache_matched {
        let _ = writeln!(stdout, "program_image_cache_match=ok");
    }
    0
}

fn verify_requested_eth_block_binding(
    role: &str,
    proof_bin: &str,
    public_values_path: &str,
    eth_block_input: Option<&str>,
    eth_public_input: Option<&str>,
    eth_public_input_allow_trailing: bool,
    stderr: &mut dyn Write,
) -> Option<Option<EthBlockInputBinding>> {
    match (eth_block_input, eth_public_input) {
        (Some(path), None) => {
            match verify_eth_block_input_binding(proof_bin, public_values_path, path) {
                Ok(binding) => Some(Some(binding)),
                Err(message) => {
                    let _ = writeln!(stderr, "{role} failed: {message}");
                    None
                }
            }
        }
        (None, Some(path)) => match verify_eth_public_input_binding_with_mode(
            proof_bin,
            public_values_path,
            path,
            if eth_public_input_allow_trailing {
                EthPublicInputMode::AllowTrailing
            } else {
                EthPublicInputMode::Strict
            },
        ) {
            Ok(binding) => Some(Some(binding)),
            Err(message) => {
                let _ = writeln!(stderr, "{role} failed: {message}");
                None
            }
        },
        (None, None) => Some(None),
        (Some(_), Some(_)) => {
            let _ = writeln!(
                stderr,
                "{role} failed: cannot combine --eth-block-input and --eth-public-input"
            );
            None
        }
    }
}

fn verify_requested_program_image_cache_binding(
    role: &str,
    proof_bin: &str,
    program_image_cache: Option<&str>,
    stderr: &mut dyn Write,
) -> Option<bool> {
    let Some(path) = program_image_cache else {
        return Some(false);
    };
    match verify_program_image_cache_binding(proof_bin, path) {
        Ok(()) => Some(true),
        Err(message) => {
            let _ = writeln!(stderr, "{role} failed: {message}");
            None
        }
    }
}

pub(crate) fn verify_requested_contribution_bindings(
    request: ContributionBindingRequest<'_>,
    stderr: &mut dyn Write,
) -> Option<VerifiedContributionBindings> {
    let eth_block_input_binding = verify_requested_eth_block_bindings(
        request.role,
        request.proof_bins,
        request.public_values_path,
        request.eth_block_input,
        request.eth_public_input,
        request.eth_public_input_allow_trailing,
        stderr,
    )?;
    let program_image_cache_matched = verify_requested_program_image_cache_bindings(
        request.role,
        request.proof_bins,
        request.program_image_cache,
        stderr,
    )?;
    Some(VerifiedContributionBindings {
        eth_block_input_binding,
        program_image_cache_matched,
    })
}

fn verify_requested_eth_block_bindings(
    role: &str,
    proof_bins: &[&str],
    public_values_path: &str,
    eth_block_input: Option<&str>,
    eth_public_input: Option<&str>,
    eth_public_input_allow_trailing: bool,
    stderr: &mut dyn Write,
) -> Option<Option<EthBlockInputBinding>> {
    if eth_block_input.is_none() && eth_public_input.is_none() {
        return Some(None);
    }

    let mut first_binding = None;
    for proof_bin in proof_bins {
        let binding = match verify_requested_eth_block_binding(
            role,
            proof_bin,
            public_values_path,
            eth_block_input,
            eth_public_input,
            eth_public_input_allow_trailing,
            stderr,
        ) {
            Some(Some(binding)) => binding,
            Some(None) => return Some(None),
            None => return None,
        };
        if first_binding.is_none() {
            first_binding = Some(binding);
        }
    }
    Some(first_binding)
}

fn verify_requested_program_image_cache_bindings(
    role: &str,
    proof_bins: &[&str],
    program_image_cache: Option<&str>,
    stderr: &mut dyn Write,
) -> Option<bool> {
    if program_image_cache.is_none() {
        return Some(false);
    }
    for proof_bin in proof_bins {
        verify_requested_program_image_cache_binding(role, proof_bin, program_image_cache, stderr)?;
    }
    Some(true)
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

pub(crate) fn write_contribution_binding_summary(
    stdout: &mut dyn Write,
    report: &ContributionChallengeReport,
    bindings: &VerifiedContributionBindings,
) {
    let has_eth_block_input_binding = bindings.eth_block_input_binding.is_some();
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
        if has_eth_block_input_binding {
            for hash in &report.eth_block_input_hashes {
                let _ = writeln!(
                    stdout,
                    "eth_block_input_hash={}",
                    prove_plan::format_hash(hash)
                );
            }
        } else {
            for input in &report.eth_block_inputs {
                eth_block_output::write_contribution_eth_block_input(stdout, input);
            }
        }
    }
    if let Some(binding) = bindings.eth_block_input_binding.as_ref() {
        write_eth_block_input_binding_summary(stdout, &report.eth_block_input_hashes, binding);
    }
    if bindings.program_image_cache_matched {
        let _ = writeln!(stdout, "program_image_cache_match=ok");
    }
}

pub(super) fn write_verify_preflight_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify preflight [--eth-block-input <block-input>] [--eth-public-input <public-input>] [--eth-public-input-allow-trailing] [--program-image-cache <cache-bin>] <proof-bin> <public-values>"
    );
    2
}

pub(super) fn write_verify_setup_preflight_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify setup-preflight [--eth-block-input <block-input>] [--eth-public-input <public-input>] [--eth-public-input-allow-trailing] [--program-image-cache <cache-bin>] <setup-dir> <proof-bin> <public-values>"
    );
    2
}

fn write_verify_proof_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify proof [--eth-block-input <block-input>] [--eth-public-input <public-input>] [--eth-public-input-allow-trailing] [--program-image-cache <cache-bin>] <setup-dir> <proof-bin> <public-values>"
    );
    2
}

pub(super) fn write_verify_contribution_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify contribution [--eth-block-input <block-input>] [--eth-public-input <public-input>] [--eth-public-input-allow-trailing] [--program-image-cache <cache-bin>] <setup-dir> <proof-bin> <public-values>"
    );
    2
}

pub(super) fn write_verify_contribution_set_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify contribution-set [--eth-block-input <block-input>] [--eth-public-input <public-input>] [--eth-public-input-allow-trailing] [--program-image-cache <cache-bin>] <setup-dir> <public-values> <proof-bin> [proof-bin ...]"
    );
    2
}

#[cfg(test)]
mod tests;
