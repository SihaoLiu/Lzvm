use lzvm_artifacts::eth_block_input::EthBlockInput;
use lzvm_artifacts::eth_block_public_values::{
    public_values_from_eth_block_input, public_values_from_eth_block_input_for_metadata,
    try_public_values_from_eth_block_input, validate_eth_block_public_values,
    validate_program_image_cache_public_values,
};
use lzvm_artifacts::eth_trie::IndexedTrieBuild;
use lzvm_artifacts::global_info::{CurveKind, GlobalInfo, PublicValue};
use lzvm_artifacts::program_image::{ProgramImageCommitmentCache, ProgramImageGpuMode};
use lzvm_artifacts::public_values::{PublicValueEntry, PublicValues};

fn sample_cache() -> ProgramImageCommitmentCache {
    ProgramImageCommitmentCache {
        program_digest: [0x11; 32],
        source_image_digest: [0x22; 32],
        constraint_system_digest: [0x44; 32],
        tree_root: [1, 2, 3, 4],
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
    }
}

fn sample_block_input() -> EthBlockInput {
    EthBlockInput {
        block_rlp: Vec::new(),
        block_hash: [0; 32],
        parent_hash: [0; 32],
        beneficiary: [0; 20],
        state_root: [0; 32],
        receipts_root: [0; 32],
        logs_bloom: [0; 256],
        difficulty: [0; 32],
        block_number: 0,
        timestamp: 0,
        extra_data: Vec::new(),
        gas_limit: 0,
        gas_used: 0,
        base_fee_per_gas: None,
        mix_hash: [0; 32],
        nonce: [0; 8],
        ommers_hash: [0; 32],
        transactions_root: [0; 32],
        withdrawals_root: None,
        transactions: IndexedTrieBuild {
            root: [0; 32],
            hash_preimages: Vec::new(),
        },
        receipts_rlp: None,
        receipts: None,
        withdrawals: None,
    }
}

fn global_info_with_rom_root() -> GlobalInfo {
    GlobalInfo {
        name: "Test".to_owned(),
        air_groups: vec!["Main".to_owned()],
        airs: Vec::new(),
        curve: CurveKind::None,
        lattice_size: None,
        aggregation_types: Vec::new(),
        n_publics: 4,
        num_challenges: Vec::new(),
        num_proof_values: Vec::new(),
        proof_values_map: Vec::new(),
        publics_map: vec![PublicValue {
            name: "rom_root".to_owned(),
            stage: 1,
            lengths: vec![4],
        }],
        transcript_arity: 16,
    }
}

fn global_info_with_public(name: &str, element_count: u64) -> GlobalInfo {
    GlobalInfo {
        name: "Test".to_owned(),
        air_groups: vec!["Main".to_owned()],
        airs: Vec::new(),
        curve: CurveKind::None,
        lattice_size: None,
        aggregation_types: Vec::new(),
        n_publics: element_count,
        num_challenges: Vec::new(),
        num_proof_values: Vec::new(),
        proof_values_map: Vec::new(),
        publics_map: vec![PublicValue {
            name: name.to_owned(),
            stage: 1,
            lengths: vec![element_count],
        }],
        transcript_arity: 16,
    }
}

#[test]
fn derives_selected_block_public_metadata_without_reading_unselected_extra_data() {
    let mut input = sample_block_input();
    input.block_hash = [0x2a; 32];
    input.extra_data = vec![0x99; 33];
    let global_info = global_info_with_public("eth_block_hash_u32_be", 8);

    let public_values =
        public_values_from_eth_block_input_for_metadata([0x44; 32], &input, &global_info, None)
            .expect("selected block hash public value should derive");

    assert_eq!(public_values.values.len(), 1);
    assert_eq!(public_values.values[0].name, "eth_block_hash_u32_be");
    assert_eq!(public_values.values[0].elements, vec![0x2a2a_2a2a; 8]);
}

