use lzvm_artifacts::eth_block_public_values::validate_program_image_cache_public_values;
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
