use lzvm_artifacts::eth_block_input::build_eth_block_input;
use lzvm_artifacts::eth_block_input_segment::{
    encode_eth_block_input_segment, ETH_BLOCK_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_public_values::public_values_from_eth_block_input;
use lzvm_artifacts::program_image::{ProgramImageCommitmentCache, ProgramImageGpuMode};
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, program_image_cache_segment_digest,
    ProgramImageCacheSegmentError, PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};
use lzvm_artifacts::proof::{ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{public_values_digest, PublicValueEntry, PublicValues};
use lzvm_field::{Felt, FieldError, MODULUS};
use lzvm_prover::proof_preflight::{
    public_values_as_fields, validate_proof_public_values, ProofPreflightError,
    ProofPreflightReport, PublicValueFieldError,
};

fn sample_hash(byte: u8) -> [u8; 32] {
    [byte; 32]
}

fn sample_public_values() -> PublicValues {
    PublicValues {
        schema_version: 1,
        setup_hash: sample_hash(0x44),
        values: vec![PublicValueEntry {
            name: "block_number".to_owned(),
            elements: vec![12_345],
        }],
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
        tree_root: [10, 11, 12, 13],
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

#[test]
fn validates_proof_public_value_preflight_hashes() {
    let public_values = sample_public_values();
    let proof = sample_proof(&public_values);

    let report = validate_proof_public_values(&proof, &public_values)
        .expect("proof and public values should match");

    assert_eq!(
        report,
        ProofPreflightReport {
            segment_count: 1,
            public_value_count: 1,
            public_values_hash: public_values_digest(&public_values)
                .expect("digest should compute"),
            public_value_field_count: 1,
            program_image_cache_count: 0,
            program_image_caches: Vec::new(),
            program_image_cache_hashes: Vec::new(),
            eth_block_input_count: 0,
            eth_block_input_hashes: Vec::new(),
            eth_block_input_ommers_hashes: Vec::new(),
            eth_block_input_beneficiaries: Vec::new(),
            eth_block_input_state_roots: Vec::new(),
            eth_block_input_receipt_roots: Vec::new(),
            eth_block_input_logs_blooms: Vec::new(),
            eth_block_input_difficulties: Vec::new(),
            eth_block_input_block_numbers: Vec::new(),
            eth_block_input_timestamps: Vec::new(),
            eth_block_input_transaction_preimage_counts: Vec::new(),
            eth_block_input_legacy_transaction_counts: Vec::new(),
            eth_block_input_typed_transaction_counts: Vec::new(),
            eth_block_input_receipt_preimage_counts: Vec::new(),
            eth_block_input_legacy_receipt_counts: Vec::new(),
            eth_block_input_typed_receipt_counts: Vec::new(),
            eth_block_input_withdrawal_counts: Vec::new(),
            eth_block_input_withdrawal_preimage_counts: Vec::new(),
        }
    );
}

#[test]
fn rejects_proof_public_value_setup_hash_mismatches() {
    let public_values = sample_public_values();
    let mut proof = sample_proof(&public_values);
    proof.setup_hash = sample_hash(0x55);

    let error = validate_proof_public_values(&proof, &public_values)
        .expect_err("setup hashes should match");

    assert_eq!(error, ProofPreflightError::SetupHashMismatch);
}

#[test]
fn rejects_proof_public_value_digest_mismatches() {
    let public_values = sample_public_values();
    let mut proof = sample_proof(&public_values);
    proof.public_values_hash = sample_hash(0x99);

    let error = validate_proof_public_values(&proof, &public_values)
        .expect_err("public value digest should match");

    assert_eq!(error, ProofPreflightError::PublicValuesHashMismatch);
}

#[test]
fn rejects_duplicate_proof_segments_in_memory() {
    let public_values = sample_public_values();
    let mut proof = sample_proof(&public_values);
    proof.segments.push(ProofSegment {
        id: 100,
        data: vec![5, 6, 7, 8],
    });

    let error = validate_proof_public_values(&proof, &public_values)
        .expect_err("proof preflight should validate artifact invariants");

    assert_eq!(error.to_string(), "duplicate proof segment id: 100");
}

#[test]
fn rejects_invalid_program_image_cache_segments() {
    let public_values = sample_public_values();
    let mut proof = sample_proof(&public_values);
    proof.segments.push(ProofSegment {
        id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
        data: vec![1],
    });

    let error = validate_proof_public_values(&proof, &public_values)
        .expect_err("cache segment should parse");

    assert_eq!(
        error,
        ProofPreflightError::ProgramImageCache(ProgramImageCacheSegmentError::UnexpectedEof {
            needed: 8,
            available: 1
        })
    );
}

#[test]
fn reports_program_image_cache_segment_hashes() {
    let public_values = sample_public_values();
    let mut proof = sample_proof(&public_values);
    let segment_data =
        encode_program_image_cache_segment(&sample_program_image_cache()).expect("cache encodes");
    let segment_hash = program_image_cache_segment_digest(&segment_data);
    proof.segments.push(ProofSegment {
        id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
        data: segment_data,
    });

    let report = validate_proof_public_values(&proof, &public_values)
        .expect("proof and public values should match");

    assert_eq!(report.program_image_cache_count, 1);
    assert_eq!(report.program_image_cache_hashes, vec![segment_hash]);
}

#[test]
fn counts_eth_block_input_segments() {
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    let public_values = public_values_from_eth_block_input(sample_hash(0x44), &block_input);
    let mut proof = sample_proof(&public_values);
    proof.segments.push(ProofSegment {
        id: ETH_BLOCK_INPUT_SEGMENT_ID,
        data: encode_eth_block_input_segment(&block_input).expect("segment should encode"),
    });

    let report = validate_proof_public_values(&proof, &public_values)
        .expect("proof and public values should match");

    assert_eq!(report.eth_block_input_count, 1);
    assert_eq!(report.eth_block_input_transaction_preimage_counts, vec![1]);
    assert_eq!(report.eth_block_input_legacy_transaction_counts, vec![1]);
    assert_eq!(report.eth_block_input_typed_transaction_counts, vec![0]);
    assert_eq!(report.eth_block_input_receipt_preimage_counts, vec![None]);
    assert_eq!(report.eth_block_input_legacy_receipt_counts, vec![None]);
    assert_eq!(report.eth_block_input_typed_receipt_counts, vec![None]);
    assert_eq!(
        report.eth_block_input_withdrawal_preimage_counts,
        vec![None]
    );
}

#[test]
fn rejects_eth_block_segments_without_matching_public_values() {
    let public_values = sample_public_values();
    let mut proof = sample_proof(&public_values);
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    proof.segments.push(ProofSegment {
        id: ETH_BLOCK_INPUT_SEGMENT_ID,
        data: encode_eth_block_input_segment(&block_input).expect("segment should encode"),
    });

    let error = validate_proof_public_values(&proof, &public_values)
        .expect_err("ETH block segment should require matching public values");

    assert_eq!(
        error.to_string(),
        "missing ETH block public value: eth_block_hash_u32_be"
    );
}

#[test]
fn rejects_eth_block_public_values_without_input_segment() {
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    let public_values = public_values_from_eth_block_input(sample_hash(0x44), &block_input);
    let proof = sample_proof(&public_values);

    let error = validate_proof_public_values(&proof, &public_values)
        .expect_err("ETH block public values should require an input segment");

    assert_eq!(error.to_string(), "missing ETH block input proof segment");
}

#[test]
fn rejects_base_fee_public_values_without_input_segment() {
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash: sample_hash(0x44),
        values: vec![PublicValueEntry {
            name: "eth_base_fee_per_gas_present".to_owned(),
            elements: vec![1],
        }],
    };
    let proof = sample_proof(&public_values);

    let error = validate_proof_public_values(&proof, &public_values)
        .expect_err("ETH block public values should require an input segment");

    assert_eq!(error.to_string(), "missing ETH block input proof segment");
}

#[test]
fn rejects_difficulty_public_values_without_input_segment() {
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash: sample_hash(0x44),
        values: vec![PublicValueEntry {
            name: "eth_difficulty_u32_be".to_owned(),
            elements: vec![0, 0, 0, 0, 0, 0, 0, 1],
        }],
    };
    let proof = sample_proof(&public_values);

    let error = validate_proof_public_values(&proof, &public_values)
        .expect_err("ETH block public values should require an input segment");

    assert_eq!(error.to_string(), "missing ETH block input proof segment");
}

#[test]
fn rejects_logs_bloom_public_values_without_input_segment() {
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash: sample_hash(0x44),
        values: vec![PublicValueEntry {
            name: "eth_logs_bloom_u32_be".to_owned(),
            elements: vec![0x7777_7777; 64],
        }],
    };
    let proof = sample_proof(&public_values);

    let error = validate_proof_public_values(&proof, &public_values)
        .expect_err("ETH block public values should require an input segment");

    assert_eq!(error.to_string(), "missing ETH block input proof segment");
}

