use std::collections::BTreeMap;

use lzvm_artifacts::constant_opening_segment::{
    encode_constant_opening_segment, ConstantOpeningLevelSegment, ConstantOpeningQuerySegment,
    ConstantOpeningSegment, ConstantOpeningUnitSegment, CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::constant_tree::{ConstantTree, ConstantTreeHashKind};
use lzvm_artifacts::constraint_program::GlobalConstraintProgram;
use lzvm_artifacts::expression_info::ExpressionInfo;
use lzvm_artifacts::expression_program::ExpressionProgram;
use lzvm_artifacts::global_info::{CurveKind, GlobalInfo, PublicValue};
use lzvm_artifacts::guest_input_segment::{
    encode_framed_guest_input_segment, framed_guest_input_segment_digest,
    FRAMED_GUEST_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::hint_program::HintProgram;
use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, GlobalKeyPaths, KeyDirectoryCatalog, KeyDirectoryLayout,
    KeyUnitCatalogEntry, KeyUnitKind, KeyUnitPaths,
};
use lzvm_artifacts::metadata_bundle::UnitMetadataBundle;
use lzvm_artifacts::pcs_material::PcsSetupMaterial;
use lzvm_artifacts::pcs_plan::derive_pcs_setup_plan;
use lzvm_artifacts::pcs_query_segment::parse_pcs_query_plan_segment;
use lzvm_artifacts::proof::{ProofArtifact, ProofArtifactError, ProofSegment};
use lzvm_artifacts::public_values::{
    public_values_digest, PublicValueEntry, PublicValues, PublicValuesError,
};
use lzvm_artifacts::setup_info::{FriStep, StageValue, StarkStruct, UnitSetupInfo};
use lzvm_artifacts::trace_constraint_segment::{
    encode_trace_constraint_segment, TraceConstraintSegment, TraceConstraintUnitSegment,
    TRACE_CONSTRAINT_SEGMENT_ID,
};
use lzvm_artifacts::unit_values_segment::{
    encode_unit_values_segment, UnitValuesSegment, UnitValuesUnitSegment, UNIT_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_artifacts::verifier_info::{VerifierCode, VerifierInfo};
use lzvm_artifacts::witness_opening_segment::{
    encode_witness_opening_segment, WitnessOpeningLevelSegment, WitnessOpeningQuerySegment,
    WitnessOpeningSegment, WitnessOpeningStageSegment, WitnessOpeningUnitSegment,
    WITNESS_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, parse_witness_commitment_segment, WitnessCommitmentSegment,
    WitnessCommitmentStageSegment, WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::{poseidon2_hash_16, Felt};
use lzvm_prover::constant_tree_opening::{open_constant_tree_row, ConstantTreeOpening};
use lzvm_prover::pcs_fri::{
    LoadPcsFriOpeningSegmentError, ValidateOptionalPcsFriOpeningProofSegmentsError,
};
use lzvm_prover::pcs_query_plan::{
    build_pcs_query_plan_segment, ValidatePcsQueryPlanSegmentsError,
};
use lzvm_prover::proof_preflight::{ProofPreflightError, PublicValueFieldError};
use lzvm_prover::setup_preflight::{
    validate_public_values_metadata, validate_setup_preflight, validate_setup_preflight_hashes,
    SetupPreflightError, SetupPreflightReport,
};
use lzvm_prover::unit_values::LoadUnitValuesSegmentError;
use lzvm_prover::witness_commitment::{
    commit_witness_stage_leaves, extend_witness_stage_leaves, open_witness_stage_commitment,
    WitnessStageOpening,
};
use lzvm_prover::witness_layout::derive_witness_trace_layout;
use lzvm_prover::witness_trace::parse_witness_trace;
use lzvm_prover::{build_pcs_material_manifest_segment, derive_prove_schedule, ProveScheduleError};

const SAMPLE_AUX_SEGMENT_ID: u32 = CONSTANT_OPENING_SEGMENT_ID;

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
                n_publics: 5,
                num_challenges: Vec::new(),
                num_proof_values: Vec::new(),
                proof_values_map: Vec::new(),
                publics_map: vec![
                    PublicValue {
                        name: "block_number".to_owned(),
                        stage: 1,
                        lengths: Vec::new(),
                    },
                    PublicValue {
                        name: "state_root_words".to_owned(),
                        stage: 1,
                        lengths: vec![4],
                    },
                ],
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
            id: SAMPLE_AUX_SEGMENT_ID,
            data: vec![1, 2, 3, 4],
        }],
    }
}

