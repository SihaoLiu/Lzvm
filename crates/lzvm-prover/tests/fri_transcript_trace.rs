use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::expression_program::{ExpressionEntry, ExpressionProgram};
use lzvm_artifacts::fixed::{write_raw_fixed_columns_file, FixedColumn, FixedColumns};
use lzvm_artifacts::global_info::{CurveKind, GlobalInfo};
use lzvm_artifacts::key_directory::{
    KeyDirectoryCatalog, KeyDirectoryLayout, KeyUnitCatalogEntry, KeyUnitKind, KeyUnitPaths,
};
use lzvm_artifacts::metadata_bundle::UnitMetadataBundle;
use lzvm_artifacts::pcs_evaluation_segment::parse_pcs_evaluation_segment;
use lzvm_artifacts::pcs_fri_segment::parse_pcs_fri_opening_segment;
use lzvm_artifacts::pcs_nonce_segment::parse_pcs_query_nonce_segment;
use lzvm_artifacts::pcs_plan::derive_pcs_setup_plan;
use lzvm_artifacts::pcs_query_segment::parse_pcs_query_plan_segment;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::setup_info::{FriStep, StarkStruct, UnitSetupInfo};
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_artifacts::verifier_info::{VerifierCode, VerifierInfo};
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, WitnessCommitmentSegment, WitnessCommitmentStageSegment,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::pcs_fri::{verify_fri_opening_folds, PcsFriOpeningFoldRequest};
use lzvm_prover::pcs_transcript::{derive_pcs_transcript_challenges, PcsTranscriptInputs};
use lzvm_prover::witness_trace::parse_witness_trace;
use lzvm_prover::{
    build_pcs_evaluation_segment, build_pcs_fri_opening_segment_from_transcript_values,
    build_pcs_fri_polynomial_values, build_pcs_fri_transcript_values_from_trace_segments,
    build_pcs_material_manifest_segment, build_pcs_query_nonce_segment,
    build_pcs_query_plan_segment_from_challenge, derive_prove_schedule,
    ProveExecutionUnitArtifacts, ProvePcsEvaluationValues, ProvePcsFriTranscriptTraceSegmentValues,
    ProveWitnessAuxiliaryInputs,
};

