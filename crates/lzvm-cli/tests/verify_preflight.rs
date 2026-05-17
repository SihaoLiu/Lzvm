use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::eth_block_input::{build_eth_block_input, eth_block_input_bytes_digest};
use lzvm_artifacts::eth_block_input_segment::{
    encode_eth_block_input_segment, ETH_BLOCK_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_public_values::public_values_from_eth_block_input;
use lzvm_artifacts::program_image::{ProgramImageCommitmentCache, ProgramImageGpuMode};
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};
use lzvm_artifacts::proof::{encode_proof_artifact, ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{
    encode_public_values, public_values_digest, PublicValueEntry, PublicValues,
};
use lzvm_cli::run_cli;
use lzvm_field::MODULUS;
use lzvm_prover::proof_preflight::validate_proof_public_values_from_files;

fn sample_hash(byte: u8) -> [u8; 32] {
    [byte; 32]
}

fn to_hex(hash: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in hash {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-verify-preflight-{}-{name}",
        std::process::id()
    ))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, bytes).expect("fixture should be written");
}

fn sample_public_values() -> PublicValues {
    PublicValues {
        schema_version: 1,
        setup_hash: sample_hash(0x44),
        values: vec![
            PublicValueEntry {
                name: "block_number".to_owned(),
                elements: vec![12_345],
            },
            PublicValueEntry {
                name: "state_root_words".to_owned(),
                elements: vec![1, 2, 3, 4],
            },
        ],
    }
}

fn sample_proof(public_values: &PublicValues) -> ProofArtifact {
    ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(public_values).expect("digest should compute"),
        segments: vec![ProofSegment {
            id: 100,
            data: vec![1, 2, 3, 4],
        }],
    }
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

fn sample_block_rlp() -> Vec<u8> {
    let header_rlp = rlp_list(&legacy_header_items(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        None,
    ));
    let transactions = rlp_list(&[rlp_list(&[rlp_bytes(&[1])])]);
    let empty_list = rlp_list(&[]);
    rlp_list(&[header_rlp, transactions, empty_list])
}

fn legacy_header_items(
    transactions_root: [u8; 32],
    withdrawals_root: Option<[u8; 32]>,
) -> Vec<Vec<u8>> {
    let mut items = vec![
        rlp_bytes(&[0x11; 32]),
        rlp_bytes(&hex32(
            "1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
        )),
        rlp_bytes(&[0x33; 20]),
        rlp_bytes(&[0x44; 32]),
        rlp_bytes(&transactions_root),
        rlp_bytes(&[0x66; 32]),
        rlp_bytes(&[0x77; 256]),
        rlp_bytes(&[1]),
        rlp_bytes(&[2]),
        rlp_bytes(&[0x0f, 0x42, 0x40]),
        rlp_bytes(&[0x0d, 0xbb, 0xa0]),
        rlp_bytes(&[0x65]),
        rlp_bytes(b"lzvm"),
        rlp_bytes(&[0xaa; 32]),
        rlp_bytes(&[0xbb; 8]),
    ];
    if let Some(root) = withdrawals_root {
        items.push(rlp_bytes(&[1]));
        items.push(rlp_bytes(&root));
    }
    items
}

fn rlp_bytes(payload: &[u8]) -> Vec<u8> {
    if payload.len() == 1 && payload[0] <= 0x7f {
        return vec![payload[0]];
    }
    rlp_with_payload(0x80, 0xb7, payload)
}

fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload = items.iter().flatten().copied().collect::<Vec<_>>();
    rlp_with_payload(0xc0, 0xf7, &payload)
}

fn rlp_with_payload(short_base: u8, long_base: u8, payload: &[u8]) -> Vec<u8> {
    if payload.len() <= 55 {
        let mut output = vec![short_base + payload.len() as u8];
        output.extend_from_slice(payload);
        return output;
    }

    let length = length_bytes(payload.len());
    let mut output = vec![long_base + length.len() as u8];
    output.extend_from_slice(&length);
    output.extend_from_slice(payload);
    output
}

fn length_bytes(mut value: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    while value > 0 {
        bytes.push((value & 0xff) as u8);
        value >>= 8;
    }
    bytes.reverse();
    bytes
}

fn hex32(value: &str) -> [u8; 32] {
    let bytes = hex_bytes(value);
    bytes.try_into().expect("hex string should be 32 bytes")
}

fn hex_bytes(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| {
            let text = std::str::from_utf8(chunk).expect("hex should be utf-8");
            u8::from_str_radix(text, 16).expect("hex byte should parse")
        })
        .collect()
}

fn write_fixture_pair(
    name: &str,
    proof: &ProofArtifact,
    values: &PublicValues,
) -> (PathBuf, PathBuf, PathBuf) {
    let dir = temp_dir(name);
    let _ = fs::remove_dir_all(&dir);
    let proof_path = dir.join("proof.bin");
    let public_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(proof).expect("proof should encode"),
    );
    write_bytes(
        &public_path,
        encode_public_values(values).expect("public values should encode"),
    );
    (dir, proof_path, public_path)
}

