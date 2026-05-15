use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::constraint_program::{ConstraintProgram, GlobalConstraintProgram};
use lzvm_artifacts::expression_info::ExpressionInfo;
use lzvm_artifacts::expression_program::ExpressionProgram;
use lzvm_artifacts::global_info::{CurveKind, GlobalInfo};
use lzvm_artifacts::guest_image::GuestImageError;
use lzvm_artifacts::hint_program::HintProgram;
use lzvm_artifacts::key_directory::{
    KeyDirectoryCatalog, KeyDirectoryLayout, KeyUnitCatalogEntry, KeyUnitKind, KeyUnitPaths,
};
use lzvm_artifacts::metadata_bundle::UnitMetadataBundle;
use lzvm_artifacts::pcs_material::PcsSetupMaterial;
use lzvm_artifacts::pcs_plan::derive_pcs_setup_plan;
use lzvm_artifacts::setup_info::{
    CommitmentColumn, EvaluationMapEntry, FriStep, StageValue, StarkStruct, UnitSetupInfo,
};
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_artifacts::verifier_info::{VerifierCode, VerifierInfo};
use lzvm_artifacts::witness_library::WitnessLibraryError;
use lzvm_prover::{
    derive_prove_execution_plan, derive_prove_run_plan, derive_prove_schedule, GpuRunOptions,
    ProveExecutionInputArtifacts, ProveExecutionPlanError, ProvePartitionPlan, ProvePassKind,
    ProvePassRequest, ProveRunOptions, ProveRunPlanError, ProveRunRequest, ProveScheduleError,
};

fn sample_setup(n_bits: u32, n_bits_ext: u32, query_count: u32) -> UnitSetupInfo {
    let mut section_widths = BTreeMap::new();
    section_widths.insert("cm1".to_owned(), 2);
    section_widths.insert("cm2".to_owned(), 3);

    UnitSetupInfo {
        n_stages: 1,
        n_constants: 2,
        constant_columns: Vec::new(),
        commitment_columns: vec![
            CommitmentColumn {
                name: "trace.a".to_owned(),
                stage: 1,
                dimension: 1,
                pols_map_id: 0,
                stage_id: 0,
                stage_position: 0,
                intermediate: false,
                lengths: Vec::new(),
            },
            CommitmentColumn {
                name: "aux.a".to_owned(),
                stage: 2,
                dimension: 3,
                pols_map_id: 1,
                stage_id: 0,
                stage_position: 0,
                intermediate: true,
                lengths: Vec::new(),
            },
        ],
        n_publics: Some(0),
        n_constraints: Some(0),
        q_degree: 3,
        opening_points: vec![0, 1],
        section_widths,
        challenge_count: 1,
        eval_count: 2,
        evaluation_map: vec![EvaluationMapEntry::default(); 2],
        boundaries: Vec::new(),
        unit_value_map: vec![
            StageValue {
                name: "unit.alpha".to_owned(),
                stage: 1,
                lengths: vec![2],
            },
            StageValue {
                name: "unit.beta".to_owned(),
                stage: 2,
                lengths: Vec::new(),
            },
        ],
        group_value_map: vec![StageValue {
            name: "group.alpha".to_owned(),
            stage: 2,
            lengths: Vec::new(),
        }],
        stark: StarkStruct {
            n_bits,
            n_bits_ext,
            n_queries: query_count,
            steps: vec![FriStep { n_bits: n_bits_ext }, FriStep { n_bits }],
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

fn empty_regular_hints() -> HintProgram {
    HintProgram { hints: Vec::new() }
}

fn sample_unit(kind: KeyUnitKind, unit_id: usize, fixed_bytes: u64) -> KeyUnitCatalogEntry {
    let setup = sample_setup(4 + unit_id as u32, 6 + unit_id as u32, 2 + unit_id as u32);
    let pcs_plan = derive_pcs_setup_plan(&setup).expect("PCS setup plan should derive");

    KeyUnitCatalogEntry {
        paths: KeyUnitPaths {
            kind,
            group_id: Some(0),
            unit_id: Some(unit_id),
            group_name: Some("group-a".to_owned()),
            unit_name: Some(format!("unit-{unit_id}")),
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
        expected_fixed_bytes: fixed_bytes as usize,
        actual_fixed_bytes: fixed_bytes,
        constant_tree_present: true,
        constant_tree_bytes: Some(224),
        constant_tree_root: Some(VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4])),
        pcs_material_present: false,
        pcs_material_bytes: None,
        pcs_material: None,
    }
}

fn sample_catalog(units: Vec<KeyUnitCatalogEntry>) -> KeyDirectoryCatalog {
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
        global_constraints: GlobalConstraintProgram {
            entries: Vec::new(),
            ops: Vec::new(),
            args: Vec::new(),
            numbers: Vec::new(),
        },
        global_hints: empty_regular_hints(),
        units,
    }
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-prover-schedule-{}-{name}",
        std::process::id()
    ))
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
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
    bytes
}

