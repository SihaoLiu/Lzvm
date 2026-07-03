use super::*;
use lzvm_artifacts::eth_block_input::{
    build_eth_block_input, encode_eth_block_input, eth_block_input_bytes_digest, EthBlockInput,
};
use lzvm_artifacts::eth_block_input_segment::{
    encode_eth_block_input_segment, ETH_BLOCK_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_public_values::public_values_from_eth_block_input;
use lzvm_artifacts::eth_public_input::parse_eth_public_block_prefix;
use lzvm_artifacts::guest_input_segment::FRAMED_GUEST_INPUT_SEGMENT_ID;
use lzvm_artifacts::program_image::{
    encode_program_image_commitment_cache, ProgramImageCommitmentCache, ProgramImageGpuMode,
};
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};
use lzvm_artifacts::proof::{encode_proof_artifact, ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{encode_public_values, public_values_digest};
use lzvm_artifacts::sectioned::{encode_sectioned_file, parse_sectioned_file};
use lzvm_prover::proof_preflight::TraceConstraintPreflightUnit;

fn test_fixture_dir(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!("lzvm-verify-{name}-{}", std::process::id()))
}

fn framed_input_bytes(payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes.resize(bytes.len().next_multiple_of(8), 0);
    bytes
}

fn sample_program_image_cache() -> ProgramImageCommitmentCache {
    ProgramImageCommitmentCache {
        program_digest: [0x11; 32],
        source_image_digest: [0x22; 32],
        constraint_system_digest: [0x33; 32],
        tree_root: [1, 2, 3, 4],
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
    }
}

#[test]
fn parses_eth_public_input_option_for_verify_proof_args() {
    let result = parse_verify_proof_args(&[
        "--eth-public-input",
        "public.bin",
        "setup",
        "proof.bin",
        "public-values.bin",
    ])
    .expect("verify args should parse");

    assert_eq!(result.eth_public_input, Some("public.bin"));
}

#[test]
fn parses_eth_public_input_allow_trailing_for_verify_proof_args() {
    let result = parse_verify_proof_args(&[
        "--eth-public-input",
        "public.bin",
        "--eth-public-input-allow-trailing",
        "setup",
        "proof.bin",
        "public-values.bin",
    ])
    .expect("verify args should parse");

    assert_eq!(result.eth_public_input, Some("public.bin"));
    assert!(result.eth_public_input_allow_trailing);
}

#[test]
fn parses_binding_options_for_verify_preflight_args() {
    let result = parse_verify_preflight_args(&[
        "--eth-public-input",
        "public.bin",
        "--eth-public-input-allow-trailing",
        "--program-image-cache",
        "cache.bin",
        "--input-data",
        "input.bin",
        "proof.bin",
        "public-values.bin",
    ])
    .expect("verify preflight args should parse");

    assert_eq!(result.proof_bin, "proof.bin");
    assert_eq!(result.public_values_path, "public-values.bin");
    assert_eq!(result.eth_public_input, Some("public.bin"));
    assert!(result.eth_public_input_allow_trailing);
    assert_eq!(result.program_image_cache, Some("cache.bin"));
    assert_eq!(result.input_data, Some("input.bin"));
}

#[test]
fn parses_binding_options_for_verify_contribution_args() {
    let result = parse_verify_contribution_args(&[
        "--eth-block-input",
        "block.input",
        "--program-image-cache",
        "cache.bin",
        "--input-data",
        "input.bin",
        "setup",
        "proof.bin",
        "public-values.bin",
    ])
    .expect("verify contribution args should parse");

    assert_eq!(result.setup_dir, "setup");
    assert_eq!(result.proof_bin, "proof.bin");
    assert_eq!(result.public_values_path, "public-values.bin");
    assert_eq!(result.eth_block_input, Some("block.input"));
    assert_eq!(result.program_image_cache, Some("cache.bin"));
    assert_eq!(result.input_data, Some("input.bin"));
}