#[test]
fn derives_fri_transcript_values_from_trace_and_proof_segments() {
    let dir = temp_dir("fri-transcript-trace-segments");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");

    let expression_id = 42;
    let fixed_path = dir.join("unit.const");
    let mut key_unit = sample_unit(fixed_path.clone());
    key_unit.metadata.setup.challenge_count = 5;
    key_unit.metadata.verifier.quotient.expression_id = Some(expression_id);
    key_unit.expression_program = fixed_plus_stage_expression_program(expression_id);
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
    write_raw_fixed_columns_file(&fixed_path, &fixed_columns, &key_unit.metadata.setup)
        .expect("fixed columns should be written");

    let execution_unit = ProveExecutionUnitArtifacts {
        fixed_columns: fixed_path,
        expression_program: key_unit.expression_program.clone(),
        fri_expression_id: key_unit.metadata.verifier.quotient.expression_id,
        regular_constraints: key_unit.regular_constraints.clone(),
        setup: key_unit.metadata.setup.clone(),
        fixed_column_count: 2,
        stage_count: 1,
        opening_point_offsets: key_unit.metadata.setup.opening_points.clone(),
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
    };
    let schedule =
        derive_prove_schedule(&sample_catalog(key_unit)).expect("schedule should derive");
    let unit = &schedule.units[0];
    let trace_words = (0..16 * 5)
        .map(|index| index as u64 + 1)
        .collect::<Vec<_>>();
    let trace =
        parse_witness_trace(&encode_trace_words(&trace_words), 16, 5).expect("trace should parse");
    let material_segment =
        build_pcs_material_manifest_segment(&schedule).expect("material segment should build");
    let witness = sample_witness_commitment_segment(0, &[10, 20]);
    let witness_segment = ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
        data: encode_witness_commitment_segment(&witness).expect("witness segment should encode"),
    };
    let evaluations = vec![Ext3::from_u64s([30, 31, 32]), Ext3::from_u64s([40, 41, 42])];
    let evaluation_segment = build_pcs_evaluation_segment(
        &schedule,
        &[ProvePcsEvaluationValues {
            unit_index: 0,
            values: evaluations.clone(),
        }],
    )
    .expect("evaluation segment should build");
    let auxiliary = ProveWitnessAuxiliaryInputs::default();

    let values = build_pcs_fri_transcript_values_from_trace_segments(
        &schedule,
        &[ProvePcsFriTranscriptTraceSegmentValues {
            unit_index: 0,
            execution_unit: &execution_unit,
            trace: &trace,
            publics: &[],
            auxiliary_inputs: &auxiliary,
            material_segment: &material_segment,
            witness_segment: &witness_segment,
            evaluation_segment: &evaluation_segment,
        }],
    )
    .expect("FRI transcript values should build from proof segments");

    let transcript_value = &values[0];
    let constant_root = unit
        .pcs_material_constant_tree_root
        .expect("constant root should be present")
        .map(Felt::from_u64);
    let witness_roots = witness
        .stages
        .iter()
        .map(|stage| stage.root.map(Felt::from_u64))
        .collect::<Vec<_>>();
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
    })
    .expect("transcript challenges should derive");
    let xi_challenge = expected_challenges[unit.challenge_count - 3];
    let expected_polynomial = build_pcs_fri_polynomial_values(
        0,
        unit,
        &execution_unit,
        &trace,
        &[],
        &auxiliary,
        xi_challenge,
    )
    .expect("FRI polynomial should build");
    let parsed_evaluations = parse_pcs_evaluation_segment(&evaluation_segment.data)
        .expect("evaluation segment should parse");
    let nonce_segment = build_pcs_query_nonce_segment(
        &schedule,
        transcript_value.commitments.final_query_challenge,
    )
    .expect("nonce segment should build");
    let nonce = Felt::from_u64(
        parse_pcs_query_nonce_segment(&nonce_segment.data)
            .expect("nonce segment should parse")
            .nonce,
    );
    let query_segment = build_pcs_query_plan_segment_from_challenge(
        &schedule,
        std::slice::from_ref(&witness_segment),
        transcript_value.commitments.final_query_challenge,
        nonce,
    )
    .expect("query segment should build");
    let opening_segment =
        build_pcs_fri_opening_segment_from_transcript_values(&schedule, &query_segment, &values)
            .expect("FRI opening segment should build from transcript values");
    let query_plan =
        parse_pcs_query_plan_segment(&query_segment.data).expect("query segment should parse");
    let opening =
        parse_pcs_fri_opening_segment(&opening_segment.data).expect("FRI opening should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(values.len(), 1);
    assert_eq!(transcript_value.unit_index, 0);
    assert_eq!(
        parsed_evaluations.units[0].values,
        vec![[30, 31, 32], [40, 41, 42]]
    );
    assert_eq!(transcript_value.commitments.challenges, expected_challenges);
    assert_eq!(transcript_value.polynomial, expected_polynomial);
    assert!(verify_fri_opening_folds(
        unit,
        PcsFriOpeningFoldRequest {
            unit_index: 0,
            query_rows: &query_plan.units[0].queries,
            challenges: &transcript_value.commitments.challenges,
            fri: &opening.units[0],
        },
    )
    .expect("FRI folds should verify"));
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

fn sample_unit(fixed_columns: PathBuf) -> KeyUnitCatalogEntry {
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
            fixed_columns,
            constant_tree: "unit.consttree".into(),
        },
        metadata: UnitMetadataBundle {
            setup,
            expressions: lzvm_artifacts::expression_info::ExpressionInfo {
                hints: Vec::new(),
                expressions: Vec::new(),
                constraints: Vec::new(),
            },
            verifier: empty_verifier_info(),
        },
        pcs_plan,
        verification_key: VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4]),
        expression_program: ExpressionProgram {
            max_tmp1: 0,
            max_tmp3: 0,
            max_args: 0,
            max_ops: 0,
            entries: Vec::new(),
            ops: Vec::new(),
            args: Vec::new(),
            numbers: Vec::new(),
        },
        regular_constraints: lzvm_artifacts::constraint_program::ConstraintProgram {
            entries: Vec::new(),
            ops: Vec::new(),
            args: Vec::new(),
            numbers: Vec::new(),
        },
        verifier_program: ExpressionProgram {
            max_tmp1: 0,
            max_tmp3: 0,
            max_args: 0,
            max_ops: 0,
            entries: Vec::new(),
            ops: Vec::new(),
            args: Vec::new(),
            numbers: Vec::new(),
        },
        expected_fixed_bytes: 64,
        actual_fixed_bytes: 64,
        constant_tree_present: true,
        constant_tree_bytes: Some(224),
        constant_tree_root: Some(VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4])),
        pcs_material_present: true,
        pcs_material_bytes: Some(184),
        pcs_material: Some(lzvm_artifacts::pcs_material::PcsSetupMaterial {
            plan_digest: [7; 32],
            fixed_column_digest: [8; 32],
            constant_tree_digest: [9; 32],
            constant_tree_root: [1, 2, 3, 4],
            fixed_byte_count: 64,
            constant_tree_byte_count: 224,
            leaf_byte_count: 64,
            node_byte_count: 160,
        }),
    }
}

fn sample_catalog(unit: KeyUnitCatalogEntry) -> KeyDirectoryCatalog {
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
            units: Vec::new(),
        },
        global_constraints: lzvm_artifacts::constraint_program::GlobalConstraintProgram {
            entries: Vec::new(),
            ops: Vec::new(),
            args: Vec::new(),
            numbers: Vec::new(),
        },
        units: vec![unit],
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

fn encode_trace_words(values: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 8);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-prover-{name}-{}", std::process::id()))
}
