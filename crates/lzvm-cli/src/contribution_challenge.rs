use std::io::Write;
use std::path::{Path, PathBuf};

use lzvm_artifacts::challenge_values_segment::{
    encode_challenge_values_segment, parse_challenge_values_segment, ChallengeValuesSegment,
};
use lzvm_prover::contribution::derive_global_challenge_from_contribution_proofs;

use crate::prove_plan;

pub(crate) fn run(
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
        "public_values_hash={}",
        prove_plan::format_hash(&report.public_values_hash)
    );
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

pub(crate) fn verify(
    setup_dir: &str,
    public_values_path: &str,
    challenge_values_segment_path: &str,
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
        "usage: lzvm prove write-contribution-challenges <setup-dir> <public-values> <out-challenge-values-segment> <proof-bin> [proof-bin ...]"
    );
    2
}

pub(crate) fn write_verify_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify contribution-challenge <setup-dir> <public-values> <challenge-values-segment> <proof-bin> [proof-bin ...]"
    );
    2
}