#[test]
fn parses_eth_public_input_options_for_verify_contribution_args() {
    let result = parse_verify_contribution_args(&[
        "--eth-public-input",
        "public.bin",
        "--eth-public-input-allow-trailing",
        "setup",
        "proof.bin",
        "public-values.bin",
    ])
    .expect("verify contribution args should parse");

    assert_eq!(result.setup_dir, "setup");
    assert_eq!(result.proof_bin, "proof.bin");
    assert_eq!(result.public_values_path, "public-values.bin");
    assert_eq!(result.eth_public_input, Some("public.bin"));
    assert!(result.eth_public_input_allow_trailing);
}

#[test]
fn parses_binding_options_for_verify_contribution_set_args() {
    let result = parse_verify_contribution_set_args(&[
        "--eth-block-input",
        "block.input",
        "--program-image-cache",
        "cache.bin",
        "--input-data",
        "input.bin",
        "setup",
        "public-values.bin",
        "proof-a.bin",
        "proof-b.bin",
    ])
    .expect("verify contribution-set args should parse");

    assert_eq!(result.setup_dir, "setup");
    assert_eq!(result.public_values_path, "public-values.bin");
    assert_eq!(result.proof_bins, vec!["proof-a.bin", "proof-b.bin"]);
    assert_eq!(result.eth_block_input, Some("block.input"));
    assert_eq!(result.program_image_cache, Some("cache.bin"));
    assert_eq!(result.input_data, Some("input.bin"));
}

#[test]
fn rejects_conflicting_binding_options_for_verify_contribution_set_args() {
    let result = parse_verify_contribution_set_args(&[
        "--eth-block-input",
        "block.input",
        "--eth-public-input",
        "public.bin",
        "setup",
        "public-values.bin",
        "proof.bin",
    ]);

    assert!(matches!(
        result,
        Err(SetupValidationArgError::Invalid(message))
            if message == "cannot combine --eth-block-input and --eth-public-input"
    ));
}

#[test]
fn rejects_eth_public_input_allow_trailing_without_eth_public_input_for_verify_proof_args() {
    let result = parse_verify_proof_args(&[
        "--eth-public-input-allow-trailing",
        "setup",
        "proof.bin",
        "public-values.bin",
    ]);

    assert!(matches!(
        result,
        Err(SetupValidationArgError::Invalid(message))
            if message == "cannot use --eth-public-input-allow-trailing without --eth-public-input"
    ));
}

#[test]
fn rejects_missing_eth_public_input_value_for_verify_preflight_args() {
    let result = parse_verify_preflight_args(&[
        "--eth-public-input",
        "--program-image-cache",
        "cache.bin",
        "proof.bin",
        "public-values.bin",
    ]);

    assert!(matches!(
        result,
        Err(SetupValidationArgError::Invalid(message)) if message == "missing --eth-public-input value"
    ));
}

#[test]
fn rejects_missing_input_data_value_for_verify_preflight_args() {
    let result = parse_verify_preflight_args(&[
        "--input-data",
        "--program-image-cache",
        "cache.bin",
        "proof.bin",
        "public-values.bin",
    ]);

    assert!(matches!(
        result,
        Err(SetupValidationArgError::Invalid(message)) if message == "missing --input-data value"
    ));
}

#[test]
fn rejects_duplicate_binding_options_for_verify_preflight_args() {
    for (args, expected) in [
        (
            &[
                "--eth-block-input",
                "block-a.input",
                "--eth-block-input",
                "block-b.input",
                "proof.bin",
                "public-values.bin",
            ][..],
            "duplicate --eth-block-input option",
        ),
        (
            &[
                "--eth-public-input",
                "public-a.bin",
                "--eth-public-input",
                "public-b.bin",
                "proof.bin",
                "public-values.bin",
            ],
            "duplicate --eth-public-input option",
        ),
        (
            &[
                "--eth-public-input",
                "public.bin",
                "--eth-public-input-allow-trailing",
                "--eth-public-input-allow-trailing",
                "proof.bin",
                "public-values.bin",
            ],
            "duplicate --eth-public-input-allow-trailing option",
        ),
        (
            &[
                "--program-image-cache",
                "cache-a.bin",
                "--program-image-cache",
                "cache-b.bin",
                "proof.bin",
                "public-values.bin",
            ],
            "duplicate --program-image-cache option",
        ),
        (
            &[
                "--input-data",
                "input-a.bin",
                "--input-data",
                "input-b.bin",
                "proof.bin",
                "public-values.bin",
            ],
            "duplicate --input-data option",
        ),
    ] {
        let result = parse_verify_preflight_args(args);

        assert!(matches!(
            result,
            Err(SetupValidationArgError::Invalid(message)) if message == expected
        ));
    }
}

