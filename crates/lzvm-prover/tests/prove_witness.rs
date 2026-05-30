use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lzvm_artifacts::challenge_values_segment::{
    encode_challenge_values_segment, ChallengeValuesSegment, CHALLENGE_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::constant_tree::expected_constant_tree_byte_count;
use lzvm_artifacts::constraint_program::{
    ConstraintEntry, ConstraintProgram, GlobalConstraintProgram,
};
use lzvm_artifacts::contribution_segment::CONTRIBUTION_SEGMENT_ID;
use lzvm_artifacts::eth_block_input::build_eth_block_input;
use lzvm_artifacts::eth_block_public_values::public_values_from_eth_block_input;
use lzvm_artifacts::expression_info::ExpressionInfo;
use lzvm_artifacts::expression_program::{ExpressionEntry, ExpressionProgram};
use lzvm_artifacts::fixed::{write_raw_fixed_columns_file, FixedColumn, FixedColumns};
use lzvm_artifacts::global_info::{
    AggregationType, CurveKind, GlobalAir, GlobalInfo, NamedStageValue, PublicValue,
};
use lzvm_artifacts::group_values_segment::{parse_group_values_segment, GROUP_VALUES_SEGMENT_ID};
use lzvm_artifacts::hint_program::{
    Hint, HintField, HintOperand, HintProgram, HintValue, SOURCE_LOOKUP_ASSUMES_HINT,
    SOURCE_LOOKUP_PROVES_HINT,
};
use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, KeyDirectoryCatalog, KeyDirectoryLayout, KeyUnitCatalogEntry,
    KeyUnitKind, KeyUnitPaths,
};
use lzvm_artifacts::metadata_bundle::UnitMetadataBundle;
use lzvm_artifacts::pcs_evaluation_segment::{
    parse_pcs_evaluation_segment, PcsEvaluationUnitSegment, PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_fri_segment::{
    parse_pcs_fri_opening_segment, PcsFriOpeningLayerSegment, PcsFriOpeningUnitSegment,
    PCS_FRI_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::pcs_material::PcsSetupMaterial;
use lzvm_artifacts::pcs_material_segment::{
    parse_pcs_material_manifest_segment, PcsMaterialManifestUnit,
};
use lzvm_artifacts::pcs_nonce_segment::{
    parse_pcs_query_nonce_segment, PCS_QUERY_NONCE_SEGMENT_ID,
};
use lzvm_artifacts::pcs_plan::derive_pcs_setup_plan;
use lzvm_artifacts::pcs_proof_values_segment::{
    parse_pcs_proof_values_segment, PCS_PROOF_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, parse_pcs_query_plan_segment, PcsQueryPlanSegment,
    PcsQueryPlanUnit, PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::proof::{encode_proof_artifact, parse_proof_artifact, ProofSegment};
use lzvm_artifacts::public_values::{
    encode_public_values, public_values_digest, PublicValueEntry, PublicValues,
};
use lzvm_artifacts::setup_info::{
    CommitmentColumn, EvaluationMapEntry, FriStep, StageValue, StarkStruct, UnitSetupInfo,
};
use lzvm_artifacts::trace_bundle::{TraceBundle, TraceBundleUnit};
use lzvm_artifacts::unit_values_segment::{parse_unit_values_segment, UNIT_VALUES_SEGMENT_ID};
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_artifacts::verifier_info::{VerifierCode, VerifierInfo};
use lzvm_artifacts::witness_opening_segment::{
    parse_witness_opening_segment, WITNESS_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, parse_witness_commitment_segment, WitnessCommitmentSegment,
    WitnessCommitmentStageSegment, WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::{coset_extend_evaluations, Ext3, Felt};
use lzvm_prover::contribution::build_witness_contribution_input;
use lzvm_prover::pcs_challenge::{derive_fri_queries, verify_query_nonce};
use lzvm_prover::pcs_fri::{verify_fri_opening_folds, PcsFriOpeningFoldRequest};
use lzvm_prover::pcs_transcript::{
    derive_pcs_final_query_challenge_from_segments, derive_pcs_transcript_challenges,
    PcsTranscriptInputs, PcsTranscriptSegmentInputs,
};
use lzvm_prover::witness_commitment::commit_witness_trace_stages;
use lzvm_prover::witness_layout::derive_witness_trace_layout;
use lzvm_prover::witness_loader::{load_witness_library, TraceBytesBackend};
use lzvm_prover::witness_runner::run_witness_trace;
use lzvm_prover::{
    build_pcs_evaluation_segment, build_pcs_fri_opening_segment,
    build_pcs_fri_opening_segment_from_trace, build_pcs_fri_polynomial_values,
    build_pcs_fri_transcript_values_from_trace, build_pcs_material_manifest_segment,
    build_pcs_query_nonce_segment, build_pcs_query_nonce_segment_from_transcript_segments,
    build_pcs_query_plan_segment, build_pcs_query_plan_segment_from_challenge,
    build_pcs_query_plan_segment_from_transcript_segments, build_witness_commitment_segment,
    build_witness_opening_segment, build_witness_opening_segment_batch,
    derive_prove_execution_plan, derive_prove_schedule, run_prove_witness_commitments,
    run_prove_witness_commitments_with_auxiliary_inputs, run_prove_witness_commitments_with_trace,
    run_prove_witness_commitments_with_trace_backend, GpuRunOptions, ProveExecutionInputArtifacts,
    ProvePartitionPlan, ProvePassRequest, ProvePcsEvaluationValues, ProvePcsFriOpeningTraceValues,
    ProvePcsFriOpeningValues, ProvePcsFriTranscriptTraceValues, ProvePcsQueryPlanSegmentError,
    ProveRunOptions, ProveRunRequest, ProveSchedule, ProveWitnessAuxiliaryInputs,
    ProveWitnessCommitmentError,
};
use sha2::{Digest, Sha256};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-prover-witness-{}-{name}", std::process::id()))
}

fn build_shared_library(dir: &Path, name: &str, source: &str) -> PathBuf {
    fs::create_dir_all(dir).expect("fixture directory should be created");
    let source_path = dir.join(format!("{name}.c"));
    let library_path = dir.join(format!("lib{name}.so"));
    fs::write(&source_path, source).expect("fixture source should be written");
    let status = Command::new("cc")
        .arg("-shared")
        .arg("-fPIC")
        .arg(&source_path)
        .arg("-o")
        .arg(&library_path)
        .status()
        .expect("cc should run");
    assert!(status.success(), "cc should build the fixture library");
    library_path
}

fn sample_guest_image() -> Vec<u8> {
    let mut bytes = vec![0_u8; 64];
    bytes[0..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2;
    bytes[5] = 1;
    bytes[6] = 1;
    bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&243_u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&0x8000_0000_u64.to_le_bytes());
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
    bytes
}

fn sample_witness_library() -> Vec<u8> {
    let mut bytes = vec![0_u8; 64];
    bytes[0..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2;
    bytes[5] = 1;
    bytes[6] = 1;
    bytes[16..18].copy_from_slice(&3_u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&62_u16.to_le_bytes());
    bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
    bytes
}

fn sample_block_rlp_with_parent(parent_hash: [u8; 32]) -> Vec<u8> {
    let header_rlp = rlp_list(&legacy_header_items(
        parent_hash,
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
    ));
    let transactions = rlp_list(&[rlp_list(&[rlp_bytes(&[1])])]);
    let empty_list = rlp_list(&[]);
    rlp_list(&[header_rlp, transactions, empty_list])
}

fn legacy_header_items(parent_hash: [u8; 32], transactions_root: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        rlp_bytes(&parent_hash),
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
    ]
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

fn sample_setup() -> UnitSetupInfo {
    let mut section_widths = BTreeMap::new();
    section_widths.insert("cm1".to_owned(), 2);
    section_widths.insert("cm2".to_owned(), 3);

    UnitSetupInfo {
        n_stages: 1,
        n_constants: 2,
        constant_columns: Vec::new(),
        commitment_columns: Vec::new(),
        n_publics: Some(0),
        n_constraints: Some(0),
        q_degree: 3,
        opening_points: vec![0],
        section_widths,
        challenge_count: 1,
        eval_count: 2,
        evaluation_map: vec![EvaluationMapEntry::default(); 2],
        boundaries: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        stark: StarkStruct {
            n_bits: 4,
            n_bits_ext: 6,
            n_queries: 2,
            steps: vec![FriStep { n_bits: 6 }, FriStep { n_bits: 4 }],
            hash_commits: true,
            last_level_verification: 2,
            pow_bits: 10,
            merkle_tree_arity: 4,
            verification_hash_type: Some("GL".to_owned()),
            transcript_arity: Some(4),
            merkle_tree_custom: Some(true),
        },
    }
}

fn stage_value(name: &str, stage: u32) -> StageValue {
    StageValue {
        name: name.to_owned(),
        stage,
        lengths: vec![1],
    }
}

fn empty_expression_info() -> ExpressionInfo {
    ExpressionInfo {
        hints: Vec::new(),
        expressions: Vec::new(),
        constraints: Vec::new(),
    }
}

fn empty_verifier_info() -> VerifierInfo {
    VerifierInfo {
        quotient: VerifierCode {
            expression_id: None,
            stage: None,
            line: String::new(),
            temporary_count: 0,
            operations: Vec::new(),
        },
        query: VerifierCode {
            expression_id: None,
            stage: None,
            line: String::new(),
            temporary_count: 0,
            operations: Vec::new(),
        },
    }
}

fn empty_program() -> ExpressionProgram {
    ExpressionProgram {
        max_tmp1: 0,
        max_tmp3: 0,
        max_args: 0,
        max_ops: 0,
        entries: Vec::new(),
        ops: Vec::new(),
        args: Vec::new(),
        numbers: Vec::new(),
    }
}

fn empty_regular_constraints() -> ConstraintProgram {
    ConstraintProgram {
        entries: Vec::new(),
        ops: Vec::new(),
        args: Vec::new(),
        numbers: Vec::new(),
    }
}

fn empty_regular_hints() -> HintProgram {
    HintProgram { hints: Vec::new() }
}

fn proof_value_regular_hint() -> HintProgram {
    HintProgram {
        hints: vec![Hint {
            name: "runtime-hint".to_owned(),
            fields: vec![HintField {
                name: "values".to_owned(),
                values: vec![HintValue {
                    operand: HintOperand::ProofValue { id: 0 },
                    positions: vec![0],
                }],
            }],
        }],
    }
}

fn row_zero_stage_constraint(expected: u64) -> ConstraintProgram {
    ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 1,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "row zero stage residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![3, 0, 1, 0, 0, 8, 0, 0],
        numbers: vec![expected],
    }
}

fn public_row_zero_stage_constraint() -> ConstraintProgram {
    ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 1,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "public row zero stage residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 7, 0, 0, 1, 0, 0],
        numbers: Vec::new(),
    }
}

fn proof_value_row_zero_stage_constraint() -> ConstraintProgram {
    ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 1,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "proof value row zero stage residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 10, 0, 0, 1, 0, 0],
        numbers: Vec::new(),
    }
}

fn challenge_row_zero_stage_constraint() -> ConstraintProgram {
    ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 1,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "challenge row zero stage residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 12, 0, 0, 1, 0, 0],
        numbers: Vec::new(),
    }
}

fn domain_helper_row_zero_stage_constraint() -> ConstraintProgram {
    ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 1,
            first_row: 0,
            last_row: 1,
            temp1_count: 2,
            temp3_count: 0,
            ops_count: 2,
            ops_offset: 0,
            args_count: 16,
            args_offset: 0,
            intermediate: false,
            source_line: "domain helper row zero residual".to_owned(),
        }],
        ops: vec![0, 0],
        args: vec![
            1, 0, 1, 0, 0, 3, 0, 0, //
            1, 1, 5, 0, 0, 8, 0, 0,
        ],
        numbers: vec![1],
    }
}