fn sample_pcs_material(seed: u8) -> PcsSetupMaterial {
    PcsSetupMaterial {
        plan_digest: [seed; 32],
        fixed_column_digest: [seed.wrapping_add(1); 32],
        constant_tree_digest: [seed.wrapping_add(2); 32],
        constant_tree_root: [1, 2, 3, 4],
        fixed_byte_count: 64,
        constant_tree_byte_count: 224,
        leaf_byte_count: 64,
        node_byte_count: 160,
    }
}

fn sample_unit_with_pcs_material(
    kind: KeyUnitKind,
    unit_id: usize,
    fixed_bytes: u64,
) -> KeyUnitCatalogEntry {
    let mut unit = sample_unit(kind, unit_id, fixed_bytes);
    unit.pcs_material_present = true;
    unit.pcs_material_bytes = Some(184);
    unit.pcs_material = Some(sample_pcs_material(unit_id as u8 + 7));
    unit
}

#[test]
fn derives_prove_schedule_from_key_directory_catalog() {
    let catalog = sample_catalog(vec![
        sample_unit(KeyUnitKind::Basic, 0, 64),
        sample_unit(KeyUnitKind::RecursiveFirst, 1, 128),
    ]);

    let schedule = derive_prove_schedule(&catalog).expect("prove schedule should derive");

    assert_eq!(schedule.unit_count, 2);
    assert_eq!(schedule.total_fixed_bytes, 192);
    assert_eq!(schedule.total_query_count, 5);
    assert_eq!(schedule.max_extended_domain_bits, 7);
    assert_eq!(schedule.units[0].kind, KeyUnitKind::Basic);
    assert_eq!(schedule.units[0].base_domain_size, 16);
    assert_eq!(schedule.units[0].extended_domain_size, 64);
    assert_eq!(schedule.units[0].blowup_factor, 4);
    assert_eq!(schedule.units[0].query_count, 2);
    assert_eq!(schedule.units[0].proof_of_work_bits, 10);
    assert_eq!(schedule.units[0].merkle_tree_arity, 4);
    assert_eq!(schedule.units[0].transcript_arity, Some(4));
    assert!(schedule.units[0].hash_commits);
    assert_eq!(
        schedule.units[0].transcript_root_challenge_draws,
        vec![2, 1]
    );
    assert_eq!(schedule.units[0].challenge_count, 1);
    assert_eq!(schedule.units[0].evaluation_value_count, 2);
    assert_eq!(
        schedule.units[0].evaluation_map,
        vec![EvaluationMapEntry::default(); 2]
    );
    assert_eq!(schedule.units[0].expected_evaluation_value_count(), 2);
    assert_eq!(schedule.units[0].transcript_evaluation_challenge_draws, 2);
    assert_eq!(schedule.units[0].constant_width, 2);
    assert_eq!(schedule.units[0].stage_commit_widths, vec![2, 3]);
    assert_eq!(schedule.units[0].commitment_columns.len(), 2);
    assert_eq!(schedule.units[0].commitment_columns[1].stage, 2);
    assert_eq!(schedule.units[0].commitment_columns[1].stage_position, 0);
    assert_eq!(schedule.units[0].unit_value_map.len(), 2);
    assert_eq!(schedule.units[0].unit_value_map[0].name, "unit.alpha");
    assert_eq!(schedule.units[0].unit_value_map[0].lengths, [2]);
    assert_eq!(schedule.units[0].group_value_map.len(), 1);
    assert_eq!(schedule.units[0].group_value_map[0].name, "group.alpha");
    assert_eq!(schedule.units[0].opening_points, vec![0, 1]);
    assert_eq!(schedule.units[0].fri_layers.len(), 1);
    assert_eq!(schedule.units[0].fri_layers[0].input_bits, 6);
    assert_eq!(schedule.units[0].fri_layers[0].output_bits, 4);
    assert_eq!(schedule.units[0].fri_layers[0].folding_factor, 4);
    assert_eq!(schedule.units[0].final_layer_bits, 4);
    assert_eq!(schedule.units[1].kind, KeyUnitKind::RecursiveFirst);
    assert_eq!(schedule.units[1].extended_domain_bits, 7);
    assert_ne!(schedule.setup_hash, [0_u8; 32]);
}

