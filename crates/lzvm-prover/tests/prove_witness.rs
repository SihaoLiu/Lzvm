use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lzvm_artifacts::constraint_program::{
    ConstraintEntry, ConstraintProgram, GlobalConstraintProgram,
};
use lzvm_artifacts::expression_info::ExpressionInfo;
use lzvm_artifacts::expression_program::{ExpressionEntry, ExpressionProgram};
use lzvm_artifacts::fixed::{write_raw_fixed_columns_file, FixedColumn, FixedColumns};
use lzvm_artifacts::global_info::{CurveKind, GlobalInfo};
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
use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, parse_pcs_query_plan_segment, PcsQueryPlanSegment,
    PcsQueryPlanUnit, PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::public_values::{encode_public_values_json, PublicValueEntry, PublicValues};
use lzvm_artifacts::setup_info::{FriStep, StarkStruct, UnitSetupInfo};
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
use lzvm_prover::pcs_challenge::{derive_fri_queries, verify_query_nonce};
use lzvm_prover::pcs_fri::{verify_fri_opening_folds, PcsFriOpeningFoldRequest};
use lzvm_prover::pcs_transcript::{
    derive_pcs_final_query_challenge_from_segments, derive_pcs_transcript_challenges,
    PcsTranscriptInputs, PcsTranscriptSegmentInputs,
};
use lzvm_prover::witness_commitment::commit_witness_trace_stages;
use lzvm_prover::witness_layout::derive_witness_trace_layout;
use lzvm_prover::witness_loader::load_witness_library;
use lzvm_prover::witness_runner::run_witness_trace;
use lzvm_prover::{
    build_pcs_evaluation_segment, build_pcs_fri_opening_segment,
    build_pcs_fri_opening_segment_from_trace, build_pcs_fri_polynomial_values,
    build_pcs_fri_transcript_values_from_trace, build_pcs_material_manifest_segment,
    build_pcs_query_nonce_segment, build_pcs_query_nonce_segment_from_transcript_segments,
    build_pcs_query_plan_segment, build_pcs_query_plan_segment_from_challenge,
    build_pcs_query_plan_segment_from_transcript_segments, build_witness_commitment_segment,
    build_witness_opening_segment, derive_prove_execution_plan, derive_prove_schedule,
    run_prove_witness_commitments, run_prove_witness_commitments_with_auxiliary_inputs,
    run_prove_witness_commitments_with_trace, GpuRunOptions, ProveExecutionInputArtifacts,
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

fn row_zero_stage_constraint(expected: u64) -> ConstraintProgram {
    ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 0,
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
            last_row: 0,
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
            last_row: 0,
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
            last_row: 0,
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
        encode_public_values_json(&values).expect("public values should encode"),
    )
    .expect("public values should be written");
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
                constraints_json: "global-constraints.json".into(),
                constraints_program: "global-constraints.bin".into(),
            },
            units: Vec::new(),
        },
        global_constraints: GlobalConstraintProgram {
            entries: Vec::new(),
            ops: Vec::new(),
            args: Vec::new(),
            numbers: Vec::new(),
        },
        units: vec![unit],
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
            witness_library: witness_library.clone(),
            guest_image,
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");

    let output = run_prove_witness_commitments(&plan, 0).expect("witness commitments should run");

    let library = load_witness_library(&witness_library).expect("witness library should load");
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
fn preserves_trace_inputs_and_commitments_for_pcs_openings() {
    let dir = temp_dir("trace-commitments");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = build_shared_library(&dir, "witness", witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let public_inputs = dir.join("public-values.json");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.paths.fixed_columns = dir.join("unit.const");
    fs::write(&unit.paths.fixed_columns, vec![0_u8; 16 * 2 * 8])
        .expect("fixed columns should be written");
    unit.regular_constraints = public_row_zero_stage_constraint();
    let catalog = sample_catalog(unit);
    write_public_values(
        &public_inputs,
        key_directory_catalog_digest(&catalog).expect("catalog digest should compute"),
        vec![8],
    );
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library,
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
            witness_library,
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
    let public_inputs = dir.join("public-values.json");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&input_data, [7_u8]).expect("input data should be written");

    let mut unit = sample_unit();
    unit.paths.fixed_columns = dir.join("unit.const");
    fs::write(&unit.paths.fixed_columns, vec![0_u8; 16 * 2 * 8])
        .expect("fixed columns should be written");
    unit.regular_constraints = public_row_zero_stage_constraint();
    let catalog = sample_catalog(unit);
    write_public_values(
        &public_inputs,
        key_directory_catalog_digest(&catalog).expect("catalog digest should compute"),
        vec![8],
    );
    let plan = derive_prove_execution_plan(
        &catalog,
        sample_request(dir.join("out"), Some(input_data)),
        ProveExecutionInputArtifacts {
            witness_library,
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
            witness_library,
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
            witness_library,
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
            witness_library,
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
            witness_library,
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
            witness_library,
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
            witness_library,
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
            witness_library,
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
            witness_library,
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
            witness_library,
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
            witness_library,
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
            witness_library,
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