#[test]
fn input_data_file_matches_framed_guest_segment_bytes() {
    let dir = test_fixture_dir("framed-input-match");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let input_path = dir.join("input.bin");
    let input_data = framed_input_bytes(&[3, 5, 8, 13]);
    std::fs::write(&input_path, &input_data).expect("input data should write");

    let matched = input_data_file_matches_segment(
        std::fs::File::open(&input_path).expect("input data should open"),
        input_path.to_str().expect("input path should be utf-8"),
        &input_data,
    )
    .expect("input data comparison should succeed");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matched, "matching framed input bytes should compare equal");
}

#[test]
fn input_data_file_mismatch_detects_framed_guest_segment_payload_change() {
    let dir = test_fixture_dir("framed-input-payload-mismatch");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let input_path = dir.join("input.bin");
    let input_data = framed_input_bytes(&[3, 5, 8, 13]);
    let segment_data = framed_input_bytes(&[3, 5, 8, 21]);
    std::fs::write(&input_path, &input_data).expect("input data should write");

    let matched = input_data_file_matches_segment(
        std::fs::File::open(&input_path).expect("input data should open"),
        input_path.to_str().expect("input path should be utf-8"),
        &segment_data,
    )
    .expect("input data comparison should succeed");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(
        !matched,
        "framed input payload changes should be reported as a mismatch"
    );
}

#[test]
fn input_data_file_mismatch_detects_framed_guest_segment_length_change() {
    let dir = test_fixture_dir("framed-input-length-mismatch");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let input_path = dir.join("input.bin");
    let input_data = framed_input_bytes(&[3, 5, 8, 13]);
    let segment_data = framed_input_bytes(&[3, 5, 8, 13, 21, 34, 55, 89, 144]);
    std::fs::write(&input_path, &input_data).expect("input data should write");

    let matched = input_data_file_matches_segment(
        std::fs::File::open(&input_path).expect("input data should open"),
        input_path.to_str().expect("input path should be utf-8"),
        &segment_data,
    )
    .expect("input data comparison should succeed");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(
        !matched,
        "framed input length changes should be reported as a mismatch"
    );
}

#[test]
fn writes_trace_constraint_summary() {
    let mut output = Vec::new();
    write_trace_constraint_summary(
        &mut output,
        1,
        &[36],
        &[TraceConstraintPreflightUnit {
            unit_index: 3,
            trace_instance_index: 2,
            trace_row_count: 1024,
            trace_column_count: 9,
            regular_constraint_count: 17,
            trace_extracted: true,
            regular_constraints_evaluated: true,
            witness_values_committed: true,
            constraint_checker_conformant: true,
        }],
    );

    let output = String::from_utf8(output).expect("summary should be utf-8");
    assert!(output.contains("trace_constraint_segments=1\n"));
    assert!(output.contains("trace_constraint_segment_bytes=36\n"));
    assert!(output.contains("trace_constraint_units=1\n"));
    assert!(output.contains("trace_constraint_semantic_evidence_units=1\n"));
    assert!(output.contains("trace_constraint_semantic_evidence_complete=1\n"));
    assert!(output.contains("trace_constraint_unit=3,2,1024,9,17\n"));
    assert!(output.contains("trace_constraint_unit_flags=1,1,1,1\n"));
}