#[test]
fn distinguishes_evaluation_value_count_from_transcript_draws() {
    let mut unit = sample_unit(KeyUnitKind::Basic, 0, 64);
    unit.metadata.setup.eval_count = 5;
    let catalog = sample_catalog(vec![unit]);

    let schedule = derive_prove_schedule(&catalog).expect("prove schedule should derive");

    assert_eq!(schedule.units[0].evaluation_value_count, 5);
    assert_eq!(schedule.units[0].transcript_evaluation_challenge_draws, 2);
}

#[test]
fn derives_prove_schedule_with_pcs_material_inputs() {
    let mut with_material = sample_unit(KeyUnitKind::Basic, 0, 64);
    with_material.pcs_material_present = true;
    with_material.pcs_material_bytes = Some(184);
    with_material.pcs_material = Some(sample_pcs_material(7));
    let without_material = sample_unit(KeyUnitKind::RecursiveFirst, 1, 128);
    let catalog = sample_catalog(vec![with_material, without_material]);

    let schedule = derive_prove_schedule(&catalog).expect("prove schedule should derive");

    assert_eq!(schedule.pcs_material_unit_count, 1);
    assert_eq!(schedule.total_pcs_material_bytes, 184);
    assert_eq!(schedule.units[0].pcs_material_bytes, Some(184));
    assert_eq!(schedule.units[0].pcs_material_plan_digest, Some([7; 32]));
    assert_eq!(
        schedule.units[0].pcs_material_fixed_column_digest,
        Some([8; 32])
    );
    assert_eq!(
        schedule.units[0].pcs_material_constant_tree_digest,
        Some([9; 32])
    );
    assert_eq!(
        schedule.units[0].pcs_material_constant_tree_root,
        Some([1, 2, 3, 4])
    );
    assert_eq!(schedule.units[0].pcs_material_fixed_byte_count, Some(64));
    assert_eq!(
        schedule.units[0].pcs_material_constant_tree_byte_count,
        Some(224)
    );
    assert_eq!(schedule.units[0].pcs_material_leaf_byte_count, Some(64));
    assert_eq!(schedule.units[0].pcs_material_node_byte_count, Some(160));
    assert_eq!(schedule.units[1].pcs_material_bytes, None);
    assert_eq!(schedule.units[1].pcs_material_plan_digest, None);
}

#[test]
fn rejects_empty_prove_schedule_catalogs() {
    let catalog = sample_catalog(Vec::new());

    assert!(matches!(
        derive_prove_schedule(&catalog),
        Err(ProveScheduleError::EmptyCatalog)
    ));
}