#[test]
fn rejects_selected_extra_data_public_metadata_overflow() {
    let mut input = sample_block_input();
    input.extra_data = vec![0x99; 33];
    let global_info = global_info_with_public("eth_extra_data_u32_be", 8);

    let error =
        public_values_from_eth_block_input_for_metadata([0x44; 32], &input, &global_info, None)
            .expect_err("selected extra data public value should validate its payload length");

    assert_eq!(
        error.to_string(),
        "ETH block public value extra data exceeds 32 bytes, found 33"
    );
}

#[test]
fn validates_selected_block_public_value_without_reading_unselected_extra_data() {
    let mut input = sample_block_input();
    input.block_hash = [0x2a; 32];
    input.extra_data = vec![0x99; 33];
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash: [0x44; 32],
        values: vec![PublicValueEntry {
            name: "eth_block_hash_u32_be".to_owned(),
            elements: vec![0x2a2a_2a2a; 8],
        }],
    };

    validate_eth_block_public_values(&input, &public_values)
        .expect("selected block hash public value should validate");
}

#[test]
fn rejects_selected_extra_data_public_value_overflow() {
    let mut input = sample_block_input();
    input.extra_data = vec![0x99; 33];
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash: [0x44; 32],
        values: vec![PublicValueEntry {
            name: "eth_extra_data_u32_be".to_owned(),
            elements: vec![0; 8],
        }],
    };

    let error = validate_eth_block_public_values(&input, &public_values)
        .expect_err("selected extra data public value should validate its payload length");

    assert_eq!(
        error.to_string(),
        "ETH block public value extra data exceeds 32 bytes, found 33"
    );
}

#[test]
fn fallible_full_public_value_generation_matches_infallible_api_for_valid_input() {
    let input = sample_block_input();

    let public_values = try_public_values_from_eth_block_input([0x44; 32], &input)
        .expect("valid block input public values should derive");

    assert_eq!(
        public_values,
        public_values_from_eth_block_input([0x44; 32], &input)
    );
}

#[test]
fn rejects_full_public_value_generation_extra_data_overflow() {
    let mut input = sample_block_input();
    input.extra_data = vec![0x99; 33];

    let error = try_public_values_from_eth_block_input([0x44; 32], &input)
        .expect_err("full public value generation should validate extra data length");

    assert_eq!(
        error.to_string(),
        "ETH block public value extra data exceeds 32 bytes, found 33"
    );
}

#[test]
fn rejects_program_image_cache_public_values_with_wrong_element_count() {
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash: [0x44; 32],
        values: vec![PublicValueEntry {
            name: "rom_root".to_owned(),
            elements: vec![1, 2, 3],
        }],
    };
    let cache = sample_cache();

    let error = validate_program_image_cache_public_values(&public_values, Some(&cache))
        .expect_err("program image cache public value shape should be checked");

    assert_eq!(
        error.to_string(),
        "program image cache public value rom_root element count mismatch: expected 4, found 3"
    );
}

#[test]
fn rejects_program_image_cache_public_values_with_mismatched_setup_hash() {
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash: [0x55; 32],
        values: vec![PublicValueEntry {
            name: "rom_root".to_owned(),
            elements: vec![1, 2, 3, 4],
        }],
    };
    let cache = sample_cache();

    let error = validate_program_image_cache_public_values(&public_values, Some(&cache))
        .expect_err("program image cache setup hash should match public values");

    assert_eq!(error.to_string(), "program image cache setup hash mismatch");
}

#[test]
fn rejects_metadata_generation_with_mismatched_program_image_cache_setup_hash() {
    let input = sample_block_input();
    let global_info = global_info_with_rom_root();
    let cache = sample_cache();

    let error = public_values_from_eth_block_input_for_metadata(
        [0x55; 32],
        &input,
        &global_info,
        Some(&cache),
    )
    .expect_err("program image cache setup hash should match generated public values");

    assert_eq!(error.to_string(), "program image cache setup hash mismatch");
}