#[test]
fn rejects_extra_data_public_values_without_input_segment() {
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash: sample_hash(0x44),
        values: vec![PublicValueEntry {
            name: "eth_extra_data_len".to_owned(),
            elements: vec![4],
        }],
    };
    let proof = sample_proof(&public_values);

    let error = validate_proof_public_values(&proof, &public_values)
        .expect_err("ETH block public values should require an input segment");

    assert_eq!(error.to_string(), "missing ETH block input proof segment");
}

#[test]
fn converts_public_values_to_field_elements_in_entry_order() {
    let public_values = PublicValues {
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
    };

    let fields = public_values_as_fields(&public_values)
        .expect("canonical public values should convert to fields");

    assert_eq!(
        fields,
        vec![
            Felt::from_canonical(12_345).expect("value should be canonical"),
            Felt::from_canonical(1).expect("value should be canonical"),
            Felt::from_canonical(2).expect("value should be canonical"),
            Felt::from_canonical(3).expect("value should be canonical"),
            Felt::from_canonical(4).expect("value should be canonical"),
        ]
    );
}

#[test]
fn rejects_noncanonical_public_values_for_field_conversion() {
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash: sample_hash(0x44),
        values: vec![PublicValueEntry {
            name: "bad_value".to_owned(),
            elements: vec![MODULUS],
        }],
    };

    let error = public_values_as_fields(&public_values)
        .expect_err("field conversion should reject noncanonical values");

    assert_eq!(
        error,
        PublicValueFieldError::Field(FieldError::NonCanonical { value: MODULUS })
    );
}