fn framed_stdin_chunk(data: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&(data.len() as u64).to_le_bytes());
    encoded.extend_from_slice(data);
    let padding = (8 - ((8 + data.len()) % 8)) % 8;
    encoded.extend(std::iter::repeat_n(0, padding));
    encoded
}

fn sample_empty_public_values(setup_hash: [u8; 32]) -> PublicValues {
    PublicValues {
        schema_version: 1,
        setup_hash,
        values: Vec::new(),
    }
}

fn sample_catalog_with_fri_unit() -> KeyDirectoryCatalog {
    let mut catalog = sample_catalog();
    catalog.layout.global_info.air_groups = vec!["group-a".to_owned()];
    catalog.layout.global_info.n_publics = 0;
    catalog.layout.global_info.publics_map.clear();
    catalog.units = vec![sample_fri_unit()];
    catalog
}

fn sample_catalog_with_unit_value_units() -> KeyDirectoryCatalog {
    let mut catalog = sample_catalog();
    catalog.layout.global_info.air_groups = vec!["group-a".to_owned()];
    catalog.layout.global_info.n_publics = 0;
    catalog.layout.global_info.publics_map.clear();
    catalog.units = vec![
        sample_fri_unit_with_unit_values(0, "unit-value-a"),
        sample_fri_unit_with_unit_values(1, "unit-value-b"),
    ];
    catalog
}

fn sample_fri_unit() -> KeyUnitCatalogEntry {
    let setup = sample_fri_setup();
    let pcs_plan = derive_pcs_setup_plan(&setup).expect("PCS setup plan should derive");
    let (_tree, root) = sample_constant_tree();
    let root_words = root.map(Felt::to_u64);

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
            expressions: ExpressionInfo {
                hints: Vec::new(),
                expressions: Vec::new(),
                constraints: Vec::new(),
            },
            verifier: fri_verifier_info(),
        },
        pcs_plan,
        verification_key: VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4]),
        expression_program: empty_expression_program(),
        regular_constraints: lzvm_artifacts::constraint_program::ConstraintProgram {
            entries: Vec::new(),
            ops: Vec::new(),
            args: Vec::new(),
            numbers: Vec::new(),
        },
        regular_hints: HintProgram { hints: Vec::new() },
        verifier_program: empty_expression_program(),
        expected_fixed_bytes: 64,
        actual_fixed_bytes: 64,
        constant_tree_present: true,
        constant_tree_bytes: Some(224),
        constant_tree_root: Some(VerificationKeyRoot::FieldElements(root_words.to_vec())),
        pcs_material_present: true,
        pcs_material_bytes: Some(184),
        pcs_material: Some(PcsSetupMaterial {
            plan_digest: [7; 32],
            fixed_column_digest: [8; 32],
            constant_tree_digest: [9; 32],
            constant_tree_root: root_words,
            fixed_byte_count: 64,
            constant_tree_byte_count: 224,
            leaf_byte_count: 64,
            node_byte_count: 160,
        }),
    }
}

fn sample_fri_unit_with_unit_values(unit_id: usize, unit_name: &str) -> KeyUnitCatalogEntry {
    let mut unit = sample_fri_unit();
    unit.paths.unit_id = Some(unit_id);
    unit.paths.unit_name = Some(unit_name.to_owned());
    unit.paths.prefix = unit_name.into();
    unit.paths.metadata_prefix = Some(unit_name.into());
    unit.paths.program_prefix = Some(unit_name.into());
    unit.paths.verification_key_prefix = unit_name.into();
    unit.paths.fixed_columns = format!("{unit_name}.const").into();
    unit.paths.constant_tree = format!("{unit_name}.consttree").into();
    unit.metadata.setup.unit_value_map = vec![stage_value(&format!("{unit_name}.value"), 1)];
    unit
}

fn stage_value(name: &str, stage: u32) -> StageValue {
    StageValue {
        name: name.to_owned(),
        stage,
        lengths: Vec::new(),
    }
}

fn sample_fri_setup() -> UnitSetupInfo {
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
        eval_count: 0,
        evaluation_map: Vec::new(),
        boundaries: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        stark: StarkStruct {
            n_bits: 1,
            n_bits_ext: 2,
            n_queries: 1,
            steps: vec![FriStep { n_bits: 2 }, FriStep { n_bits: 1 }],
            hash_commits: false,
            last_level_verification: 0,
            pow_bits: 0,
            merkle_tree_arity: 4,
            verification_hash_type: Some("GL".to_owned()),
            transcript_arity: Some(4),
            merkle_tree_custom: Some(true),
        },
    }
}