#[test]
fn derives_full_prove_run_plan_from_catalog_and_request() {
    let catalog = sample_catalog(vec![sample_unit(KeyUnitKind::Basic, 0, 64)]);
    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan {
            input_data: Some(PathBuf::from("input.bin")),
            partition_count: 4,
            partition_ids: vec![1, 3],
            worker_index: 2,
        }),
        options: ProveRunOptions {
            aggregate: true,
            remote_aggregation: false,
            final_wrap: true,
            verify_outputs: true,
            save_outputs: true,
            minimal_memory: false,
            output_dir: PathBuf::from("out"),
        },
        gpu: GpuRunOptions {
            preallocate: true,
            max_streams: 8,
            witness_thread_pools: 2,
            max_stored_witnesses: 3,
            pack_trace: true,
        },
    };

    let plan = derive_prove_run_plan(&catalog, request).expect("run plan should derive");

    assert_eq!(plan.schedule.unit_count, 1);
    assert_eq!(plan.pass.kind(), ProvePassKind::Full);
    assert_eq!(plan.options.output_dir, PathBuf::from("out"));
    assert_eq!(plan.gpu.max_streams, 8);
    match plan.pass {
        ProvePassRequest::Full(partitions) => {
            assert_eq!(partitions.input_data, Some(PathBuf::from("input.bin")));
            assert_eq!(partitions.partition_count, 4);
            assert_eq!(partitions.partition_ids, vec![1, 3]);
            assert_eq!(partitions.worker_index, 2);
        }
        _ => panic!("expected full pass"),
    }
}

#[test]
fn rejects_prove_run_plans_with_invalid_partitions() {
    let catalog = sample_catalog(vec![sample_unit(KeyUnitKind::Basic, 0, 64)]);
    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan {
            input_data: None,
            partition_count: 2,
            partition_ids: vec![2],
            worker_index: 0,
        }),
        options: ProveRunOptions::default_for_output(PathBuf::from("out")),
        gpu: GpuRunOptions::default(),
    };

    assert!(matches!(
        derive_prove_run_plan(&catalog, request),
        Err(ProveRunPlanError::PartitionOutOfRange {
            partition_id: 2,
            partition_count: 2,
        })
    ));
}

#[test]
fn rejects_final_wrap_without_aggregation() {
    let catalog = sample_catalog(vec![sample_unit(KeyUnitKind::Basic, 0, 64)]);
    let mut options = ProveRunOptions::default_for_output(PathBuf::from("out"));
    options.final_wrap = true;
    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan::single()),
        options,
        gpu: GpuRunOptions::default(),
    };

    assert!(matches!(
        derive_prove_run_plan(&catalog, request),
        Err(ProveRunPlanError::AggregationRequired { option })
            if option == "final_wrap"
    ));
}

#[test]
fn derives_prove_execution_plan_with_input_artifacts() {
    let dir = temp_dir("execution-plan");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let public_inputs = dir.join("public-inputs.bin");
    fs::write(&witness_library, sample_witness_library())
        .expect("witness library should be written");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");
    fs::write(&public_inputs, [3_u8]).expect("public inputs should be written");

    let mut unit = sample_unit_with_pcs_material(KeyUnitKind::Basic, 0, 64);
    unit.expression_program.numbers = vec![17, 19, 23];
    unit.metadata.verifier.quotient.expression_id = Some(42);
    let expected_expression_program = unit.expression_program.clone();
    let catalog = sample_catalog(vec![unit]);
    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan::single()),
        options: ProveRunOptions::default_for_output(dir.join("out")),
        gpu: GpuRunOptions::default(),
    };
    let inputs = ProveExecutionInputArtifacts {
        witness_library: witness_library.clone(),
        guest_image: guest_image.clone(),
        public_inputs: Some(public_inputs.clone()),
    };

    let plan = derive_prove_execution_plan(&catalog, request, inputs)
        .expect("execution plan should derive");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(plan.run_plan.schedule.unit_count, 1);
    assert_eq!(plan.inputs.witness_library, witness_library);
    assert_eq!(plan.witness_library_info.byte_len, 64);
    assert_eq!(plan.witness_library_info.machine, 62);
    assert_eq!(plan.inputs.guest_image, guest_image);
    assert_eq!(plan.inputs.public_inputs, Some(public_inputs));
    assert_eq!(plan.guest_image_info.byte_len, 64);
    assert_eq!(plan.guest_image_info.machine, 243);
    assert_eq!(plan.guest_image_info.entry, 0x8000_0000);
    assert_eq!(
        plan.units[0].expression_program,
        expected_expression_program
    );
    assert_eq!(plan.units[0].fri_expression_id, Some(42));
}