fn fixed_plus_stage_expression_program(expression_id: u32) -> ExpressionProgram {
    ExpressionProgram {
        max_tmp1: 1,
        max_tmp3: 0,
        max_args: 8,
        max_ops: 1,
        entries: vec![ExpressionEntry {
            expression_id,
            destination_dimension: 1,
            destination_id: 0,
            stage: 2,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            source_line: "fixed plus stage".to_owned(),
        }],
        ops: vec![0],
        args: vec![0, 0, 0, 0, 0, 1, 0, 0],
        numbers: Vec::new(),
    }
}

fn evaluation_plus_zero_expression_program(expression_id: u32) -> ExpressionProgram {
    ExpressionProgram {
        max_tmp1: 0,
        max_tmp3: 1,
        max_args: 8,
        max_ops: 1,
        entries: vec![ExpressionEntry {
            expression_id,
            destination_dimension: 3,
            destination_id: 0,
            stage: 2,
            temp1_count: 0,
            temp3_count: 1,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            source_line: "evaluation plus zero".to_owned(),
        }],
        ops: vec![1],
        args: vec![0, 0, 13, 0, 0, 8, 0, 0],
        numbers: vec![0],
    }
}

fn encode_trace_words(values: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 8);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn write_public_values(path: &Path, setup_hash: [u8; 32], elements: Vec<u64>) {
    let values = PublicValues {
        schema_version: 1,
        setup_hash,
        values: vec![PublicValueEntry {
            name: "sample_public".to_owned(),
            elements,
        }],
    };
    fs::write(
        path,
        encode_public_values(&values).expect("public values should encode"),
    )
    .expect("public values should be written");
}

fn declare_sample_public_value_metadata(catalog: &mut KeyDirectoryCatalog) {
    catalog.layout.global_info.n_publics = 1;
    catalog.layout.global_info.publics_map = vec![PublicValue {
        name: "sample_public".to_owned(),
        stage: 1,
        lengths: Vec::new(),
    }];
}

fn write_sample_fixed_columns(path: &Path, setup: &UnitSetupInfo, unit_name: &str) {
    let fixed_columns = FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: unit_name.to_owned(),
        row_count: 16,
        columns: vec![
            FixedColumn {
                name: "const_0".to_owned(),
                dimensions: Vec::new(),
                values: (0..16).map(|row| row + 10).collect(),
            },
            FixedColumn {
                name: "const_1".to_owned(),
                dimensions: Vec::new(),
                values: vec![0; 16],
            },
        ],
    };
    write_raw_fixed_columns_file(path, &fixed_columns, setup)
        .expect("fixed columns should be written");
}

fn sample_pcs_material() -> PcsSetupMaterial {
    PcsSetupMaterial {
        plan_digest: [7; 32],
        fixed_column_digest: [8; 32],
        constant_tree_digest: [9; 32],
        constant_tree_root: [1, 2, 3, 4],
        fixed_byte_count: 64,
        constant_tree_byte_count: 224,
        leaf_byte_count: 64,
        node_byte_count: 160,
    }
}

fn sample_witness_commitment_segment(
    unit_index: u32,
    root_seeds: &[u64],
) -> WitnessCommitmentSegment {
    WitnessCommitmentSegment {
        unit_index,
        input_byte_count: 0,
        trace_rows: 16,
        trace_columns: 5,
        stages: root_seeds
            .iter()
            .enumerate()
            .map(|(index, seed)| WitnessCommitmentStageSegment {
                stage_index: (index + 1) as u32,
                arity: 4,
                root: [*seed, *seed + 1, *seed + 2, *seed + 3],
                tree_byte_count: 224,
                tree_digest: [index as u8; 32],
            })
            .collect(),
    }
}

struct TranscriptQueryFixture {
    schedule: ProveSchedule,
    material: PcsMaterialManifestUnit,
    witness: WitnessCommitmentSegment,
    witness_segment: ProofSegment,
    evaluations: PcsEvaluationUnitSegment,
    fri: PcsFriOpeningUnitSegment,
}

impl TranscriptQueryFixture {
    fn inputs(&self) -> PcsTranscriptSegmentInputs<'_> {
        PcsTranscriptSegmentInputs {
            unit_index: 0,
            unit: &self.schedule.units[0],
            material: &self.material,
            public_values: &[],
            unit_values: &[],
            witness: &self.witness,
            evaluations: &self.evaluations,
            fri: &self.fri,
            root_challenge_draws: &self.schedule.units[0].transcript_root_challenge_draws,
            evaluation_challenge_draws: self.schedule.units[0]
                .transcript_evaluation_challenge_draws,
            binding_segments: &[],
        }
    }
}

fn sample_transcript_query_fixture() -> TranscriptQueryFixture {
    let catalog = sample_catalog(sample_unit());
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let material_segment =
        build_pcs_material_manifest_segment(&schedule).expect("material segment should build");
    let material = parse_pcs_material_manifest_segment(&material_segment.data)
        .expect("material segment should parse")
        .units[0]
        .clone();
    let witness = sample_witness_commitment_segment(0, &[10, 20]);
    let witness_segment = ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
        data: encode_witness_commitment_segment(&witness).expect("witness segment should encode"),
    };
    let evaluations = PcsEvaluationUnitSegment {
        unit_index: 0,
        values: vec![[30, 31, 32], [40, 41, 42]],
    };
    let fri = PcsFriOpeningUnitSegment {
        unit_index: 0,
        layers: vec![PcsFriOpeningLayerSegment {
            layer_index: 0,
            root: [50, 51, 52, 53],
            last_level: Vec::new(),
            queries: Vec::new(),
        }],
        final_polynomial: vec![[60, 61, 62], [70, 71, 72]],
    };
    TranscriptQueryFixture {
        schedule,
        material,
        witness,
        witness_segment,
        evaluations,
        fri,
    }
}

fn sample_unit() -> KeyUnitCatalogEntry {
    let setup = sample_setup();
    let pcs_plan = derive_pcs_setup_plan(&setup).expect("PCS setup plan should derive");

    KeyUnitCatalogEntry {
        paths: KeyUnitPaths {
            kind: KeyUnitKind::Basic,
            group_id: Some(0),
            unit_id: Some(0),
            group_name: Some("group-a".to_owned()),
            unit_name: Some("unit-a".to_owned()),
            prefix: "unit".into(),
            metadata_prefix: Some("unit".into()),
            program_prefix: Some("unit".into()),
            verification_key_prefix: "unit".into(),
            fixed_columns: "unit.const".into(),
            constant_tree: "unit.consttree".into(),
        },
        metadata: UnitMetadataBundle {
            setup,
            expressions: empty_expression_info(),
            verifier: empty_verifier_info(),
        },
        pcs_plan,
        verification_key: VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4]),
        expression_program: empty_program(),
        regular_constraints: empty_regular_constraints(),
        regular_hints: empty_regular_hints(),
        verifier_program: empty_program(),
        expected_fixed_bytes: 64,
        actual_fixed_bytes: 64,
        constant_tree_present: true,
        constant_tree_bytes: Some(224),
        constant_tree_root: Some(VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4])),
        pcs_material_present: true,
        pcs_material_bytes: Some(184),
        pcs_material: Some(sample_pcs_material()),
    }
}

fn sample_catalog(unit: KeyUnitCatalogEntry) -> KeyDirectoryCatalog {
    sample_catalog_units(vec![unit])
}

fn sample_catalog_units(units: Vec<KeyUnitCatalogEntry>) -> KeyDirectoryCatalog {
    KeyDirectoryCatalog {
        layout: KeyDirectoryLayout {
            root: ".".into(),
            global_info: GlobalInfo {
                name: "sample-program".to_owned(),
                air_groups: vec!["group-a".to_owned()],
                airs: Vec::new(),
                curve: CurveKind::None,
                lattice_size: None,
                aggregation_types: Vec::new(),
                n_publics: 0,
                num_challenges: vec![1],
                num_proof_values: Vec::new(),
                proof_values_map: Vec::new(),
                publics_map: Vec::new(),
                transcript_arity: 4,
            },
            global_paths: lzvm_artifacts::key_directory::GlobalKeyPaths {
                info: "global-info.json".into(),
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
        global_hints: empty_regular_hints(),
        source_fixed_file_manifest: None,
        source_program_archive: None,
        units,
    }
}

fn sample_request(output_dir: PathBuf, input_data: Option<PathBuf>) -> ProveRunRequest {
    ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan {
            input_data,
            partition_count: 1,
            partition_ids: vec![0],
            worker_index: 0,
        }),
        options: ProveRunOptions::default_for_output(output_dir),
        gpu: GpuRunOptions::default(),
    }
}

fn sample_trace_bytes(seed: u64) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(16 * 5 * 8);
    for value in seed + 1..=seed + 80 {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn sample_trace_bundle(unit_count: u32, seed: u64) -> TraceBundle {
    TraceBundle {
        units: (0..unit_count)
            .map(|unit_index| TraceBundleUnit {
                unit_index,
                trace_bytes: sample_trace_bytes(seed + unit_index as u64),
            })
            .collect(),
    }
}

fn source_lookup_balance_hint(name: &str) -> Hint {
    Hint {
        name: name.to_owned(),
        fields: vec![
            HintField {
                name: "bus_id".to_owned(),
                values: vec![HintValue {
                    operand: HintOperand::Number(7),
                    positions: Vec::new(),
                }],
            },
            HintField {
                name: "values".to_owned(),
                values: vec![HintValue {
                    operand: HintOperand::Commitment {
                        id: 0,
                        row_offset_index: 0,
                    },
                    positions: Vec::new(),
                }],
            },
        ],
    }
}

fn declare_source_lookup_commitment_column(unit: &mut KeyUnitCatalogEntry) {
    unit.metadata.setup.commitment_columns = vec![CommitmentColumn {
        name: "lookup_value".to_owned(),
        stage: 1,
        dimension: 1,
        pols_map_id: 0,
        stage_id: 0,
        stage_position: 0,
        intermediate: false,
        lengths: Vec::new(),
    }];
}

fn witness_source() -> &'static str {
    r#"#include <stddef.h>
typedef struct {
    const unsigned char *input_ptr;
    size_t input_len;
    unsigned char *output_ptr;
    size_t output_len;
} LzvmWitnessCall;
typedef struct {
    int status;
    size_t produced_len;
} LzvmWitnessResult;
static void write_u64_le(unsigned char *out, unsigned long long value) {
    for (size_t i = 0; i < 8; i++) {
        out[i] = (unsigned char)((value >> (i * 8)) & 0xff);
    }
}
unsigned int lzvm_witness_abi_version(void) { return 1; }
int lzvm_witness_compute(const LzvmWitnessCall *call, LzvmWitnessResult *result) {
    const size_t rows = 16;
    const size_t columns = 5;
    const size_t word_bytes = 8;
    const size_t element_count = rows * columns;
    if (!call || !result || call->output_len < element_count * word_bytes) {
        return -1;
    }
    unsigned long long seed = call->input_len > 0 ? call->input_ptr[0] : 0;
    for (size_t index = 0; index < element_count; index++) {
        write_u64_le(call->output_ptr + index * word_bytes, seed + index + 1);
    }
    result->status = 0;
    result->produced_len = element_count * word_bytes;
    return 0;
}
"#
}

#[test]
fn runs_witness_and_commits_stages_from_execution_plan() {
    let dir = temp_dir("commitments");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let catalog = sample_catalog(sample_unit());
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data.clone())),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library.clone()),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");

    let library = load_witness_library(&witness_library).expect("witness library should load");
    let output = run_prove_witness_commitments_with_trace_backend(
        &plan,
        0,
        ProveWitnessAuxiliaryInputs::default(),
        &library,
    )
    .expect("witness commitments should run")
    .into_commitments();

    let unit = &plan.run_plan.schedule.units[0];
    let layout = derive_witness_trace_layout(unit).expect("layout should derive");
    let trace = run_witness_trace(
        &library,
        layout.request(fs::read(&input_data).expect("input data should read")),
    )
    .expect("witness trace should run");
    let expected = commit_witness_trace_stages(&trace, unit).expect("trace stages should commit");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(output.unit_index(), 0);
    assert_eq!(output.input_byte_count(), 1);
    assert_eq!(output.trace_row_count(), 16);
    assert_eq!(output.trace_column_count(), 5);
    assert_eq!(output.stage_commitments(), &expected);
}