#[test]
fn trace_constraint_summary_reports_incomplete_semantic_evidence() {
    let mut output = Vec::new();
    write_trace_constraint_summary(
        &mut output,
        1,
        &[36],
        &[TraceConstraintPreflightUnit {
            unit_index: 0,
            trace_instance_index: 0,
            trace_row_count: 16,
            trace_column_count: 3,
            regular_constraint_count: 5,
            trace_extracted: true,
            regular_constraints_evaluated: true,
            witness_values_committed: false,
            constraint_checker_conformant: true,
        }],
    );

    let output = String::from_utf8(output).expect("summary should be utf-8");
    assert!(output.contains("trace_constraint_semantic_evidence_units=0\n"));
    assert!(output.contains("trace_constraint_semantic_evidence_complete=0\n"));
    assert!(output.contains("trace_constraint_unit_flags=1,1,0,1\n"));
}

#[test]
fn rejects_combined_eth_block_and_public_input_options() {
    let result = parse_verify_proof_args(&[
        "--eth-block-input",
        "block.input",
        "--eth-public-input",
        "public.bin",
        "setup",
        "proof.bin",
        "public-values.bin",
    ]);

    assert!(matches!(
        result,
        Err(SetupValidationArgError::Invalid(message))
            if message == "cannot combine --eth-block-input and --eth-public-input"
    ));
}

#[test]
fn rejects_missing_eth_public_input_value_during_parse() {
    let result = parse_verify_proof_args(&[
        "--eth-public-input",
        "--program-image-cache",
        "cache.bin",
        "setup",
        "proof.bin",
        "public-values.bin",
    ]);

    assert!(matches!(
        result,
        Err(SetupValidationArgError::Invalid(message)) if message == "missing --eth-public-input value"
    ));
}

#[test]
fn verifies_eth_public_input_against_embedded_block_input_segment() {
    let dir = test_fixture_dir("proof-eth-public");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let public_input_path = dir.join("public.bin");
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public-values.bin");
    let public_input = sample_public_block_bytes_with_matching_roots();
    std::fs::write(&public_input_path, &public_input).expect("public input should write");
    let block_input = block_input_from_public_bytes(&public_input);
    let block_input_bytes = encode_eth_block_input(&block_input).expect("input should encode");
    let setup_hash = [7; 32];
    let public_values = public_values_from_eth_block_input(setup_hash, &block_input);
    std::fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should write");
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![ProofSegment {
            id: ETH_BLOCK_INPUT_SEGMENT_ID,
            data: encode_eth_block_input_segment(&block_input).expect("segment should encode"),
        }],
    };
    std::fs::write(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    )
    .expect("proof should write");

    let binding = eth_block_input::verify_eth_public_input_binding_with_mode(
        proof_path.to_str().expect("proof path should be utf-8"),
        public_values_path
            .to_str()
            .expect("public values path should be utf-8"),
        public_input_path
            .to_str()
            .expect("public input path should be utf-8"),
        crate::eth_block_prove_input::EthPublicInputMode::Strict,
    )
    .expect("public input should match proof");

    assert_eq!(binding.bytes, block_input_bytes.len());
    assert_eq!(binding.block_hash, block_input.block_hash);
    assert_eq!(binding.transaction_preimage_count, 1);
    assert_eq!(binding.withdrawal_count, Some(1));
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn reports_embedded_block_input_segment_hash_for_reordered_input_file() {
    let dir = test_fixture_dir("proof-eth-block-reordered");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let input_path = dir.join("block.input");
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public-values.bin");
    let public_input = sample_public_block_bytes_with_matching_roots();
    let block_input = block_input_from_public_bytes(&public_input);
    let canonical_input_bytes = encode_eth_block_input(&block_input).expect("input should encode");
    let reordered_input_bytes = reordered_eth_block_input_file(&canonical_input_bytes);
    assert_ne!(reordered_input_bytes, canonical_input_bytes);
    std::fs::write(&input_path, reordered_input_bytes).expect("block input should write");
    let setup_hash = [7; 32];
    let public_values = public_values_from_eth_block_input(setup_hash, &block_input);
    std::fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should write");
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![ProofSegment {
            id: ETH_BLOCK_INPUT_SEGMENT_ID,
            data: canonical_input_bytes.clone(),
        }],
    };
    std::fs::write(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    )
    .expect("proof should write");

    let binding = eth_block_input::verify_eth_block_input_binding(
        proof_path.to_str().expect("proof path should be utf-8"),
        public_values_path
            .to_str()
            .expect("public values path should be utf-8"),
        input_path.to_str().expect("input path should be utf-8"),
    )
    .expect("reordered input file should match proof semantically");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        binding.hash,
        eth_block_input_bytes_digest(&canonical_input_bytes)
    );
    assert_eq!(binding.bytes, canonical_input_bytes.len());
}