#[test]
fn rejects_prove_execution_plan_without_pcs_material() {
    let dir = temp_dir("missing-pcs-material");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    fs::write(&witness_library, sample_witness_library())
        .expect("witness library should be written");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");

    let catalog = sample_catalog(vec![sample_unit(KeyUnitKind::Basic, 0, 64)]);
    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan::single()),
        options: ProveRunOptions::default_for_output(dir.join("out")),
        gpu: GpuRunOptions::default(),
    };
    let inputs = ProveExecutionInputArtifacts {
        witness_library,
        guest_image,
        public_inputs: None,
    };

    let result = derive_prove_execution_plan(&catalog, request, inputs);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(ProveExecutionPlanError::MissingPcsMaterial {
            unit_index: 0,
            kind: KeyUnitKind::Basic,
        })
    ));
}

#[test]
fn rejects_prove_execution_plan_with_missing_witness_library() {
    let dir = temp_dir("missing-witness");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = dir.join("missing.so");
    let guest_image = dir.join("guest.elf");
    fs::write(&guest_image, [2_u8]).expect("guest image should be written");

    let catalog = sample_catalog(vec![sample_unit_with_pcs_material(
        KeyUnitKind::Basic,
        0,
        64,
    )]);
    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan::single()),
        options: ProveRunOptions::default_for_output(dir.join("out")),
        gpu: GpuRunOptions::default(),
    };
    let inputs = ProveExecutionInputArtifacts {
        witness_library: witness_library.clone(),
        guest_image,
        public_inputs: None,
    };

    let result = derive_prove_execution_plan(&catalog, request, inputs);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(ProveExecutionPlanError::MissingWitnessLibrary { path }) if path == witness_library
    ));
}

#[test]
fn rejects_prove_execution_plan_with_invalid_guest_image() {
    let dir = temp_dir("invalid-guest-image");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    fs::write(&witness_library, sample_witness_library())
        .expect("witness library should be written");
    fs::write(&guest_image, b"not-an-elf").expect("guest image should be written");

    let catalog = sample_catalog(vec![sample_unit_with_pcs_material(
        KeyUnitKind::Basic,
        0,
        64,
    )]);
    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan::single()),
        options: ProveRunOptions::default_for_output(dir.join("out")),
        gpu: GpuRunOptions::default(),
    };
    let inputs = ProveExecutionInputArtifacts {
        witness_library,
        guest_image: guest_image.clone(),
        public_inputs: None,
    };

    let result = derive_prove_execution_plan(&catalog, request, inputs);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(ProveExecutionPlanError::InvalidGuestImage { path, source })
            if path == guest_image && source == GuestImageError::InvalidMagic
    ));
}

#[test]
fn rejects_prove_execution_plan_with_invalid_witness_library() {
    let dir = temp_dir("invalid-witness-library");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    fs::write(&witness_library, b"not-an-elf").expect("witness library should be written");
    fs::write(&guest_image, sample_guest_image()).expect("guest image should be written");

    let catalog = sample_catalog(vec![sample_unit_with_pcs_material(
        KeyUnitKind::Basic,
        0,
        64,
    )]);
    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan::single()),
        options: ProveRunOptions::default_for_output(dir.join("out")),
        gpu: GpuRunOptions::default(),
    };
    let inputs = ProveExecutionInputArtifacts {
        witness_library: witness_library.clone(),
        guest_image,
        public_inputs: None,
    };

    let result = derive_prove_execution_plan(&catalog, request, inputs);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(ProveExecutionPlanError::InvalidWitnessLibrary { path, source })
            if path == witness_library && source == WitnessLibraryError::InvalidMagic
    ));
}
