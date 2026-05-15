use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lzvm_artifacts::constraint_program::GlobalConstraintProgram;
use lzvm_artifacts::expression_info::ExpressionInfo;
use lzvm_artifacts::expression_program::ExpressionProgram;
use lzvm_artifacts::global_info::{CurveKind, GlobalInfo};
use lzvm_artifacts::key_directory::{
    KeyDirectoryCatalog, KeyDirectoryLayout, KeyUnitCatalogEntry, KeyUnitKind, KeyUnitPaths,
};
use lzvm_artifacts::metadata_bundle::UnitMetadataBundle;
use lzvm_artifacts::pcs_plan::derive_pcs_setup_plan;
use lzvm_artifacts::setup_info::{FriStep, StarkStruct, UnitSetupInfo};
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_artifacts::verifier_info::{VerifierCode, VerifierInfo};
use lzvm_prover::witness_commitment::commit_witness_trace_stages;
use lzvm_prover::witness_layout::derive_witness_trace_layout;
use lzvm_prover::witness_loader::load_witness_library;
use lzvm_prover::witness_runner::run_witness_trace;
use lzvm_prover::{
    derive_prove_execution_plan, run_prove_witness_commitments, GpuRunOptions,
    ProveExecutionInputArtifacts, ProvePartitionPlan, ProvePassRequest, ProveRunOptions,
    ProveRunRequest, ProveWitnessCommitmentError,
};

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
        n_publics: Some(0),
        n_constraints: Some(0),
        q_degree: 3,
        opening_points: vec![0],
        section_widths,
        challenge_count: 1,
        eval_count: 2,
        boundaries: Vec::new(),
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
        verifier_program: empty_program(),
        expected_fixed_bytes: 64,
        actual_fixed_bytes: 64,
        constant_tree_present: true,
        constant_tree_bytes: Some(224),
        constant_tree_root: Some(VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4])),
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