#[test]
fn reports_embedded_block_input_segment_hash_for_reordered_proof_segment() {
    let dir = test_fixture_dir("proof-eth-proof-reordered");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let input_path = dir.join("block.input");
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public-values.bin");
    let public_input = sample_public_block_bytes_with_matching_roots();
    let block_input = block_input_from_public_bytes(&public_input);
    let canonical_input_bytes = encode_eth_block_input(&block_input).expect("input should encode");
    let reordered_segment = reordered_eth_block_input_file(&canonical_input_bytes);
    assert_ne!(reordered_segment, canonical_input_bytes);
    std::fs::write(&input_path, &canonical_input_bytes).expect("block input should write");
    let setup_hash = [7; 32];
    let public_values = public_values_from_eth_block_input(setup_hash, &block_input);
    std::fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should write");
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![ProofSegment {
            id: ETH_BLOCK_INPUT_SEGMENT_ID,
            data: reordered_segment.clone(),
        }],
    };
    std::fs::write(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    )
    .expect("proof should write");

    let binding = eth_block_input::verify_eth_block_input_binding(
        proof_path.to_str().expect("proof path should be utf-8"),
        public_values_path
            .to_str()
            .expect("public values path should be utf-8"),
        input_path.to_str().expect("input path should be utf-8"),
    )
    .expect("reordered proof segment should match input semantically");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        binding.hash,
        eth_block_input_bytes_digest(&reordered_segment)
    );
    assert_eq!(binding.bytes, reordered_segment.len());
}

#[test]
fn rejects_block_input_binding_when_proof_public_values_hash_differs() {
    let dir = test_fixture_dir("proof-eth-block-public-hash");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let input_path = dir.join("block.input");
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public-values.bin");
    let public_input = sample_public_block_bytes_with_matching_roots();
    let block_input = block_input_from_public_bytes(&public_input);
    let block_input_bytes = encode_eth_block_input(&block_input).expect("input should encode");
    std::fs::write(&input_path, &block_input_bytes).expect("block input should write");
    let setup_hash = [7; 32];
    let public_values = public_values_from_eth_block_input(setup_hash, &block_input);
    std::fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should write");
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: [0x55; 32],
        segments: vec![ProofSegment {
            id: ETH_BLOCK_INPUT_SEGMENT_ID,
            data: block_input_bytes,
        }],
    };
    std::fs::write(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    )
    .expect("proof should write");

    let result = eth_block_input::verify_eth_block_input_binding(
        proof_path.to_str().expect("proof path should be utf-8"),
        public_values_path
            .to_str()
            .expect("public values path should be utf-8"),
        input_path.to_str().expect("input path should be utf-8"),
    );
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(message) if message == "public-values hash mismatch"
    ));
}

#[test]
fn rejects_block_input_binding_when_proof_has_unexpected_segment_id() {
    let dir = test_fixture_dir("proof-eth-block-unexpected-segment");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let input_path = dir.join("block.input");
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public-values.bin");
    let public_input = sample_public_block_bytes_with_matching_roots();
    let block_input = block_input_from_public_bytes(&public_input);
    let block_input_bytes = encode_eth_block_input(&block_input).expect("input should encode");
    std::fs::write(&input_path, &block_input_bytes).expect("block input should write");
    let setup_hash = [7; 32];
    let public_values = public_values_from_eth_block_input(setup_hash, &block_input);
    std::fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should write");
    let unexpected_segment_id = 20_000;
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            ProofSegment {
                id: ETH_BLOCK_INPUT_SEGMENT_ID,
                data: block_input_bytes,
            },
            ProofSegment {
                id: unexpected_segment_id,
                data: vec![1],
            },
        ],
    };
    std::fs::write(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    )
    .expect("proof should write");

    let result = eth_block_input::verify_eth_block_input_binding(
        proof_path.to_str().expect("proof path should be utf-8"),
        public_values_path
            .to_str()
            .expect("public values path should be utf-8"),
        input_path.to_str().expect("input path should be utf-8"),
    );
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(message)
            if message.ends_with(&format!("unexpected proof segment id: {unexpected_segment_id}"))
    ));
}

