use lzvm_artifacts::constraint_program::GlobalConstraintProgram;
use lzvm_artifacts::global_info::{CurveKind, GlobalInfo};
use lzvm_artifacts::hint_program::HintProgram;
use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, GlobalKeyPaths, KeyDirectoryCatalog, KeyDirectoryLayout,
};
use lzvm_artifacts::proof::{ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{public_values_digest, PublicValueEntry, PublicValues};
use lzvm_prover::proof_preflight::ProofPreflightError;
use lzvm_prover::setup_preflight::{
    validate_setup_preflight, validate_setup_preflight_hashes, SetupPreflightError,
    SetupPreflightReport,
};
use lzvm_prover::ProveScheduleError;

fn sample_catalog() -> KeyDirectoryCatalog {
    KeyDirectoryCatalog {
        layout: KeyDirectoryLayout {
            root: ".".into(),
            global_info: GlobalInfo {
                name: "sample-program".to_owned(),
                air_groups: Vec::new(),
                airs: Vec::new(),
                curve: CurveKind::None,
                lattice_size: None,
                aggregation_types: Vec::new(),
                n_publics: 0,
                num_challenges: Vec::new(),
                num_proof_values: Vec::new(),
                proof_values_map: Vec::new(),
                publics_map: Vec::new(),
                transcript_arity: 4,
            },
            global_paths: GlobalKeyPaths {
                info: "global-info.bin".into(),
                constraints_program: "global-constraints.bin".into(),
            },
            source_fixed_file_manifest: "lzvm.source-fixed-file-manifest".into(),
            source_program_archive: "lzvm.source-program-archive".into(),
            units: Vec::new(),
        },
        global_constraints: GlobalConstraintProgram {
            entries: Vec::new(),
            ops: Vec::new(),
            args: Vec::new(),
            numbers: Vec::new(),
        },
        global_hints: HintProgram { hints: Vec::new() },
        source_fixed_file_manifest: None,
        source_program_archive: None,
        units: Vec::new(),
    }
}

fn sample_public_values(setup_hash: [u8; 32]) -> PublicValues {
    PublicValues {
        schema_version: 1,
        setup_hash,
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

#[test]
fn validates_setup_preflight_hashes() {
    let catalog = sample_catalog();
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);
    let proof = sample_proof(&public_values);

    let report = validate_setup_preflight_hashes(&catalog, &proof, &public_values)
        .expect("setup preflight hashes should validate");

    assert_eq!(
        report,
        SetupPreflightReport {
            unit_count: 0,
            segment_count: 1,
            public_value_count: 2,
            public_values_hash: public_values_digest(&public_values)
                .expect("digest should compute"),
            public_value_field_count: 5,
            program_image_cache_count: 0,
            eth_block_input_count: 0,
            eth_block_input_hashes: Vec::new(),
        }
    );
}

#[test]
fn rejects_setup_preflight_catalog_hash_mismatches() {
    let catalog = sample_catalog();
    let mut wrong_setup_hash =
        key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    wrong_setup_hash[0] ^= 1;
    let public_values = sample_public_values(wrong_setup_hash);
    let proof = sample_proof(&public_values);

    let error = validate_setup_preflight_hashes(&catalog, &proof, &public_values)
        .expect_err("catalog hash should match proof setup hash");

    assert_eq!(error, SetupPreflightError::CatalogHashMismatch);
}

#[test]
fn rejects_setup_preflight_public_values_hash_mismatches() {
    let catalog = sample_catalog();
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof(&public_values);
    proof.public_values_hash = [0x99; 32];

    let error = validate_setup_preflight_hashes(&catalog, &proof, &public_values)
        .expect_err("public values digest should match proof hash");

    assert_eq!(
        error,
        SetupPreflightError::Proof(ProofPreflightError::PublicValuesHashMismatch)
    );
}

#[test]
fn rejects_setup_preflight_with_empty_catalog() {
    let catalog = sample_catalog();
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);
    let proof = sample_proof(&public_values);

    let error = validate_setup_preflight(&catalog, &proof, &public_values)
        .expect_err("setup preflight should require scheduled units");

    assert_eq!(
        error,
        SetupPreflightError::Schedule(ProveScheduleError::EmptyCatalog)
    );
}