#[test]
fn verifies_preflight_reports_eth_block_input_digest() {
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    let values = public_values_from_eth_block_input(sample_hash(0x44), &block_input);
    let public_values_hash = public_values_digest(&values).expect("digest should compute");
    let segment_data = encode_eth_block_input_segment(&block_input).expect("segment should encode");
    let eth_block_input_hash = eth_block_input_bytes_digest(&segment_data);
    let proof = ProofArtifact {
        setup_hash: values.setup_hash,
        public_values_hash,
        segments: vec![ProofSegment {
            id: ETH_BLOCK_INPUT_SEGMENT_ID,
            data: segment_data,
        }],
    };
    let (dir, proof_path, public_path) = write_fixture_pair("eth-block-input", &proof, &values);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "preflight",
            proof_path.to_str().expect("proof path should be utf-8"),
            public_path.to_str().expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nsegments=1\npublic_values=15\npublic_values_hash={}\npublic_value_fields=80\neth_block_inputs=1\neth_block_input_hash={}\n",
            to_hex(&public_values_hash),
            to_hex(&eth_block_input_hash)
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn verifies_preflight_reports_program_image_cache_segments() {
    let values = sample_public_values();
    let public_values_hash = public_values_digest(&values).expect("digest should compute");
    let mut proof = sample_proof(&values);
    proof.segments.push(ProofSegment {
        id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
        data: encode_program_image_cache_segment(&sample_program_image_cache())
            .expect("program image cache segment should encode"),
    });
    let (dir, proof_path, public_path) = write_fixture_pair("program-image-cache", &proof, &values);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "preflight",
            proof_path.to_str().expect("proof path should be utf-8"),
            public_path.to_str().expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            concat!(
                "status=ok\n",
                "segments=2\n",
                "public_values=2\n",
                "public_values_hash={}\n",
                "public_value_fields=5\n",
                "program_image_caches=1\n",
                "program_image_cache_segment_hash=fe67425635287707deccb4174bdf1e9296a954b9cbf378c98c6b339124a82230\n",
                "program_image_cache_program_digest={}\n",
                "program_image_cache_source_image_digest={}\n",
                "program_image_cache_constraint_system_digest={}\n",
                "program_image_cache_tree_root=1,2,3,4\n",
                "program_image_cache_trace_rows=1024\n",
                "program_image_cache_trace_columns=17\n",
                "program_image_cache_blowup_factor=8\n",
                "program_image_cache_arity=4\n",
                "program_image_cache_gpu_mode=cuda\n",
            ),
            to_hex(&public_values_hash),
            to_hex(&sample_hash(0x11)),
            to_hex(&sample_hash(0x22)),
            to_hex(&sample_hash(0x33))
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn verifies_proof_artifact_preflight() {
    let values = sample_public_values();
    let public_values_hash = public_values_digest(&values).expect("digest should compute");
    let proof = sample_proof(&values);
    let (dir, proof_path, public_path) = write_fixture_pair("valid", &proof, &values);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "preflight",
            proof_path.to_str().expect("proof path should be utf-8"),
            public_path.to_str().expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nsegments=1\npublic_values=2\npublic_values_hash={}\npublic_value_fields=5\n",
            to_hex(&public_values_hash)
        )
    );
    assert!(stderr.is_empty());

    let report = validate_proof_public_values_from_files(&proof_path, &public_path)
        .expect("file-based preflight should validate");
    assert_eq!(report.segment_count, 1);
    assert_eq!(report.public_value_count, 2);
    assert_eq!(report.public_value_field_count, 5);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn verifies_proof_artifact_preflight_with_binary_public_values() {
    let values = sample_public_values();
    let public_values_hash = public_values_digest(&values).expect("digest should compute");
    let proof = sample_proof(&values);
    let dir = temp_dir("valid-bin");
    let _ = fs::remove_dir_all(&dir);
    let proof_path = dir.join("proof.bin");
    let public_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_path,
        encode_public_values(&values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "preflight",
            proof_path.to_str().expect("proof path should be utf-8"),
            public_path.to_str().expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nsegments=1\npublic_values=2\npublic_values_hash={}\npublic_value_fields=5\n",
            to_hex(&public_values_hash)
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn rejects_preflight_with_mismatched_setup_hashes() {
    let values = sample_public_values();
    let mut proof = sample_proof(&values);
    proof.setup_hash = sample_hash(0x55);
    proof.public_values_hash = public_values_digest(&values).expect("digest should compute");
    let (dir, proof_path, public_path) = write_fixture_pair("bad-setup", &proof, &values);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "preflight",
            proof_path.to_str().expect("proof path should be utf-8"),
            public_path.to_str().expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify preflight failed: setup hash mismatch\n"
    );
}

#[test]
fn rejects_preflight_with_mismatched_public_values_hashes() {
    let values = sample_public_values();
    let mut proof = sample_proof(&values);
    proof.public_values_hash = sample_hash(0x99);
    let (dir, proof_path, public_path) = write_fixture_pair("bad-public", &proof, &values);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "preflight",
            proof_path.to_str().expect("proof path should be utf-8"),
            public_path.to_str().expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify preflight failed: public-values hash mismatch\n"
    );
}

#[test]
fn rejects_preflight_with_noncanonical_public_values() {
    let mut values = sample_public_values();
    values.values.push(PublicValueEntry {
        name: "bad_value".to_owned(),
        elements: vec![MODULUS],
    });
    let proof = sample_proof(&values);
    let (dir, proof_path, public_path) = write_fixture_pair("bad-public-field", &proof, &values);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "preflight",
            proof_path.to_str().expect("proof path should be utf-8"),
            public_path.to_str().expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "verify preflight failed: invalid PCS transcript public value: non-canonical field element: {MODULUS}\n"
        )
    );
}

#[test]
fn reports_usage_for_missing_preflight_inputs() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["verify", "preflight"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm verify preflight <proof-bin> <public-values>\n"
    );
}