#[test]
fn builds_witness_contribution_input_from_stage_one_root() {
    let dir = temp_dir("contribution-input");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [9_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.metadata.setup.unit_value_map = vec![
        stage_value("local_a", 1),
        stage_value("local_b", 2),
        stage_value("local_c", 1),
    ];
    let mut catalog = sample_catalog(unit);
    catalog.layout.global_info.lattice_size = Some(32);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library.clone()),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let library = load_witness_library(&witness_library).expect("witness library should load");
    let auxiliary_inputs = ProveWitnessAuxiliaryInputs {
        unit_values: vec![
            Felt::from_u64(701),
            Felt::from_u64(801),
            Felt::from_u64(802),
            Felt::from_u64(803),
            Felt::from_u64(702),
        ],
        ..ProveWitnessAuxiliaryInputs::default()
    };
    let output =
        run_prove_witness_commitments_with_trace_backend(&plan, 0, auxiliary_inputs, &library)
            .expect("witness commitments should run");
    let stage_one_root = output
        .commitments()
        .stage_commitments()
        .commitments()
        .iter()
        .find(|commitment| commitment.stage_index() == 1)
        .expect("stage-one commitment should exist")
        .root();

    let input = build_witness_contribution_input(
        &catalog.units[0].verification_key,
        &plan.run_plan.schedule.units[0],
        &output,
        &output.auxiliary_inputs().unit_values,
    )
    .expect("witness contribution input should build");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(input.root, stage_one_root);
    assert_eq!(
        input.values,
        vec![
            Felt::from_u64(1),
            Felt::from_u64(2),
            Felt::from_u64(3),
            Felt::from_u64(4),
            Felt::ZERO,
            Felt::ZERO,
            Felt::ZERO,
            Felt::ZERO,
            Felt::from_u64(701),
            Felt::from_u64(702),
        ]
    );
}

#[test]
fn builds_witness_proof_artifact_in_prover() {
    let dir = temp_dir("proof-artifact");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.paths.constant_tree = dir.join("unit.consttree");
    let constant_tree_bytes =
        expected_constant_tree_byte_count(&unit.metadata.setup).expect("tree size should derive");
    fs::write(&unit.paths.constant_tree, vec![0_u8; constant_tree_bytes])
        .expect("constant tree should be written");
    let mut catalog = sample_catalog(unit);
    catalog.layout.global_info.lattice_size = Some(32);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let witness = run_prove_witness_commitments(&plan, 0).expect("witness should run");
    let public_values_hash = [13_u8; 32];

    let proof = lzvm_prover::build_witness_proof_core_artifact(
        &catalog,
        &plan.run_plan.schedule,
        public_values_hash,
        &[&witness],
    )
    .expect("proof artifact should build");
    let proof = parse_proof_artifact(&encode_proof_artifact(&proof).expect("proof should encode"))
        .expect("proof should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(proof.setup_hash, plan.run_plan.schedule.setup_hash);
    assert_eq!(proof.public_values_hash, public_values_hash);
    assert!(!proof.segments.is_empty());
}

#[test]
fn builds_witness_proof_artifact_for_unit_in_prover() {
    let dir = temp_dir("proof-artifact-unit");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [5_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.metadata.setup.unit_value_map = vec![
        stage_value("local_a", 1),
        stage_value("local_b", 2),
        stage_value("local_c", 1),
    ];
    unit.paths.constant_tree = dir.join("unit.consttree");
    let constant_tree_bytes =
        expected_constant_tree_byte_count(&unit.metadata.setup).expect("tree size should derive");
    fs::write(&unit.paths.constant_tree, vec![0_u8; constant_tree_bytes])
        .expect("constant tree should be written");
    let mut catalog = sample_catalog(unit);
    catalog.layout.global_info.lattice_size = Some(32);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let auxiliary_inputs = ProveWitnessAuxiliaryInputs {
        unit_values: vec![
            Felt::from_u64(701),
            Felt::from_u64(801),
            Felt::from_u64(802),
            Felt::from_u64(803),
            Felt::from_u64(702),
        ],
        ..ProveWitnessAuxiliaryInputs::default()
    };
    let output = run_prove_witness_commitments_with_trace(&plan, 0, auxiliary_inputs)
        .expect("witness commitments should run");
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash: plan.run_plan.schedule.setup_hash,
        values: vec![PublicValueEntry {
            name: "sample_public".to_owned(),
            elements: vec![19],
        }],
    };

    let proof =
        lzvm_prover::build_witness_proof_artifact_for_unit(&lzvm_prover::WitnessProofRequest {
            catalog: &catalog,
            schedule: &plan.run_plan.schedule,
            execution_unit: &plan.units[0],
            gpu_streams: plan.run_plan.gpu.max_streams,
            public_values: Some(&public_values),
            unit_values: None,
            output: &output,
            verify_outputs: false,
            program_image_cache: None,
            eth_block_input: None,
            challenge_values_segment: None,
        })
        .expect("proof artifact should build")
        .expect("proof artifact should exist");
    let proof = parse_proof_artifact(&encode_proof_artifact(&proof).expect("proof should encode"))
        .expect("proof should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(proof.setup_hash, plan.run_plan.schedule.setup_hash);
    assert_eq!(
        proof.public_values_hash,
        public_values_digest(&public_values).expect("digest should compute")
    );
    assert!(proof
        .segments
        .iter()
        .any(|segment| { segment.id == WITNESS_COMMITMENT_SEGMENT_BASE_ID }));
    assert!(!proof
        .segments
        .iter()
        .any(|segment| { segment.id == CONTRIBUTION_SEGMENT_ID }));
}

#[test]
fn rejects_mismatched_eth_block_public_values_in_prover_unit_request() {
    let dir = temp_dir("proof-artifact-unit-eth-mismatch");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [5_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.paths.constant_tree = dir.join("unit.consttree");
    let constant_tree_bytes =
        expected_constant_tree_byte_count(&unit.metadata.setup).expect("tree size should derive");
    fs::write(&unit.paths.constant_tree, vec![0_u8; constant_tree_bytes])
        .expect("constant tree should be written");
    let mut catalog = sample_catalog(unit);
    catalog.layout.global_info.lattice_size = Some(32);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let output =
        run_prove_witness_commitments_with_trace(&plan, 0, ProveWitnessAuxiliaryInputs::default())
            .expect("witness commitments should run");
    let public_block_input = build_eth_block_input(&sample_block_rlp_with_parent([0x11; 32]))
        .expect("public block input should build");
    let proof_block_input = build_eth_block_input(&sample_block_rlp_with_parent([0x22; 32]))
        .expect("proof block input should build");
    let public_values =
        public_values_from_eth_block_input(plan.run_plan.schedule.setup_hash, &public_block_input);

    let error =
        lzvm_prover::build_witness_proof_artifact_for_unit(&lzvm_prover::WitnessProofRequest {
            catalog: &catalog,
            schedule: &plan.run_plan.schedule,
            execution_unit: &plan.units[0],
            gpu_streams: plan.run_plan.gpu.max_streams,
            public_values: Some(&public_values),
            unit_values: None,
            output: &output,
            verify_outputs: false,
            program_image_cache: None,
            eth_block_input: Some(&proof_block_input),
            challenge_values_segment: None,
        })
        .expect_err("mismatched block public values should reject");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        error,
        "ETH block public value mismatch: eth_block_hash_u32_be"
    );
}

#[test]
fn rejects_unbound_program_image_cache_public_values_in_prover_unit_request() {
    let dir = temp_dir("proof-artifact-unit-program-image-cache-missing");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [5_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.paths.constant_tree = dir.join("unit.consttree");
    let constant_tree_bytes =
        expected_constant_tree_byte_count(&unit.metadata.setup).expect("tree size should derive");
    fs::write(&unit.paths.constant_tree, vec![0_u8; constant_tree_bytes])
        .expect("constant tree should be written");
    let mut catalog = sample_catalog(unit);
    catalog.layout.global_info.lattice_size = Some(32);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let output =
        run_prove_witness_commitments_with_trace(&plan, 0, ProveWitnessAuxiliaryInputs::default())
            .expect("witness commitments should run");
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash: plan.run_plan.schedule.setup_hash,
        values: vec![PublicValueEntry {
            name: "rom_root".to_owned(),
            elements: vec![1, 2, 3, 4],
        }],
    };

    let error =
        lzvm_prover::build_witness_proof_artifact_for_unit(&lzvm_prover::WitnessProofRequest {
            catalog: &catalog,
            schedule: &plan.run_plan.schedule,
            execution_unit: &plan.units[0],
            gpu_streams: plan.run_plan.gpu.max_streams,
            public_values: Some(&public_values),
            unit_values: None,
            output: &output,
            verify_outputs: false,
            program_image_cache: None,
            eth_block_input: None,
            challenge_values_segment: None,
        })
        .expect_err("program image cache public values should require a bound cache");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        error,
        "program image cache is required for public value: rom_root"
    );
}

#[test]
fn builds_witness_proof_artifact_for_all_units_in_prover() {
    let dir = temp_dir("proof-artifact-all-units");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [11_u8]).expect("input data should be written");

    let mut second_unit = sample_unit();
    second_unit.metadata.setup.unit_value_map = vec![
        stage_value("local_a", 1),
        stage_value("local_b", 2),
        stage_value("local_c", 1),
    ];
    second_unit.paths.unit_id = Some(1);
    second_unit.paths.unit_name = Some("unit-b".to_owned());
    second_unit.paths.prefix = "unit-b".into();
    second_unit.paths.metadata_prefix = Some("unit-b".into());
    second_unit.paths.program_prefix = Some("unit-b".into());
    second_unit.paths.verification_key_prefix = "unit-b".into();
    second_unit.paths.constant_tree = dir.join("unit-b.consttree");
    let mut first_unit = sample_unit();
    first_unit.metadata.setup.unit_value_map = vec![
        stage_value("local_a", 1),
        stage_value("local_b", 2),
        stage_value("local_c", 1),
    ];
    first_unit.paths.constant_tree = dir.join("unit.consttree");
    let first_tree_bytes = expected_constant_tree_byte_count(&first_unit.metadata.setup)
        .expect("tree size should derive");
    let second_tree_bytes = expected_constant_tree_byte_count(&second_unit.metadata.setup)
        .expect("tree size should derive");
    fs::write(
        &first_unit.paths.constant_tree,
        vec![0_u8; first_tree_bytes],
    )
    .expect("first constant tree should be written");
    fs::write(
        &second_unit.paths.constant_tree,
        vec![0_u8; second_tree_bytes],
    )
    .expect("second constant tree should be written");
    let mut catalog = sample_catalog_units(vec![first_unit, second_unit]);
    catalog.layout.global_info.lattice_size = Some(32);
    catalog.layout.global_info.airs = vec![vec![
        GlobalAir {
            name: "unit-a".to_owned(),
            num_rows: 16,
            has_compressor: false,
        },
        GlobalAir {
            name: "unit-b".to_owned(),
            num_rows: 16,
            has_compressor: false,
        },
    ]];
    catalog.layout.global_info.aggregation_types = vec![vec![AggregationType {
        aggregation_type: 0,
    }]];
    catalog.layout.global_info.num_proof_values = vec![1];
    catalog.layout.global_info.proof_values_map = vec![NamedStageValue {
        name: "global-proof".to_owned(),
        stage: 1,
        id: None,
        lengths: Vec::new(),
    }];
    declare_sample_public_value_metadata(&mut catalog);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values_path = dir.join("public.bin");
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash,
        values: vec![PublicValueEntry {
            name: "sample_public".to_owned(),
            elements: vec![13],
        }],
    };
    fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should be written");
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: Some(public_values_path),
        },
    )
    .expect("execution plan should derive");
    let output_auxiliary_inputs = ProveWitnessAuxiliaryInputs {
        unit_values: vec![
            Felt::from_u64(901),
            Felt::from_u64(1001),
            Felt::from_u64(1002),
            Felt::from_u64(1003),
            Felt::from_u64(902),
        ],
        proof_values: vec![Felt::from_u64(31)],
        group_values: vec![Ext3::from_u64s([41, 42, 43])],
        ..ProveWitnessAuxiliaryInputs::default()
    };
    let request_auxiliary_inputs = ProveWitnessAuxiliaryInputs::default();
    let outputs = vec![
        run_prove_witness_commitments_with_trace(&plan, 0, output_auxiliary_inputs.clone())
            .expect("first unit should run"),
        run_prove_witness_commitments_with_trace(&plan, 1, output_auxiliary_inputs.clone())
            .expect("second unit should run"),
    ];

    let proof = lzvm_prover::build_witness_proof_artifact_for_all_units(
        &lzvm_prover::WitnessAllUnitsProofRequest {
            catalog: &catalog,
            schedule: &plan.run_plan.schedule,
            execution_units: &plan.units,
            gpu_streams: plan.run_plan.gpu.max_streams,
            public_values: Some(&public_values),
            outputs: &outputs,
            auxiliary_inputs: &request_auxiliary_inputs,
            unit_values: &[],
            evaluation_values_segment: None,
            verify_outputs: false,
            program_image_cache: None,
            eth_block_input: None,
            challenge_values_segment: None,
        },
    )
    .expect("proof artifact should build")
    .expect("proof artifact should exist");
    let proof = parse_proof_artifact(&encode_proof_artifact(&proof).expect("proof should encode"))
        .expect("proof should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(proof.setup_hash, plan.run_plan.schedule.setup_hash);
    assert_eq!(
        proof.public_values_hash,
        public_values_digest(&public_values).expect("digest should compute")
    );
    let second_witness_id = WITNESS_COMMITMENT_SEGMENT_BASE_ID
        .checked_add(1)
        .expect("second witness id should fit");
    assert_eq!(
        proof
            .segments
            .iter()
            .filter(|segment| {
                segment.id == WITNESS_COMMITMENT_SEGMENT_BASE_ID || segment.id == second_witness_id
            })
            .map(|segment| segment.id)
            .collect::<Vec<_>>(),
        vec![WITNESS_COMMITMENT_SEGMENT_BASE_ID, second_witness_id]
    );
    assert!(!proof
        .segments
        .iter()
        .any(|segment| { segment.id == CONTRIBUTION_SEGMENT_ID }));
    let proof_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_PROOF_VALUES_SEGMENT_ID)
        .expect("proof values segment should exist");
    let proof_values = parse_pcs_proof_values_segment(&proof_values_segment.data)
        .expect("proof values should parse");
    assert_eq!(proof_values.values, vec![[31, 0, 0]]);
    let group_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == GROUP_VALUES_SEGMENT_ID)
        .expect("group values segment should exist");
    let group_values =
        parse_group_values_segment(&group_values_segment.data).expect("group values should parse");
    assert_eq!(group_values.values, vec![[41, 42, 43]]);
    let unit_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == UNIT_VALUES_SEGMENT_ID)
        .expect("unit values segment should exist");
    let unit_values =
        parse_unit_values_segment(&unit_values_segment.data).expect("unit values should parse");
    assert_eq!(unit_values.units.len(), 2);
    assert_eq!(unit_values.units[0].unit_index, 0);
    assert_eq!(unit_values.units[1].unit_index, 1);
    assert_eq!(
        unit_values.units[0].values,
        vec![901, 1001, 1002, 1003, 902]
    );
    assert_eq!(
        unit_values.units[1].values,
        vec![901, 1001, 1002, 1003, 902]
    );
}