fn fri_verifier_info() -> VerifierInfo {
    VerifierInfo {
        quotient: VerifierCode {
            expression_id: Some(1),
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

fn empty_expression_program() -> ExpressionProgram {
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

fn sample_constant_tree() -> (ConstantTree, [Felt; 4]) {
    let rows = [
        [Felt::from_u64(1), Felt::from_u64(10)],
        [Felt::from_u64(2), Felt::from_u64(20)],
        [Felt::from_u64(3), Felt::from_u64(30)],
        [Felt::from_u64(4), Felt::from_u64(40)],
    ];
    let leaves = rows
        .iter()
        .map(|row| [row[0], row[1], Felt::ZERO, Felt::ZERO])
        .collect::<Vec<_>>();
    let state = poseidon2_hash_16([
        leaves[0][0],
        leaves[0][1],
        leaves[0][2],
        leaves[0][3],
        leaves[1][0],
        leaves[1][1],
        leaves[1][2],
        leaves[1][3],
        leaves[2][0],
        leaves[2][1],
        leaves[2][2],
        leaves[2][3],
        leaves[3][0],
        leaves[3][1],
        leaves[3][2],
        leaves[3][3],
    ]);
    let root = [state[0], state[1], state[2], state[3]];

    let mut bytes = Vec::new();
    for row in rows {
        for value in row {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for digest in &leaves {
        append_digest(&mut bytes, *digest);
    }
    append_digest(&mut bytes, root);

    (
        ConstantTree {
            hash_kind: ConstantTreeHashKind::Gl,
            extended_row_count: 4,
            constant_count: 2,
            leaf_byte_count: 64,
            node_byte_count: 160,
            bytes,
        },
        root,
    )
}

fn append_digest(out: &mut Vec<u8>, digest: [Felt; 4]) {
    for value in digest {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

fn constant_opening_segment(query_row: u64) -> ProofSegment {
    constant_opening_segment_for_units(vec![constant_opening_unit_segment(0, query_row)])
}

fn constant_opening_segment_for_units(units: Vec<ConstantOpeningUnitSegment>) -> ProofSegment {
    ProofSegment {
        id: CONSTANT_OPENING_SEGMENT_ID,
        data: encode_constant_opening_segment(&ConstantOpeningSegment { units })
            .expect("constant opening should encode"),
    }
}

fn constant_opening_unit_segment(unit_index: u32, query_row: u64) -> ConstantOpeningUnitSegment {
    let (tree, _root) = sample_constant_tree();
    let opening = open_constant_tree_row(&tree, query_row, 4).expect("constant row should open");
    ConstantOpeningUnitSegment {
        unit_index,
        trace_instance_index: 0,
        queries: vec![constant_opening_query(&opening)],
    }
}

fn constant_opening_query(opening: &ConstantTreeOpening) -> ConstantOpeningQuerySegment {
    ConstantOpeningQuerySegment {
        row_index: opening.row_index(),
        values: opening
            .values()
            .iter()
            .map(|value| value.to_u64())
            .collect(),
        siblings: opening
            .siblings()
            .iter()
            .map(|level| ConstantOpeningLevelSegment {
                siblings: level
                    .iter()
                    .map(|digest| digest.map(Felt::to_u64))
                    .collect(),
            })
            .collect(),
    }
}

fn trace_column_count(unit: &lzvm_prover::ProveUnitSchedule) -> u64 {
    unit.stage_commit_widths
        .iter()
        .map(|width| u64::from(*width))
        .sum()
}

fn witness_trace_words(unit: &lzvm_prover::ProveUnitSchedule) -> Vec<u8> {
    let row_count = usize::try_from(unit.base_domain_size).expect("row count should fit");
    let column_count = usize::try_from(trace_column_count(unit)).expect("column count should fit");
    (0..row_count * column_count)
        .map(|index| u64::try_from(index + 1).expect("sample value should fit"))
        .flat_map(u64::to_le_bytes)
        .collect()
}

fn witness_stage_commitment(
    unit: &lzvm_prover::ProveUnitSchedule,
    stage_index: u32,
) -> lzvm_prover::witness_commitment::WitnessStageCommitment {
    let layout = derive_witness_trace_layout(unit).expect("witness layout should derive");
    let base_domain_size =
        usize::try_from(unit.base_domain_size).expect("base domain size should fit");
    let base_domain_bits =
        usize::try_from(unit.base_domain_bits).expect("base domain bits should fit");
    let extended_domain_bits =
        usize::try_from(unit.extended_domain_bits).expect("extended domain bits should fit");
    let arity = usize::try_from(unit.merkle_tree_arity).expect("arity should fit");
    let column_count = usize::try_from(trace_column_count(unit)).expect("column count should fit");
    let trace = parse_witness_trace(&witness_trace_words(unit), base_domain_size, column_count)
        .expect("witness trace should parse");
    let stage_index_usize = usize::try_from(stage_index).expect("stage index should fit");
    let stage = layout
        .stage_trace(&trace, stage_index_usize)
        .expect("witness stage should extract");
    let leaves = extend_witness_stage_leaves(&stage, base_domain_bits, extended_domain_bits)
        .expect("witness leaves should extend");
    commit_witness_stage_leaves(&leaves, arity).expect("witness stage should commit")
}

fn witness_commitment_segment(unit: &lzvm_prover::ProveUnitSchedule) -> ProofSegment {
    witness_commitment_segment_for_unit(unit, 0)
}

fn witness_commitment_segment_for_unit(
    unit: &lzvm_prover::ProveUnitSchedule,
    unit_index: u32,
) -> ProofSegment {
    let stages = unit
        .stage_commit_widths
        .iter()
        .enumerate()
        .map(|(index, _width)| {
            let stage_index = u32::try_from(index + 1).expect("stage index should fit");
            let commitment = witness_stage_commitment(unit, stage_index);
            WitnessCommitmentStageSegment {
                stage_index,
                arity: unit.merkle_tree_arity,
                root: commitment.root().map(Felt::to_u64),
                tree_byte_count: commitment.tree_bytes().len() as u64,
                tree_digest: [index as u8; 32],
            }
        })
        .collect();
    ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID + unit_index,
        data: encode_witness_commitment_segment(&WitnessCommitmentSegment {
            unit_index,
            input_byte_count: 0,
            trace_rows: unit.base_domain_size,
            trace_columns: trace_column_count(unit),
            stages,
        })
        .expect("witness commitment should encode"),
    }
}

fn witness_opening_segment(unit: &lzvm_prover::ProveUnitSchedule, query_row: u64) -> ProofSegment {
    witness_opening_segment_for_units(vec![witness_opening_unit_segment(unit, 0, query_row)])
}

fn witness_opening_segment_for_units(units: Vec<WitnessOpeningUnitSegment>) -> ProofSegment {
    ProofSegment {
        id: WITNESS_OPENING_SEGMENT_ID,
        data: encode_witness_opening_segment(&WitnessOpeningSegment { units })
            .expect("witness opening should encode"),
    }
}

fn witness_opening_unit_segment(
    unit: &lzvm_prover::ProveUnitSchedule,
    unit_index: u32,
    query_row: u64,
) -> WitnessOpeningUnitSegment {
    let stages = unit
        .stage_commit_widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            let stage_index = u32::try_from(index + 1).expect("stage index should fit");
            let commitment = witness_stage_commitment(unit, stage_index);
            let stage_width = usize::try_from(*width).expect("stage width should fit");
            let opening = open_witness_stage_commitment(
                &commitment,
                query_row,
                unit.extended_domain_size,
                stage_width,
            )
            .expect("witness row should open");
            witness_opening_stage(stage_index, &opening)
        })
        .collect();
    WitnessOpeningUnitSegment {
        unit_index,
        trace_instance_index: 0,
        queries: vec![WitnessOpeningQuerySegment {
            row_index: query_row,
            stages,
        }],
    }
}

fn witness_opening_stage(
    stage_index: u32,
    opening: &WitnessStageOpening,
) -> WitnessOpeningStageSegment {
    WitnessOpeningStageSegment {
        stage_index,
        values: opening
            .values()
            .iter()
            .map(|value| value.to_u64())
            .collect(),
        siblings: opening
            .siblings()
            .iter()
            .map(|level| WitnessOpeningLevelSegment {
                siblings: level
                    .iter()
                    .map(|digest| digest.map(Felt::to_u64))
                    .collect(),
            })
            .collect(),
    }
}

fn trace_constraint_segment(unit: &lzvm_prover::ProveUnitSchedule) -> ProofSegment {
    trace_constraint_segment_for_units(vec![trace_constraint_unit_segment(unit, 0)])
}

fn trace_constraint_segment_for_units(units: Vec<TraceConstraintUnitSegment>) -> ProofSegment {
    ProofSegment {
        id: TRACE_CONSTRAINT_SEGMENT_ID,
        data: encode_trace_constraint_segment(&TraceConstraintSegment { units })
            .expect("trace constraint evidence should encode"),
    }
}

fn trace_constraint_unit_segment(
    unit: &lzvm_prover::ProveUnitSchedule,
    unit_index: u32,
) -> TraceConstraintUnitSegment {
    TraceConstraintUnitSegment {
        unit_index,
        trace_instance_index: 0,
        trace_row_count: unit.base_domain_size,
        trace_column_count: u32::try_from(trace_column_count(unit))
            .expect("trace column count should fit"),
        regular_constraint_count: 0,
        trace_extracted: true,
        regular_constraints_evaluated: true,
        witness_values_committed: true,
        constraint_checker_conformant: true,
    }
}

fn seeded_required_fri_proof_without_opening(
    catalog: &KeyDirectoryCatalog,
    public_values: &PublicValues,
) -> ProofArtifact {
    let schedule = derive_prove_schedule(catalog).expect("schedule should derive");
    let unit = &schedule.units[0];
    let material =
        build_pcs_material_manifest_segment(&schedule).expect("material manifest should build");
    let witness = witness_commitment_segment(unit);
    let query = build_pcs_query_plan_segment(
        &schedule,
        public_values_digest(public_values).expect("digest should compute"),
        &material,
        std::slice::from_ref(&witness),
    )
    .expect("query plan should build");
    let query_plan = parse_pcs_query_plan_segment(&query.data).expect("query plan should parse");
    let query_row = query_plan.units[0].queries[0];

    ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(public_values).expect("digest should compute"),
        segments: vec![
            material,
            witness,
            trace_constraint_segment(unit),
            query,
            constant_opening_segment(query_row),
            witness_opening_segment(unit, query_row),
        ],
    }
}

fn seeded_proof_with_partial_unit_values(
    catalog: &KeyDirectoryCatalog,
    public_values: &PublicValues,
) -> ProofArtifact {
    let schedule = derive_prove_schedule(catalog).expect("schedule should derive");
    let material =
        build_pcs_material_manifest_segment(&schedule).expect("material manifest should build");
    let witness_segments = schedule
        .units
        .iter()
        .enumerate()
        .map(|(index, unit)| {
            witness_commitment_segment_for_unit(
                unit,
                u32::try_from(index).expect("unit index should fit"),
            )
        })
        .collect::<Vec<_>>();
    let query = build_pcs_query_plan_segment(
        &schedule,
        public_values_digest(public_values).expect("digest should compute"),
        &material,
        &witness_segments,
    )
    .expect("query plan should build");
    let query_plan = parse_pcs_query_plan_segment(&query.data).expect("query plan should parse");
    let constant_units = query_plan
        .units
        .iter()
        .map(|query_unit| {
            constant_opening_unit_segment(query_unit.unit_index, query_unit.queries[0])
        })
        .collect::<Vec<_>>();
    let witness_units = query_plan
        .units
        .iter()
        .map(|query_unit| {
            let unit_index = usize::try_from(query_unit.unit_index).expect("unit index should fit");
            witness_opening_unit_segment(
                &schedule.units[unit_index],
                query_unit.unit_index,
                query_unit.queries[0],
            )
        })
        .collect::<Vec<_>>();
    let trace_units = query_plan
        .units
        .iter()
        .map(|query_unit| {
            let unit_index = usize::try_from(query_unit.unit_index).expect("unit index should fit");
            trace_constraint_unit_segment(&schedule.units[unit_index], query_unit.unit_index)
        })
        .collect::<Vec<_>>();
    let unit_values = ProofSegment {
        id: UNIT_VALUES_SEGMENT_ID,
        data: encode_unit_values_segment(&UnitValuesSegment {
            units: vec![UnitValuesUnitSegment {
                unit_index: 1,
                trace_instance_index: 0,
                values: vec![17],
            }],
        })
        .expect("unit values should encode"),
    };

    let mut segments = vec![material];
    segments.extend(witness_segments);
    segments.push(trace_constraint_segment_for_units(trace_units));
    segments.push(query);
    segments.push(unit_values);
    segments.push(constant_opening_segment_for_units(constant_units));
    segments.push(witness_opening_segment_for_units(witness_units));

    ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(public_values).expect("digest should compute"),
        segments,
    }
}

fn tamper_first_witness_root(proof: &mut ProofArtifact) {
    let witness_segment = proof
        .segments
        .iter_mut()
        .find(|segment| segment.id == WITNESS_COMMITMENT_SEGMENT_BASE_ID)
        .expect("sample proof should contain a witness commitment segment");
    let mut witness = parse_witness_commitment_segment(&witness_segment.data)
        .expect("sample witness commitment should parse");
    witness.stages[0].root[0] ^= 1;
    witness_segment.data =
        encode_witness_commitment_segment(&witness).expect("tampered witness should encode");
}

fn tamper_first_witness_tree_digest(proof: &mut ProofArtifact) {
    let witness_segment = proof
        .segments
        .iter_mut()
        .find(|segment| segment.id == WITNESS_COMMITMENT_SEGMENT_BASE_ID)
        .expect("sample proof should contain a witness commitment segment");
    let mut witness = parse_witness_commitment_segment(&witness_segment.data)
        .expect("sample witness commitment should parse");
    witness.stages[0].tree_digest[0] ^= 1;
    witness_segment.data =
        encode_witness_commitment_segment(&witness).expect("tampered witness should encode");
}

fn assert_setup_preflight_hashes_reject_proof_artifact(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
    public_values: &PublicValues,
    expected: ProofArtifactError,
    message: &str,
) {
    assert_eq!(
        validate_setup_preflight_hashes(catalog, proof, public_values).expect_err(message),
        SetupPreflightError::Proof(ProofPreflightError::ProofArtifact(expected))
    );
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
            source_fixed_file_manifest_present: false,
            source_fixed_file_manifest_entry_count: 0,
            source_program_archive_present: false,
            source_program_archive_source_count: 0,
            source_program_archive_edge_count: 0,
            program_image_cache_count: 0,
            program_image_caches: Vec::new(),
            program_image_cache_hashes: Vec::new(),
            challenge_values_segment_count: 0,
            challenge_values_segment_byte_counts: Vec::new(),
            challenge_values_value_counts: Vec::new(),
            trace_constraint_segment_count: 0,
            trace_constraint_segment_byte_counts: Vec::new(),
            trace_constraint_units: Vec::new(),
            framed_guest_input_count: 0,
            framed_guest_input_hashes: Vec::new(),
            framed_guest_input_byte_counts: Vec::new(),
            framed_guest_input_chunk_counts: Vec::new(),
            eth_block_input_count: 0,
            eth_block_input_hashes: Vec::new(),
            eth_block_input_byte_counts: Vec::new(),
            eth_block_input_block_rlp_byte_counts: Vec::new(),
            eth_block_input_extra_header_field_counts: Vec::new(),
            eth_block_input_extra_body_field_counts: Vec::new(),
            eth_block_input_block_hashes: Vec::new(),
            eth_block_input_parent_hashes: Vec::new(),
            eth_block_input_ommers_hashes: Vec::new(),
            eth_block_input_beneficiaries: Vec::new(),
            eth_block_input_state_roots: Vec::new(),
            eth_block_input_receipt_roots: Vec::new(),
            eth_block_input_logs_blooms: Vec::new(),
            eth_block_input_difficulties: Vec::new(),
            eth_block_input_block_numbers: Vec::new(),
            eth_block_input_timestamps: Vec::new(),
            eth_block_input_extra_data: Vec::new(),
            eth_block_input_gas_limits: Vec::new(),
            eth_block_input_gas_used_values: Vec::new(),
            eth_block_input_base_fees_per_gas: Vec::new(),
            eth_block_input_mix_hashes: Vec::new(),
            eth_block_input_nonces: Vec::new(),
            eth_block_input_transaction_roots: Vec::new(),
            eth_block_input_transaction_preimage_counts: Vec::new(),
            eth_block_input_legacy_transaction_counts: Vec::new(),
            eth_block_input_typed_transaction_counts: Vec::new(),
            eth_block_input_receipts_rlp_byte_counts: Vec::new(),
            eth_block_input_receipt_preimage_counts: Vec::new(),
            eth_block_input_legacy_receipt_counts: Vec::new(),
            eth_block_input_typed_receipt_counts: Vec::new(),
            eth_block_input_withdrawal_roots: Vec::new(),
            eth_block_input_withdrawal_counts: Vec::new(),
            eth_block_input_withdrawal_preimage_counts: Vec::new(),
        }
    );
}

#[test]
fn validates_setup_preflight_reports_framed_guest_input() {
    let catalog = sample_catalog();
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof(&public_values);
    let framed_input = framed_stdin_chunk(&[3_u8, 5_u8, 8_u8]);
    let framed_segment = encode_framed_guest_input_segment(&framed_input)
        .expect("framed guest input segment should encode");
    let framed_segment_hash = framed_guest_input_segment_digest(&framed_segment);
    let framed_segment_len = framed_segment.len();
    proof.segments.push(ProofSegment {
        id: FRAMED_GUEST_INPUT_SEGMENT_ID,
        data: framed_segment,
    });

    let report = validate_setup_preflight_hashes(&catalog, &proof, &public_values)
        .expect("setup preflight hashes should validate framed guest input");

    assert_eq!(report.segment_count, 2);
    assert_eq!(report.framed_guest_input_count, 1);
    assert_eq!(report.framed_guest_input_hashes, vec![framed_segment_hash]);
    assert_eq!(
        report.framed_guest_input_byte_counts,
        vec![framed_segment_len]
    );
    assert_eq!(report.framed_guest_input_chunk_counts, vec![1]);
}

#[test]
fn rejects_setup_preflight_hashes_with_malformed_proof_artifacts() {
    let catalog = sample_catalog();
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);

    let mut missing_segments = sample_proof(&public_values);
    missing_segments.segments.clear();
    assert_setup_preflight_hashes_reject_proof_artifact(
        &catalog,
        &missing_segments,
        &public_values,
        ProofArtifactError::MissingSegments,
        "setup preflight hashes should reject proofs without segments",
    );

    let mut reserved_segment_id = sample_proof(&public_values);
    reserved_segment_id.segments[0].id = 1;
    assert_setup_preflight_hashes_reject_proof_artifact(
        &catalog,
        &reserved_segment_id,
        &public_values,
        ProofArtifactError::ReservedSegmentId { id: 1 },
        "setup preflight hashes should reject reserved segment ids",
    );

    let mut duplicate_segment_id = sample_proof(&public_values);
    duplicate_segment_id
        .segments
        .push(duplicate_segment_id.segments[0].clone());
    assert_setup_preflight_hashes_reject_proof_artifact(
        &catalog,
        &duplicate_segment_id,
        &public_values,
        ProofArtifactError::DuplicateSegmentId {
            id: SAMPLE_AUX_SEGMENT_ID,
        },
        "setup preflight hashes should reject duplicate segment ids",
    );

    let mut empty_segment = sample_proof(&public_values);
    empty_segment.segments[0].data.clear();
    assert_setup_preflight_hashes_reject_proof_artifact(
        &catalog,
        &empty_segment,
        &public_values,
        ProofArtifactError::EmptySegment {
            id: SAMPLE_AUX_SEGMENT_ID,
        },
        "setup preflight hashes should reject empty proof segments",
    );
}