#[test]
fn rejects_program_image_cache_binding_when_proof_has_unexpected_segment_id() {
    let dir = test_fixture_dir("program-image-cache-unexpected-segment");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let cache_path = dir.join("cache.bin");
    let proof_path = dir.join("proof.bin");
    let cache = sample_program_image_cache();
    std::fs::write(
        &cache_path,
        encode_program_image_commitment_cache(&cache).expect("cache should encode"),
    )
    .expect("cache should write");
    let unexpected_segment_id = 20_000;
    let proof = ProofArtifact {
        setup_hash: [7; 32],
        public_values_hash: [8; 32],
        segments: vec![
            ProofSegment {
                id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
                data: encode_program_image_cache_segment(&cache)
                    .expect("program image cache segment should encode"),
            },
            ProofSegment {
                id: unexpected_segment_id,
                data: vec![1],
            },
        ],
    };
    std::fs::write(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    )
    .expect("proof should write");

    let mut stderr = Vec::new();
    let result = verify_requested_program_image_cache_binding(
        "verify preflight",
        proof_path.to_str().expect("proof path should be utf-8"),
        Some(cache_path.to_str().expect("cache path should be utf-8")),
        &mut stderr,
    );
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(result, None);
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains(&format!(
            "unexpected proof segment id: {unexpected_segment_id}"
        )));
}

#[test]
fn rejects_framed_guest_input_binding_when_proof_has_unexpected_segment_id() {
    let dir = test_fixture_dir("framed-input-unexpected-segment");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let input_path = dir.join("input.bin");
    let proof_path = dir.join("proof.bin");
    let input_data = framed_input_bytes(&[3, 5, 8, 13]);
    std::fs::write(&input_path, &input_data).expect("input data should write");
    let unexpected_segment_id = 20_000;
    let proof = ProofArtifact {
        setup_hash: [7; 32],
        public_values_hash: [8; 32],
        segments: vec![
            ProofSegment {
                id: FRAMED_GUEST_INPUT_SEGMENT_ID,
                data: input_data,
            },
            ProofSegment {
                id: unexpected_segment_id,
                data: vec![1],
            },
        ],
    };
    std::fs::write(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    )
    .expect("proof should write");

    let mut stderr = Vec::new();
    let result = verify_requested_framed_guest_input_binding(
        "verify preflight",
        proof_path.to_str().expect("proof path should be utf-8"),
        Some(input_path.to_str().expect("input path should be utf-8")),
        &mut stderr,
    );
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(result, None);
    assert!(String::from_utf8(stderr)
        .expect("stderr should be utf-8")
        .contains(&format!(
            "unexpected proof segment id: {unexpected_segment_id}"
        )));
}

#[test]
fn rejects_public_input_binding_when_proof_public_values_hash_differs() {
    let dir = test_fixture_dir("proof-eth-public-hash");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let public_input_path = dir.join("public.bin");
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public-values.bin");
    let public_input = sample_public_block_bytes_with_matching_roots();
    std::fs::write(&public_input_path, &public_input).expect("public input should write");
    let block_input = block_input_from_public_bytes(&public_input);
    let block_input_bytes = encode_eth_block_input(&block_input).expect("input should encode");
    let setup_hash = [7; 32];
    let public_values = public_values_from_eth_block_input(setup_hash, &block_input);
    std::fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should write");
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: [0x55; 32],
        segments: vec![ProofSegment {
            id: ETH_BLOCK_INPUT_SEGMENT_ID,
            data: block_input_bytes,
        }],
    };
    std::fs::write(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    )
    .expect("proof should write");

    let result = eth_block_input::verify_eth_public_input_binding_with_mode(
        proof_path.to_str().expect("proof path should be utf-8"),
        public_values_path
            .to_str()
            .expect("public values path should be utf-8"),
        public_input_path
            .to_str()
            .expect("public input path should be utf-8"),
        crate::eth_block_prove_input::EthPublicInputMode::Strict,
    );
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(message) if message == "public-values hash mismatch"
    ));
}