#[test]
fn builds_all_units_contribution_proof_artifact_from_output_proof_values() {
    let dir = temp_dir("contribution-proof-all-units-output-proof-values");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [17_u8]).expect("input data should be written");

    let mut second_unit = sample_unit();
    second_unit.paths.unit_id = Some(1);
    second_unit.paths.unit_name = Some("unit-b".to_owned());
    second_unit.paths.prefix = "unit-b".into();
    second_unit.paths.metadata_prefix = Some("unit-b".into());
    second_unit.paths.program_prefix = Some("unit-b".into());
    second_unit.paths.verification_key_prefix = "unit-b".into();
    second_unit.paths.constant_tree = dir.join("unit-b.consttree");
    let mut first_unit = sample_unit();
    first_unit.paths.constant_tree = dir.join("unit.consttree");
    let first_tree_bytes = expected_constant_tree_byte_count(&first_unit.metadata.setup)
        .expect("tree size should derive");
    let second_tree_bytes = expected_constant_tree_byte_count(&second_unit.metadata.setup)
        .expect("tree size should derive");
    fs::write(
        &first_unit.paths.constant_tree,
        vec![0_u8; first_tree_bytes],
    )
    .expect("first constant tree should be written");
    fs::write(
        &second_unit.paths.constant_tree,
        vec![0_u8; second_tree_bytes],
    )
    .expect("second constant tree should be written");
    let mut catalog = sample_catalog_units(vec![first_unit, second_unit]);
    catalog.layout.global_info.lattice_size = Some(32);
    catalog.layout.global_info.num_proof_values = vec![1];
    catalog.layout.global_info.proof_values_map = vec![NamedStageValue {
        name: "global-proof".to_owned(),
        stage: 1,
        id: None,
        lengths: Vec::new(),
    }];
    declare_sample_public_value_metadata(&mut catalog);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values_path = dir.join("public.bin");
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash,
        values: vec![PublicValueEntry {
            name: "sample_public".to_owned(),
            elements: vec![17],
        }],
    };
    fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should be written");
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: Some(public_values_path),
        },
    )
    .expect("execution plan should derive");
    let output_auxiliary_inputs = ProveWitnessAuxiliaryInputs {
        proof_values: vec![Felt::from_u64(31)],
        ..ProveWitnessAuxiliaryInputs::default()
    };
    let request_auxiliary_inputs = ProveWitnessAuxiliaryInputs::default();
    let outputs = vec![
        run_prove_witness_commitments_with_trace(&plan, 0, output_auxiliary_inputs.clone())
            .expect("first unit should run"),
        run_prove_witness_commitments_with_trace(&plan, 1, output_auxiliary_inputs.clone())
            .expect("second unit should run"),
    ];

    let proof = lzvm_prover::build_witness_contribution_proof_artifact_for_all_units(
        &lzvm_prover::WitnessAllUnitsProofRequest {
            catalog: &catalog,
            schedule: &plan.run_plan.schedule,
            execution_units: &plan.units,
            gpu_streams: plan.run_plan.gpu.max_streams,
            public_values: Some(&public_values),
            outputs: &outputs,
            auxiliary_inputs: &request_auxiliary_inputs,
            unit_values: &[],
            evaluation_values_segment: None,
            verify_outputs: false,
            program_image_cache: None,
            eth_block_input: None,
            challenge_values_segment: None,
        },
    )
    .expect("proof artifact should build")
    .expect("proof artifact should exist");
    let proof = parse_proof_artifact(&encode_proof_artifact(&proof).expect("proof should encode"))
        .expect("proof should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let proof_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_PROOF_VALUES_SEGMENT_ID)
        .expect("proof values segment should exist");
    let proof_values = parse_pcs_proof_values_segment(&proof_values_segment.data)
        .expect("proof values should parse");
    assert_eq!(proof_values.values, vec![[31, 0, 0]]);
    assert!(proof
        .segments
        .iter()
        .any(|segment| segment.id == CONTRIBUTION_SEGMENT_ID));
}

#[test]
fn rejects_all_units_contribution_proof_artifact_with_mismatched_challenge_segment() {
    let dir = temp_dir("contribution-proof-bad-challenge");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [19_u8]).expect("input data should be written");

    let mut catalog = sample_catalog(sample_unit());
    catalog.layout.global_info.lattice_size = Some(32);
    declare_sample_public_value_metadata(&mut catalog);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values_path = dir.join("public.bin");
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash,
        values: vec![PublicValueEntry {
            name: "sample_public".to_owned(),
            elements: vec![19],
        }],
    };
    fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should be written");
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: Some(public_values_path),
        },
    )
    .expect("execution plan should derive");
    let outputs = vec![run_prove_witness_commitments_with_trace(
        &plan,
        0,
        ProveWitnessAuxiliaryInputs::default(),
    )
    .expect("unit should run")];
    let challenge_segment = ProofSegment {
        id: CHALLENGE_VALUES_SEGMENT_ID,
        data: encode_challenge_values_segment(&ChallengeValuesSegment {
            values: vec![[1, 2, 3]],
        })
        .expect("challenge values segment should encode"),
    };

    let error = lzvm_prover::build_witness_contribution_proof_artifact_for_all_units(
        &lzvm_prover::WitnessAllUnitsProofRequest {
            catalog: &catalog,
            schedule: &plan.run_plan.schedule,
            execution_units: &plan.units,
            gpu_streams: plan.run_plan.gpu.max_streams,
            public_values: Some(&public_values),
            outputs: &outputs,
            auxiliary_inputs: &ProveWitnessAuxiliaryInputs::default(),
            unit_values: &[],
            evaluation_values_segment: None,
            verify_outputs: true,
            program_image_cache: None,
            eth_block_input: None,
            challenge_values_segment: Some(&challenge_segment),
        },
    )
    .expect_err("mismatched challenge segment should reject");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        error,
        "verify contribution proof output failed: contribution challenge values mismatch"
    );
}

#[test]
fn rejects_contribution_challenge_mismatch_without_output_verification() {
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(format!(
            "lzvm-prover-witness-{}-contribution-proof-bad-challenge-no-verify",
            std::process::id()
        ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [19_u8]).expect("input data should be written");

    let mut catalog = sample_catalog(sample_unit());
    catalog.layout.global_info.lattice_size = Some(32);
    declare_sample_public_value_metadata(&mut catalog);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values_path = dir.join("public.bin");
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash,
        values: vec![PublicValueEntry {
            name: "sample_public".to_owned(),
            elements: vec![19],
        }],
    };
    fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should be written");
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: Some(public_values_path),
        },
    )
    .expect("execution plan should derive");
    let outputs = vec![run_prove_witness_commitments_with_trace(
        &plan,
        0,
        ProveWitnessAuxiliaryInputs::default(),
    )
    .expect("unit should run")];
    let challenge_segment = ProofSegment {
        id: CHALLENGE_VALUES_SEGMENT_ID,
        data: encode_challenge_values_segment(&ChallengeValuesSegment {
            values: vec![[1, 2, 3]],
        })
        .expect("challenge values segment should encode"),
    };

    let error = lzvm_prover::build_witness_contribution_proof_artifact_for_all_units(
        &lzvm_prover::WitnessAllUnitsProofRequest {
            catalog: &catalog,
            schedule: &plan.run_plan.schedule,
            execution_units: &plan.units,
            gpu_streams: plan.run_plan.gpu.max_streams,
            public_values: Some(&public_values),
            outputs: &outputs,
            auxiliary_inputs: &ProveWitnessAuxiliaryInputs::default(),
            unit_values: &[],
            evaluation_values_segment: None,
            verify_outputs: false,
            program_image_cache: None,
            eth_block_input: None,
            challenge_values_segment: Some(&challenge_segment),
        },
    )
    .expect_err("mismatched challenge segment should reject during construction");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        error,
        "verify contribution proof output failed: contribution challenge values mismatch"
    );
}