#[test]
fn rejects_setup_preflight_hashes_with_runtime_unknown_proof_segments() {
    let catalog = sample_catalog();
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);
    let unknown_segment_id = 20_000;
    let mut proof = sample_proof(&public_values);
    proof.segments.push(ProofSegment {
        id: unknown_segment_id,
        data: vec![1],
    });

    let error = validate_setup_preflight_hashes(&catalog, &proof, &public_values)
        .expect_err("setup preflight should reject runtime-unknown proof segments");

    assert_eq!(
        error,
        SetupPreflightError::Proof(ProofPreflightError::UnexpectedProofSegment {
            id: unknown_segment_id
        })
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
fn rejects_setup_preflight_public_value_array_shape_mismatches() {
    let catalog = sample_catalog();
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let mut public_values = sample_public_values(setup_hash);
    public_values.values[1].elements.pop();
    let proof = sample_proof(&public_values);

    let error = validate_setup_preflight_hashes(&catalog, &proof, &public_values)
        .expect_err("public value array shape should match setup metadata");

    assert_eq!(
        error,
        SetupPreflightError::PublicValueElementCountMismatch {
            name: "state_root_words".to_owned(),
            expected: 4,
            found: 3,
        }
    );
}

#[test]
fn rejects_public_values_metadata_with_zero_array_dimensions() {
    let mut catalog = sample_catalog();
    catalog.layout.global_info.publics_map[1].lengths = vec![0];
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);

    let error = validate_public_values_metadata(&catalog.layout.global_info, &public_values)
        .expect_err("setup metadata validation should reject zero public dimensions");

    assert_eq!(error, SetupPreflightError::PublicValueCountOverflow);
}

