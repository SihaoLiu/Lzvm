use std::io::Write;
use std::path::{Path, PathBuf};

use lzvm_artifacts::challenge_values_segment::{
    encode_challenge_values_segment, parse_challenge_values_segment, ChallengeValuesSegment,
};
use lzvm_prover::contribution::derive_global_challenge_from_contribution_proofs;

use crate::prove_plan;
use crate::verify_commands;

struct ParsedWriteArgs<'a> {
    setup_dir: &'a str,
    public_values_path: &'a str,
    output_path: &'a str,
    proof_bins: Vec<&'a str>,
    eth_block_input: Option<&'a str>,
    eth_public_input: Option<&'a str>,
    eth_public_input_allow_trailing: bool,
    program_image_cache: Option<&'a str>,
    input_data: Option<&'a str>,
}

fn parse_write_args<'a>(
    args: &'a [&'a str],
) -> Result<ParsedWriteArgs<'a>, verify_commands::SetupValidationArgError> {
    let parsed = verify_commands::parse_binding_args(args)?;
    if parsed.positionals.len() < 4 {
        return Err(verify_commands::SetupValidationArgError::Usage);
    }
    verify_commands::validate_binding_args(&parsed)?;
    Ok(ParsedWriteArgs {
        setup_dir: parsed.positionals[0],
        public_values_path: parsed.positionals[1],
        output_path: parsed.positionals[2],
        proof_bins: parsed.positionals[3..].to_vec(),
        eth_block_input: parsed.eth_block_input,
        eth_public_input: parsed.eth_public_input,
        eth_public_input_allow_trailing: parsed.eth_public_input_allow_trailing,
        program_image_cache: parsed.program_image_cache,
        input_data: parsed.input_data,
    })
}

pub(crate) fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_write_args(args) {
        Ok(parsed) => parsed,
        Err(verify_commands::SetupValidationArgError::Usage) => return write_usage(stderr),
        Err(verify_commands::SetupValidationArgError::Invalid(message)) => {
            let _ = writeln!(
                stderr,
                "prove contribution challenges write failed: {message}"
            );
            return 1;
        }
    };
    run_parsed(parsed, stdout, stderr)
}

fn run_parsed(parsed: ParsedWriteArgs<'_>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let bindings = match verify_commands::verify_requested_contribution_bindings(
        verify_commands::ContributionBindingRequest {
            role: "prove contribution challenges write",
            proof_bins: &parsed.proof_bins,
            public_values_path: parsed.public_values_path,
            eth_block_input: parsed.eth_block_input,
            eth_public_input: parsed.eth_public_input,
            eth_public_input_allow_trailing: parsed.eth_public_input_allow_trailing,
            program_image_cache: parsed.program_image_cache,
            input_data: parsed.input_data,
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
    let report = match derive_global_challenge_from_contribution_proofs(
        parsed.setup_dir,
        parsed.public_values_path,
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

    let output_path = Path::new(parsed.output_path);
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
        "public_values_hash={}",
        prove_plan::format_hash(&report.public_values_hash)
    );
    let _ = writeln!(
        stdout,
        "public_value_fields={}",
        report.public_value_field_count
    );
    verify_commands::write_contribution_binding_summary(stdout, &report, &bindings);
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

struct ParsedVerifyArgs<'a> {
    setup_dir: &'a str,
    public_values_path: &'a str,
    challenge_values_segment_path: &'a str,
    proof_bins: Vec<&'a str>,
    eth_block_input: Option<&'a str>,
    eth_public_input: Option<&'a str>,
    eth_public_input_allow_trailing: bool,
    program_image_cache: Option<&'a str>,
    input_data: Option<&'a str>,
}

fn parse_verify_args<'a>(
    args: &'a [&'a str],
) -> Result<ParsedVerifyArgs<'a>, verify_commands::SetupValidationArgError> {
    let parsed = verify_commands::parse_binding_args(args)?;
    if parsed.positionals.len() < 4 {
        return Err(verify_commands::SetupValidationArgError::Usage);
    }
    verify_commands::validate_binding_args(&parsed)?;
    Ok(ParsedVerifyArgs {
        setup_dir: parsed.positionals[0],
        public_values_path: parsed.positionals[1],
        challenge_values_segment_path: parsed.positionals[2],
        proof_bins: parsed.positionals[3..].to_vec(),
        eth_block_input: parsed.eth_block_input,
        eth_public_input: parsed.eth_public_input,
        eth_public_input_allow_trailing: parsed.eth_public_input_allow_trailing,
        program_image_cache: parsed.program_image_cache,
        input_data: parsed.input_data,
    })
}