#[test]
fn builds_all_units_transcript_proof_artifact_from_output_evaluation_values() {
    let dir = temp_dir("proof-artifact-all-units-fri-output-evals");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [13_u8]).expect("input data should be written");

    let expression_id = 42;
    let mut first_unit = sample_unit();
    first_unit.paths.fixed_columns = dir.join("unit.const");
    first_unit.paths.constant_tree = dir.join("unit.consttree");
    first_unit.metadata.setup.challenge_count = 4;
    first_unit.pcs_plan =
        derive_pcs_setup_plan(&first_unit.metadata.setup).expect("PCS setup plan should derive");
    first_unit.metadata.verifier.quotient.expression_id = Some(expression_id);
    first_unit.expression_program = evaluation_plus_zero_expression_program(expression_id);
    write_sample_fixed_columns(
        &first_unit.paths.fixed_columns,
        &first_unit.metadata.setup,
        "unit-a",
    );
    let first_tree_bytes = expected_constant_tree_byte_count(&first_unit.metadata.setup)
        .expect("tree size should derive");
    fs::write(
        &first_unit.paths.constant_tree,
        vec![0_u8; first_tree_bytes],
    )
    .expect("first constant tree should be written");

    let mut second_unit = sample_unit();
    second_unit.paths.unit_id = Some(1);
    second_unit.paths.unit_name = Some("unit-b".to_owned());
    second_unit.paths.prefix = "unit-b".into();
    second_unit.paths.metadata_prefix = Some("unit-b".into());
    second_unit.paths.program_prefix = Some("unit-b".into());
    second_unit.paths.verification_key_prefix = "unit-b".into();
    second_unit.paths.fixed_columns = dir.join("unit-b.const");
    second_unit.paths.constant_tree = dir.join("unit-b.consttree");
    second_unit.metadata.setup.challenge_count = 4;
    second_unit.pcs_plan =
        derive_pcs_setup_plan(&second_unit.metadata.setup).expect("PCS setup plan should derive");
    second_unit.metadata.verifier.quotient.expression_id = Some(expression_id);
    second_unit.expression_program = evaluation_plus_zero_expression_program(expression_id);
    write_sample_fixed_columns(
        &second_unit.paths.fixed_columns,
        &second_unit.metadata.setup,
        "unit-b",
    );
    let second_tree_bytes = expected_constant_tree_byte_count(&second_unit.metadata.setup)
        .expect("tree size should derive");
    fs::write(
        &second_unit.paths.constant_tree,
        vec![0_u8; second_tree_bytes],
    )
    .expect("second constant tree should be written");

    let mut catalog = sample_catalog_units(vec![first_unit, second_unit]);
    declare_sample_public_value_metadata(&mut catalog);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values_path = dir.join("public.bin");
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash,
        values: vec![PublicValueEntry {
            name: "sample_public".to_owned(),
            elements: vec![13],
        }],
    };
    fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should be written");
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: Some(public_values_path),
        },
    )
    .expect("execution plan should derive");
    let evaluation_values = vec![Ext3::from_u64s([30, 31, 32]), Ext3::from_u64s([40, 41, 42])];
    let output_auxiliary_inputs = ProveWitnessAuxiliaryInputs {
        evaluations: evaluation_values.clone(),
        ..ProveWitnessAuxiliaryInputs::default()
    };
    let request_auxiliary_inputs = ProveWitnessAuxiliaryInputs::default();
    let outputs = vec![
        run_prove_witness_commitments_with_trace(&plan, 0, output_auxiliary_inputs.clone())
            .expect("first unit should run"),
        run_prove_witness_commitments_with_trace(&plan, 1, output_auxiliary_inputs.clone())
            .expect("second unit should run"),
    ];

    let proof = lzvm_prover::build_witness_proof_artifact_for_all_units(
        &lzvm_prover::WitnessAllUnitsProofRequest {
            catalog: &catalog,
            schedule: &plan.run_plan.schedule,
            execution_units: &plan.units,
            gpu_streams: plan.run_plan.gpu.max_streams,
            public_values: Some(&public_values),
            outputs: &outputs,
            auxiliary_inputs: &request_auxiliary_inputs,
            unit_values: &[],
            evaluation_values_segment: None,
            verify_outputs: false,
            program_image_cache: None,
            eth_block_input: None,
            challenge_values_segment: None,
        },
    )
    .expect("proof artifact should build")
    .expect("proof artifact should exist");
    let proof = parse_proof_artifact(&encode_proof_artifact(&proof).expect("proof should encode"))
        .expect("proof should parse");

    let evaluation_values_segment = build_pcs_evaluation_segment(
        &plan.run_plan.schedule,
        &[
            ProvePcsEvaluationValues {
                unit_index: 0,
                values: evaluation_values.clone(),
            },
            ProvePcsEvaluationValues {
                unit_index: 1,
                values: evaluation_values.clone(),
            },
        ],
    )
    .expect("evaluation segment should build");
    let outputs_without_evaluations = vec![
        run_prove_witness_commitments_with_trace(&plan, 0, ProveWitnessAuxiliaryInputs::default())
            .expect("first unit should run without evaluations"),
        run_prove_witness_commitments_with_trace(&plan, 1, ProveWitnessAuxiliaryInputs::default())
            .expect("second unit should run without evaluations"),
    ];
    let proof_from_segment = lzvm_prover::build_witness_proof_artifact_for_all_units(
        &lzvm_prover::WitnessAllUnitsProofRequest {
            catalog: &catalog,
            schedule: &plan.run_plan.schedule,
            execution_units: &plan.units,
            gpu_streams: plan.run_plan.gpu.max_streams,
            public_values: Some(&public_values),
            outputs: &outputs_without_evaluations,
            auxiliary_inputs: &request_auxiliary_inputs,
            unit_values: &[],
            evaluation_values_segment: Some(&evaluation_values_segment),
            verify_outputs: false,
            program_image_cache: None,
            eth_block_input: None,
            challenge_values_segment: None,
        },
    )
    .expect("proof artifact should build from evaluation segment")
    .expect("proof artifact should exist from evaluation segment");
    let proof_from_segment = parse_proof_artifact(
        &encode_proof_artifact(&proof_from_segment).expect("proof should encode"),
    )
    .expect("proof should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    for proof in [&proof, &proof_from_segment] {
        let evaluation_segment = proof
            .segments
            .iter()
            .find(|segment| segment.id == PCS_EVALUATION_SEGMENT_ID)
            .expect("evaluation segment should exist");
        let evaluations = parse_pcs_evaluation_segment(&evaluation_segment.data)
            .expect("evaluation segment should parse");
        assert_eq!(evaluations.units.len(), 2);
        assert_eq!(evaluations.units[0].unit_index, 0);
        assert_eq!(evaluations.units[1].unit_index, 1);
        assert_eq!(
            evaluations.units[0].values,
            vec![[30, 31, 32], [40, 41, 42]]
        );
        assert_eq!(
            evaluations.units[1].values,
            vec![[30, 31, 32], [40, 41, 42]]
        );
    }
}

#[test]
fn runs_witness_commitments_for_all_units_in_prover() {
    let dir = temp_dir("all-units-commitments");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [11_u8]).expect("input data should be written");

    let mut second_unit = sample_unit();
    second_unit.paths.unit_id = Some(1);
    second_unit.paths.unit_name = Some("unit-b".to_owned());
    second_unit.paths.prefix = "unit-b".into();
    second_unit.paths.metadata_prefix = Some("unit-b".into());
    second_unit.paths.program_prefix = Some("unit-b".into());
    second_unit.paths.verification_key_prefix = "unit-b".into();
    second_unit.paths.constant_tree = dir.join("unit-b.consttree");
    let mut first_unit = sample_unit();
    first_unit.paths.constant_tree = dir.join("unit.consttree");
    let first_tree_bytes = expected_constant_tree_byte_count(&first_unit.metadata.setup)
        .expect("tree size should derive");
    let second_tree_bytes = expected_constant_tree_byte_count(&second_unit.metadata.setup)
        .expect("tree size should derive");
    fs::write(
        &first_unit.paths.constant_tree,
        vec![0_u8; first_tree_bytes],
    )
    .expect("first constant tree should be written");
    fs::write(
        &second_unit.paths.constant_tree,
        vec![0_u8; second_tree_bytes],
    )
    .expect("second constant tree should be written");
    let catalog = sample_catalog_units(vec![first_unit, second_unit]);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let auxiliary_inputs = ProveWitnessAuxiliaryInputs::default();
    let trace_bundle = sample_trace_bundle(2, 17);

    let outputs = lzvm_prover::run_prove_witness_commitments_for_all_units_with_trace_bundle(
        &plan,
        &auxiliary_inputs,
        &trace_bundle,
    )
    .expect("batch witness commitments should run");
    let expected = vec![
        run_prove_witness_commitments_with_trace_backend(
            &plan,
            0,
            auxiliary_inputs.clone(),
            &TraceBytesBackend::new(sample_trace_bytes(17)),
        )
        .expect("first unit should run"),
        run_prove_witness_commitments_with_trace_backend(
            &plan,
            1,
            auxiliary_inputs.clone(),
            &TraceBytesBackend::new(sample_trace_bytes(18)),
        )
        .expect("second unit should run"),
    ];
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(outputs, expected);
    assert_eq!(outputs[0].commitments().unit_index(), 0);
    assert_eq!(outputs[1].commitments().unit_index(), 1);
}

#[test]
fn runs_all_units_with_cross_unit_source_lookup_balance() {
    let dir = temp_dir("all-units-cross-unit-source-lookup");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [11_u8]).expect("input data should be written");

    let mut first_unit = sample_unit();
    first_unit.paths.constant_tree = dir.join("unit-a.consttree");
    declare_source_lookup_commitment_column(&mut first_unit);
    first_unit.regular_hints = HintProgram {
        hints: vec![source_lookup_balance_hint(SOURCE_LOOKUP_PROVES_HINT)],
    };

    let mut second_unit = sample_unit();
    second_unit.paths.unit_id = Some(1);
    second_unit.paths.unit_name = Some("unit-b".to_owned());
    second_unit.paths.prefix = "unit-b".into();
    second_unit.paths.metadata_prefix = Some("unit-b".into());
    second_unit.paths.program_prefix = Some("unit-b".into());
    second_unit.paths.verification_key_prefix = "unit-b".into();
    second_unit.paths.constant_tree = dir.join("unit-b.consttree");
    declare_source_lookup_commitment_column(&mut second_unit);
    second_unit.regular_hints = HintProgram {
        hints: vec![source_lookup_balance_hint(SOURCE_LOOKUP_ASSUMES_HINT)],
    };

    let first_tree_bytes = expected_constant_tree_byte_count(&first_unit.metadata.setup)
        .expect("tree size should derive");
    let second_tree_bytes = expected_constant_tree_byte_count(&second_unit.metadata.setup)
        .expect("tree size should derive");
    fs::write(
        &first_unit.paths.constant_tree,
        vec![0_u8; first_tree_bytes],
    )
    .expect("first constant tree should be written");
    fs::write(
        &second_unit.paths.constant_tree,
        vec![0_u8; second_tree_bytes],
    )
    .expect("second constant tree should be written");

    let catalog = sample_catalog_units(vec![first_unit, second_unit]);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: None,
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let trace_bytes = sample_trace_bytes(17);
    let trace_bundle = TraceBundle {
        units: vec![
            TraceBundleUnit {
                unit_index: 0,
                trace_bytes: trace_bytes.clone(),
            },
            TraceBundleUnit {
                unit_index: 1,
                trace_bytes,
            },
        ],
    };

    let outputs = lzvm_prover::run_prove_witness_commitments_for_all_units_with_trace_bundle(
        &plan,
        &ProveWitnessAuxiliaryInputs::default(),
        &trace_bundle,
    )
    .expect("cross-unit source lookup balance should validate");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(outputs.len(), 2);
    assert_eq!(outputs[0].commitments().unit_index(), 0);
    assert_eq!(outputs[1].commitments().unit_index(), 1);
}