#[test]
fn rejects_public_values_metadata_with_duplicate_names() {
    let catalog = sample_catalog();
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let mut public_values = sample_public_values(setup_hash);
    public_values.values[1].name = "block_number".to_owned();

    let error = validate_public_values_metadata(&catalog.layout.global_info, &public_values)
        .expect_err("setup metadata validation should reject duplicate public values");

    assert_eq!(
        error,
        SetupPreflightError::PublicValues(PublicValueFieldError::PublicValues(
            PublicValuesError::DuplicateName {
                name: "block_number".to_owned()
            }
        ))
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

#[test]
fn rejects_seeded_required_pcs_fri_unit_without_opening_segment() {
    let catalog = sample_catalog_with_fri_unit();
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_empty_public_values(setup_hash);
    let proof = seeded_required_fri_proof_without_opening(&catalog, &public_values);

    let error = validate_setup_preflight(&catalog, &proof, &public_values)
        .expect_err("seeded FRI-bearing proof should require an opening segment");

    assert_eq!(
        error,
        SetupPreflightError::PcsFri(
            ValidateOptionalPcsFriOpeningProofSegmentsError::OpeningSegment(
                LoadPcsFriOpeningSegmentError::MissingSegment
            )
        )
    );
}

#[test]
fn rejects_seeded_proof_when_unit_values_omit_query_unit() {
    let catalog = sample_catalog_with_unit_value_units();
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_empty_public_values(setup_hash);
    let proof = seeded_proof_with_partial_unit_values(&catalog, &public_values);

    let error = validate_setup_preflight(&catalog, &proof, &public_values)
        .expect_err("unit values should cover every query unit that requires them");

    assert_eq!(
        error,
        SetupPreflightError::UnitValues(LoadUnitValuesSegmentError::MissingUnit { unit_index: 0 })
    );
}

#[test]
fn rejects_seeded_proof_with_duplicate_trace_constraint_segments() {
    let catalog = sample_catalog_with_fri_unit();
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_empty_public_values(setup_hash);
    let mut proof = seeded_required_fri_proof_without_opening(&catalog, &public_values);
    let duplicate = proof
        .segments
        .iter()
        .find(|segment| segment.id == TRACE_CONSTRAINT_SEGMENT_ID)
        .expect("sample proof should contain trace constraint evidence")
        .clone();
    proof.segments.push(duplicate);

    let error = validate_setup_preflight(&catalog, &proof, &public_values)
        .expect_err("setup preflight should reject ambiguous trace constraint evidence");

    assert_eq!(
        error,
        SetupPreflightError::Proof(ProofPreflightError::ProofArtifact(
            ProofArtifactError::DuplicateSegmentId {
                id: TRACE_CONSTRAINT_SEGMENT_ID
            }
        ))
    );
}

#[test]
fn rejects_seeded_proof_when_witness_root_is_forged_after_query_plan() {
    let catalog = sample_catalog_with_fri_unit();
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_empty_public_values(setup_hash);
    let mut proof = seeded_required_fri_proof_without_opening(&catalog, &public_values);
    tamper_first_witness_root(&mut proof);

    let error = validate_setup_preflight(&catalog, &proof, &public_values)
        .expect_err("seeded proof should bind the query seed to witness roots");

    assert_eq!(
        error,
        SetupPreflightError::PcsQueryPlan(ValidatePcsQueryPlanSegmentsError::QueryPlanMismatch)
    );
}

#[test]
fn rejects_seeded_proof_when_witness_tree_digest_is_forged_after_query_plan() {
    let catalog = sample_catalog_with_fri_unit();
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_empty_public_values(setup_hash);
    let mut proof = seeded_required_fri_proof_without_opening(&catalog, &public_values);
    tamper_first_witness_tree_digest(&mut proof);

    let error = validate_setup_preflight(&catalog, &proof, &public_values)
        .expect_err("seeded proof should bind the query seed to witness tree digests");

    assert_eq!(
        error,
        SetupPreflightError::PcsQueryPlan(ValidatePcsQueryPlanSegmentsError::QueryPlanMismatch)
    );
}