pub(crate) fn verify(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_verify_args(args) {
        Ok(parsed) => parsed,
        Err(verify_commands::SetupValidationArgError::Usage) => {
            return write_verify_usage(stderr);
        }
        Err(verify_commands::SetupValidationArgError::Invalid(message)) => {
            let _ = writeln!(stderr, "verify contribution-challenge failed: {message}");
            return 1;
        }
    };
    verify_parsed(parsed, stdout, stderr)
}

fn verify_parsed(
    parsed: ParsedVerifyArgs<'_>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let bindings = match verify_commands::verify_requested_contribution_bindings(
        verify_commands::ContributionBindingRequest {
            role: "verify contribution-challenge",
            proof_bins: &parsed.proof_bins,
            public_values_path: parsed.public_values_path,
            eth_block_input: parsed.eth_block_input,
            eth_public_input: parsed.eth_public_input,
            eth_public_input_allow_trailing: parsed.eth_public_input_allow_trailing,
            program_image_cache: parsed.program_image_cache,
            input_data: parsed.input_data,
        },
        stderr,
    ) {
        Some(bindings) => bindings,
        None => return 1,
    };
    verify_inner(
        parsed.setup_dir,
        parsed.public_values_path,
        parsed.challenge_values_segment_path,
        &parsed.proof_bins,
        &bindings,
        stdout,
        stderr,
    )
}

fn verify_inner(
    setup_dir: &str,
    public_values_path: &str,
    challenge_values_segment_path: &str,
    proof_bins: &[&str],
    bindings: &verify_commands::VerifiedContributionBindings,
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
            let _ = writeln!(stderr, "verify contribution-challenge failed: {error}");
            return 1;
        }
    };

    let challenge_bytes = match std::fs::read(challenge_values_segment_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "verify contribution-challenge failed: read challenge values segment failed: {}: {error}",
                challenge_values_segment_path
            );
            return 1;
        }
    };
    let challenge_values = match parse_challenge_values_segment(&challenge_bytes) {
        Ok(segment) => segment.values,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "verify contribution-challenge failed: parse challenge values segment failed: {error}"
            );
            return 1;
        }
    };
    let expected_challenge = [
        report.challenge.c0.to_u64(),
        report.challenge.c1.to_u64(),
        report.challenge.c2.to_u64(),
    ];
    if challenge_values.as_slice() != [expected_challenge] {
        let _ = writeln!(
            stderr,
            "verify contribution-challenge failed: contribution challenge values mismatch"
        );
        return 1;
    }

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
    verify_commands::write_contribution_binding_summary(stdout, &report, bindings);
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
    0
}

pub(crate) fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm prove write-contribution-challenges [--eth-block-input <block-input>] [--eth-public-input <public-input>] [--eth-public-input-allow-trailing] [--program-image-cache <cache-bin>] [--input-data <input>] <setup-dir> <public-values> <out-challenge-values-segment> <proof-bin> [proof-bin ...]"
    );
    2
}

pub(crate) fn write_verify_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify contribution-challenge [--eth-block-input <block-input>] [--eth-public-input <public-input>] [--eth-public-input-allow-trailing] [--program-image-cache <cache-bin>] [--input-data <input>] <setup-dir> <public-values> <challenge-values-segment> <proof-bin> [proof-bin ...]"
    );
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_proof_for_write_contribution_challenges_args() {
        let result = parse_write_args(&["setup", "public-values.bin", "challenge-values.bin"]);

        assert!(matches!(
            result,
            Err(verify_commands::SetupValidationArgError::Usage)
        ));
    }

    #[test]
    fn rejects_missing_proof_for_verify_contribution_challenge_args() {
        let result = parse_verify_args(&["setup", "public-values.bin", "challenge-values.bin"]);

        assert!(matches!(
            result,
            Err(verify_commands::SetupValidationArgError::Usage)
        ));
    }
}