#[test]
fn preserves_trace_inputs_and_commitments_for_pcs_openings() {
    let dir = temp_dir("trace-commitments");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let public_inputs = dir.join("public-values.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.paths.fixed_columns = dir.join("unit.const");
    fs::write(&unit.paths.fixed_columns, vec![0_u8; 16 * 2 * 8])
        .expect("fixed columns should be written");
    unit.regular_constraints = public_row_zero_stage_constraint();
    let mut catalog = sample_catalog(unit);
    declare_sample_public_value_metadata(&mut catalog);
    write_public_values(
        &public_inputs,
        key_directory_catalog_digest(&catalog).expect("catalog digest should compute"),
        vec![8],
    );
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: Some(public_inputs),
        },
    )
    .expect("execution plan should derive");
    let auxiliary_inputs = ProveWitnessAuxiliaryInputs {
        proof_values: vec![Felt::from_u64(31)],
        challenges: vec![Ext3::from_u64s([41, 42, 43])],
        ..ProveWitnessAuxiliaryInputs::default()
    };

    let output = run_prove_witness_commitments_with_trace(&plan, 0, auxiliary_inputs.clone())
        .expect("trace commitments should run");
    let expected_commitments =
        run_prove_witness_commitments_with_auxiliary_inputs(&plan, 0, auxiliary_inputs.clone())
            .expect("witness commitments should run");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(output.commitments(), &expected_commitments);
    assert_eq!(
        output.trace().row_count(),
        expected_commitments.trace_row_count()
    );
    assert_eq!(
        output.trace().column_count(),
        expected_commitments.trace_column_count()
    );
    assert_eq!(output.trace().value(0, 0), Some(Felt::from_u64(8)));
    assert_eq!(output.publics(), &[Felt::from_u64(8)]);
    assert_eq!(output.auxiliary_inputs(), &auxiliary_inputs);
}

#[test]
fn runs_witness_commitments_with_native_trace_backend() {
    let dir = temp_dir("native-trace-backend");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = dir.join("witness.elf");
    fs::write(&witness_library, sample_witness_library())
        .expect("witness library should be written");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let catalog = sample_catalog(sample_unit());
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");

    let mut trace_bytes = Vec::with_capacity(16 * 5 * 8);
    for value in 1_u64..=80 {
        trace_bytes.extend_from_slice(&value.to_le_bytes());
    }
    let backend = TraceBytesBackend::new(trace_bytes);

    let output = run_prove_witness_commitments_with_trace_backend(
        &plan,
        0,
        ProveWitnessAuxiliaryInputs::default(),
        &backend,
    )
    .expect("witness commitments should run");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(output.trace().row_count(), 16);
    assert_eq!(output.trace().column_count(), 5);
    assert_eq!(output.trace().value(0, 0), Some(Felt::from_u64(1)));
    assert_eq!(output.trace().value(15, 4), Some(Felt::from_u64(80)));
}

#[test]
fn rejects_default_witness_run_without_witness_library() {
    let dir = temp_dir("missing-runtime-library");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let catalog = sample_catalog(sample_unit());
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: None,
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");

    let error = run_prove_witness_commitments(&plan, 0)
        .expect_err("default witness run should require a library");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        error,
        ProveWitnessCommitmentError::MissingWitnessLibrary
    ));
}

#[test]
fn rejects_witness_traces_that_violate_regular_constraints() {
    let dir = temp_dir("constraint-violation");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.paths.fixed_columns = dir.join("unit.const");
    fs::write(&unit.paths.fixed_columns, vec![0_u8; 16 * 2 * 8])
        .expect("fixed columns should be written");
    unit.regular_constraints = row_zero_stage_constraint(9);
    let catalog = sample_catalog(unit);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");

    let error = run_prove_witness_commitments(&plan, 0)
        .expect_err("constraint violation should reject witness trace");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(error.to_string().contains("regular constraint"));
}

#[test]
fn uses_public_inputs_when_checking_regular_constraints() {
    let dir = temp_dir("public-constraint");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let public_inputs = dir.join("public-values.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.paths.fixed_columns = dir.join("unit.const");
    fs::write(&unit.paths.fixed_columns, vec![0_u8; 16 * 2 * 8])
        .expect("fixed columns should be written");
    unit.regular_constraints = public_row_zero_stage_constraint();
    let mut catalog = sample_catalog(unit);
    declare_sample_public_value_metadata(&mut catalog);
    write_public_values(
        &public_inputs,
        key_directory_catalog_digest(&catalog).expect("catalog digest should compute"),
        vec![8],
    );
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: Some(public_inputs),
        },
    )
    .expect("execution plan should derive");

    let output = run_prove_witness_commitments(&plan, 0)
        .expect("public-valued regular constraint should pass");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(output.unit_index(), 0);
    assert_eq!(output.trace_row_count(), 16);
}

#[test]
fn uses_domain_helpers_when_checking_regular_constraints() {
    let dir = temp_dir("domain-helper-constraint");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.paths.fixed_columns = dir.join("unit.const");
    fs::write(&unit.paths.fixed_columns, vec![0_u8; 16 * 2 * 8])
        .expect("fixed columns should be written");
    unit.regular_constraints = domain_helper_row_zero_stage_constraint();
    let catalog = sample_catalog(unit);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");

    let output =
        run_prove_witness_commitments(&plan, 0).expect("domain helper constraint should pass");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(output.unit_index(), 0);
    assert_eq!(output.trace_row_count(), 16);
}

#[test]
fn uses_proof_values_when_checking_regular_constraints() {
    let dir = temp_dir("proof-value-constraint");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.paths.fixed_columns = dir.join("unit.const");
    fs::write(&unit.paths.fixed_columns, vec![0_u8; 16 * 2 * 8])
        .expect("fixed columns should be written");
    unit.regular_constraints = proof_value_row_zero_stage_constraint();
    let catalog = sample_catalog(unit);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");

    let output = run_prove_witness_commitments_with_auxiliary_inputs(
        &plan,
        0,
        ProveWitnessAuxiliaryInputs {
            proof_values: vec![Felt::from_canonical(8).expect("value should be canonical")],
            ..ProveWitnessAuxiliaryInputs::default()
        },
    )
    .expect("proof-valued regular constraint should pass");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(output.unit_index(), 0);
    assert_eq!(output.trace_row_count(), 16);
}

#[test]
fn rejects_regular_hints_with_missing_proof_values() {
    let dir = temp_dir("missing-regular-hint-proof-value");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.regular_hints = proof_value_regular_hint();
    let catalog = sample_catalog(unit);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");

    let error = run_prove_witness_commitments(&plan, 0)
        .expect_err("missing proof value input should reject regular hint check");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(
        error
            .to_string()
            .contains("missing regular hint proof value input"),
        "{error}"
    );
}

#[test]
fn uses_proof_values_when_checking_regular_hints() {
    let dir = temp_dir("proof-value-regular-hint");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.regular_hints = proof_value_regular_hint();
    let catalog = sample_catalog(unit);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");

    let output = run_prove_witness_commitments_with_auxiliary_inputs(
        &plan,
        0,
        ProveWitnessAuxiliaryInputs {
            proof_values: vec![Felt::from_canonical(8).expect("value should be canonical")],
            ..ProveWitnessAuxiliaryInputs::default()
        },
    )
    .expect("proof-valued regular hint should pass");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(output.unit_index(), 0);
    assert_eq!(output.trace_row_count(), 16);
}

#[test]
fn reports_missing_challenges_for_regular_constraints() {
    let dir = temp_dir("missing-challenge-constraint");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.paths.fixed_columns = dir.join("unit.const");
    fs::write(&unit.paths.fixed_columns, vec![0_u8; 16 * 2 * 8])
        .expect("fixed columns should be written");
    unit.regular_constraints = challenge_row_zero_stage_constraint();
    let catalog = sample_catalog(unit);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");

    let error = run_prove_witness_commitments(&plan, 0)
        .expect_err("missing challenge input should reject regular constraint check");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(
        error
            .to_string()
            .contains("missing regular constraint challenge input"),
        "{error}"
    );
}

#[test]
fn builds_witness_commitment_proof_segments() {
    let dir = temp_dir("segment");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [13_u8]).expect("input data should be written");

    let catalog = sample_catalog(sample_unit());
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let output = run_prove_witness_commitments(&plan, 0).expect("witness commitments should run");

    let segment = build_witness_commitment_segment(&output).expect("witness segment should build");
    let parsed =
        parse_witness_commitment_segment(&segment.data).expect("witness segment should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(segment.id, WITNESS_COMMITMENT_SEGMENT_BASE_ID);
    assert_eq!(parsed.unit_index, 0);
    assert_eq!(parsed.input_byte_count, 1);
    assert_eq!(parsed.trace_rows, 16);
    assert_eq!(parsed.trace_columns, 5);
    assert_eq!(
        parsed.stages.len(),
        output.stage_commitments().stage_count()
    );
    for (stage, commitment) in parsed
        .stages
        .iter()
        .zip(output.stage_commitments().commitments())
    {
        assert_eq!(stage.stage_index, commitment.stage_index() as u32);
        assert_eq!(stage.arity, commitment.arity() as u32);
        assert_eq!(stage.root, commitment.root().map(|value| value.to_u64()));
        assert_eq!(stage.tree_byte_count, commitment.tree_bytes().len() as u64);
        let expected_digest: [u8; 32] = Sha256::digest(commitment.tree_bytes()).into();
        assert_eq!(stage.tree_digest, expected_digest);
    }
}

#[test]
fn builds_pcs_query_plan_segments_from_proof_inputs() {
    let dir = temp_dir("query-plan");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [17_u8]).expect("input data should be written");

    let catalog = sample_catalog(sample_unit());
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let output = run_prove_witness_commitments(&plan, 0).expect("witness commitments should run");
    let material_segment = build_pcs_material_manifest_segment(&plan.run_plan.schedule)
        .expect("material segment should build");
    let witness_segment =
        build_witness_commitment_segment(&output).expect("witness segment should build");

    let query_segment = build_pcs_query_plan_segment(
        &plan.run_plan.schedule,
        [0x44; 32],
        &material_segment,
        std::slice::from_ref(&witness_segment),
    )
    .expect("query segment should build");
    let repeat = build_pcs_query_plan_segment(
        &plan.run_plan.schedule,
        [0x44; 32],
        &material_segment,
        std::slice::from_ref(&witness_segment),
    )
    .expect("query segment should build again");
    let parsed =
        parse_pcs_query_plan_segment(&query_segment.data).expect("query segment should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(query_segment.id, PCS_QUERY_PLAN_SEGMENT_ID);
    assert_eq!(query_segment, repeat);
    assert_eq!(parsed.units.len(), 1);
    assert_eq!(parsed.units[0].unit_index, 0);
    assert_eq!(
        parsed.units[0].queries.len(),
        plan.run_plan.schedule.units[0].query_count as usize
    );
    for query in &parsed.units[0].queries {
        assert!(*query < plan.run_plan.schedule.units[0].extended_domain_size);
    }
}

