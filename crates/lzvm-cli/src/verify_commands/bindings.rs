use std::fs;
use std::io::{Read, Write};

use lzvm_artifacts::guest_input_segment::{
    validate_framed_guest_input_segment, FRAMED_GUEST_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::program_image::read_program_image_commitment_cache_file;
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};
use lzvm_prover::proof_preflight::read_checked_proof_artifact_file;

use super::eth_block_input::{
    verify_eth_block_input_binding, verify_eth_public_input_binding_with_mode, EthBlockInputBinding,
};
use super::SetupValidationArgError;
use crate::eth_block_prove_input::EthPublicInputMode;

pub(crate) struct ParsedBindingArgs<'a> {
    pub(crate) positionals: Vec<&'a str>,
    pub(crate) eth_block_input: Option<&'a str>,
    pub(crate) eth_public_input: Option<&'a str>,
    pub(crate) eth_public_input_allow_trailing: bool,
    pub(crate) program_image_cache: Option<&'a str>,
    pub(crate) input_data: Option<&'a str>,
}

pub(crate) struct VerifiedContributionBindings {
    pub(super) eth_block_input_binding: Option<EthBlockInputBinding>,
    pub(super) program_image_cache_matched: bool,
    pub(super) framed_guest_input_matched: bool,
}

pub(crate) struct ContributionBindingRequest<'a> {
    pub(crate) role: &'a str,
    pub(crate) proof_bins: &'a [&'a str],
    pub(crate) public_values_path: &'a str,
    pub(crate) eth_block_input: Option<&'a str>,
    pub(crate) eth_public_input: Option<&'a str>,
    pub(crate) eth_public_input_allow_trailing: bool,
    pub(crate) program_image_cache: Option<&'a str>,
    pub(crate) input_data: Option<&'a str>,
}

pub(crate) fn parse_binding_args<'a>(
    args: &'a [&'a str],
) -> Result<ParsedBindingArgs<'a>, SetupValidationArgError> {
    let mut eth_block_input = None;
    let mut eth_public_input = None;
    let mut eth_public_input_allow_trailing = false;
    let mut program_image_cache = None;
    let mut input_data = None;
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
            "--input-data" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    SetupValidationArgError::Invalid("missing --input-data value".to_owned())
                })?;
                if value.starts_with("--") {
                    return Err(SetupValidationArgError::Invalid(
                        "missing --input-data value".to_owned(),
                    ));
                }
                if input_data.replace(*value).is_some() {
                    return Err(SetupValidationArgError::Invalid(
                        "duplicate --input-data option".to_owned(),
                    ));
                }
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
        input_data,
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

pub(super) fn verify_requested_eth_block_binding(
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

pub(super) fn verify_requested_program_image_cache_binding(
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

pub(super) fn verify_requested_framed_guest_input_binding(
    role: &str,
    proof_bin: &str,
    input_data: Option<&str>,
    stderr: &mut dyn Write,
) -> Option<bool> {
    let Some(path) = input_data else {
        return Some(false);
    };
    match verify_framed_guest_input_binding(proof_bin, path) {
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
    let framed_guest_input_matched = verify_requested_framed_guest_input_bindings(
        request.role,
        request.proof_bins,
        request.input_data,
        stderr,
    )?;
    Some(VerifiedContributionBindings {
        eth_block_input_binding,
        program_image_cache_matched,
        framed_guest_input_matched,
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

fn verify_requested_framed_guest_input_bindings(
    role: &str,
    proof_bins: &[&str],
    input_data: Option<&str>,
    stderr: &mut dyn Write,
) -> Option<bool> {
    if input_data.is_none() {
        return Some(false);
    }
    for proof_bin in proof_bins {
        verify_requested_framed_guest_input_binding(role, proof_bin, input_data, stderr)?;
    }
    Some(true)
}

fn verify_program_image_cache_binding(proof_bin: &str, cache_path: &str) -> Result<(), String> {
    let proof = read_checked_proof_artifact_file(proof_bin)
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

fn verify_framed_guest_input_binding(proof_bin: &str, input_data_path: &str) -> Result<(), String> {
    let proof = read_checked_proof_artifact_file(proof_bin)
        .map_err(|error| format!("read proof artifact failed: {proof_bin}: {error}"))?;
    let input = fs::File::open(input_data_path)
        .map_err(|error| format!("read input data failed: {input_data_path}: {error}"))?;
    let mut segments = proof
        .segments
        .iter()
        .filter(|segment| segment.id == FRAMED_GUEST_INPUT_SEGMENT_ID);
    let segment = segments
        .next()
        .ok_or_else(|| "missing framed guest input proof segment".to_owned())?;
    if segments.next().is_some() {
        return Err("duplicate framed guest input proof segment".to_owned());
    }
    validate_framed_guest_input_segment(&segment.data)
        .map_err(|error| format!("framed guest input proof segment is invalid: {error}"))?;
    if !input_data_file_matches_segment(input, input_data_path, &segment.data)? {
        return Err("framed guest input proof segment mismatch".to_owned());
    }
    Ok(())
}

pub(super) fn input_data_file_matches_segment(
    mut input: fs::File,
    input_data_path: &str,
    segment_data: &[u8],
) -> Result<bool, String> {
    let input_len = input
        .metadata()
        .map_err(|error| format!("read input data failed: {input_data_path}: {error}"))?
        .len();
    let segment_len = u64::try_from(segment_data.len())
        .map_err(|_| "framed guest input proof segment is too large".to_owned())?;
    if input_len != segment_len {
        return Ok(false);
    }

    let mut offset = 0;
    let mut buffer = [0_u8; 8192];
    while offset < segment_data.len() {
        let chunk_len = buffer.len().min(segment_data.len() - offset);
        input
            .read_exact(&mut buffer[..chunk_len])
            .map_err(|error| format!("read input data failed: {input_data_path}: {error}"))?;
        if buffer[..chunk_len] != segment_data[offset..offset + chunk_len] {
            return Ok(false);
        }
        offset += chunk_len;
    }
    Ok(true)
}

pub(super) fn pipeline_input_bindings_matched(
    eth_block_input_binding: Option<&EthBlockInputBinding>,
    program_image_cache_matched: bool,
    framed_guest_input_matched: bool,
) -> bool {
    eth_block_input_binding.is_some() && program_image_cache_matched && framed_guest_input_matched
}
