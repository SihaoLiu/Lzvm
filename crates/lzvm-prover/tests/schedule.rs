use std::collections::BTreeMap;
use std::path::PathBuf;

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
use lzvm_prover::{
    derive_prove_run_plan, derive_prove_schedule, GpuRunOptions, ProvePartitionPlan, ProvePassKind,
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
        n_publics: Some(0),
        n_constraints: Some(0),
        q_degree: 3,
        opening_points: vec![0, 1],
        section_widths,
        challenge_count: 1,
        eval_count: 2,
        boundaries: Vec::new(),
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
        verifier_program: empty_program(),
        expected_fixed_bytes: fixed_bytes as usize,
        actual_fixed_bytes: fixed_bytes,
        constant_tree_present: true,
        constant_tree_bytes: Some(224),
        constant_tree_root: Some(VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4])),
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
        units,
    }
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
    assert_eq!(schedule.units[0].constant_width, 2);
    assert_eq!(schedule.units[0].stage_commit_widths, vec![2, 3]);
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