#[test]
fn builds_pcs_query_plan_segments_from_transcript_challenge() {
    let dir = temp_dir("query-plan-transcript");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [23_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.pcs_plan.proof_of_work_bits = 0;
    unit.metadata.setup.stark.pow_bits = 0;
    let catalog = sample_catalog(unit);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let output = run_prove_witness_commitments(&plan, 0).expect("witness commitments should run");
    let witness_segment =
        build_witness_commitment_segment(&output).expect("witness segment should build");
    let challenge = Ext3::from_u64s([11, 22, 33]);
    let nonce = Felt::ZERO;

    let query_segment = build_pcs_query_plan_segment_from_challenge(
        &plan.run_plan.schedule,
        std::slice::from_ref(&witness_segment),
        challenge,
        nonce,
    )
    .expect("query segment should build");
    let parsed =
        parse_pcs_query_plan_segment(&query_segment.data).expect("query segment should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let unit = &plan.run_plan.schedule.units[0];
    let expected_queries = derive_fri_queries(
        unit.transcript_arity
            .expect("transcript arity should exist") as usize,
        challenge,
        nonce,
        unit.query_count as usize,
        unit.extended_domain_bits,
    )
    .expect("query sampling should fit");
    assert_eq!(query_segment.id, PCS_QUERY_PLAN_SEGMENT_ID);
    assert_eq!(parsed.units.len(), 1);
    assert_eq!(parsed.units[0].unit_index, 0);
    assert_eq!(parsed.units[0].queries, expected_queries);
}

#[test]
fn builds_pcs_query_nonce_segments_for_transcript_query_plans() {
    let catalog = sample_catalog(sample_unit());
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let challenge = Ext3::from_u64s([7, 8, 9]);

    let nonce_segment =
        build_pcs_query_nonce_segment(&schedule, challenge).expect("nonce segment should build");
    let parsed =
        parse_pcs_query_nonce_segment(&nonce_segment.data).expect("nonce segment should parse");

    assert_eq!(nonce_segment.id, PCS_QUERY_NONCE_SEGMENT_ID);
    assert!(verify_query_nonce(
        challenge,
        Felt::from_u64(parsed.nonce),
        schedule.units[0].proof_of_work_bits
    )
    .expect("nonce should verify"));
}

#[test]
fn builds_pcs_evaluation_segments_from_values() {
    let catalog = sample_catalog(sample_unit());
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let values = vec![Ext3::from_u64s([30, 31, 32]), Ext3::from_u64s([40, 41, 42])];

    let segment = build_pcs_evaluation_segment(
        &schedule,
        &[ProvePcsEvaluationValues {
            unit_index: 0,
            values: values.clone(),
        }],
    )
    .expect("evaluation segment should build");
    let parsed =
        parse_pcs_evaluation_segment(&segment.data).expect("evaluation segment should parse");

    assert_eq!(segment.id, PCS_EVALUATION_SEGMENT_ID);
    assert_eq!(parsed.units.len(), 1);
    assert_eq!(parsed.units[0].unit_index, 0);
    assert_eq!(
        parsed.units[0].values,
        values.into_iter().map(Ext3::to_u64s).collect::<Vec<_>>()
    );
}

#[test]
fn builds_pcs_fri_opening_segments_from_polynomial_values() {
    let catalog = sample_catalog(sample_unit());
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let unit = &schedule.units[0];
    let query_plan = PcsQueryPlanSegment {
        units: vec![PcsQueryPlanUnit {
            unit_index: 0,
            queries: vec![1, unit.extended_domain_size - 1],
        }],
    };
    let query_segment = ProofSegment {
        id: PCS_QUERY_PLAN_SEGMENT_ID,
        data: encode_pcs_query_plan_segment(&query_plan).expect("query segment should encode"),
    };
    let polynomial = (0..unit.extended_domain_size)
        .map(|index| Ext3::from_u64s([index + 1, index + 101, index + 201]))
        .collect::<Vec<_>>();
    let mut challenges = vec![Ext3::ZERO; unit.challenge_count + unit.fri_layers.len() + 1];
    challenges[unit.challenge_count + 1] = Ext3::from_u64s([17, 18, 19]);

    let segment = build_pcs_fri_opening_segment(
        &schedule,
        &query_segment,
        &[ProvePcsFriOpeningValues {
            unit_index: 0,
            challenges: challenges.clone(),
            polynomial,
        }],
    )
    .expect("FRI opening segment should build");
    let parsed = parse_pcs_fri_opening_segment(&segment.data).expect("FRI segment should parse");

    assert_eq!(segment.id, PCS_FRI_OPENING_SEGMENT_ID);
    assert_eq!(parsed.units.len(), 1);
    assert_eq!(parsed.units[0].unit_index, 0);
    assert_eq!(parsed.units[0].layers.len(), unit.fri_layers.len());
    assert_eq!(
        parsed.units[0].layers[0].queries.len(),
        unit.query_count as usize
    );
    assert_eq!(parsed.units[0].layers[0].queries[0].row_index, 1);
    assert_eq!(parsed.units[0].layers[0].queries[1].row_index, 15);
    assert_eq!(
        parsed.units[0].final_polynomial.len(),
        1_usize << unit.final_layer_bits
    );
    assert!(verify_fri_opening_folds(
        unit,
        PcsFriOpeningFoldRequest {
            unit_index: 0,
            query_rows: &query_plan.units[0].queries,
            challenges: &challenges,
            fri: &parsed.units[0],
        },
    )
    .expect("folds should verify"));
}

#[test]
fn builds_pcs_fri_polynomial_from_execution_material() {
    let dir = temp_dir("fri-polynomial");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");

    let expression_id = 42;
    let mut unit = sample_unit();
    unit.paths.fixed_columns = dir.join("unit.const");
    unit.metadata.verifier.quotient.expression_id = Some(expression_id);
    unit.expression_program = fixed_plus_stage_expression_program(expression_id);

    let fixed_left = (0..16).map(|row| row + 10).collect::<Vec<_>>();
    let fixed_columns = FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 16,
        columns: vec![
            FixedColumn {
                name: "const_0".to_owned(),
                dimensions: Vec::new(),
                values: fixed_left.clone(),
            },
            FixedColumn {
                name: "const_1".to_owned(),
                dimensions: Vec::new(),
                values: vec![0; 16],
            },
        ],
    };
    write_raw_fixed_columns_file(
        &unit.paths.fixed_columns,
        &fixed_columns,
        &unit.metadata.setup,
    )
    .expect("fixed columns should be written");

    let catalog = sample_catalog(unit);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), None),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let unit = &plan.run_plan.schedule.units[0];
    let trace_words = (0..16 * 5)
        .map(|index| index as u64 + 1)
        .collect::<Vec<_>>();
    let trace =
        lzvm_prover::witness_trace::parse_witness_trace(&encode_trace_words(&trace_words), 16, 5)
            .expect("trace should parse");

    let polynomial = build_pcs_fri_polynomial_values(
        0,
        unit,
        &plan.units[0],
        &trace,
        &[],
        &ProveWitnessAuxiliaryInputs::default(),
        Ext3::from_u64s([3, 0, 0]),
    )
    .expect("polynomial should build");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let fixed_extended = coset_extend_evaluations(
        &fixed_left
            .iter()
            .copied()
            .map(Felt::from_u64)
            .collect::<Vec<_>>(),
        4,
        6,
    )
    .expect("fixed column should extend");
    let stage_source = (0..16)
        .map(|row| Felt::from_u64(trace_words[row * 5]))
        .collect::<Vec<_>>();
    let stage_extended =
        coset_extend_evaluations(&stage_source, 4, 6).expect("stage column should extend");
    let expected = fixed_extended
        .iter()
        .zip(stage_extended.iter())
        .map(|(fixed, stage)| Ext3::new(*fixed + *stage, Felt::ZERO, Felt::ZERO))
        .collect::<Vec<_>>();

    assert_eq!(polynomial, expected);
}

#[test]
fn builds_pcs_fri_opening_segments_from_execution_material() {
    let dir = temp_dir("fri-opening-from-trace");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");

    let expression_id = 42;
    let mut unit = sample_unit();
    unit.paths.fixed_columns = dir.join("unit.const");
    unit.metadata.verifier.quotient.expression_id = Some(expression_id);
    unit.expression_program = fixed_plus_stage_expression_program(expression_id);
    let fixed_columns = FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 16,
        columns: vec![
            FixedColumn {
                name: "const_0".to_owned(),
                dimensions: Vec::new(),
                values: (0..16).map(|row| row + 10).collect(),
            },
            FixedColumn {
                name: "const_1".to_owned(),
                dimensions: Vec::new(),
                values: vec![0; 16],
            },
        ],
    };
    write_raw_fixed_columns_file(
        &unit.paths.fixed_columns,
        &fixed_columns,
        &unit.metadata.setup,
    )
    .expect("fixed columns should be written");

    let catalog = sample_catalog(unit);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), None),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let unit = &plan.run_plan.schedule.units[0];
    let query_plan = PcsQueryPlanSegment {
        units: vec![PcsQueryPlanUnit {
            unit_index: 0,
            queries: vec![1, unit.extended_domain_size - 1],
        }],
    };
    let query_segment = ProofSegment {
        id: PCS_QUERY_PLAN_SEGMENT_ID,
        data: encode_pcs_query_plan_segment(&query_plan).expect("query segment should encode"),
    };
    let trace_words = (0..16 * 5)
        .map(|index| index as u64 + 1)
        .collect::<Vec<_>>();
    let trace =
        lzvm_prover::witness_trace::parse_witness_trace(&encode_trace_words(&trace_words), 16, 5)
            .expect("trace should parse");
    let mut challenges = vec![Ext3::ZERO; unit.challenge_count + unit.fri_layers.len() + 1];
    challenges[unit.challenge_count + 1] = Ext3::from_u64s([17, 18, 19]);
    let auxiliary = ProveWitnessAuxiliaryInputs::default();

    let segment = build_pcs_fri_opening_segment_from_trace(
        &plan.run_plan.schedule,
        &query_segment,
        &[ProvePcsFriOpeningTraceValues {
            unit_index: 0,
            execution_unit: &plan.units[0],
            trace: &trace,
            publics: &[],
            auxiliary_inputs: &auxiliary,
            challenges: &challenges,
            xi_challenge: Ext3::from_u64s([3, 0, 0]),
        }],
    )
    .expect("FRI opening segment should build");
    let parsed = parse_pcs_fri_opening_segment(&segment.data).expect("FRI segment should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(segment.id, PCS_FRI_OPENING_SEGMENT_ID);
    assert_eq!(parsed.units.len(), 1);
    assert!(verify_fri_opening_folds(
        unit,
        PcsFriOpeningFoldRequest {
            unit_index: 0,
            query_rows: &query_plan.units[0].queries,
            challenges: &challenges,
            fri: &parsed.units[0],
        },
    )
    .expect("folds should verify"));
}