#[test]
fn rejects_eth_public_input_with_trailing_bytes_for_verify_binding() {
    let dir = test_fixture_dir("proof-eth-public-trailing");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let public_input_path = dir.join("public.bin");
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public-values.bin");
    let public_input = sample_public_block_bytes_with_matching_roots();
    let block_input = block_input_from_public_bytes(&public_input);
    let setup_hash = [7; 32];
    let public_values = public_values_from_eth_block_input(setup_hash, &block_input);
    let mut public_input_with_tail = public_input;
    public_input_with_tail.extend_from_slice(b"tail");
    std::fs::write(&public_input_path, public_input_with_tail).expect("public input should write");
    std::fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should write");
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![ProofSegment {
            id: ETH_BLOCK_INPUT_SEGMENT_ID,
            data: encode_eth_block_input_segment(&block_input).expect("segment should encode"),
        }],
    };
    std::fs::write(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    )
    .expect("proof should write");

    let result = eth_block_input::verify_eth_public_input_binding_with_mode(
        proof_path.to_str().expect("proof path should be utf-8"),
        public_values_path
            .to_str()
            .expect("public values path should be utf-8"),
        public_input_path
            .to_str()
            .expect("public input path should be utf-8"),
        crate::eth_block_prove_input::EthPublicInputMode::Strict,
    );
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(message)
            if message
                == format!(
                    "ETH public input failed: {}: unexpected trailing bytes in ETH public input: 4",
                    public_input_path.display()
                )
    ));
}

#[test]
fn verifies_allowed_trailing_eth_public_input_against_embedded_block_input_segment() {
    let dir = test_fixture_dir("proof-eth-public-allow-trailing");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let public_input_path = dir.join("public.bin");
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public-values.bin");
    let public_input = sample_public_block_bytes_with_matching_roots();
    let block_input = block_input_from_public_bytes(&public_input);
    let block_input_bytes = encode_eth_block_input(&block_input).expect("input should encode");
    let setup_hash = [7; 32];
    let public_values = public_values_from_eth_block_input(setup_hash, &block_input);
    let mut public_input_with_tail = public_input;
    public_input_with_tail.extend_from_slice(b"tail");
    std::fs::write(&public_input_path, public_input_with_tail).expect("public input should write");
    std::fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should write");
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![ProofSegment {
            id: ETH_BLOCK_INPUT_SEGMENT_ID,
            data: encode_eth_block_input_segment(&block_input).expect("segment should encode"),
        }],
    };
    std::fs::write(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    )
    .expect("proof should write");

    let binding = eth_block_input::verify_eth_public_input_binding_with_mode(
        proof_path.to_str().expect("proof path should be utf-8"),
        public_values_path
            .to_str()
            .expect("public values path should be utf-8"),
        public_input_path
            .to_str()
            .expect("public input path should be utf-8"),
        crate::eth_block_prove_input::EthPublicInputMode::AllowTrailing,
    )
    .expect("public input should match proof");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(binding.bytes, block_input_bytes.len());
    assert_eq!(binding.block_hash, block_input.block_hash);
    assert_eq!(binding.transaction_preimage_count, 1);
    assert_eq!(binding.withdrawal_count, Some(1));
}

fn block_input_from_public_bytes(bytes: &[u8]) -> EthBlockInput {
    let public_block = parse_eth_public_block_prefix(bytes).expect("block should parse");
    let block_rlp = public_block.block_rlp();
    build_eth_block_input(&block_rlp).expect("block input should build")
}