#[test]
fn builds_pcs_fri_transcript_values_from_execution_material() {
    let dir = temp_dir("fri-transcript-from-trace");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");

    let expression_id = 42;
    let mut unit = sample_unit();
    unit.paths.fixed_columns = dir.join("unit.const");
    unit.metadata.verifier.quotient.expression_id = Some(expression_id);
    unit.expression_program = fixed_plus_stage_expression_program(expression_id);
    let fixed_columns = FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 16,
        columns: vec![
            FixedColumn {
                name: "const_0".to_owned(),
                dimensions: Vec::new(),
                values: (0..16).map(|row| row + 10).collect(),
            },
            FixedColumn {
                name: "const_1".to_owned(),
                dimensions: Vec::new(),
                values: vec![0; 16],
            },
        ],
    };
    write_raw_fixed_columns_file(
        &unit.paths.fixed_columns,
        &fixed_columns,
        &unit.metadata.setup,
    )
    .expect("fixed columns should be written");

    let catalog = sample_catalog(unit);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), None),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let unit = &plan.run_plan.schedule.units[0];
    let trace_words = (0..16 * 5)
        .map(|index| index as u64 + 1)
        .collect::<Vec<_>>();
    let trace =
        lzvm_prover::witness_trace::parse_witness_trace(&encode_trace_words(&trace_words), 16, 5)
            .expect("trace should parse");
    let constant_root = unit
        .pcs_material_constant_tree_root
        .expect("constant root should be present")
        .map(Felt::from_u64);
    let witness = sample_witness_commitment_segment(0, &[10, 20]);
    let witness_roots = witness
        .stages
        .iter()
        .map(|stage| stage.root.map(Felt::from_u64))
        .collect::<Vec<_>>();
    let witness_segment = ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
        data: encode_witness_commitment_segment(&witness).expect("witness segment should encode"),
    };
    let evaluations = vec![Ext3::from_u64s([30, 31, 32]), Ext3::from_u64s([40, 41, 42])];
    let auxiliary = ProveWitnessAuxiliaryInputs::default();

    let values = build_pcs_fri_transcript_values_from_trace(
        &plan.run_plan.schedule,
        &[ProvePcsFriTranscriptTraceValues {
            unit_index: 0,
            execution_unit: &plan.units[0],
            trace: &trace,
            publics: &[],
            auxiliary_inputs: &auxiliary,
            constant_root,
            witness_roots: &witness_roots,
            evaluation_values: &evaluations,
            xi_challenge: Ext3::from_u64s([3, 0, 0]),
            binding_segments: &[],
        }],
    )
    .expect("FRI transcript values should build");
    let transcript_value = &values[0];
    let nonce_segment = build_pcs_query_nonce_segment(
        &plan.run_plan.schedule,
        transcript_value.commitments.final_query_challenge,
    )
    .expect("nonce segment should build");
    let nonce = Felt::from_u64(
        parse_pcs_query_nonce_segment(&nonce_segment.data)
            .expect("nonce")
            .nonce,
    );
    let query_segment = build_pcs_query_plan_segment_from_challenge(
        &plan.run_plan.schedule,
        std::slice::from_ref(&witness_segment),
        transcript_value.commitments.final_query_challenge,
        nonce,
    )
    .expect("query plan should build");
    let query_plan =
        parse_pcs_query_plan_segment(&query_segment.data).expect("query plan should parse");
    let opening_segment = build_pcs_fri_opening_segment(
        &plan.run_plan.schedule,
        &query_segment,
        &[ProvePcsFriOpeningValues {
            unit_index: transcript_value.unit_index,
            challenges: transcript_value.commitments.challenges.clone(),
            polynomial: transcript_value.polynomial.clone(),
        }],
    )
    .expect("FRI opening segment should build");
    let opening =
        parse_pcs_fri_opening_segment(&opening_segment.data).expect("FRI opening should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let expected_challenges = derive_pcs_transcript_challenges(PcsTranscriptInputs {
        arity: unit.transcript_arity.expect("transcript arity") as usize,
        hash_values: unit.hash_commits,
        constant_root,
        public_values: &[],
        witness_roots: &witness_roots,
        root_challenge_draws: &unit.transcript_root_challenge_draws,
        unit_value_map: &unit.unit_value_map,
        unit_values: &auxiliary.unit_values,
        evaluation_values: &evaluations,
        evaluation_challenge_draws: unit.transcript_evaluation_challenge_draws,
        fri_roots: &transcript_value.commitments.layer_roots,
        final_polynomial: &transcript_value.commitments.final_polynomial,
        binding_segments: &[],
    })
    .expect("transcript challenges should derive");

    assert_eq!(values.len(), 1);
    assert_eq!(transcript_value.unit_index, 0);
    assert_eq!(transcript_value.commitments.challenges, expected_challenges);
    assert_eq!(
        transcript_value.commitments.layer_roots[0].map(Felt::to_u64),
        opening.units[0].layers[0].root
    );
    assert!(verify_fri_opening_folds(
        unit,
        PcsFriOpeningFoldRequest {
            unit_index: 0,
            query_rows: &query_plan.units[0].queries,
            challenges: &transcript_value.commitments.challenges,
            fri: &opening.units[0],
        },
    )
    .expect("folds should verify"));
}

#[test]
fn rejects_pcs_evaluation_segments_with_wrong_value_count() {
    let catalog = sample_catalog(sample_unit());
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");

    let result = build_pcs_evaluation_segment(
        &schedule,
        &[ProvePcsEvaluationValues {
            unit_index: 0,
            values: vec![Ext3::from_u64s([30, 31, 32])],
        }],
    );

    assert!(matches!(
        result,
        Err(
            lzvm_prover::ProvePcsEvaluationSegmentError::ValueCountMismatch {
                unit_index: 0,
                expected: 2,
                found: 1
            }
        )
    ));
}

#[test]
fn builds_pcs_query_nonce_segments_from_transcript_segments() {
    let fixture = sample_transcript_query_fixture();
    let transcript_inputs = fixture.inputs();

    let challenge = derive_pcs_final_query_challenge_from_segments(transcript_inputs)
        .expect("challenge should derive");
    let nonce_segment = build_pcs_query_nonce_segment_from_transcript_segments(
        &fixture.schedule,
        transcript_inputs,
    )
    .expect("nonce segment should build");
    let parsed =
        parse_pcs_query_nonce_segment(&nonce_segment.data).expect("nonce segment should parse");

    assert_eq!(nonce_segment.id, PCS_QUERY_NONCE_SEGMENT_ID);
    assert!(verify_query_nonce(
        challenge,
        Felt::from_u64(parsed.nonce),
        fixture.schedule.units[0].proof_of_work_bits
    )
    .expect("nonce should verify"));
}

#[test]
fn builds_pcs_query_plan_segments_from_transcript_segments() {
    let fixture = sample_transcript_query_fixture();
    let transcript_inputs = fixture.inputs();

    let challenge = derive_pcs_final_query_challenge_from_segments(transcript_inputs)
        .expect("challenge should derive");
    let nonce_segment = build_pcs_query_nonce_segment_from_transcript_segments(
        &fixture.schedule,
        transcript_inputs,
    )
    .expect("nonce segment should build");
    let nonce = Felt::from_u64(
        parse_pcs_query_nonce_segment(&nonce_segment.data)
            .expect("nonce segment should parse")
            .nonce,
    );
    let query_segment = build_pcs_query_plan_segment_from_transcript_segments(
        &fixture.schedule,
        std::slice::from_ref(&fixture.witness_segment),
        transcript_inputs,
        &nonce_segment,
    )
    .expect("query segment should build");
    let expected = build_pcs_query_plan_segment_from_challenge(
        &fixture.schedule,
        std::slice::from_ref(&fixture.witness_segment),
        challenge,
        nonce,
    )
    .expect("challenge query segment should build");

    assert_eq!(query_segment.id, PCS_QUERY_PLAN_SEGMENT_ID);
    assert_eq!(query_segment, expected);
}

#[test]
fn rejects_pcs_query_plan_segments_with_wrong_nonce_segment_id() {
    let fixture = sample_transcript_query_fixture();
    let transcript_inputs = fixture.inputs();
    let mut nonce_segment = build_pcs_query_nonce_segment_from_transcript_segments(
        &fixture.schedule,
        transcript_inputs,
    )
    .expect("nonce segment should build");
    nonce_segment.id = PCS_QUERY_PLAN_SEGMENT_ID;

    let result = build_pcs_query_plan_segment_from_transcript_segments(
        &fixture.schedule,
        std::slice::from_ref(&fixture.witness_segment),
        transcript_inputs,
        &nonce_segment,
    );

    assert!(matches!(
        result,
        Err(ProvePcsQueryPlanSegmentError::InvalidNonceSegmentId { segment_id })
            if segment_id == PCS_QUERY_PLAN_SEGMENT_ID
    ));
}

#[test]
fn builds_witness_opening_segments_from_query_plans() {
    let dir = temp_dir("openings");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [19_u8]).expect("input data should be written");

    let catalog = sample_catalog(sample_unit());
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let output = run_prove_witness_commitments(&plan, 0).expect("witness commitments should run");
    let material_segment = build_pcs_material_manifest_segment(&plan.run_plan.schedule)
        .expect("material segment should build");
    let witness_segment =
        build_witness_commitment_segment(&output).expect("witness segment should build");
    let query_segment = build_pcs_query_plan_segment(
        &plan.run_plan.schedule,
        [0x55; 32],
        &material_segment,
        std::slice::from_ref(&witness_segment),
    )
    .expect("query segment should build");

    let opening_segment =
        build_witness_opening_segment(&plan.run_plan.schedule, &query_segment, &output)
            .expect("opening segment should build");
    let query_plan =
        parse_pcs_query_plan_segment(&query_segment.data).expect("query segment should parse");
    let opening =
        parse_witness_opening_segment(&opening_segment.data).expect("opening segment should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let unit = &plan.run_plan.schedule.units[0];
    assert_eq!(opening_segment.id, WITNESS_OPENING_SEGMENT_ID);
    assert_eq!(opening.units.len(), 1);
    assert_eq!(opening.units[0].unit_index, 0);
    assert_eq!(opening.units[0].queries.len(), unit.query_count as usize);
    for (query, expected_row) in opening.units[0]
        .queries
        .iter()
        .zip(query_plan.units[0].queries.iter())
    {
        assert_eq!(query.row_index, *expected_row);
        assert_eq!(query.stages.len(), unit.stage_commit_widths.len());
        for (stage, width) in query.stages.iter().zip(unit.stage_commit_widths.iter()) {
            assert_eq!(stage.values.len(), *width as usize);
            assert_eq!(stage.siblings.len(), 3);
        }
    }
}

#[test]
fn builds_witness_opening_segment_for_all_query_units() {
    let dir = temp_dir("openings-all-units");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [29_u8]).expect("input data should be written");

    let mut second_unit = sample_unit();
    second_unit.paths.unit_id = Some(1);
    second_unit.paths.unit_name = Some("unit-b".to_owned());
    second_unit.paths.prefix = "unit-b".into();
    second_unit.paths.metadata_prefix = Some("unit-b".into());
    second_unit.paths.program_prefix = Some("unit-b".into());
    second_unit.paths.verification_key_prefix = "unit-b".into();
    let catalog = sample_catalog_units(vec![sample_unit(), second_unit]);
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let first = run_prove_witness_commitments(&plan, 0).expect("first unit should run");
    let second = run_prove_witness_commitments(&plan, 1).expect("second unit should run");
    let material_segment = build_pcs_material_manifest_segment(&plan.run_plan.schedule)
        .expect("material segment should build");
    let witness_segments = vec![
        build_witness_commitment_segment(&first).expect("first witness segment should build"),
        build_witness_commitment_segment(&second).expect("second witness segment should build"),
    ];
    let query_segment = build_pcs_query_plan_segment(
        &plan.run_plan.schedule,
        [0x66; 32],
        &material_segment,
        &witness_segments,
    )
    .expect("query segment should build");

    let opening_segment = build_witness_opening_segment_batch(
        &plan.run_plan.schedule,
        &query_segment,
        &[&first, &second],
    )
    .expect("opening segment should build");
    let query_plan =
        parse_pcs_query_plan_segment(&query_segment.data).expect("query segment should parse");
    let opening =
        parse_witness_opening_segment(&opening_segment.data).expect("opening segment should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(opening_segment.id, WITNESS_OPENING_SEGMENT_ID);
    assert_eq!(query_plan.units.len(), 2);
    assert_eq!(opening.units.len(), 2);
    assert_eq!(opening.units[0].unit_index, 0);
    assert_eq!(opening.units[1].unit_index, 1);
    for (opening_unit, query_unit) in opening.units.iter().zip(query_plan.units.iter()) {
        assert_eq!(opening_unit.unit_index, query_unit.unit_index);
        assert_eq!(opening_unit.queries.len(), query_unit.queries.len());
    }
}

#[test]
fn rejects_missing_witness_input_data_when_running_commitments() {
    let dir = temp_dir("missing-input");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("missing.bin");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");

    let catalog = sample_catalog(sample_unit());
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data.clone())),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");

    let result = run_prove_witness_commitments(&plan, 0);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(ProveWitnessCommitmentError::InputData { path, .. }) if path == input_data
    ));
}

#[test]
fn rejects_witness_commitment_unit_indexes_outside_the_schedule() {
    let dir = temp_dir("bad-unit");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");

    let catalog = sample_catalog(sample_unit());
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), None),
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");

    let result = run_prove_witness_commitments(&plan, 1);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(ProveWitnessCommitmentError::UnitIndexOutOfRange {
            unit_index: 1,
            unit_count: 1
        })
    ));
}