fn reordered_eth_block_input_file(bytes: &[u8]) -> Vec<u8> {
    let mut file =
        parse_sectioned_file(bytes, *b"ethi", 1).expect("ETH block input should parse as sections");
    let first = file.sections.remove(0);
    file.sections.push(first);
    encode_sectioned_file(&file).expect("reordered ETH block input should encode")
}

fn sample_public_block_bytes_with_matching_roots() -> Vec<u8> {
    let mut input = sample_public_header_bytes();
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&eip1559_transaction_bytes());
    input.extend_from_slice(&0_u64.to_le_bytes());
    input.push(1);
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&withdrawal_bytes());

    let parsed = parse_eth_public_block_prefix(&input).expect("block should parse");
    let transaction_root = parsed.transactions_root();
    let ommers_hash = parsed.ommers_hash();
    let withdrawal_root = parsed
        .withdrawals_root()
        .expect("withdrawals root should be present");
    input[48..80].copy_from_slice(&ommers_hash);
    input[156..188].copy_from_slice(&transaction_root);
    input[237..269].copy_from_slice(&withdrawal_root);
    input
}

fn sample_public_header_bytes() -> Vec<u8> {
    let mut input = Vec::new();
    push_public_bytes(&mut input, &[1; 32]);
    push_public_bytes(&mut input, &[2; 32]);
    push_public_bytes(&mut input, &[3; 20]);
    push_public_bytes(&mut input, &[4; 32]);
    push_public_bytes(&mut input, &[5; 32]);
    push_public_bytes(&mut input, &[6; 32]);
    push_public_option_bytes(&mut input, Some(&[7; 32]));
    push_public_bytes(&mut input, &[8; 256]);
    push_public_bytes(&mut input, &u256_bytes(9));
    input.extend_from_slice(&42_u64.to_le_bytes());
    input.extend_from_slice(&100_u64.to_le_bytes());
    input.extend_from_slice(&90_u64.to_le_bytes());
    input.extend_from_slice(&77_u64.to_le_bytes());
    push_public_bytes(&mut input, &[10; 32]);
    push_public_bytes(&mut input, &[11; 8]);
    push_public_option_u64(&mut input, Some(123));
    push_public_option_u64(&mut input, Some(456));
    push_public_option_u64(&mut input, Some(789));
    push_public_option_bytes(&mut input, Some(&[12; 32]));
    push_public_option_bytes(&mut input, Some(&[13; 32]));
    push_public_bytes(&mut input, b"abc");
    input
}

fn eip1559_transaction_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_public_u256(&mut bytes, 0x11);
    push_public_u256(&mut bytes, 0x22);
    push_public_uint_u64(&mut bytes, 1);
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&7_u64.to_le_bytes());
    bytes.extend_from_slice(&21_000_u64.to_le_bytes());
    bytes.extend_from_slice(&300_u128.to_le_bytes());
    bytes.extend_from_slice(&20_u128.to_le_bytes());
    push_public_option_bytes(&mut bytes, Some(&[9; 20]));
    push_public_u256(&mut bytes, 123);
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    push_public_bytes(&mut bytes, b"call-data");
    bytes
}

fn withdrawal_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_public_uint_u64(&mut bytes, 7);
    push_public_uint_u64(&mut bytes, 8);
    push_public_bytes(&mut bytes, &[6; 20]);
    push_public_uint_u64(&mut bytes, 9);
    bytes
}

fn push_public_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_public_option_bytes(out: &mut Vec<u8>, bytes: Option<&[u8]>) {
    match bytes {
        Some(bytes) => {
            out.push(1);
            push_public_bytes(out, bytes);
        }
        None => out.push(0),
    }
}

fn push_public_option_u64(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            out.push(1);
            out.extend_from_slice(&value.to_le_bytes());
        }
        None => out.push(0),
    }
}

fn push_public_u256(out: &mut Vec<u8>, value: u8) {
    let mut bytes = [0; 32];
    bytes[31] = value;
    push_public_bytes(out, &bytes);
}

fn push_public_uint_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&8_u64.to_le_bytes());
    out.extend_from_slice(&value.to_be_bytes());
}

fn u256_bytes(value: u8) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[31] = value;
    bytes
}
