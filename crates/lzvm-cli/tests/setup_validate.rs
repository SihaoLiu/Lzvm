use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod fixtures;

use lzvm_artifacts::challenge_values_segment::{
    encode_challenge_values_segment, parse_challenge_values_segment, ChallengeValuesSegment,
    CHALLENGE_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::constant_opening_segment::{
    parse_constant_opening_segment, CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::constraint_program::{
    encode_global_constraint_program, encode_regular_constraint_program, ConstraintEntry,
    ConstraintProgram, GlobalConstraintEntry, GlobalConstraintProgram,
};
use lzvm_artifacts::contribution_segment::CONTRIBUTION_SEGMENT_ID;
use lzvm_artifacts::eth_block_input::{
    build_eth_block_input, build_eth_block_input_with_receipts, encode_eth_block_input,
    eth_block_input_bytes_digest, parse_eth_block_input, EthBlockInput,
};
use lzvm_artifacts::eth_block_input_segment::{
    encode_eth_block_input_segment, parse_eth_block_input_segment, ETH_BLOCK_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_public_values::public_values_from_eth_block_input;
use lzvm_artifacts::eth_public_input::parse_eth_public_block_prefix;
use lzvm_artifacts::eth_trie::{receipt_trie_build, withdrawals_trie_build};
use lzvm_artifacts::expression_info::{encode_expression_info, ExpressionInfo};
use lzvm_artifacts::expression_program::{
    encode_expression_program, ExpressionEntry, ExpressionProgram,
};
use lzvm_artifacts::global_info::{encode_global_info, GlobalInfo, PublicValue};
use lzvm_artifacts::group_values_segment::{
    encode_group_values_segment, parse_group_values_segment, GroupValuesSegment,
    GROUP_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::guest_image::parse_guest_image;
use lzvm_artifacts::hint_program::{
    encode_global_hint_program, encode_regular_hint_program,
    regular_hint_program_from_expression_info, Hint, HintField, HintOperand, HintProgram,
    HintValue, SOURCE_LOOKUP_PROVES_HINT,
};
use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, key_directory_catalog_digest_hex, read_key_directory_catalog,
    read_key_directory_layout, KeyUnitPaths,
};
use lzvm_artifacts::pcs_evaluation_segment::{
    encode_pcs_evaluation_segment, parse_pcs_evaluation_segment, PcsEvaluationSegment,
    PcsEvaluationUnitSegment, PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_fri_segment::{
    encode_pcs_fri_opening_segment, parse_pcs_fri_opening_segment, PcsFriOpeningLayerSegment,
    PcsFriOpeningLevelSegment, PcsFriOpeningQuerySegment, PcsFriOpeningSegment,
    PcsFriOpeningUnitSegment, PCS_FRI_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::pcs_material_segment::{
    parse_pcs_material_manifest_segment, PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::pcs_nonce_segment::PCS_QUERY_NONCE_SEGMENT_ID;
use lzvm_artifacts::pcs_proof_values_segment::{
    encode_pcs_proof_values_segment, parse_pcs_proof_values_segment, PcsProofValuesSegment,
    PCS_PROOF_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::{parse_pcs_query_plan_segment, PCS_QUERY_PLAN_SEGMENT_ID};
use lzvm_artifacts::program_image::{
    read_program_image_commitment_cache_file, ProgramImageCommitmentCache, ProgramImageGpuMode,
};
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, parse_program_image_cache_segment,
    program_image_cache_segment_digest, PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};
use lzvm_artifacts::proof::{
    encode_proof_artifact, parse_proof_artifact, ProofArtifact, ProofSegment,
};
use lzvm_artifacts::public_values::{
    encode_public_values, parse_public_values, public_values_digest, PublicValueEntry, PublicValues,
};
use lzvm_artifacts::rlp::parse_rlp;
use lzvm_artifacts::sectioned::{encode_sectioned_file, parse_sectioned_file, SectionedFile};
use lzvm_artifacts::setup_info::{encode_unit_setup_info, UnitSetupInfo};
use lzvm_artifacts::setup_manifest::{
    encode_setup_directory_manifest, read_setup_directory_manifest_file,
    SETUP_DIRECTORY_MANIFEST_FILE,
};
use lzvm_artifacts::source_fixed_file_manifest::{
    encode_source_fixed_file_manifest, SourceFixedFileManifest, SourceFixedFileManifestEntry,
    SourceFixedFileManifestKind,
};
use lzvm_artifacts::source_program::{
    encode_source_program_archive, SourceProgramArchive, SourceProgramArchiveEdge,
    SourceProgramArchiveIncludeKind, SourceProgramArchiveIncludeVisibility,
    SourceProgramArchiveSource,
};
use lzvm_artifacts::trace_bundle::{encode_trace_bundle, TraceBundle, TraceBundleUnit};
use lzvm_artifacts::unit_values_segment::{
    encode_unit_values_segment, parse_unit_values_segment, UnitValuesSegment,
    UnitValuesUnitSegment, UNIT_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::verification_key::{encode_verification_key_binary, VerificationKeyRoot};
use lzvm_artifacts::verifier_info::{encode_verifier_info, VerifierInfo};
use lzvm_artifacts::witness_library::parse_witness_library;
use lzvm_artifacts::witness_opening_segment::{
    parse_witness_opening_segment, WitnessOpeningLevelSegment, WitnessOpeningQuerySegment,
    WitnessOpeningSegment, WitnessOpeningStageSegment, WitnessOpeningUnitSegment,
    WITNESS_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, parse_witness_commitment_segment, WitnessCommitmentSegment,
    WitnessCommitmentStageSegment, WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_cli::{build_witness_proof_artifact, build_witness_proof_core_artifact, run_cli};
use lzvm_field::{poseidon2_hash_16, Ext3, Felt, MODULUS};
use lzvm_prover::contribution::{
    build_contribution_segment, derive_global_challenge_from_contributions,
    derive_global_challenge_from_proof_segments, ProveContributionEntry,
};
use lzvm_prover::guest_pc_trace_backend::GuestPcTraceBackend;
use lzvm_prover::pcs_fri::{verify_fri_fold, verify_fri_opening_folds, PcsFriOpeningFoldRequest};
use lzvm_prover::pcs_transcript::{
    derive_pcs_transcript_challenges_from_segments, PcsTranscriptSegmentInputs,
};
use lzvm_prover::proof_preflight::public_values_as_fields;
use lzvm_prover::setup_preflight::{validate_setup_preflight, validate_setup_preflight_from_files};
use lzvm_prover::unit_values::ProveUnitValues;
use lzvm_prover::verifier_query::{
    evaluate_verifier_unit_queries, verify_query_outputs_against_fri_opening,
    VerifierFriComparisonRequest, VerifierUnitQueryEvalRequest,
};
use lzvm_prover::witness_loader::TraceBytesBackend;
use lzvm_prover::{
    build_constant_opening_segment, build_pcs_material_manifest_segment,
    build_pcs_query_nonce_segment_from_transcript_segments, build_pcs_query_plan_segment,
    build_pcs_query_plan_segment_from_transcript_segments,
    build_pcs_query_plan_segment_with_bindings, build_witness_commitment_segment,
    build_witness_opening_segment, derive_prove_execution_plan, derive_prove_schedule,
    derive_prove_schedule_from_directory, run_prove_witness_commitments,
    run_prove_witness_commitments_with_trace_backend, GpuRunOptions, ProveExecutionInputArtifacts,
    ProvePartitionPlan, ProvePassRequest, ProveRunOptions, ProveRunRequest,
};
use lzvm_setup::{
    summarize_setup_directory, write_program_image_commitment_cache_file,
    ProgramImageCommitmentCacheFileRequest,
};

fn sample_expression_program() -> ExpressionProgram {
    ExpressionProgram {
        max_tmp1: 1,
        max_tmp3: 1,
        max_args: 1,
        max_ops: 1,
        entries: vec![ExpressionEntry {
            expression_id: 7,
            destination_dimension: 1,
            destination_id: 0,
            stage: 1,
            temp1_count: 0,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 1,
            args_offset: 0,
            source_line: "program-line".to_owned(),
        }],
        ops: vec![1],
        args: vec![2],
        numbers: vec![],
    }
}

fn sample_constant_fri_expression_program() -> ExpressionProgram {
    ExpressionProgram {
        max_tmp1: 0,
        max_tmp3: 1,
        max_args: 8,
        max_ops: 1,
        entries: vec![ExpressionEntry {
            expression_id: 7,
            destination_dimension: 3,
            destination_id: 0,
            stage: 2,
            temp1_count: 0,
            temp3_count: 1,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            source_line: "constant quotient expression".to_owned(),
        }],
        ops: vec![2],
        args: vec![0, 0, 8, 0, 0, 8, 3, 0],
        numbers: vec![10, 0, 0, 0, 0, 0],
    }
}

fn sample_regular_constraint_program() -> ConstraintProgram {
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
            source_line: "fixture regular constraint".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 0, 0, 0, 0, 0, 0],
        numbers: vec![],
    }
}

fn sample_proof_value_regular_constraint_program() -> ConstraintProgram {
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
            source_line: "proof value regular constraint".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 10, 0, 0, 1, 0, 0],
        numbers: Vec::new(),
    }
}

fn sample_unit_value_regular_constraint_program() -> ConstraintProgram {
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
            source_line: "unit value regular constraint".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 9, 0, 0, 1, 0, 0],
        numbers: Vec::new(),
    }
}

fn sample_challenge_regular_constraint_program() -> ConstraintProgram {
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
            source_line: "challenge regular constraint".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 12, 0, 0, 1, 0, 0],
        numbers: Vec::new(),
    }
}

fn sample_program_file_with_regular_constraints(program: ConstraintProgram) -> Vec<u8> {
    sample_program_file_with_expression_and_regular_constraints(
        sample_expression_program(),
        program,
    )
}

fn sample_program_file_with_expression_and_regular_constraints(
    expression_program: ExpressionProgram,
    program: ConstraintProgram,
) -> Vec<u8> {
    let expression =
        encode_expression_program(&expression_program).expect("expression program should encode");
    let regular =
        encode_regular_constraint_program(&program).expect("regular constraints should encode");
    let expression_info = fixtures::sample_expression_info();
    let regular_hints =
        regular_hint_program_from_expression_info(&expression_info).expect("hints should derive");
    let hints = encode_regular_hint_program(&regular_hints).expect("hints should encode");
    let mut expression_file =
        parse_sectioned_file(&expression, *b"chps", 1).expect("expression file should parse");
    let regular_file =
        parse_sectioned_file(&regular, *b"chps", 1).expect("regular file should parse");
    let hint_file = parse_sectioned_file(&hints, *b"chps", 1).expect("hints should parse");
    expression_file.sections.extend(regular_file.sections);
    expression_file.sections.extend(hint_file.sections);
    encode_sectioned_file(&SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: expression_file.sections,
    })
    .expect("combined program should encode")
}

fn empty_hint_program() -> HintProgram {
    HintProgram { hints: Vec::new() }
}

fn global_constraint_program_file(program: &GlobalConstraintProgram) -> Vec<u8> {
    global_program_file(program, &empty_hint_program())
}

fn global_program_file(program: &GlobalConstraintProgram, hints: &HintProgram) -> Vec<u8> {
    let constraints =
        encode_global_constraint_program(program).expect("global constraints should encode");
    let hints = encode_global_hint_program(hints).expect("global hints should encode");
    let mut constraints_file =
        parse_sectioned_file(&constraints, *b"chps", 1).expect("constraints should parse");
    let hint_file = parse_sectioned_file(&hints, *b"chps", 1).expect("hints should parse");
    constraints_file.sections.extend(hint_file.sections);
    encode_sectioned_file(&SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: constraints_file.sections,
    })
    .expect("combined global program should encode")
}

fn sample_public_values(setup_hash: [u8; 32]) -> PublicValues {
    PublicValues {
        schema_version: 1,
        setup_hash,
        values: vec![PublicValueEntry {
            name: "block_number".to_owned(),
            elements: vec![12_345],
        }],
    }
}

fn eth_block_public_values_with_rom_root(
    setup_hash: [u8; 32],
    input: &EthBlockInput,
    rom_root: [u64; 4],
) -> PublicValues {
    let mut public_values = public_values_from_eth_block_input(setup_hash, input);
    public_values.values.insert(
        0,
        PublicValueEntry {
            name: "rom_root".to_owned(),
            elements: rom_root.to_vec(),
        },
    );
    public_values
}

fn public_values_publics_map(public_values: &PublicValues) -> Vec<PublicValue> {
    public_values
        .values
        .iter()
        .map(|entry| PublicValue {
            name: entry.name.clone(),
            stage: 1,
            lengths: if entry.elements.len() == 1 {
                Vec::new()
            } else {
                vec![u64::try_from(entry.elements.len()).expect("public value count should fit")]
            },
        })
        .collect()
}

fn public_values_field_count(public_values: &PublicValues) -> u64 {
    public_values
        .values
        .iter()
        .map(|entry| u64::try_from(entry.elements.len()).expect("public value count should fit"))
        .sum()
}

fn global_info_with_public_values(public_values: &PublicValues) -> GlobalInfo {
    let mut info = fixtures::sample_global_info();
    info.n_publics = public_values_field_count(public_values);
    info.publics_map = public_values_publics_map(public_values);
    info
}

fn eth_block_public_values_metadata(input: &EthBlockInput) -> PublicValues {
    public_values_from_eth_block_input([0; 32], input)
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

fn assert_has_no_contribution_segment(proof: &ProofArtifact) {
    assert!(!proof
        .segments
        .iter()
        .any(|segment| segment.id == CONTRIBUTION_SEGMENT_ID));
}

fn sample_proof_with_material(
    public_values: &PublicValues,
    catalog: &lzvm_artifacts::key_directory::KeyDirectoryCatalog,
) -> ProofArtifact {
    let schedule = derive_prove_schedule(catalog).expect("schedule should derive");
    let material_segment =
        build_pcs_material_manifest_segment(&schedule).expect("material segment should build");
    let witness_segment = sample_witness_proof_segment(&schedule, 0);
    let query_segment = build_pcs_query_plan_segment(
        &schedule,
        public_values_digest(public_values).expect("digest should compute"),
        &material_segment,
        std::slice::from_ref(&witness_segment),
    )
    .expect("query segment should build");
    let constant_opening_segment =
        build_constant_opening_segment(catalog, &schedule, &query_segment)
            .expect("constant opening segment should build");
    let opening_segment = sample_witness_opening_segment(&schedule, &query_segment, 0);
    ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(public_values).expect("digest should compute"),
        segments: vec![
            material_segment,
            query_segment,
            constant_opening_segment,
            opening_segment,
            witness_segment,
        ],
    }
}

fn sample_proof_with_material_and_bad_witness(
    public_values: &PublicValues,
    catalog: &lzvm_artifacts::key_directory::KeyDirectoryCatalog,
) -> ProofArtifact {
    let schedule = derive_prove_schedule(catalog).expect("schedule should derive");
    let material_segment =
        build_pcs_material_manifest_segment(&schedule).expect("material segment should build");
    ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(public_values).expect("digest should compute"),
        segments: vec![
            material_segment,
            ProofSegment {
                id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
                data: vec![1, 2, 3, 4],
            },
        ],
    }
}

fn sample_witness_proof_segment(
    schedule: &lzvm_prover::ProveSchedule,
    unit_index: usize,
) -> ProofSegment {
    let unit = &schedule.units[unit_index];
    let trace_columns = unit
        .stage_commit_widths
        .iter()
        .map(|width| u64::from(*width))
        .sum();
    let stages = unit
        .stage_commit_widths
        .iter()
        .enumerate()
        .map(|(stage_offset, width)| {
            let stage_index = stage_offset + 1;
            WitnessCommitmentStageSegment {
                stage_index: stage_index as u32,
                arity: unit.merkle_tree_arity,
                root: sample_uniform_stage_root(stage_index, *width),
                tree_byte_count: 32,
                tree_digest: [stage_index as u8 + 11; 32],
            }
        })
        .collect();
    let segment = WitnessCommitmentSegment {
        unit_index: unit_index as u32,
        input_byte_count: 0,
        trace_rows: unit.base_domain_size,
        trace_columns,
        stages,
    };
    ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID + unit_index as u32,
        data: encode_witness_commitment_segment(&segment).expect("witness segment should encode"),
    }
}

fn sample_witness_opening_segment(
    schedule: &lzvm_prover::ProveSchedule,
    query_segment: &ProofSegment,
    unit_index: usize,
) -> ProofSegment {
    let unit = &schedule.units[unit_index];
    let query_plan =
        parse_pcs_query_plan_segment(&query_segment.data).expect("query plan should parse");
    let query_unit = query_plan
        .units
        .iter()
        .find(|unit| unit.unit_index == unit_index as u32)
        .expect("query unit should exist");
    let queries = query_unit
        .queries
        .iter()
        .map(|row_index| {
            let stages = unit
                .stage_commit_widths
                .iter()
                .enumerate()
                .map(|(stage_offset, width)| {
                    let stage_index = stage_offset + 1;
                    let value = sample_stage_value(stage_index);
                    let digest = sample_uniform_stage_leaf_digest(stage_index, *width);
                    WitnessOpeningStageSegment {
                        stage_index: stage_index as u32,
                        values: vec![value; *width as usize],
                        siblings: vec![WitnessOpeningLevelSegment {
                            siblings: vec![digest; unit.merkle_tree_arity as usize - 1],
                        }],
                    }
                })
                .collect();
            WitnessOpeningQuerySegment {
                row_index: *row_index,
                stages,
            }
        })
        .collect();
    let segment = WitnessOpeningSegment {
        units: vec![WitnessOpeningUnitSegment {
            unit_index: unit_index as u32,
            queries,
        }],
    };
    ProofSegment {
        id: WITNESS_OPENING_SEGMENT_ID,
        data: lzvm_artifacts::witness_opening_segment::encode_witness_opening_segment(&segment)
            .expect("opening segment should encode"),
    }
}

fn sample_pcs_evaluation_segment(unit_index: usize) -> ProofSegment {
    sample_pcs_evaluation_segment_with_values(unit_index, vec![[31, 32, 33], [41, 42, 43]])
}

fn sample_pcs_evaluation_segment_with_values(
    unit_index: usize,
    values: Vec<[u64; 3]>,
) -> ProofSegment {
    let segment = PcsEvaluationSegment {
        units: vec![PcsEvaluationUnitSegment {
            unit_index: unit_index as u32,
            values,
        }],
    };
    ProofSegment {
        id: PCS_EVALUATION_SEGMENT_ID,
        data: encode_pcs_evaluation_segment(&segment).expect("evaluation segment should encode"),
    }
}

fn sample_pcs_proof_values_segment(values: Vec<[u64; 3]>) -> ProofSegment {
    let segment = PcsProofValuesSegment { values };
    ProofSegment {
        id: PCS_PROOF_VALUES_SEGMENT_ID,
        data: encode_pcs_proof_values_segment(&segment)
            .expect("proof values segment should encode"),
    }
}

fn sample_challenge_values_segment(values: [u64; 3]) -> ProofSegment {
    ProofSegment {
        id: CHALLENGE_VALUES_SEGMENT_ID,
        data: encode_challenge_values_segment(&ChallengeValuesSegment {
            values: vec![values],
        })
        .expect("challenge values segment should encode"),
    }
}

fn sample_group_values_segment(values: Vec<[u64; 3]>) -> ProofSegment {
    let segment = GroupValuesSegment { values };
    ProofSegment {
        id: GROUP_VALUES_SEGMENT_ID,
        data: encode_group_values_segment(&segment).expect("group values segment should encode"),
    }
}

fn sample_unit_values_segment(unit_index: u32, values: Vec<u64>) -> ProofSegment {
    let segment = UnitValuesSegment {
        units: vec![UnitValuesUnitSegment { unit_index, values }],
    };
    ProofSegment {
        id: UNIT_VALUES_SEGMENT_ID,
        data: encode_unit_values_segment(&segment).expect("unit values segment should encode"),
    }
}

fn write_preflight_artifacts(
    root: &Path,
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> (PathBuf, PathBuf) {
    let proof_path = root.join("proof.bin");
    let public_values_path = root.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(public_values).expect("public values should encode"),
    );
    (proof_path, public_values_path)
}

fn sample_pcs_fri_opening_segment(
    schedule: &lzvm_prover::ProveSchedule,
    query_segment: &ProofSegment,
    unit_index: usize,
) -> ProofSegment {
    let unit = &schedule.units[unit_index];
    let query_plan =
        parse_pcs_query_plan_segment(&query_segment.data).expect("query plan should parse");
    let query_unit = query_plan
        .units
        .iter()
        .find(|unit| unit.unit_index == unit_index as u32)
        .expect("query unit should exist");
    let layers = unit
        .fri_layers
        .iter()
        .enumerate()
        .map(|(layer_index, layer)| {
            let output_domain = 1_u64 << layer.output_bits;
            let value_count = layer.folding_factor as usize;
            let query_values = vec![[11, 12, 13]; value_count];
            let leaf_digest = sample_fri_value_digest(&query_values);
            let mut last_level =
                vec![[0_u64; 4]; sample_fri_last_level_count(unit, layer.output_bits)];
            let queries = query_unit
                .queries
                .iter()
                .map(|row_index| {
                    let row_index = *row_index % output_domain;
                    if !last_level.is_empty() {
                        last_level[row_index as usize] = leaf_digest;
                    }
                    PcsFriOpeningQuerySegment {
                        row_index,
                        values: query_values.clone(),
                        siblings: Vec::<PcsFriOpeningLevelSegment>::new(),
                    }
                })
                .collect();
            let root = if last_level.is_empty() {
                leaf_digest
            } else {
                sample_digest_tree_root(last_level.clone(), unit.merkle_tree_arity as usize)
            };
            PcsFriOpeningLayerSegment {
                layer_index: layer_index as u32,
                root,
                last_level,
                queries,
            }
        })
        .collect();
    let segment = PcsFriOpeningSegment {
        units: vec![PcsFriOpeningUnitSegment {
            unit_index: unit_index as u32,
            layers,
            final_polynomial: vec![[21, 22, 23]; 1_usize << unit.final_layer_bits],
        }],
    };
    ProofSegment {
        id: PCS_FRI_OPENING_SEGMENT_ID,
        data: encode_pcs_fri_opening_segment(&segment).expect("FRI segment should encode"),
    }
}

fn sample_stable_pcs_fri_opening_segment(
    schedule: &lzvm_prover::ProveSchedule,
    query_segment: &ProofSegment,
    unit_index: usize,
) -> ProofSegment {
    let query_plan =
        parse_pcs_query_plan_segment(&query_segment.data).expect("query plan should parse");
    let query_unit = query_plan
        .units
        .iter()
        .find(|unit| unit.unit_index == unit_index as u32)
        .expect("query unit should exist");
    let unit = sample_stable_pcs_fri_opening_unit(schedule, &query_unit.queries, unit_index);
    let segment = PcsFriOpeningSegment { units: vec![unit] };
    ProofSegment {
        id: PCS_FRI_OPENING_SEGMENT_ID,
        data: encode_pcs_fri_opening_segment(&segment).expect("FRI segment should encode"),
    }
}

fn sample_stable_pcs_fri_opening_unit(
    schedule: &lzvm_prover::ProveSchedule,
    query_rows: &[u64],
    unit_index: usize,
) -> PcsFriOpeningUnitSegment {
    let unit = &schedule.units[unit_index];
    let layers = unit
        .fri_layers
        .iter()
        .enumerate()
        .map(|(layer_index, layer)| {
            let output_domain = 1_u64 << layer.output_bits;
            let value_count = layer.folding_factor as usize;
            let query_values = vec![[11, 12, 13]; value_count];
            let leaf_digest = sample_fri_value_digest(&query_values);
            let last_level =
                vec![leaf_digest; sample_fri_last_level_count(unit, layer.output_bits)];
            let root = sample_digest_tree_root(last_level.clone(), unit.merkle_tree_arity as usize);
            let queries = query_rows
                .iter()
                .map(|row_index| PcsFriOpeningQuerySegment {
                    row_index: *row_index % output_domain,
                    values: query_values.clone(),
                    siblings: Vec::<PcsFriOpeningLevelSegment>::new(),
                })
                .collect();
            PcsFriOpeningLayerSegment {
                layer_index: layer_index as u32,
                root,
                last_level,
                queries,
            }
        })
        .collect();
    PcsFriOpeningUnitSegment {
        unit_index: unit_index as u32,
        layers,
        final_polynomial: vec![[21, 22, 23]; 1_usize << unit.final_layer_bits],
    }
}

fn sample_folded_pcs_fri_opening_template(
    schedule: &lzvm_prover::ProveSchedule,
    material: &lzvm_artifacts::pcs_material_segment::PcsMaterialManifestUnit,
    public_values: &[Felt],
    witness: &WitnessCommitmentSegment,
    evaluations: &PcsEvaluationUnitSegment,
    unit_index: usize,
) -> PcsFriOpeningUnitSegment {
    sample_folded_pcs_fri_opening_template_with_values(
        schedule,
        material,
        public_values,
        witness,
        evaluations,
        unit_index,
        [31, 32, 33],
    )
}

fn sample_folded_pcs_fri_opening_template_with_values(
    schedule: &lzvm_prover::ProveSchedule,
    material: &lzvm_artifacts::pcs_material_segment::PcsMaterialManifestUnit,
    public_values: &[Felt],
    witness: &WitnessCommitmentSegment,
    evaluations: &PcsEvaluationUnitSegment,
    unit_index: usize,
    query_value: [u64; 3],
) -> PcsFriOpeningUnitSegment {
    sample_folded_pcs_fri_opening_template_with_values_and_unit_values(FoldedPcsFriTemplateInputs {
        schedule,
        material,
        public_values,
        witness,
        evaluations,
        unit_index,
        query_value,
        unit_values: &[],
    })
}

struct FoldedPcsFriTemplateInputs<'a> {
    schedule: &'a lzvm_prover::ProveSchedule,
    material: &'a lzvm_artifacts::pcs_material_segment::PcsMaterialManifestUnit,
    public_values: &'a [Felt],
    witness: &'a WitnessCommitmentSegment,
    evaluations: &'a PcsEvaluationUnitSegment,
    unit_index: usize,
    query_value: [u64; 3],
    unit_values: &'a [Felt],
}

fn sample_folded_pcs_fri_opening_template_with_values_and_unit_values(
    input: FoldedPcsFriTemplateInputs<'_>,
) -> PcsFriOpeningUnitSegment {
    let FoldedPcsFriTemplateInputs {
        schedule,
        material,
        public_values,
        witness,
        evaluations,
        unit_index,
        query_value,
        unit_values,
    } = input;
    let unit = &schedule.units[unit_index];
    assert_eq!(unit.fri_layers.len(), 1);
    let layer = &unit.fri_layers[0];
    let values = vec![query_value; layer.folding_factor as usize];
    let leaf_digest = sample_fri_value_digest(&values);
    let last_level = vec![leaf_digest; sample_fri_last_level_count(unit, layer.output_bits)];
    let root = sample_digest_tree_root(last_level.clone(), unit.merkle_tree_arity as usize);
    let mut template = PcsFriOpeningUnitSegment {
        unit_index: unit_index as u32,
        layers: vec![PcsFriOpeningLayerSegment {
            layer_index: 0,
            root,
            last_level,
            queries: Vec::new(),
        }],
        final_polynomial: vec![Ext3::ZERO.to_u64s(); 1_usize << unit.final_layer_bits],
    };
    let challenges = derive_pcs_transcript_challenges_from_segments(PcsTranscriptSegmentInputs {
        unit_index,
        unit,
        material,
        public_values,
        unit_values,
        witness,
        evaluations,
        fri: &template,
        root_challenge_draws: &unit.transcript_root_challenge_draws,
        evaluation_challenge_draws: unit.transcript_evaluation_challenge_draws,
        binding_segments: &[],
    })
    .expect("transcript challenges should derive");
    let fold_values = values
        .iter()
        .map(|value| Ext3::from_u64s(*value))
        .collect::<Vec<_>>();
    let challenge = challenges[unit.challenge_count + 1];
    for row_index in 0..template.final_polynomial.len() {
        template.final_polynomial[row_index] = verify_fri_fold(
            unit.extended_domain_bits,
            layer.output_bits,
            layer.input_bits,
            challenge,
            row_index as u64,
            &fold_values,
        )
        .expect("fold should evaluate")
        .to_u64s();
    }
    template
}

fn sample_folded_pcs_fri_opening_segment(
    schedule: &lzvm_prover::ProveSchedule,
    query_segment: &ProofSegment,
    unit_index: usize,
    unit: PcsFriOpeningUnitSegment,
) -> ProofSegment {
    sample_folded_pcs_fri_opening_segment_with_values(
        schedule,
        query_segment,
        unit_index,
        unit,
        [31, 32, 33],
    )
}

fn sample_folded_pcs_fri_opening_segment_with_values(
    schedule: &lzvm_prover::ProveSchedule,
    query_segment: &ProofSegment,
    unit_index: usize,
    mut unit: PcsFriOpeningUnitSegment,
    query_value: [u64; 3],
) -> ProofSegment {
    let query_plan =
        parse_pcs_query_plan_segment(&query_segment.data).expect("query plan should parse");
    let query_unit = query_plan
        .units
        .iter()
        .find(|unit| unit.unit_index == unit_index as u32)
        .expect("query unit should exist");
    let layer = &schedule.units[unit_index].fri_layers[0];
    let output_domain = 1_u64 << layer.output_bits;
    let values = vec![query_value; layer.folding_factor as usize];
    unit.layers[0].queries = query_unit
        .queries
        .iter()
        .map(|row_index| PcsFriOpeningQuerySegment {
            row_index: *row_index % output_domain,
            values: values.clone(),
            siblings: Vec::<PcsFriOpeningLevelSegment>::new(),
        })
        .collect();
    let segment = PcsFriOpeningSegment { units: vec![unit] };
    ProofSegment {
        id: PCS_FRI_OPENING_SEGMENT_ID,
        data: encode_pcs_fri_opening_segment(&segment).expect("FRI segment should encode"),
    }
}

fn sample_fri_value_digest(values: &[[u64; 3]]) -> [u64; 4] {
    let flattened = values
        .iter()
        .flat_map(|value| value.iter().copied().map(Felt::from_u64))
        .collect::<Vec<_>>();
    if flattened.len() <= 4 {
        let mut digest = [Felt::ZERO; 4];
        digest[..flattened.len()].copy_from_slice(&flattened);
        return digest.map(Felt::to_u64);
    }

    let mut state = [Felt::ZERO; 16];
    let mut offset = 0;
    while offset < flattened.len() {
        let capacity = [state[0], state[1], state[2], state[3]];
        state[12..].copy_from_slice(&capacity);
        state[..12].fill(Felt::ZERO);

        let chunk_len = (flattened.len() - offset).min(12);
        state[..chunk_len].copy_from_slice(&flattened[offset..offset + chunk_len]);
        state = poseidon2_hash_16(state);
        offset += chunk_len;
    }
    [state[0], state[1], state[2], state[3]].map(Felt::to_u64)
}

fn sample_fri_last_level_count(unit: &lzvm_prover::ProveUnitSchedule, output_bits: u32) -> usize {
    if unit.last_level_verification == 0 {
        return 0;
    }
    let arity = unit.merkle_tree_arity as usize;
    let mut count = 1_usize << output_bits;
    let target = arity.pow(unit.last_level_verification);
    while count > target {
        count = count.div_ceil(arity);
    }
    count
}

fn sample_digest_tree_root(mut level: Vec<[u64; 4]>, arity: usize) -> [u64; 4] {
    assert_eq!(arity, 4);
    if level.is_empty() {
        return [0; 4];
    }
    while level.len() > 1 {
        let extra_zeros = (arity - (level.len() % arity)) % arity;
        level.resize(level.len() + extra_zeros, [0; 4]);
        level = level
            .chunks_exact(arity)
            .map(sample_parent_arity4)
            .collect();
    }
    level[0]
}

fn sample_parent_arity4(children: &[[u64; 4]]) -> [u64; 4] {
    let input = [
        children[0][0],
        children[0][1],
        children[0][2],
        children[0][3],
        children[1][0],
        children[1][1],
        children[1][2],
        children[1][3],
        children[2][0],
        children[2][1],
        children[2][2],
        children[2][3],
        children[3][0],
        children[3][1],
        children[3][2],
        children[3][3],
    ]
    .map(Felt::from_u64);
    let state = poseidon2_hash_16(input);
    [state[0], state[1], state[2], state[3]].map(Felt::to_u64)
}

fn sample_uniform_stage_root(stage_index: usize, width: u32) -> [u64; 4] {
    let digest = sample_uniform_stage_leaf_digest(stage_index, width);
    let values = digest.map(Felt::from_u64);
    let state = poseidon2_hash_16([
        values[0], values[1], values[2], values[3], values[0], values[1], values[2], values[3],
        values[0], values[1], values[2], values[3], values[0], values[1], values[2], values[3],
    ]);
    [
        state[0].to_u64(),
        state[1].to_u64(),
        state[2].to_u64(),
        state[3].to_u64(),
    ]
}

fn sample_uniform_stage_leaf_digest(stage_index: usize, width: u32) -> [u64; 4] {
    assert!(width <= 4);
    let mut digest = [0_u64; 4];
    for value in digest.iter_mut().take(width as usize) {
        *value = sample_stage_value(stage_index);
    }
    digest
}

fn sample_stage_value(stage_index: usize) -> u64 {
    stage_index as u64 + 10
}

fn sample_raw_fixed_columns() -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [1_u64, 10, 2, 20] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
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

fn sample_guest_pc_trace_image() -> Vec<u8> {
    sample_guest_image_with_words(&[riscv_addi(1, 0, 7), riscv_addi(2, 1, 3), 0x0000_0073])
}

fn sample_guest_image_with_words(words: &[u32]) -> Vec<u8> {
    const ENTRY: u64 = 0x8000_0000;
    const ELF_HEADER_BYTES: usize = 64;
    const PROGRAM_HEADER_BYTES: usize = 56;
    const CODE_OFFSET: usize = ELF_HEADER_BYTES + PROGRAM_HEADER_BYTES;

    let mut bytes = vec![0_u8; CODE_OFFSET];
    bytes[0..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2;
    bytes[5] = 1;
    bytes[6] = 1;
    bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&243_u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&ENTRY.to_le_bytes());
    bytes[32..40].copy_from_slice(&(ELF_HEADER_BYTES as u64).to_le_bytes());
    bytes[52..54].copy_from_slice(&(ELF_HEADER_BYTES as u16).to_le_bytes());
    bytes[54..56].copy_from_slice(&(PROGRAM_HEADER_BYTES as u16).to_le_bytes());
    bytes[56..58].copy_from_slice(&1_u16.to_le_bytes());

    let code_bytes = (words.len() * 4) as u64;
    let program_header = &mut bytes[ELF_HEADER_BYTES..CODE_OFFSET];
    program_header[0..4].copy_from_slice(&1_u32.to_le_bytes());
    program_header[4..8].copy_from_slice(&5_u32.to_le_bytes());
    program_header[8..16].copy_from_slice(&(CODE_OFFSET as u64).to_le_bytes());
    program_header[16..24].copy_from_slice(&ENTRY.to_le_bytes());
    program_header[24..32].copy_from_slice(&ENTRY.to_le_bytes());
    program_header[32..40].copy_from_slice(&code_bytes.to_le_bytes());
    program_header[40..48].copy_from_slice(&code_bytes.to_le_bytes());
    program_header[48..56].copy_from_slice(&4_u64.to_le_bytes());

    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes
}

fn riscv_addi(rd: u32, rs1: u32, immediate: i32) -> u32 {
    assert!((-2048..=2047).contains(&immediate));
    let immediate = (immediate as u32) & 0x0fff;
    (immediate << 20) | (rs1 << 15) | (rd << 7) | 0x13
}

fn sample_block_rlp() -> Vec<u8> {
    sample_block_rlp_with_extra(b"lzvm")
}

fn sample_block_rlp_variant() -> Vec<u8> {
    sample_block_rlp_with_extra(b"lzvm-alt")
}

fn sample_public_block_bytes_with_matching_roots() -> Vec<u8> {
    let mut input = sample_public_header_bytes();
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&eip1559_transaction_bytes());
    input.extend_from_slice(&0_u64.to_le_bytes());
    input.push(1);
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&public_withdrawal_bytes());

    let parsed = parse_eth_public_block_prefix(&input).expect("block should parse");
    let transaction_root = parsed.transactions_root();
    let ommers_hash = parsed.ommers_hash();
    let withdrawal_root = parsed
        .withdrawals_root()
        .expect("withdrawals root should be present");
    input[48..80].copy_from_slice(&ommers_hash);
    input[156..188].copy_from_slice(&transaction_root);
    input[237..269].copy_from_slice(&withdrawal_root);
    input
}

fn sample_public_header_bytes() -> Vec<u8> {
    let mut input = Vec::new();
    push_public_bytes(&mut input, &[1; 32]);
    push_public_bytes(&mut input, &[2; 32]);
    push_public_bytes(&mut input, &[3; 20]);
    push_public_bytes(&mut input, &[4; 32]);
    push_public_bytes(&mut input, &[5; 32]);
    push_public_bytes(&mut input, &[6; 32]);
    push_public_option_bytes(&mut input, Some(&[7; 32]));
    push_public_bytes(&mut input, &[8; 256]);
    push_public_bytes(&mut input, &u256_bytes(9));
    input.extend_from_slice(&42_u64.to_le_bytes());
    input.extend_from_slice(&100_u64.to_le_bytes());
    input.extend_from_slice(&90_u64.to_le_bytes());
    input.extend_from_slice(&77_u64.to_le_bytes());
    push_public_bytes(&mut input, &[10; 32]);
    push_public_bytes(&mut input, &[11; 8]);
    push_public_option_u64(&mut input, Some(123));
    push_public_option_u64(&mut input, Some(456));
    push_public_option_u64(&mut input, Some(789));
    push_public_option_bytes(&mut input, Some(&[12; 32]));
    push_public_option_bytes(&mut input, Some(&[13; 32]));
    push_public_bytes(&mut input, b"abc");
    input
}

fn eip1559_transaction_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_public_u256(&mut bytes, 0x11);
    push_public_u256(&mut bytes, 0x22);
    push_public_uint_u64(&mut bytes, 1);
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&7_u64.to_le_bytes());
    bytes.extend_from_slice(&21_000_u64.to_le_bytes());
    bytes.extend_from_slice(&300_u128.to_le_bytes());
    bytes.extend_from_slice(&20_u128.to_le_bytes());
    push_public_option_bytes(&mut bytes, Some(&[9; 20]));
    push_public_u256(&mut bytes, 123);
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    push_public_bytes(&mut bytes, b"call-data");
    bytes
}

fn public_withdrawal_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_public_uint_u64(&mut bytes, 7);
    push_public_uint_u64(&mut bytes, 8);
    push_public_bytes(&mut bytes, &[6; 20]);
    push_public_uint_u64(&mut bytes, 9);
    bytes
}

fn push_public_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_public_option_bytes(out: &mut Vec<u8>, bytes: Option<&[u8]>) {
    match bytes {
        Some(bytes) => {
            out.push(1);
            push_public_bytes(out, bytes);
        }
        None => out.push(0),
    }
}

fn push_public_option_u64(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            out.push(1);
            out.extend_from_slice(&value.to_le_bytes());
        }
        None => out.push(0),
    }
}

fn push_public_u256(out: &mut Vec<u8>, value: u8) {
    let mut bytes = [0; 32];
    bytes[31] = value;
    push_public_bytes(out, &bytes);
}

fn push_public_uint_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&8_u64.to_le_bytes());
    out.extend_from_slice(&value.to_be_bytes());
}

fn u256_bytes(value: u8) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[31] = value;
    bytes
}

fn sample_block_rlp_with_receipts_root(receipts_root: [u8; 32]) -> Vec<u8> {
    let header_rlp = rlp_list(&legacy_header_items_with_receipts_and_logs_bloom(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        receipts_root,
        [0; 256],
        &[0x52, 0x08],
        None,
        b"lzvm",
    ));
    let transactions = rlp_list(&[rlp_list(&[rlp_bytes(&[1])])]);
    let empty_list = rlp_list(&[]);
    rlp_list(&[header_rlp, transactions, empty_list])
}

fn sample_block_rlp_with_withdrawals(
    withdrawals_root: [u8; 32],
    withdrawal_items: Vec<Vec<u8>>,
) -> Vec<u8> {
    let header_rlp = rlp_list(&legacy_header_items(
        hex32("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"),
        Some(withdrawals_root),
        b"lzvm",
    ));
    let empty_list = rlp_list(&[]);
    let withdrawals = rlp_list(&withdrawal_items);
    rlp_list(&[header_rlp, empty_list.clone(), empty_list, withdrawals])
}

fn sample_block_rlp_with_extra(extra_data: &[u8]) -> Vec<u8> {
    let header_rlp = rlp_list(&legacy_header_items(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        None,
        extra_data,
    ));
    let transactions = rlp_list(&[rlp_list(&[rlp_bytes(&[1])])]);
    let empty_list = rlp_list(&[]);
    rlp_list(&[header_rlp, transactions, empty_list])
}

fn sample_block_rlp_with_extra_fields() -> Vec<u8> {
    let empty_root = hex32("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421");
    let mut header_items = legacy_header_items(empty_root, Some(empty_root), b"lzvm");
    header_items.push(rlp_bytes(&[0xee]));
    let header_rlp = rlp_list(&header_items);
    let empty_list = rlp_list(&[]);
    rlp_list(&[
        header_rlp,
        empty_list.clone(),
        empty_list.clone(),
        empty_list,
        rlp_bytes(&[0xdd]),
    ])
}

fn legacy_header_items(
    transactions_root: [u8; 32],
    withdrawals_root: Option<[u8; 32]>,
    extra_data: &[u8],
) -> Vec<Vec<u8>> {
    legacy_header_items_with_receipts_and_logs_bloom(
        transactions_root,
        [0x66; 32],
        [0x77; 256],
        &[0x0d, 0xbb, 0xa0],
        withdrawals_root,
        extra_data,
    )
}

fn legacy_header_items_with_receipts_and_logs_bloom(
    transactions_root: [u8; 32],
    receipts_root: [u8; 32],
    logs_bloom: [u8; 256],
    gas_used: &[u8],
    withdrawals_root: Option<[u8; 32]>,
    extra_data: &[u8],
) -> Vec<Vec<u8>> {
    let mut items = vec![
        rlp_bytes(&[0x11; 32]),
        rlp_bytes(&hex32(
            "1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
        )),
        rlp_bytes(&[0x33; 20]),
        rlp_bytes(&[0x44; 32]),
        rlp_bytes(&transactions_root),
        rlp_bytes(&receipts_root),
        rlp_bytes(&logs_bloom),
        rlp_bytes(&[1]),
        rlp_bytes(&[2]),
        rlp_bytes(&[0x0f, 0x42, 0x40]),
        rlp_bytes(gas_used),
        rlp_bytes(&[0x65]),
        rlp_bytes(extra_data),
        rlp_bytes(&[0xaa; 32]),
        rlp_bytes(&[0xbb; 8]),
    ];
    if let Some(root) = withdrawals_root {
        items.push(rlp_bytes(&[1]));
        items.push(rlp_bytes(&root));
    }
    items
}

fn sample_receipt_item() -> Vec<u8> {
    rlp_list(&[
        rlp_bytes(&[1]),
        rlp_bytes(&[0x52, 0x08]),
        rlp_bytes(&[0; 256]),
        rlp_list(&[]),
    ])
}

fn sample_withdrawal_item() -> Vec<u8> {
    rlp_list(&[
        rlp_bytes(&[]),
        rlp_bytes(&[1]),
        rlp_bytes(&[0x22; 20]),
        rlp_bytes(&[0x40]),
    ])
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

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-cli-{}-{name}", std::process::id()))
}

fn write_bytes(path: &Path, value: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, value).expect("fixture file should be written");
}

fn assert_external_contribution_challenge_verifies(
    setup_dir: &Path,
    public_values_path: &Path,
    proof_path: &Path,
    challenge_values_path: &Path,
) {
    let mut writer_stdout = Vec::new();
    let mut writer_stderr = Vec::new();
    let writer_code = run_cli(
        &[
            "prove",
            "write-contribution-challenges",
            setup_dir.to_str().expect("setup path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
            challenge_values_path
                .to_str()
                .expect("challenge values path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
        ],
        &mut writer_stdout,
        &mut writer_stderr,
    );
    assert_eq!(
        writer_code,
        0,
        "{}",
        String::from_utf8_lossy(&writer_stderr)
    );
    assert!(writer_stderr.is_empty());

    let mut challenge_stdout = Vec::new();
    let mut challenge_stderr = Vec::new();
    let challenge_code = run_cli(
        &[
            "verify",
            "contribution-challenge",
            setup_dir.to_str().expect("setup path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
            challenge_values_path
                .to_str()
                .expect("challenge values path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
        ],
        &mut challenge_stdout,
        &mut challenge_stderr,
    );
    assert_eq!(
        challenge_code,
        0,
        "{}",
        String::from_utf8_lossy(&challenge_stderr)
    );
    assert!(challenge_stderr.is_empty());
}

fn write_sample_program_image_cache(
    root: &Path,
    guest_image: &Path,
    setup_hash: [u8; 32],
    tree_root: [u64; 4],
) -> PathBuf {
    let program_path = root.join("program.bin");
    let constraint_digest_path = root.join("constraint.digest");
    let root_path = root.join("root.bin");
    let cache_path = root.join("program_image.cache");
    write_bytes(&program_path, b"packed-program");
    write_bytes(&constraint_digest_path, setup_hash);
    write_bytes(
        &root_path,
        encode_verification_key_binary(&VerificationKeyRoot::FieldElements(tree_root.to_vec()))
            .expect("root should encode"),
    );
    write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
        program_path: &program_path,
        guest_image_path: guest_image,
        constraint_digest_path: &constraint_digest_path,
        root_path: &root_path,
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
        output_path: &cache_path,
    })
    .expect("cache should write");
    cache_path
}

fn write_unit_setup_metadata(path: &Path, setup: &UnitSetupInfo) {
    let bytes = encode_unit_setup_info(setup).expect("setup metadata should encode");
    write_bytes(path, bytes);
}

fn write_expression_metadata(path: &Path, expressions: &ExpressionInfo) {
    let bytes = encode_expression_info(expressions).expect("expression metadata should encode");
    write_bytes(path, bytes);
}

fn write_verifier_metadata(path: &Path, verifier: &VerifierInfo) {
    let bytes = encode_verifier_info(verifier).expect("verifier metadata should encode");
    write_bytes(path, bytes);
}

fn write_global_metadata(path: &Path, info: &GlobalInfo) {
    let bytes = encode_global_info(info).expect("global metadata should encode");
    write_bytes(path, bytes);
}

fn write_field_words(path: &Path, values: &[u64]) {
    let mut bytes = Vec::with_capacity(values.len() * 8);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    write_bytes(path, bytes);
}

fn sample_trace_bytes(seed: u64) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(4 * 8);
    for value in seed + 1..=seed + 4 {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn sample_trace_bundle_bytes(unit_count: u32, seed: u64) -> Vec<u8> {
    encode_trace_bundle(&TraceBundle {
        units: (0..unit_count)
            .map(|unit_index| TraceBundleUnit {
                unit_index,
                trace_bytes: sample_trace_bytes(seed),
            })
            .collect(),
    })
    .expect("trace bundle should encode")
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

fn sample_witness_source() -> &'static str {
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
    const size_t rows = 2;
    const size_t columns = 2;
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

fn write_global_files_with_info(root: &Path, global_info: &GlobalInfo) {
    fs::create_dir_all(root).expect("fixture root should be created");
    write_global_metadata(&root.join("pilout.globalInfo.bin"), global_info);
    fs::write(
        root.join("pilout.globalConstraints.bin"),
        global_constraint_program_file(&GlobalConstraintProgram {
            entries: vec![],
            ops: vec![],
            args: vec![],
            numbers: vec![],
        }),
    )
    .expect("global constraints program should be written");
}

fn write_global_constraint_program(root: &Path, program: GlobalConstraintProgram) {
    fs::write(
        root.join("pilout.globalConstraints.bin"),
        global_constraint_program_file(&program),
    )
    .expect("global constraints program should be written");
}

fn write_global_program(root: &Path, program: GlobalConstraintProgram, hints: HintProgram) {
    fs::write(
        root.join("pilout.globalConstraints.bin"),
        global_program_file(&program, &hints),
    )
    .expect("global program should be written");
}

fn write_global_files(root: &Path) {
    write_global_files_with_info(root, &fixtures::sample_global_info());
}

fn write_unit_files_with_setup_info_verifier_and_regular_constraints(
    unit: &KeyUnitPaths,
    setup_info: &UnitSetupInfo,
    verifier_info: &VerifierInfo,
    regular_constraints: ConstraintProgram,
) {
    if let Some(path) = unit.setup_info_binary() {
        write_unit_setup_metadata(&path, setup_info);
    }
    if let Some(path) = unit.expression_info_binary() {
        write_expression_metadata(&path, &fixtures::sample_expression_info());
    }
    if let Some(path) = unit.verifier_info_binary() {
        write_verifier_metadata(&path, verifier_info);
    }

    let program = sample_program_file_with_regular_constraints(regular_constraints);
    if let Some(path) = unit.expression_program() {
        write_bytes(&path, &program);
    }
    let verifier_program = encode_expression_program(&sample_expression_program())
        .expect("verifier program should encode");
    if let Some(path) = unit.verifier_program() {
        write_bytes(&path, &verifier_program);
    }

    let root = VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4]);
    write_bytes(
        &unit.verification_key_binary(),
        encode_verification_key_binary(&root).expect("verification key should encode"),
    );
    write_bytes(&unit.fixed_columns, sample_raw_fixed_columns());
}

fn write_unit_files_with_verifier_info_and_regular_constraints(
    unit: &KeyUnitPaths,
    verifier_info: &VerifierInfo,
    regular_constraints: ConstraintProgram,
) {
    write_unit_files_with_setup_info_verifier_and_regular_constraints(
        unit,
        &fixtures::sample_setup_info(),
        verifier_info,
        regular_constraints,
    );
}

fn write_unit_files_with_fri_quotient(unit: &KeyUnitPaths) {
    if let Some(path) = unit.setup_info_binary() {
        write_unit_setup_metadata(&path, &fixtures::sample_setup_info());
    }
    if let Some(path) = unit.expression_info_binary() {
        write_expression_metadata(&path, &fixtures::sample_fri_quotient_expression_info());
    }
    if let Some(path) = unit.verifier_info_binary() {
        write_verifier_metadata(&path, &fixtures::sample_fri_quotient_verifier_info());
    }

    let program = sample_program_file_with_expression_and_regular_constraints(
        sample_constant_fri_expression_program(),
        sample_regular_constraint_program(),
    );
    if let Some(path) = unit.expression_program() {
        write_bytes(&path, &program);
    }
    let verifier_program = encode_expression_program(&sample_expression_program())
        .expect("verifier program should encode");
    if let Some(path) = unit.verifier_program() {
        write_bytes(&path, &verifier_program);
    }

    let root = VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4]);
    write_bytes(
        &unit.verification_key_binary(),
        encode_verification_key_binary(&root).expect("verification key should encode"),
    );
    write_bytes(&unit.fixed_columns, sample_raw_fixed_columns());
}

fn write_unit_files_with_fri_quotient_and_unit_value(unit: &KeyUnitPaths) {
    if let Some(path) = unit.setup_info_binary() {
        write_unit_setup_metadata(&path, &fixtures::sample_setup_info_with_unit_value());
    }
    if let Some(path) = unit.expression_info_binary() {
        write_expression_metadata(&path, &fixtures::sample_fri_quotient_expression_info());
    }
    if let Some(path) = unit.verifier_info_binary() {
        write_verifier_metadata(&path, &fixtures::sample_fri_quotient_verifier_info());
    }

    let program = sample_program_file_with_expression_and_regular_constraints(
        sample_constant_fri_expression_program(),
        sample_regular_constraint_program(),
    );
    if let Some(path) = unit.expression_program() {
        write_bytes(&path, &program);
    }
    let verifier_program = encode_expression_program(&sample_expression_program())
        .expect("verifier program should encode");
    if let Some(path) = unit.verifier_program() {
        write_bytes(&path, &verifier_program);
    }

    let root = VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4]);
    write_bytes(
        &unit.verification_key_binary(),
        encode_verification_key_binary(&root).expect("verification key should encode"),
    );
    write_bytes(&unit.fixed_columns, sample_raw_fixed_columns());
}

fn write_unit_files_with_verifier_info(unit: &KeyUnitPaths, verifier_info: &VerifierInfo) {
    write_unit_files_with_verifier_info_and_regular_constraints(
        unit,
        verifier_info,
        sample_regular_constraint_program(),
    );
}

fn write_unit_files(unit: &KeyUnitPaths) {
    write_unit_files_with_verifier_info(unit, &fixtures::sample_verifier_info());
}

fn write_setup_directory(root: &Path) {
    write_global_files(root);
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files(unit);
    }
}

fn write_setup_directory_with_public_values(root: &Path, public_values: &PublicValues) {
    write_global_files_with_info(root, &global_info_with_public_values(public_values));
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files(unit);
    }
}

fn sample_source_global_info() -> GlobalInfo {
    let mut info = fixtures::sample_global_info();
    info.air_groups = vec!["GroupA".to_owned()];
    info.airs[0][0].name = "UnitA".to_owned();
    info
}

fn write_source_key_inputs(root: &Path) {
    write_global_files_with_info(root, &sample_source_global_info());
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        if let Some(path) = unit.setup_info_binary() {
            write_unit_setup_metadata(&path, &fixtures::sample_setup_info());
        }
        if let Some(path) = unit.expression_info_binary() {
            write_expression_metadata(&path, &fixtures::sample_expression_info());
        }
        if let Some(path) = unit.verifier_info_binary() {
            write_verifier_metadata(&path, &fixtures::sample_verifier_info());
        }
        if let Some(path) = unit.expression_program() {
            write_bytes(
                &path,
                sample_program_file_with_regular_constraints(sample_regular_constraint_program()),
            );
        }
        let verifier_program = encode_expression_program(&sample_expression_program())
            .expect("verifier program should encode");
        if let Some(path) = unit.verifier_program() {
            write_bytes(&path, &verifier_program);
        }
    }
}

fn write_source_fixed_file_manifest(root: &Path) {
    write_source_fixed_file_manifest_with_span(root, 10, 40);
}

fn write_source_fixed_file_manifest_with_span(root: &Path, start: u64, end: u64) {
    let layout = read_key_directory_layout(root).expect("layout should parse");
    let manifest = SourceFixedFileManifest {
        entries: vec![SourceFixedFileManifestEntry {
            source_name: "main.pil".to_owned(),
            kind: SourceFixedFileManifestKind::OutputFixedFile,
            path: Some("unit-a.fixed".to_owned()),
            column: None,
            group_name: "group-a".to_owned(),
            group_id: 0,
            unit_id: 0,
            unit_name: "unit-a".to_owned(),
            template_name: "Main".to_owned(),
            virtual_instance: false,
            start,
            end,
        }],
    };
    write_bytes(
        &layout.source_fixed_file_manifest,
        encode_source_fixed_file_manifest(&manifest)
            .expect("source fixed-file manifest should encode"),
    );
}

fn write_source_program_archive(root: &Path) {
    let layout = read_key_directory_layout(root).expect("layout should parse");
    let archive = SourceProgramArchive {
        sources: vec![
            SourceProgramArchiveSource {
                source_name: "main.pil".to_owned(),
                contents:
                    "include \"shared.pil\";\ncol witness main.trace;\ncol witness aux.trace;"
                        .to_owned(),
            },
            SourceProgramArchiveSource {
                source_name: "shared.pil".to_owned(),
                contents: "col fixed shared = [1, 2];".to_owned(),
            },
        ],
        edges: vec![SourceProgramArchiveEdge {
            from_index: 0,
            to_index: 1,
            request: "shared.pil".to_owned(),
            kind: SourceProgramArchiveIncludeKind::Include,
            visibility: SourceProgramArchiveIncludeVisibility::Public,
        }],
    };
    write_bytes(
        &layout.source_program_archive,
        encode_source_program_archive(&archive).expect("source program archive should encode"),
    );
}

fn source_fixed_file_manifest_bytes(root: &Path) -> u64 {
    let layout = read_key_directory_layout(root).expect("layout should parse");
    fs::metadata(&layout.source_fixed_file_manifest)
        .expect("source fixed-file manifest should exist")
        .len()
}

fn source_program_archive_bytes(root: &Path) -> u64 {
    let layout = read_key_directory_layout(root).expect("layout should parse");
    fs::metadata(&layout.source_program_archive)
        .expect("source program archive should exist")
        .len()
}

fn write_source_program_archive_with_multibyte_manifest_source(root: &Path) {
    let layout = read_key_directory_layout(root).expect("layout should parse");
    let archive = SourceProgramArchive {
        sources: vec![
            SourceProgramArchiveSource {
                source_name: "main.pil".to_owned(),
                contents: format!("a{}z", '\u{00e9}'),
            },
            SourceProgramArchiveSource {
                source_name: "shared.pil".to_owned(),
                contents: "col fixed shared = [1, 2];".to_owned(),
            },
        ],
        edges: vec![SourceProgramArchiveEdge {
            from_index: 0,
            to_index: 1,
            request: "shared.pil".to_owned(),
            kind: SourceProgramArchiveIncludeKind::Include,
            visibility: SourceProgramArchiveIncludeVisibility::Public,
        }],
    };
    write_bytes(
        &layout.source_program_archive,
        encode_source_program_archive(&archive).expect("source program archive should encode"),
    );
}

fn write_source_program_archive_with_short_manifest_source(root: &Path) {
    let layout = read_key_directory_layout(root).expect("layout should parse");
    let archive = SourceProgramArchive {
        sources: vec![
            SourceProgramArchiveSource {
                source_name: "main.pil".to_owned(),
                contents: "col witness a;".to_owned(),
            },
            SourceProgramArchiveSource {
                source_name: "shared.pil".to_owned(),
                contents: "col fixed shared = [1, 2];".to_owned(),
            },
        ],
        edges: vec![SourceProgramArchiveEdge {
            from_index: 0,
            to_index: 1,
            request: "shared.pil".to_owned(),
            kind: SourceProgramArchiveIncludeKind::Include,
            visibility: SourceProgramArchiveIncludeVisibility::Public,
        }],
    };
    write_bytes(
        &layout.source_program_archive,
        encode_source_program_archive(&archive).expect("source program archive should encode"),
    );
}

fn write_source_program_archive_without_manifest_source(root: &Path) {
    let layout = read_key_directory_layout(root).expect("layout should parse");
    let archive = SourceProgramArchive {
        sources: vec![SourceProgramArchiveSource {
            source_name: "shared.pil".to_owned(),
            contents: "col fixed shared = [1, 2];".to_owned(),
        }],
        edges: Vec::new(),
    };
    write_bytes(
        &layout.source_program_archive,
        encode_source_program_archive(&archive).expect("source program archive should encode"),
    );
}

fn write_setup_directory_with_fri_quotient(root: &Path) {
    write_global_files(root);
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files_with_fri_quotient(unit);
    }
}

fn write_setup_directory_with_fri_quotient_and_unit_value(root: &Path) {
    write_global_files(root);
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files_with_fri_quotient_and_unit_value(unit);
    }
}

fn write_setup_directory_with_proof_value(root: &Path) {
    write_global_files_with_info(root, &fixtures::sample_global_info_with_proof_value());
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files_with_verifier_info(
            unit,
            &fixtures::sample_verifier_info_with_proof_value(),
        );
    }
}

fn write_setup_directory_with_proof_value_constraint(root: &Path) {
    write_global_files_with_info(root, &fixtures::sample_global_info_with_proof_value());
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files_with_verifier_info_and_regular_constraints(
            unit,
            &fixtures::sample_verifier_info_with_proof_value(),
            sample_proof_value_regular_constraint_program(),
        );
    }
}

fn write_setup_directory_with_unit_value(root: &Path) {
    write_global_files(root);
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files_with_setup_info_verifier_and_regular_constraints(
            unit,
            &fixtures::sample_setup_info_with_unit_value(),
            &fixtures::sample_verifier_info(),
            sample_regular_constraint_program(),
        );
    }
}

fn write_setup_directory_with_unit_value_constraint(root: &Path) {
    write_global_files(root);
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files_with_setup_info_verifier_and_regular_constraints(
            unit,
            &fixtures::sample_setup_info_with_unit_value(),
            &fixtures::sample_verifier_info(),
            sample_unit_value_regular_constraint_program(),
        );
    }
}

fn write_setup_directory_with_challenge_constraint(root: &Path) {
    write_global_files(root);
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files_with_verifier_info_and_regular_constraints(
            unit,
            &fixtures::sample_verifier_info(),
            sample_challenge_regular_constraint_program(),
        );
    }
}

fn write_setup_directory_with_group_value(root: &Path) {
    write_global_files_with_info(root, &fixtures::sample_global_info_with_group_value());
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files(unit);
    }
}

fn run_setup_command(args: &[&str]) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(args, &mut stdout, &mut stderr);
    assert_eq!(
        code,
        0,
        "setup command failed: {}\n{}",
        args.join(" "),
        String::from_utf8_lossy(&stderr)
    );
}

fn run_generate_key_command(root: &Path) {
    let root = root.to_str().expect("path should be utf-8");
    run_setup_command(&["setup", "generate-key", root]);
}

fn write_execution_ready_setup_directory(root: &Path) {
    write_setup_directory(root);
    run_generate_key_command(root);
}

fn write_execution_ready_setup_directory_with_public_values(
    root: &Path,
    public_values: &PublicValues,
) {
    write_setup_directory_with_public_values(root, public_values);
    run_generate_key_command(root);
}

fn write_execution_ready_setup_directory_with_eth_block_public_values(
    root: &Path,
    input: &EthBlockInput,
) {
    write_execution_ready_setup_directory_with_public_values(
        root,
        &eth_block_public_values_metadata(input),
    );
}

fn write_execution_ready_setup_directory_with_fri_quotient(root: &Path) {
    write_setup_directory_with_fri_quotient(root);
    run_generate_key_command(root);
}

fn write_execution_ready_setup_directory_with_fri_quotient_and_unit_value(root: &Path) {
    write_setup_directory_with_fri_quotient_and_unit_value(root);
    run_generate_key_command(root);
}

fn write_execution_ready_setup_directory_with_proof_value(root: &Path) {
    write_setup_directory_with_proof_value(root);
    run_generate_key_command(root);
}

fn write_execution_ready_setup_directory_with_proof_value_constraint(root: &Path) {
    write_setup_directory_with_proof_value_constraint(root);
    run_generate_key_command(root);
}

fn write_execution_ready_setup_directory_with_unit_value(root: &Path) {
    write_setup_directory_with_unit_value(root);
    run_generate_key_command(root);
}

fn write_execution_ready_setup_directory_with_unit_value_constraint(root: &Path) {
    write_setup_directory_with_unit_value_constraint(root);
    run_generate_key_command(root);
}

fn write_execution_ready_setup_directory_with_challenge_constraint(root: &Path) {
    write_setup_directory_with_challenge_constraint(root);
    run_generate_key_command(root);
}

fn write_setup_directory_with_proof_group_and_unit_value(root: &Path) {
    write_global_files_with_info(root, &fixtures::sample_global_info_with_proof_group_value());
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files_with_setup_info_verifier_and_regular_constraints(
            unit,
            &fixtures::sample_setup_info_with_unit_value(),
            &fixtures::sample_verifier_info(),
            sample_regular_constraint_program(),
        );
    }
}

fn sample_contribution_entries(lattice_size: usize) -> Vec<ProveContributionEntry> {
    vec![
        ProveContributionEntry {
            worker_index: 0,
            group_id: 0,
            aggregated: false,
            values: (0..lattice_size)
                .map(|index| Felt::from_u64(index as u64 + 1))
                .collect(),
        },
        ProveContributionEntry {
            worker_index: 1,
            group_id: 0,
            aggregated: true,
            values: (0..lattice_size)
                .map(|index| Felt::from_u64(index as u64 + 11))
                .collect(),
        },
    ]
}

fn write_execution_ready_setup_directory_with_proof_group_and_unit_value(root: &Path) {
    write_setup_directory_with_proof_group_and_unit_value(root);
    run_generate_key_command(root);
}

fn write_proof_value_query_preflight_fixture(
    root: &Path,
    proof_values: Option<Vec<[u64; 3]>>,
) -> (PathBuf, PathBuf, usize) {
    write_execution_ready_setup_directory_with_proof_value(root);
    let catalog = read_key_directory_catalog(root).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_value_fields = public_values
        .values
        .iter()
        .flat_map(|entry| entry.elements.iter().copied().map(Felt::from_u64))
        .collect::<Vec<_>>();
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let material_segment =
        build_pcs_material_manifest_segment(&schedule).expect("material segment should build");
    let material = parse_pcs_material_manifest_segment(&material_segment.data)
        .expect("material segment should parse")
        .units[0]
        .clone();
    let witness_segment = sample_witness_proof_segment(&schedule, 0);
    let witness = parse_witness_commitment_segment(&witness_segment.data)
        .expect("witness segment should parse");
    let evaluation_segment = sample_pcs_evaluation_segment(0);
    let evaluations = parse_pcs_evaluation_segment(&evaluation_segment.data)
        .expect("evaluation segment should parse")
        .units[0]
        .clone();
    let query_value = [51, 52, 53];
    let fri_unit = sample_folded_pcs_fri_opening_template_with_values(
        &schedule,
        &material,
        &public_value_fields,
        &witness,
        &evaluations,
        0,
        query_value,
    );
    let transcript_inputs = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &public_value_fields,
        unit_values: &[],
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri_unit,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: &[],
    };
    let nonce_segment =
        build_pcs_query_nonce_segment_from_transcript_segments(&schedule, transcript_inputs)
            .expect("nonce segment should build");
    let query_segment = build_pcs_query_plan_segment_from_transcript_segments(
        &schedule,
        std::slice::from_ref(&witness_segment),
        transcript_inputs,
        &nonce_segment,
    )
    .expect("query segment should build");
    let constant_opening_segment =
        build_constant_opening_segment(&catalog, &schedule, &query_segment)
            .expect("constant opening segment should build");
    let opening_segment = sample_witness_opening_segment(&schedule, &query_segment, 0);
    let fri_segment = sample_folded_pcs_fri_opening_segment_with_values(
        &schedule,
        &query_segment,
        0,
        fri_unit,
        query_value,
    );
    let mut segments = vec![
        material_segment,
        query_segment,
        constant_opening_segment,
        opening_segment,
        witness_segment,
        evaluation_segment,
        fri_segment,
        nonce_segment,
    ];
    if let Some(values) = proof_values {
        segments.push(sample_pcs_proof_values_segment(values));
    }
    let segment_count = segments.len();
    let proof = ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments,
    };
    let proof_path = root.join("proof.bin");
    let public_values_path = root.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );
    (proof_path, public_values_path, segment_count)
}

fn write_global_constraint_preflight_fixture(
    root: &Path,
    proof_value: [u64; 3],
) -> (PathBuf, PathBuf, usize) {
    write_setup_directory_with_proof_value(root);
    write_global_constraint_program(
        root,
        GlobalConstraintProgram {
            entries: vec![GlobalConstraintEntry {
                destination_dimension: 1,
                destination_id: 0,
                temp1_count: 1,
                temp3_count: 0,
                ops_count: 1,
                ops_offset: 0,
                args_count: 6,
                args_offset: 0,
                source_line: "proof residual".to_owned(),
            }],
            ops: vec![0],
            args: vec![1, 0, 3, 0, 2, 0],
            numbers: vec![51],
        },
    );
    run_generate_key_command(root);
    let catalog = read_key_directory_catalog(root).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    proof
        .segments
        .push(sample_pcs_proof_values_segment(vec![proof_value]));
    let segment_count = proof.segments.len();
    let proof_path = root.join("proof.bin");
    let public_values_path = root.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );
    (proof_path, public_values_path, segment_count)
}

fn write_global_hint_preflight_fixture(
    root: &Path,
    proof_values: Option<Vec<[u64; 3]>>,
) -> (PathBuf, PathBuf, usize) {
    write_setup_directory_with_proof_value(root);
    write_global_program(
        root,
        GlobalConstraintProgram {
            entries: Vec::new(),
            ops: Vec::new(),
            args: Vec::new(),
            numbers: Vec::new(),
        },
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
        },
    );
    run_generate_key_command(root);
    let catalog = read_key_directory_catalog(root).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    if let Some(proof_values) = proof_values {
        proof
            .segments
            .push(sample_pcs_proof_values_segment(proof_values));
    }
    let segment_count = proof.segments.len();
    let proof_path = root.join("proof.bin");
    let public_values_path = root.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );
    (proof_path, public_values_path, segment_count)
}

fn write_global_lookup_hint_preflight_fixture(root: &Path) -> (PathBuf, PathBuf, usize) {
    write_setup_directory(root);
    write_global_program(
        root,
        GlobalConstraintProgram {
            entries: Vec::new(),
            ops: Vec::new(),
            args: Vec::new(),
            numbers: Vec::new(),
        },
        HintProgram {
            hints: vec![Hint {
                name: SOURCE_LOOKUP_PROVES_HINT.to_owned(),
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
                            operand: HintOperand::Number(11),
                            positions: Vec::new(),
                        }],
                    },
                ],
            }],
        },
    );
    run_generate_key_command(root);
    let catalog = read_key_directory_catalog(root).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let proof = sample_proof_with_material(&public_values, &catalog);
    let segment_count = proof.segments.len();
    let proof_path = root.join("proof.bin");
    let public_values_path = root.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );
    (proof_path, public_values_path, segment_count)
}

fn write_unit_value_query_preflight_fixture(
    root: &Path,
    unit_values_segment: Option<Vec<u64>>,
) -> (PathBuf, PathBuf, usize) {
    write_execution_ready_setup_directory_with_unit_value(root);
    let catalog = read_key_directory_catalog(root).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_value_fields = public_values
        .values
        .iter()
        .flat_map(|entry| entry.elements.iter().copied().map(Felt::from_u64))
        .collect::<Vec<_>>();
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let material_segment =
        build_pcs_material_manifest_segment(&schedule).expect("material segment should build");
    let material = parse_pcs_material_manifest_segment(&material_segment.data)
        .expect("material segment should parse")
        .units[0]
        .clone();
    let witness_segment = sample_witness_proof_segment(&schedule, 0);
    let witness = parse_witness_commitment_segment(&witness_segment.data)
        .expect("witness segment should parse");
    let evaluation_segment = sample_pcs_evaluation_segment(0);
    let evaluations = parse_pcs_evaluation_segment(&evaluation_segment.data)
        .expect("evaluation segment should parse")
        .units[0]
        .clone();
    let unit_values = [101, 201, 202, 203]
        .iter()
        .copied()
        .map(Felt::from_u64)
        .collect::<Vec<_>>();
    let fri_unit = sample_folded_pcs_fri_opening_template_with_values_and_unit_values(
        FoldedPcsFriTemplateInputs {
            schedule: &schedule,
            material: &material,
            public_values: &public_value_fields,
            witness: &witness,
            evaluations: &evaluations,
            unit_index: 0,
            query_value: [31, 32, 33],
            unit_values: &unit_values,
        },
    );
    let transcript_inputs = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &public_value_fields,
        unit_values: &unit_values,
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri_unit,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: &[],
    };
    let nonce_segment =
        build_pcs_query_nonce_segment_from_transcript_segments(&schedule, transcript_inputs)
            .expect("nonce segment should build");
    let query_segment = build_pcs_query_plan_segment_from_transcript_segments(
        &schedule,
        std::slice::from_ref(&witness_segment),
        transcript_inputs,
        &nonce_segment,
    )
    .expect("query segment should build");
    let constant_opening_segment =
        build_constant_opening_segment(&catalog, &schedule, &query_segment)
            .expect("constant opening segment should build");
    let opening_segment = sample_witness_opening_segment(&schedule, &query_segment, 0);
    let fri_segment = sample_folded_pcs_fri_opening_segment(&schedule, &query_segment, 0, fri_unit);
    let mut segments = vec![
        material_segment,
        query_segment,
        constant_opening_segment,
        opening_segment,
        witness_segment,
        evaluation_segment,
        fri_segment,
        nonce_segment,
    ];
    if let Some(values) = unit_values_segment {
        segments.push(sample_unit_values_segment(0, values));
    }
    let segment_count = segments.len();
    let proof = ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments,
    };
    let proof_path = root.join("proof.bin");
    let public_values_path = root.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );
    (proof_path, public_values_path, segment_count)
}

fn write_challenge_global_constraint_preflight_fixture(root: &Path) -> (PathBuf, PathBuf, usize) {
    write_execution_ready_setup_directory(root);
    let catalog = read_key_directory_catalog(root).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_value_fields = public_values
        .values
        .iter()
        .flat_map(|entry| entry.elements.iter().copied().map(Felt::from_u64))
        .collect::<Vec<_>>();
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let material_segment =
        build_pcs_material_manifest_segment(&schedule).expect("material segment should build");
    let material = parse_pcs_material_manifest_segment(&material_segment.data)
        .expect("material segment should parse")
        .units[0]
        .clone();
    let witness_segment = sample_witness_proof_segment(&schedule, 0);
    let witness = parse_witness_commitment_segment(&witness_segment.data)
        .expect("witness segment should parse");
    let evaluation_segment = sample_pcs_evaluation_segment(0);
    let evaluations = parse_pcs_evaluation_segment(&evaluation_segment.data)
        .expect("evaluation segment should parse")
        .units[0]
        .clone();
    let fri_unit = sample_folded_pcs_fri_opening_template(
        &schedule,
        &material,
        &public_value_fields,
        &witness,
        &evaluations,
        0,
    );
    let transcript_inputs = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &public_value_fields,
        unit_values: &[],
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri_unit,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: &[],
    };
    let challenges = derive_pcs_transcript_challenges_from_segments(transcript_inputs)
        .expect("transcript challenges should derive");
    let nonce_segment =
        build_pcs_query_nonce_segment_from_transcript_segments(&schedule, transcript_inputs)
            .expect("nonce segment should build");
    let query_segment = build_pcs_query_plan_segment_from_transcript_segments(
        &schedule,
        std::slice::from_ref(&witness_segment),
        transcript_inputs,
        &nonce_segment,
    )
    .expect("query segment should build");
    let constant_opening_segment =
        build_constant_opening_segment(&catalog, &schedule, &query_segment)
            .expect("constant opening segment should build");
    let opening_segment = sample_witness_opening_segment(&schedule, &query_segment, 0);
    let fri_segment = sample_folded_pcs_fri_opening_segment(&schedule, &query_segment, 0, fri_unit);
    write_global_constraint_program(
        root,
        GlobalConstraintProgram {
            entries: vec![GlobalConstraintEntry {
                destination_dimension: 3,
                destination_id: 0,
                temp1_count: 0,
                temp3_count: 1,
                ops_count: 1,
                ops_offset: 0,
                args_count: 6,
                args_offset: 0,
                source_line: "challenge residual".to_owned(),
            }],
            ops: vec![2],
            args: vec![1, 0, 6, 0, 2, 0],
            numbers: challenges[0].to_u64s().to_vec(),
        },
    );
    run_generate_key_command(root);
    let catalog = read_key_directory_catalog(root).expect("catalog should load after rewrite");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let segments = vec![
        material_segment,
        query_segment,
        constant_opening_segment,
        opening_segment,
        witness_segment,
        evaluation_segment,
        fri_segment,
        nonce_segment,
    ];
    let segment_count = segments.len();
    let proof = ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments,
    };
    let proof_path = root.join("proof.bin");
    let public_values_path = root.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );
    (proof_path, public_values_path, segment_count)
}

fn write_group_value_global_constraint_preflight_fixture(
    root: &Path,
    group_value: [u64; 3],
) -> (PathBuf, PathBuf, usize) {
    write_setup_directory_with_group_value(root);
    write_global_constraint_program(
        root,
        GlobalConstraintProgram {
            entries: vec![GlobalConstraintEntry {
                destination_dimension: 3,
                destination_id: 0,
                temp1_count: 0,
                temp3_count: 1,
                ops_count: 1,
                ops_offset: 0,
                args_count: 6,
                args_offset: 0,
                source_line: "group residual".to_owned(),
            }],
            ops: vec![2],
            args: vec![1, 0, 5, 0, 2, 0],
            numbers: group_value.to_vec(),
        },
    );
    run_generate_key_command(root);
    let catalog = read_key_directory_catalog(root).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    proof
        .segments
        .push(sample_group_values_segment(vec![group_value]));
    let segment_count = proof.segments.len();
    let proof_path = root.join("proof.bin");
    let public_values_path = root.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );
    (proof_path, public_values_path, segment_count)
}

fn pcs_material_byte_count(catalog: &lzvm_artifacts::key_directory::KeyDirectoryCatalog) -> u64 {
    catalog
        .units
        .iter()
        .map(|unit| {
            unit.pcs_material_bytes
                .expect("execution-ready setup should include material")
        })
        .sum()
}

fn write_proof_pair(root: &Path, setup_hash: [u8; 32]) -> (PathBuf, PathBuf) {
    let public_values = sample_public_values(setup_hash);
    let proof = sample_proof(&public_values);
    let proof_path = root.join("proof.bin");
    let public_values_path = root.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );
    (proof_path, public_values_path)
}

fn write_proof_pair_with_material(
    root: &Path,
    setup_hash: [u8; 32],
    catalog: &lzvm_artifacts::key_directory::KeyDirectoryCatalog,
) -> (PathBuf, PathBuf) {
    let public_values = sample_public_values(setup_hash);
    let proof = sample_proof_with_material(&public_values, catalog);
    let proof_path = root.join("proof.bin");
    let public_values_path = root.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );
    (proof_path, public_values_path)
}

fn write_stale_setup_manifest(root: &Path, value: u8) -> PathBuf {
    let manifest_path = root.join(SETUP_DIRECTORY_MANIFEST_FILE);
    let mut manifest =
        read_setup_directory_manifest_file(&manifest_path).expect("manifest should parse");
    manifest.catalog_digest = [value; 32];
    fs::write(
        &manifest_path,
        encode_setup_directory_manifest(&manifest).expect("manifest should encode"),
    )
    .expect("stale manifest should be written");
    manifest_path
}

#[test]
fn validates_a_complete_setup_directory() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "validate",
            dir.to_str().expect("path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    let setup_hash = key_directory_catalog_digest_hex(
        &read_key_directory_catalog(&dir).expect("catalog should load"),
    )
    .expect("digest should encode");
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits=4\nglobal_constraints=0\nfixed_bytes=128\npcs_material_units=0\npcs_material_bytes=0\nsource_fixed_file_manifest=absent\nsource_fixed_file_manifest_entries=0\nsource_fixed_file_manifest_bytes=0\nsource_program_archive=absent\nsource_program_archive_sources=0\nsource_program_archive_edges=0\nsource_program_archive_bytes=0\nsetup_hash={setup_hash}\n"
        )
    );
    assert!(stderr.is_empty());

    let report = summarize_setup_directory(&dir).expect("directory summary should load");
    assert_eq!(report.unit_count, 4);
    assert_eq!(report.global_constraint_count, 0);
    assert_eq!(report.fixed_bytes, 128);
    assert_eq!(report.pcs_material_unit_count, 0);
    assert_eq!(report.pcs_material_bytes, 0);
    assert!(!report.source_fixed_file_manifest_present);
    assert_eq!(report.source_fixed_file_manifest_entry_count, 0);
    assert_eq!(report.source_fixed_file_manifest_bytes, 0);
    assert!(!report.source_program_archive_present);
    assert_eq!(report.source_program_archive_source_count, 0);
    assert_eq!(report.source_program_archive_edge_count, 0);
    assert_eq!(report.source_program_archive_bytes, 0);
    assert_eq!(
        report.fingerprint,
        key_directory_catalog_digest_hex(
            &read_key_directory_catalog(&dir).expect("catalog should load")
        )
        .expect("digest should encode")
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn reports_source_fixed_file_manifest_status_for_setup_directories() {
    let dir = temp_dir("source-fixed-file-manifest-summary");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    write_source_fixed_file_manifest(&dir);
    let manifest_bytes = source_fixed_file_manifest_bytes(&dir);
    let setup_hash = key_directory_catalog_digest_hex(
        &read_key_directory_catalog(&dir).expect("catalog should load"),
    )
    .expect("digest should encode");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "validate",
            dir.to_str().expect("path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits=4\nglobal_constraints=0\nfixed_bytes=128\npcs_material_units=0\npcs_material_bytes=0\nsource_fixed_file_manifest=present\nsource_fixed_file_manifest_entries=1\nsource_fixed_file_manifest_bytes={manifest_bytes}\nsource_program_archive=absent\nsource_program_archive_sources=0\nsource_program_archive_edges=0\nsource_program_archive_bytes=0\nsetup_hash={setup_hash}\n"
        )
    );
    assert!(stderr.is_empty());

    let report = summarize_setup_directory(&dir).expect("directory summary should load");
    assert!(report.source_fixed_file_manifest_present);
    assert_eq!(report.source_fixed_file_manifest_entry_count, 1);
    assert_eq!(report.source_fixed_file_manifest_bytes, manifest_bytes);
    assert!(!report.source_program_archive_present);
    assert_eq!(report.source_program_archive_source_count, 0);
    assert_eq!(report.source_program_archive_edge_count, 0);
    assert_eq!(report.source_program_archive_bytes, 0);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn reports_source_program_archive_status_for_setup_directories() {
    let dir = temp_dir("source-program-archive-summary");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    write_source_program_archive(&dir);
    let archive_bytes = source_program_archive_bytes(&dir);
    let setup_hash = key_directory_catalog_digest_hex(
        &read_key_directory_catalog(&dir).expect("catalog should load"),
    )
    .expect("digest should encode");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "validate",
            dir.to_str().expect("path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits=4\nglobal_constraints=0\nfixed_bytes=128\npcs_material_units=0\npcs_material_bytes=0\nsource_fixed_file_manifest=absent\nsource_fixed_file_manifest_entries=0\nsource_fixed_file_manifest_bytes=0\nsource_program_archive=present\nsource_program_archive_sources=2\nsource_program_archive_edges=1\nsource_program_archive_bytes={archive_bytes}\nsetup_hash={setup_hash}\n"
        )
    );
    assert!(stderr.is_empty());

    let report = summarize_setup_directory(&dir).expect("directory summary should load");
    assert!(!report.source_fixed_file_manifest_present);
    assert_eq!(report.source_fixed_file_manifest_entry_count, 0);
    assert_eq!(report.source_fixed_file_manifest_bytes, 0);
    assert!(report.source_program_archive_present);
    assert_eq!(report.source_program_archive_source_count, 2);
    assert_eq!(report.source_program_archive_edge_count, 1);
    assert_eq!(report.source_program_archive_bytes, archive_bytes);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_source_fixed_file_manifest_entries_missing_from_source_program_archive() {
    let dir = temp_dir("source-companion-source-mismatch");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    write_source_fixed_file_manifest(&dir);
    write_source_program_archive_without_manifest_source(&dir);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "validate",
            dir.to_str().expect("path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "setup validation failed: key-directory source fixed-file manifest entry 0 references source main.pil outside source program archive\n"
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_source_fixed_file_manifest_spans_outside_source_program_archive() {
    let dir = temp_dir("source-companion-span-mismatch");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    write_source_fixed_file_manifest(&dir);
    write_source_program_archive_with_short_manifest_source(&dir);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "validate",
            dir.to_str().expect("path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "setup validation failed: key-directory source fixed-file manifest entry 0 span 10..40 exceeds source main.pil length 14\n"
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_source_fixed_file_manifest_spans_inside_source_program_archive_utf8_codepoints() {
    let dir = temp_dir("source-companion-utf8-boundary-mismatch");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    write_source_fixed_file_manifest_with_span(&dir, 2, 3);
    write_source_program_archive_with_multibyte_manifest_source(&dir);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "validate",
            dir.to_str().expect("path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "setup validation failed: key-directory source fixed-file manifest entry 0 span 2..3 is not on UTF-8 boundary for source main.pil\n"
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn write_source_companions_refreshes_existing_setup_directory_manifest() {
    let dir = temp_dir("source-companions-refresh-manifest");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let root = dir.to_str().expect("path should be utf-8");
    run_setup_command(&["setup", "generate-key", root]);
    let manifest_path = dir.join(SETUP_DIRECTORY_MANIFEST_FILE);
    let original_manifest =
        read_setup_directory_manifest_file(&manifest_path).expect("manifest should parse");
    assert!(!original_manifest.source_program_archive_present);
    assert!(!original_manifest.source_fixed_file_manifest_present);

    let source_dir = dir.join("source");
    let main_path = source_dir.join("main.pil");
    let child_path = source_dir.join("shared.pil");
    write_bytes(
        &main_path,
        "include \"shared.pil\";\n\
         col witness main.trace;",
    );
    write_bytes(&child_path, "col fixed shared = [1, 2];");

    let mut companion_stdout = Vec::new();
    let mut companion_stderr = Vec::new();
    let companion_code = run_cli(
        &[
            "setup",
            "write-source-companions",
            main_path.to_str().expect("main path should be utf-8"),
            root,
        ],
        &mut companion_stdout,
        &mut companion_stderr,
    );

    assert_eq!(
        companion_code,
        0,
        "source companion command failed: {}",
        String::from_utf8_lossy(&companion_stderr)
    );
    assert!(companion_stderr.is_empty());
    let companion_output = String::from_utf8(companion_stdout).expect("stdout should be utf-8");
    assert!(companion_output.contains("setup_directory_manifest_refreshed=true\n"));
    assert!(companion_output.contains(&format!(
        "setup_directory_manifest={}\n",
        manifest_path.display()
    )));

    let mut validate_stdout = Vec::new();
    let mut validate_stderr = Vec::new();
    let validate_code = run_cli(
        &["setup", "validate", root],
        &mut validate_stdout,
        &mut validate_stderr,
    );

    assert_eq!(validate_code, 0);
    assert!(validate_stderr.is_empty());
    let refreshed_manifest =
        read_setup_directory_manifest_file(&manifest_path).expect("manifest should parse");
    let archive_bytes = source_program_archive_bytes(&dir);
    let fixed_manifest_bytes = source_fixed_file_manifest_bytes(&dir);
    assert!(refreshed_manifest.source_program_archive_present);
    assert_eq!(refreshed_manifest.source_program_archive_source_count, 2);
    assert_eq!(refreshed_manifest.source_program_archive_edge_count, 1);
    assert_eq!(
        refreshed_manifest.source_program_archive_byte_count,
        archive_bytes
    );
    assert!(refreshed_manifest.source_fixed_file_manifest_present);
    assert_eq!(refreshed_manifest.source_fixed_file_manifest_entry_count, 0);
    assert_eq!(
        refreshed_manifest.source_fixed_file_manifest_byte_count,
        fixed_manifest_bytes
    );
    let report = summarize_setup_directory(&dir).expect("directory summary should load");
    assert!(report.source_program_archive_present);
    assert_eq!(report.source_program_archive_source_count, 2);
    assert_eq!(report.source_program_archive_edge_count, 1);
    assert!(report.source_fixed_file_manifest_present);
    assert_eq!(report.source_fixed_file_manifest_entry_count, 0);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn write_source_companions_refreshes_setup_directory_manifest_on_request() {
    let dir = temp_dir("source-companions-request-refresh-manifest");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let root = dir.to_str().expect("path should be utf-8");
    let manifest_path = dir.join(SETUP_DIRECTORY_MANIFEST_FILE);
    assert!(!manifest_path.exists());

    let source_dir = dir.join("source");
    let main_path = source_dir.join("main.pil");
    let child_path = source_dir.join("shared.pil");
    write_bytes(
        &main_path,
        "include \"shared.pil\";\n\
         col witness main.trace;",
    );
    write_bytes(&child_path, "col fixed shared = [1, 2];");

    let mut companion_stdout = Vec::new();
    let mut companion_stderr = Vec::new();
    let companion_code = run_cli(
        &[
            "setup",
            "write-source-companions",
            "--refresh-manifest",
            main_path.to_str().expect("main path should be utf-8"),
            root,
        ],
        &mut companion_stdout,
        &mut companion_stderr,
    );

    assert_eq!(
        companion_code,
        0,
        "source companion command failed: {}",
        String::from_utf8_lossy(&companion_stderr)
    );
    assert!(companion_stderr.is_empty());
    let companion_output = String::from_utf8(companion_stdout).expect("stdout should be utf-8");
    assert!(companion_output.contains("setup_directory_manifest_refreshed=true\n"));
    assert!(companion_output.contains(&format!(
        "setup_directory_manifest={}\n",
        manifest_path.display()
    )));

    let manifest =
        read_setup_directory_manifest_file(&manifest_path).expect("manifest should parse");
    let archive_bytes = source_program_archive_bytes(&dir);
    let fixed_manifest_bytes = source_fixed_file_manifest_bytes(&dir);
    assert!(manifest.source_program_archive_present);
    assert_eq!(manifest.source_program_archive_source_count, 2);
    assert_eq!(manifest.source_program_archive_edge_count, 1);
    assert_eq!(manifest.source_program_archive_byte_count, archive_bytes);
    assert!(manifest.source_fixed_file_manifest_present);
    assert_eq!(manifest.source_fixed_file_manifest_entry_count, 0);
    assert_eq!(
        manifest.source_fixed_file_manifest_byte_count,
        fixed_manifest_bytes
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn write_source_companions_preserves_manifested_setup_when_refresh_fails() {
    let dir = temp_dir("source-companions-refresh-rollback");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let root = dir.to_str().expect("path should be utf-8");
    run_setup_command(&["setup", "generate-key", root]);

    let source_dir = dir.join("source");
    let main_path = source_dir.join("main.pil");
    write_bytes(
        &main_path,
        "airtemplate Main() {\n\
             #pragma output_fixed_file `${AIR_NAME}.fixed`\n\
         }\n\
         airgroup Main { Main(); }\n\
         col witness main.trace;",
    );

    let mut companion_stdout = Vec::new();
    let mut companion_stderr = Vec::new();
    let companion_code = run_cli(
        &[
            "setup",
            "write-source-companions",
            main_path.to_str().expect("main path should be utf-8"),
            root,
        ],
        &mut companion_stdout,
        &mut companion_stderr,
    );

    assert_eq!(companion_code, 1);
    assert!(String::from_utf8_lossy(&companion_stderr)
        .contains("source fixed-file manifest entry 0 references group 0:Main"));

    let mut validate_stdout = Vec::new();
    let mut validate_stderr = Vec::new();
    let validate_code = run_cli(
        &["setup", "validate", root],
        &mut validate_stdout,
        &mut validate_stderr,
    );

    assert_eq!(
        validate_code,
        0,
        "setup validate failed after rejected source companions: {}",
        String::from_utf8_lossy(&validate_stderr)
    );
    assert!(validate_stderr.is_empty());
    let report = summarize_setup_directory(&dir).expect("directory summary should load");
    assert!(!report.source_program_archive_present);
    assert!(!report.source_fixed_file_manifest_present);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn validates_generated_key_directory_materials() {
    let dir = temp_dir("generated-key-validate");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let root = dir.to_str().expect("path should be utf-8");
    run_setup_command(&["setup", "generate-key", root]);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let material_bytes = pcs_material_byte_count(&catalog);
    let setup_hash = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["setup", "validate", root], &mut stdout, &mut stderr);

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits=4\nglobal_constraints=0\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nsource_fixed_file_manifest=absent\nsource_fixed_file_manifest_entries=0\nsource_fixed_file_manifest_bytes=0\nsource_program_archive=absent\nsource_program_archive_sources=0\nsource_program_archive_edges=0\nsource_program_archive_bytes=0\nsetup_hash={setup_hash}\n"
        )
    );
    assert!(stderr.is_empty());

    let report = summarize_setup_directory(&dir).expect("directory summary should load");
    assert_eq!(report.unit_count, 4);
    assert_eq!(report.global_constraint_count, 0);
    assert_eq!(report.fixed_bytes, 128);
    assert_eq!(report.pcs_material_unit_count, 4);
    assert_eq!(report.pcs_material_bytes, material_bytes);
    assert!(!report.source_fixed_file_manifest_present);
    assert_eq!(report.source_fixed_file_manifest_entry_count, 0);
    assert_eq!(report.source_fixed_file_manifest_bytes, 0);
    assert!(!report.source_program_archive_present);
    assert_eq!(report.source_program_archive_source_count, 0);
    assert_eq!(report.source_program_archive_edge_count, 0);
    assert_eq!(report.source_program_archive_bytes, 0);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_stale_setup_directory_manifest() {
    let dir = temp_dir("stale-manifest");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let root = dir.to_str().expect("path should be utf-8");
    run_setup_command(&["setup", "generate-key", root]);
    let manifest_path = write_stale_setup_manifest(&dir, 0xaa);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["setup", "validate", root], &mut stdout, &mut stderr);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "setup validation failed: setup directory manifest mismatch at {}\n",
            manifest_path.display()
        )
    );
}

#[test]
fn rejects_prove_schedule_with_stale_setup_directory_manifest() {
    let dir = temp_dir("stale-manifest-schedule");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let root = dir.to_str().expect("path should be utf-8");
    run_setup_command(&["setup", "generate-key", root]);
    let manifest_path = write_stale_setup_manifest(&dir, 0xbb);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["prove", "schedule", root], &mut stdout, &mut stderr);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "prove schedule failed: setup directory manifest mismatch at {}\n",
            manifest_path.display()
        )
    );
}

#[test]
fn rejects_prove_plan_with_stale_setup_directory_manifest() {
    let dir = temp_dir("stale-manifest-plan");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let root = dir.to_str().expect("path should be utf-8");
    let output_dir = dir.join("proof-out");
    run_setup_command(&["setup", "generate-key", root]);
    let manifest_path = write_stale_setup_manifest(&dir, 0xbd);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "plan",
            root,
            output_dir.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "prove plan failed: setup directory manifest mismatch at {}\n",
            manifest_path.display()
        )
    );
}

#[test]
fn rejects_prove_inputs_with_stale_setup_directory_manifest() {
    let dir = temp_dir("stale-manifest-inputs");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let root = dir.to_str().expect("path should be utf-8");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    write_bytes(&guest_image, sample_guest_image());
    run_setup_command(&["setup", "generate-key", root]);
    let manifest_path = write_stale_setup_manifest(&dir, 0xbe);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            root,
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "prove inputs failed: setup directory manifest mismatch at {}\n",
            manifest_path.display()
        )
    );
}

#[test]
fn rejects_prove_witness_with_stale_setup_directory_manifest() {
    let dir = temp_dir("stale-manifest-witness");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(
        &public_values_path,
        encode_public_values(&sample_public_values(setup_hash))
            .expect("public values should encode"),
    );
    let manifest_path = write_stale_setup_manifest(&dir, 0xbf);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "prove witness failed: setup directory manifest mismatch at {}\n",
            manifest_path.display()
        )
    );
}

#[test]
fn rejects_setup_preflight_with_stale_setup_directory_manifest() {
    let dir = temp_dir("stale-manifest-preflight");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let root = dir.to_str().expect("path should be utf-8");
    run_setup_command(&["setup", "generate-key", root]);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let (proof_path, public_values_path) = write_proof_pair(&dir, setup_hash);
    let manifest_path = write_stale_setup_manifest(&dir, 0xcc);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            root,
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "verify setup-preflight failed: setup directory manifest mismatch at {}\n",
            manifest_path.display()
        )
    );
}

#[test]
fn fingerprints_a_complete_setup_directory() {
    let dir = temp_dir("fingerprint");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "fingerprint",
            dir.to_str().expect("path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits=4\nsource_fixed_file_manifest=absent\nsource_fixed_file_manifest_entries=0\nsource_fixed_file_manifest_bytes=0\nsource_program_archive=absent\nsource_program_archive_sources=0\nsource_program_archive_edges=0\nsource_program_archive_bytes=0\nfingerprint={expected}\n"
        )
    );
    assert!(stderr.is_empty());

    let report = summarize_setup_directory(&dir).expect("directory summary should load");
    assert_eq!(report.unit_count, 4);
    assert_eq!(report.fingerprint, expected);
    assert_eq!(report.global_constraint_count, 0);
    assert_eq!(report.fixed_bytes, 128);
    assert_eq!(report.source_fixed_file_manifest_bytes, 0);
    assert_eq!(report.source_program_archive_bytes, 0);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn fingerprints_report_source_companion_status() {
    let dir = temp_dir("fingerprint-source-companions");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    write_source_fixed_file_manifest(&dir);
    write_source_program_archive(&dir);
    let manifest_bytes = source_fixed_file_manifest_bytes(&dir);
    let archive_bytes = source_program_archive_bytes(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "fingerprint",
            dir.to_str().expect("path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits=4\nsource_fixed_file_manifest=present\nsource_fixed_file_manifest_entries=1\nsource_fixed_file_manifest_bytes={manifest_bytes}\nsource_program_archive=present\nsource_program_archive_sources=2\nsource_program_archive_edges=1\nsource_program_archive_bytes={archive_bytes}\nfingerprint={expected}\n"
        )
    );
    assert!(stderr.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prints_prove_schedule_for_setup_directory() {
    let dir = temp_dir("prove-schedule");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "schedule",
            dir.to_str().expect("path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits=4\nfixed_bytes=128\npcs_material_units=0\npcs_material_bytes=0\nqueries=4\nmax_extended_domain_bits=2\nsetup_hash={expected}\n"
        )
    );
    assert!(stderr.is_empty());

    let schedule =
        derive_prove_schedule_from_directory(&dir).expect("schedule should derive from directory");
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    assert_eq!(
        schedule,
        derive_prove_schedule(&catalog).expect("schedule should derive")
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prints_prove_run_plan_for_setup_directory() {
    let dir = temp_dir("prove-plan");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let input_path = dir.join("input.bin");
    let output_dir = dir.join("proof-out");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "plan",
            "--aggregate",
            "--save-outputs",
            "--gpu-preallocate",
            "--gpu-streams",
            "8",
            "--witness-thread-pools",
            "2",
            "--stored-witnesses",
            "3",
            "--no-pack-trace",
            "--partitions",
            "4",
            "--partition-ids",
            "1,3",
            "--worker",
            "2",
            "--input-data",
            input_path.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=0\npcs_material_bytes=0\nqueries=4\nmax_extended_domain_bits=2\npartitions=4\npartition_ids=1,3\nworker=2\ninput_data={}\naggregate=true\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=true\nminimal_memory=false\noutput={}\ngpu_preallocate=true\ngpu_streams=8\nwitness_thread_pools=2\nstored_witnesses=3\npack_trace=false\nsetup_hash={expected}\n",
            input_path.display(),
            output_dir.display()
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn rejects_prove_run_plan_with_invalid_partition() {
    let dir = temp_dir("prove-plan-bad-partition");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let output_dir = dir.join("proof-out");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "plan",
            "--partitions",
            "2",
            "--partition-ids",
            "2",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove plan failed: prove run plan partition id 2 is outside partition count 2\n"
    );
}

#[test]
fn rejects_prove_run_plan_final_wrap_with_partitioned_pass() {
    let dir = temp_dir("prove-plan-final-wrap-partitioned");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let output_dir = dir.join("proof-out");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "plan",
            "--aggregate",
            "--final-wrap",
            "--partitions",
            "2",
            "--partition-ids",
            "1",
            "--worker",
            "1",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove plan failed: prove run plan final wrap requires a single complete partition\n"
    );
}

#[test]
fn prints_prove_inputs_for_setup_directory() {
    let dir = temp_dir("prove-inputs");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let public_inputs = dir.join("public-inputs.bin");
    let witness_library_bytes = sample_witness_library();
    let witness_library_info =
        parse_witness_library(&witness_library_bytes).expect("witness library should parse");
    write_bytes(&witness_library, &witness_library_bytes);
    let guest_image_bytes = sample_guest_image();
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    write_bytes(&guest_image, &guest_image_bytes);
    let public_values = sample_public_values(setup_hash);
    let public_inputs_hash = public_values_digest(&public_values).expect("digest should compute");
    write_bytes(
        &public_inputs,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_inputs
                .to_str()
                .expect("public inputs path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={expected}\nwitness_library={}\nwitness_library_bytes=64\nwitness_library_machine=62\nwitness_library_digest={}\nguest_image={}\nguest_image_bytes=64\nguest_image_machine=243\nguest_image_entry=2147483648\nguest_image_digest={}\npublic_inputs={}\npublic_inputs_hash={}\npublic_input_values=1\npublic_input_fields=1\n",
            output_dir.display(),
            witness_library.display(),
            format_hash(&witness_library_info.digest),
            guest_image.display(),
            format_hash(&guest_image_info.digest),
            public_inputs.display(),
            format_hash(&public_inputs_hash)
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn prove_inputs_rejects_public_values_with_wrong_setup_hash() {
    let dir = temp_dir("prove-inputs-public-values-setup-mismatch");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let public_inputs = dir.join("public-inputs.bin");
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(
        &public_inputs,
        encode_public_values(&sample_public_values([0x99; 32]))
            .expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_inputs
                .to_str()
                .expect("public inputs path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "prove inputs failed: prove execution plan public inputs setup hash mismatch: {}\n",
            public_inputs.display()
        )
    );
}

#[test]
fn prove_inputs_rejects_invalid_public_values() {
    let dir = temp_dir("prove-inputs-invalid-public-values");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let public_inputs = dir.join("public-inputs.bin");
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&public_inputs, [3_u8]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_inputs
                .to_str()
                .expect("public inputs path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "prove inputs failed: prove execution plan public inputs are invalid: {}: unexpected end of public-values file at 0, needed 4, available 1\n",
            public_inputs.display()
        )
    );
}

#[test]
fn prove_inputs_rejects_noncanonical_public_value_fields() {
    let dir = temp_dir("prove-inputs-noncanonical-public-values");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let public_inputs = dir.join("public-inputs.bin");
    let public_values = PublicValues {
        schema_version: 1,
        setup_hash,
        values: vec![PublicValueEntry {
            name: "bad_value".to_owned(),
            elements: vec![7],
        }],
    };
    let mut public_values_file =
        encode_public_values(&public_values).expect("public values should encode");
    let mut sectioned = parse_sectioned_file(&public_values_file, *b"pval", 1)
        .expect("public values should parse as sectioned file");
    let element_offset = 4 + 32 + 4 + 4 + "bad_value".len() + 4;
    sectioned.sections[0].data[element_offset..element_offset + 8]
        .copy_from_slice(&MODULUS.to_le_bytes());
    public_values_file =
        encode_sectioned_file(&sectioned).expect("mutated public values should encode");
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&public_inputs, public_values_file);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_inputs
                .to_str()
                .expect("public inputs path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "prove inputs failed: prove execution plan public inputs are invalid: {}: public-values entry bad_value element 0 is non-canonical: non-canonical field element: {MODULUS}\n",
            public_inputs.display()
        )
    );
}

#[test]
fn prove_inputs_generates_eth_block_public_values_when_missing() {
    let dir = temp_dir("prove-inputs-eth-public-values");
    let _ = fs::remove_dir_all(&dir);
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let block_input_path = dir.join("block.input");
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let witness_library_bytes = sample_witness_library();
    let witness_library_info =
        parse_witness_library(&witness_library_bytes).expect("witness library should parse");
    let guest_image_bytes = sample_guest_image();
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    let block_input_hash = eth_block_input_bytes_digest(&encoded_block_input);
    write_bytes(&witness_library, &witness_library_bytes);
    write_bytes(&guest_image, &guest_image_bytes);
    write_bytes(&block_input_path, &encoded_block_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let generated_bytes =
        fs::read(&generated_public_values_path).expect("generated public values should read");
    let generated_public_values =
        parse_public_values(&generated_bytes).expect("generated public values should parse");
    let generated_public_values_hash =
        public_values_digest(&generated_public_values).expect("digest should compute");
    assert_eq!(
        generated_public_values,
        public_values_from_eth_block_input(setup_hash, &block_input)
    );
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data={}\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={expected}\nwitness_library={}\nwitness_library_bytes=64\nwitness_library_machine=62\nwitness_library_digest={}\nguest_image={}\nguest_image_bytes=64\nguest_image_machine=243\nguest_image_entry=2147483648\nguest_image_digest={}\npublic_inputs={}\npublic_inputs_hash={}\npublic_input_values=21\npublic_input_fields=170\npublic_inputs_generated=eth_block_input\neth_block_input={}\neth_block_input_bytes={}\neth_block_input_hash={}\neth_block_rlp_bytes={}\neth_block_hash={}\neth_parent_hash={}\neth_ommers_hash={}\neth_beneficiary={}\neth_state_root={}\neth_receipts_root={}\neth_logs_bloom={}\neth_difficulty=01\neth_block_number=2\neth_block_timestamp=101\neth_extra_data=6c7a766d\neth_gas_limit=1000000\neth_gas_used=900000\neth_base_fee_per_gas=absent\neth_mix_hash={}\neth_nonce={}\neth_transactions_root={}\neth_transaction_trie_preimages=1\neth_transaction_count=1\neth_legacy_transactions=1\neth_typed_transactions=0\neth_receipts=absent\neth_withdrawals=absent\n",
            block_input_path.display(),
            output_dir.display(),
            witness_library.display(),
            format_hash(&witness_library_info.digest),
            guest_image.display(),
            format_hash(&guest_image_info.digest),
            generated_public_values_path.display(),
            format_hash(&generated_public_values_hash),
            block_input_path.display(),
            encoded_block_input.len(),
            format_hash(&block_input_hash),
            block_input.block_rlp.len(),
            format_hash(&block_input.block_hash),
            format_hash(&block_input.parent_hash),
            format_hash(&block_input.ommers_hash),
            format_hex(&block_input.beneficiary),
            format_hash(&block_input.state_root),
            format_hash(&block_input.receipts_root),
            format_hex(&block_input.logs_bloom),
            format_hash(&block_input.mix_hash),
            format_hex(&block_input.nonce),
            format_hash(&block_input.transactions_root)
        )
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_inputs_preserves_explicit_input_data_with_eth_block_input() {
    let dir = temp_dir("prove-inputs-eth-explicit-input");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let block_input_path = dir.join("block.input");
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(
        &block_input_path,
        encode_eth_block_input(&block_input).expect("block input should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .contains(&format!("input_data={}\n", input_data.display())));
}

#[test]
fn prove_inputs_reports_eth_block_receipts_when_present() {
    let dir = temp_dir("prove-inputs-eth-receipts");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let block_input_path = dir.join("block.input");
    let receipt_item = sample_receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let receipts_rlp = rlp_list(&[receipt_item]);
    let block_rlp = sample_block_rlp_with_receipts_root(receipt_build.root);
    let block_input = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(
        &block_input_path,
        encode_eth_block_input(&block_input).expect("block input should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains("public_inputs_generated=eth_block_input\n"));
    assert!(stdout_text.contains("eth_receipts=present\n"));
    assert!(stdout_text.contains(&format!("eth_receipts_rlp_bytes={}\n", receipts_rlp.len())));
    assert!(stdout_text.contains(&format!(
        "eth_receipt_trie_preimages={}\n",
        receipt_build.hash_preimages.len()
    )));
    assert!(
        stdout_text.contains("eth_receipt_count=1\neth_legacy_receipts=1\neth_typed_receipts=0\n")
    );
}

#[test]
fn prove_inputs_reports_eth_block_extra_field_counts() {
    let dir = temp_dir("prove-inputs-eth-extra-fields");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let block_input_path = dir.join("block.input");
    let block_rlp = sample_block_rlp_with_extra_fields();
    let block_input = build_eth_block_input(&block_rlp).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(
        &block_input_path,
        encode_eth_block_input(&block_input).expect("block input should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains("eth_extra_header_fields=1\neth_extra_body_fields=1\n"));
}

#[test]
fn prove_inputs_reports_eth_block_withdrawal_count_when_present() {
    let dir = temp_dir("prove-inputs-eth-withdrawals");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let block_input_path = dir.join("block.input");
    let withdrawal_item = sample_withdrawal_item();
    let withdrawals = vec![parse_rlp(&withdrawal_item).expect("withdrawal should parse")];
    let withdrawal_build = withdrawals_trie_build(&withdrawals);
    let block_rlp = sample_block_rlp_with_withdrawals(withdrawal_build.root, vec![withdrawal_item]);
    let block_input = build_eth_block_input(&block_rlp).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(
        &block_input_path,
        encode_eth_block_input(&block_input).expect("block input should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains("public_inputs_generated=eth_block_input\n"));
    assert!(stdout_text.contains(&format!(
        "eth_withdrawals=present\neth_withdrawals_root={}\neth_withdrawal_count=1\n",
        format_hash(&withdrawal_build.root)
    )));
    assert!(stdout_text.contains(&format!(
        "eth_withdrawal_trie_preimages={}\n",
        withdrawal_build.hash_preimages.len()
    )));
}

#[test]
fn prove_inputs_rejects_mismatched_eth_block_public_values() {
    let dir = temp_dir("prove-inputs-eth-public-values-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let block_input_path = dir.join("block.input");
    let public_values_path = dir.join("mismatched-public-values.bin");
    let other_block_input =
        build_eth_block_input(&sample_block_rlp_variant()).expect("block input should build");
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(
        &block_input_path,
        encode_eth_block_input(&block_input).expect("block input should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values_from_eth_block_input(
            setup_hash,
            &other_block_input,
        ))
        .expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove inputs failed: ETH block public value mismatch: eth_block_hash_u32_be\n"
    );
}

#[test]
fn prove_inputs_rejects_mismatched_program_image_cache_public_values() {
    let dir = temp_dir("prove-inputs-program-image-public-values-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_execution_ready_setup_directory_with_public_values(
        &dir,
        &eth_block_public_values_with_rom_root([0; 32], &block_input, [0, 0, 0, 0]),
    );
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let block_input_path = dir.join("block.input");
    let public_values_path = dir.join("mismatched-public-values.bin");
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(
        &block_input_path,
        encode_eth_block_input(&block_input).expect("block input should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&eth_block_public_values_with_rom_root(
            setup_hash,
            &block_input,
            [99, 98, 97, 96],
        ))
        .expect("public values should encode"),
    );
    let cache_path =
        write_sample_program_image_cache(&dir, &guest_image, setup_hash, [11, 12, 13, 14]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove inputs failed: program image cache tree root does not match public value: rom_root\n"
    );
}

#[test]
fn prove_inputs_rejects_eth_block_public_values_with_wrong_setup_hash() {
    let dir = temp_dir("prove-inputs-eth-public-values-setup-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let block_input_path = dir.join("block.input");
    let public_values_path = dir.join("wrong-setup-public-values.bin");
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(
        &block_input_path,
        encode_eth_block_input(&block_input).expect("block input should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values_from_eth_block_input(
            [0x99; 32],
            &block_input,
        ))
        .expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove inputs failed: public inputs setup hash mismatch\n"
    );
}

#[test]
fn prints_prove_inputs_for_internal_contribution_pass() {
    let dir = temp_dir("prove-inputs-internal");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let witness_library_bytes = sample_witness_library();
    let witness_library_info =
        parse_witness_library(&witness_library_bytes).expect("witness library should parse");
    write_bytes(&witness_library, &witness_library_bytes);
    let guest_image_bytes = sample_guest_image();
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    write_bytes(&guest_image, &guest_image_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--internal-contributions",
            "3",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=internal\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\ncontribution_count=3\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={expected}\nwitness_library={}\nwitness_library_bytes=64\nwitness_library_machine=62\nwitness_library_digest={}\nguest_image={}\nguest_image_bytes=64\nguest_image_machine=243\nguest_image_entry=2147483648\nguest_image_digest={}\npublic_inputs=none\n",
            output_dir.display(),
            witness_library.display(),
            format_hash(&witness_library_info.digest),
            guest_image.display(),
            format_hash(&guest_image_info.digest)
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn prints_prove_inputs_from_trace_bytes() {
    let dir = temp_dir("prove-inputs-trace-bytes");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    let guest_image_bytes = sample_guest_image();
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let trace_bytes = sample_trace_bytes(17);
    write_bytes(&guest_image, &guest_image_bytes);
    write_bytes(&trace_path, &trace_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={expected}\nwitness_library=none\ntrace_bytes={}\ntrace_bytes_file_bytes={}\nguest_image={}\nguest_image_bytes=64\nguest_image_machine=243\nguest_image_entry=2147483648\nguest_image_digest={}\npublic_inputs=none\n",
            output_dir.display(),
            trace_path.display(),
            trace_bytes.len(),
            guest_image.display(),
            format_hash(&guest_image_info.digest),
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn prints_prove_inputs_from_guest_pc_trace() {
    let dir = temp_dir("prove-inputs-guest-pc-trace");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_pc_trace_image();
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    write_bytes(&guest_image, &guest_image_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--guest-pc-trace",
            "8",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={expected}\nwitness_library=none\nguest_pc_trace_instruction_limit=8\nguest_image={}\nguest_image_bytes={}\nguest_image_machine=243\nguest_image_entry=2147483648\nguest_image_digest={}\npublic_inputs=none\n",
            output_dir.display(),
            guest_image.display(),
            guest_image_info.byte_len,
            format_hash(&guest_image_info.digest),
        )
    );
    assert!(stderr.is_empty());
}

#[cfg(not(feature = "cuda"))]
#[test]
fn prove_inputs_rejects_gpu_preallocate_without_cuda() {
    if lzvm_prover::gpu_setup_available() {
        return;
    }

    let dir = temp_dir("prove-inputs-gpu-preallocate-unavailable");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&trace_path, sample_trace_bytes(17));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--gpu-preallocate",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove inputs failed: prover GPU setup is unavailable\n"
    );
}

#[test]
fn prints_prove_inputs_with_remote_aggregation() {
    let dir = temp_dir("prove-inputs-remote-aggregation");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let witness_library_bytes = sample_witness_library();
    let witness_library_info =
        parse_witness_library(&witness_library_bytes).expect("witness library should parse");
    write_bytes(&witness_library, &witness_library_bytes);
    let guest_image_bytes = sample_guest_image();
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    write_bytes(&guest_image, &guest_image_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--aggregate",
            "--remote-aggregation",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=true\nremote_aggregation=true\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={expected}\nwitness_library={}\nwitness_library_bytes=64\nwitness_library_machine=62\nwitness_library_digest={}\nguest_image={}\nguest_image_bytes=64\nguest_image_machine=243\nguest_image_entry=2147483648\nguest_image_digest={}\npublic_inputs=none\n",
            output_dir.display(),
            witness_library.display(),
            format_hash(&witness_library_info.digest),
            guest_image.display(),
            format_hash(&guest_image_info.digest)
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn prints_prove_inputs_with_final_wrap() {
    let dir = temp_dir("prove-inputs-final-wrap");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let public_inputs = dir.join("public-inputs.bin");
    let witness_library_bytes = sample_witness_library();
    let witness_library_info =
        parse_witness_library(&witness_library_bytes).expect("witness library should parse");
    write_bytes(&witness_library, &witness_library_bytes);
    let guest_image_bytes = sample_guest_image();
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    write_bytes(&guest_image, &guest_image_bytes);
    let public_values = sample_public_values(setup_hash);
    let public_inputs_hash = public_values_digest(&public_values).expect("digest should compute");
    write_bytes(
        &public_inputs,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--aggregate",
            "--final-wrap",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_inputs
                .to_str()
                .expect("public inputs path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=true\nremote_aggregation=false\nfinal_wrap=true\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={expected}\nwitness_library={}\nwitness_library_bytes=64\nwitness_library_machine=62\nwitness_library_digest={}\nguest_image={}\nguest_image_bytes=64\nguest_image_machine=243\nguest_image_entry=2147483648\nguest_image_digest={}\npublic_inputs={}\npublic_inputs_hash={}\npublic_input_values=1\npublic_input_fields=1\n",
            output_dir.display(),
            witness_library.display(),
            format_hash(&witness_library_info.digest),
            guest_image.display(),
            format_hash(&guest_image_info.digest),
            public_inputs.display(),
            format_hash(&public_inputs_hash)
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn prove_inputs_rejects_final_wrap_without_public_inputs() {
    let dir = temp_dir("prove-inputs-final-wrap-missing-public-inputs");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, sample_guest_image());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--aggregate",
            "--final-wrap",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove inputs failed: prove execution plan final wrap requires public inputs\n"
    );
}

#[test]
fn prints_prove_inputs_with_minimal_memory() {
    let dir = temp_dir("prove-inputs-minimal-memory");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let witness_library_bytes = sample_witness_library();
    let witness_library_info =
        parse_witness_library(&witness_library_bytes).expect("witness library should parse");
    write_bytes(&witness_library, &witness_library_bytes);
    let guest_image_bytes = sample_guest_image();
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    write_bytes(&guest_image, &guest_image_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--minimal-memory",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=true\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={expected}\nwitness_library={}\nwitness_library_bytes=64\nwitness_library_machine=62\nwitness_library_digest={}\nguest_image={}\nguest_image_bytes=64\nguest_image_machine=243\nguest_image_entry=2147483648\nguest_image_digest={}\npublic_inputs=none\n",
            output_dir.display(),
            witness_library.display(),
            format_hash(&witness_library_info.digest),
            guest_image.display(),
            format_hash(&guest_image_info.digest)
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn prints_prove_inputs_with_unpacked_trace() {
    let dir = temp_dir("prove-inputs-unpacked-trace");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let witness_library_bytes = sample_witness_library();
    let witness_library_info =
        parse_witness_library(&witness_library_bytes).expect("witness library should parse");
    write_bytes(&witness_library, &witness_library_bytes);
    let guest_image_bytes = sample_guest_image();
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    write_bytes(&guest_image, &guest_image_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--no-pack-trace",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=false\nsetup_hash={expected}\nwitness_library={}\nwitness_library_bytes=64\nwitness_library_machine=62\nwitness_library_digest={}\nguest_image={}\nguest_image_bytes=64\nguest_image_machine=243\nguest_image_entry=2147483648\nguest_image_digest={}\npublic_inputs=none\n",
            output_dir.display(),
            witness_library.display(),
            format_hash(&witness_library_info.digest),
            guest_image.display(),
            format_hash(&guest_image_info.digest)
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn prove_inputs_rejects_too_few_stored_witnesses() {
    let dir = temp_dir("prove-inputs-stored-witnesses");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, sample_guest_image());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--aggregate",
            "--stored-witnesses",
            "1",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove inputs failed: stored witness limit 1 is lower than required all-unit witness outputs 4\n"
    );
}

#[test]
fn prints_prove_inputs_from_trace_bundle() {
    let dir = temp_dir("prove-inputs-trace-bundle");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let bundle_path = dir.join("trace-bundle.bin");
    let guest_image_bytes = sample_guest_image();
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let bundle_bytes = sample_trace_bundle_bytes(4, 7);
    write_bytes(&guest_image, &guest_image_bytes);
    write_bytes(&bundle_path, &bundle_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--trace-bundle",
            bundle_path.to_str().expect("bundle path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={expected}\nwitness_library=none\ntrace_bundle={}\ntrace_bundle_units=4\ntrace_bundle_bytes={}\nguest_image={}\nguest_image_bytes=64\nguest_image_machine=243\nguest_image_entry=2147483648\nguest_image_digest={}\npublic_inputs=none\n",
            output_dir.display(),
            bundle_path.display(),
            bundle_bytes.len(),
            guest_image.display(),
            format_hash(&guest_image_info.digest),
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn prints_aggregate_prove_inputs_from_trace_bundle() {
    let dir = temp_dir("prove-inputs-aggregate-trace-bundle");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let bundle_path = dir.join("trace-bundle.bin");
    let guest_image_bytes = sample_guest_image();
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let bundle_bytes = sample_trace_bundle_bytes(4, 19);
    write_bytes(&guest_image, &guest_image_bytes);
    write_bytes(&bundle_path, &bundle_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--aggregate",
            "--trace-bundle",
            bundle_path.to_str().expect("bundle path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=true\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={expected}\nwitness_library=none\ntrace_bundle={}\ntrace_bundle_units=4\ntrace_bundle_bytes={}\nguest_image={}\nguest_image_bytes=64\nguest_image_machine=243\nguest_image_entry=2147483648\nguest_image_digest={}\npublic_inputs=none\n",
            output_dir.display(),
            bundle_path.display(),
            bundle_bytes.len(),
            guest_image.display(),
            format_hash(&guest_image_info.digest),
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn prints_prove_inputs_from_single_unit_trace_bundle() {
    let dir = temp_dir("prove-inputs-single-unit-trace-bundle");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let expected = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let bundle_path = dir.join("trace-bundle.bin");
    let guest_image_bytes = sample_guest_image();
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let bundle_bytes = encode_trace_bundle(&TraceBundle {
        units: vec![TraceBundleUnit {
            unit_index: 0,
            trace_bytes: sample_trace_bytes(7),
        }],
    })
    .expect("trace bundle should encode");
    write_bytes(&guest_image, &guest_image_bytes);
    write_bytes(&bundle_path, &bundle_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--trace-bundle",
            bundle_path.to_str().expect("bundle path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={expected}\nwitness_library=none\ntrace_bundle={}\ntrace_bundle_units=1\ntrace_bundle_bytes={}\nguest_image={}\nguest_image_bytes=64\nguest_image_machine=243\nguest_image_entry=2147483648\nguest_image_digest={}\npublic_inputs=none\n",
            output_dir.display(),
            bundle_path.display(),
            bundle_bytes.len(),
            guest_image.display(),
            format_hash(&guest_image_info.digest),
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn rejects_wrong_length_trace_bundle_unit_for_prove_inputs() {
    let dir = temp_dir("prove-inputs-wrong-length-trace-bundle");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let bundle_path = dir.join("trace-bundle.bin");
    let bundle_bytes = encode_trace_bundle(&TraceBundle {
        units: vec![
            TraceBundleUnit {
                unit_index: 0,
                trace_bytes: sample_trace_bytes(7),
            },
            TraceBundleUnit {
                unit_index: 1,
                trace_bytes: sample_trace_bytes(11),
            },
            TraceBundleUnit {
                unit_index: 2,
                trace_bytes: vec![1_u8, 2, 3, 4],
            },
            TraceBundleUnit {
                unit_index: 3,
                trace_bytes: sample_trace_bytes(13),
            },
        ],
    })
    .expect("trace bundle should encode");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&bundle_path, &bundle_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--aggregate",
            "--trace-bundle",
            bundle_path.to_str().expect("bundle path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove inputs failed: trace bundle unit 2 byte length mismatch: expected 32, found 4\n"
    );
}

#[test]
fn rejects_missing_trace_bundle_unit_for_prove_inputs() {
    let dir = temp_dir("prove-inputs-missing-trace-bundle-unit");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let bundle_path = dir.join("trace-bundle.bin");
    let bundle_bytes = encode_trace_bundle(&TraceBundle {
        units: vec![
            TraceBundleUnit {
                unit_index: 0,
                trace_bytes: sample_trace_bytes(7),
            },
            TraceBundleUnit {
                unit_index: 1,
                trace_bytes: sample_trace_bytes(11),
            },
            TraceBundleUnit {
                unit_index: 3,
                trace_bytes: sample_trace_bytes(13),
            },
        ],
    })
    .expect("trace bundle should encode");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&bundle_path, &bundle_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--aggregate",
            "--trace-bundle",
            bundle_path.to_str().expect("bundle path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove inputs failed: trace bundle is missing unit 2\n"
    );
}

#[test]
fn rejects_missing_trace_bytes_for_prove_inputs() {
    let dir = temp_dir("prove-inputs-missing-trace-bytes");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    write_bytes(&guest_image, sample_guest_image());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "prove inputs failed: trace bytes are missing: {}: No such file or directory (os error 2)\n",
            trace_path.display()
        )
    );
}

#[test]
fn rejects_wrong_length_trace_bytes_for_prove_inputs() {
    let dir = temp_dir("prove-inputs-wrong-length-trace-bytes");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&trace_path, [1_u8, 2, 3, 4]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove inputs failed: trace bytes unit 0 byte length mismatch: expected 32, found 4\n"
    );
}

#[test]
fn rejects_trace_bytes_for_aggregate_prove_inputs() {
    let dir = temp_dir("prove-inputs-trace-bytes-aggregate");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&trace_path, [1_u8, 2, 3, 4]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            "--aggregate",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove inputs failed: --trace-bytes requires a single-unit witness run\n"
    );
}

#[test]
fn runs_prove_witness_commitments_for_setup_directory() {
    let dir = temp_dir("prove-witness");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);

    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan {
            input_data: Some(input_data.clone()),
            partition_count: 1,
            partition_ids: vec![0],
            worker_index: 0,
        }),
        options: ProveRunOptions::default_for_output(output_dir.clone()),
        gpu: GpuRunOptions::default(),
    };
    let plan = derive_prove_execution_plan(
        &catalog,
        request,
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library.clone()),
            guest_image: guest_image.clone(),
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let output = run_prove_witness_commitments(&plan, 0).expect("witness commitments should run");
    let mut expected_stages = String::new();
    for commitment in output.stage_commitments().commitments() {
        let root = commitment
            .root()
            .iter()
            .map(|value| value.to_u64().to_string())
            .collect::<Vec<_>>()
            .join(",");
        expected_stages.push_str(&format!(
            "stage_{}_root={root}\nstage_{}_tree_bytes={}\n",
            commitment.stage_index(),
            commitment.stage_index(),
            commitment.tree_bytes().len()
        ));
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data={}\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={setup_hash}\nunit_index=0\ninput_bytes=1\ntrace_rows=2\ntrace_columns=2\nstage_count=2\n{}",
            input_data.display(),
            output_dir.display(),
            expected_stages
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn runs_prove_witness_commitments_for_internal_contribution_pass() {
    let dir = temp_dir("prove-witness-internal");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    write_bytes(&guest_image, sample_guest_image());

    let request = ProveRunRequest {
        pass: ProvePassRequest::Internal {
            contribution_count: 3,
        },
        options: ProveRunOptions::default_for_output(output_dir.clone()),
        gpu: GpuRunOptions::default(),
    };
    let plan = derive_prove_execution_plan(
        &catalog,
        request,
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library.clone()),
            guest_image: guest_image.clone(),
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let output = run_prove_witness_commitments(&plan, 0).expect("witness commitments should run");
    let mut expected_stages = String::new();
    for commitment in output.stage_commitments().commitments() {
        let root = commitment
            .root()
            .iter()
            .map(|value| value.to_u64().to_string())
            .collect::<Vec<_>>()
            .join(",");
        expected_stages.push_str(&format!(
            "stage_{}_root={root}\nstage_{}_tree_bytes={}\n",
            commitment.stage_index(),
            commitment.stage_index(),
            commitment.tree_bytes().len()
        ));
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--internal-contributions",
            "3",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=internal\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\ncontribution_count=3\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={setup_hash}\nunit_index=0\ninput_bytes=0\ntrace_rows={}\ntrace_columns={}\nstage_count=2\n{}",
            output_dir.display(),
            output.trace_row_count(),
            output.trace_column_count(),
            expected_stages
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn runs_prove_witness_commitments_from_trace_bytes() {
    let dir = temp_dir("prove-witness-trace-bytes");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let trace_path = dir.join("trace.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);

    let mut trace_bytes = Vec::with_capacity(4 * 8);
    for value in 8_u64..=11 {
        trace_bytes.extend_from_slice(&value.to_le_bytes());
    }
    write_bytes(&trace_path, &trace_bytes);

    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan {
            input_data: Some(input_data.clone()),
            partition_count: 1,
            partition_ids: vec![0],
            worker_index: 0,
        }),
        options: ProveRunOptions::default_for_output(output_dir.clone()),
        gpu: GpuRunOptions::default(),
    };
    let plan = derive_prove_execution_plan(
        &catalog,
        request,
        ProveExecutionInputArtifacts {
            witness_library: None,
            guest_image: guest_image.clone(),
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let backend = TraceBytesBackend::new(trace_bytes);
    let output =
        run_prove_witness_commitments_with_trace_backend(&plan, 0, Default::default(), &backend)
            .expect("witness commitments should run");
    let mut expected_stages = String::new();
    for commitment in output.commitments().stage_commitments().commitments() {
        let root = commitment
            .root()
            .iter()
            .map(|value| value.to_u64().to_string())
            .collect::<Vec<_>>()
            .join(",");
        expected_stages.push_str(&format!(
            "stage_{}_root={root}\nstage_{}_tree_bytes={}\n",
            commitment.stage_index(),
            commitment.stage_index(),
            commitment.tree_bytes().len()
        ));
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data={}\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={setup_hash}\nunit_index=0\ninput_bytes=1\ntrace_rows=2\ntrace_columns=2\nstage_count=2\n{}",
            input_data.display(),
            output_dir.display(),
            expected_stages
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn runs_prove_witness_commitments_from_guest_pc_trace() {
    let dir = temp_dir("prove-witness-guest-pc-trace");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    write_bytes(&guest_image, sample_guest_pc_trace_image());

    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan {
            input_data: None,
            partition_count: 1,
            partition_ids: vec![0],
            worker_index: 0,
        }),
        options: ProveRunOptions::default_for_output(output_dir.clone()),
        gpu: GpuRunOptions::default(),
    };
    let plan = derive_prove_execution_plan(
        &catalog,
        request,
        ProveExecutionInputArtifacts {
            witness_library: None,
            guest_image: guest_image.clone(),
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let backend = GuestPcTraceBackend::new(8);
    let output =
        run_prove_witness_commitments_with_trace_backend(&plan, 0, Default::default(), &backend)
            .expect("witness commitments should run");
    let mut expected_stages = String::new();
    for commitment in output.commitments().stage_commitments().commitments() {
        let root = commitment
            .root()
            .iter()
            .map(|value| value.to_u64().to_string())
            .collect::<Vec<_>>()
            .join(",");
        expected_stages.push_str(&format!(
            "stage_{}_root={root}\nstage_{}_tree_bytes={}\n",
            commitment.stage_index(),
            commitment.stage_index(),
            commitment.tree_bytes().len()
        ));
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--guest-pc-trace",
            "8",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={setup_hash}\nunit_index=0\ninput_bytes=0\ntrace_rows=2\ntrace_columns=2\nstage_count=2\n{}",
            output_dir.display(),
            expected_stages
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn verifies_prove_witness_proof_from_guest_pc_trace() {
    let dir = temp_dir("prove-witness-guest-pc-trace-proof");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let setup_hash_hex = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(&guest_image, sample_guest_pc_trace_image());
    write_bytes(
        &public_values_path,
        encode_public_values(&sample_public_values(setup_hash))
            .expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--guest-pc-trace",
            "8",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert!(stdout_text.contains(&format!("setup_hash={setup_hash_hex}\n")));
    assert!(stdout_text.contains(&format!("public_inputs={}\n", public_values_path.display())));
    assert!(stdout_text.contains("input_bytes=0\n"));
    assert!(stdout_text.contains("trace_rows=2\n"));
    assert_eq!(proof.setup_hash, setup_hash);
    assert_has_no_contribution_segment(&proof);
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    assert!(String::from_utf8(verify_stdout)
        .expect("verify stdout should be utf-8")
        .contains("status=ok\n"));
}

#[test]
fn runs_prove_witness_with_source_generated_key_directory() {
    let dir = temp_dir("prove-witness-source-generated");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_bytes(
        &source_path,
        "airtemplate UnitA() {\n\
             col witness values[2];\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );

    let mut generate_stdout = Vec::new();
    let mut generate_stderr = Vec::new();
    let generate_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut generate_stdout,
        &mut generate_stderr,
    );
    assert_eq!(
        generate_code,
        0,
        "{}",
        String::from_utf8_lossy(&generate_stderr)
    );
    assert!(generate_stderr.is_empty());
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");

    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let trace_path = dir.join("trace.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&trace_path, sample_trace_bytes(17));
    write_bytes(
        &public_values_path,
        encode_public_values(&PublicValues {
            schema_version: 1,
            setup_hash,
            values: Vec::new(),
        })
        .expect("public values should encode"),
    );

    let mut plan_stdout = Vec::new();
    let mut plan_stderr = Vec::new();
    let plan_code = run_cli(
        &[
            "prove",
            "plan",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
        ],
        &mut plan_stdout,
        &mut plan_stderr,
    );
    assert_eq!(plan_code, 0, "{}", String::from_utf8_lossy(&plan_stderr));
    assert!(plan_stderr.is_empty());
    let plan_stdout_text = String::from_utf8(plan_stdout).expect("stdout should be utf-8");
    assert!(plan_stdout_text.contains("source_fixed_file_manifest=present\n"));
    assert!(plan_stdout_text.contains("source_program_archive=present\n"));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains("source_fixed_file_manifest=present\n"));
    assert!(stdout_text.contains("source_fixed_file_manifest_entries=0\n"));
    assert!(stdout_text.contains("source_program_archive=present\n"));
    assert!(stdout_text.contains("source_program_archive_sources=1\n"));
    assert!(stdout_text.contains("source_program_archive_edges=0\n"));

    let proof_path = output_dir.join("proof.bin");
    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );

    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    let verify_stdout_text =
        String::from_utf8(verify_stdout).expect("verify stdout should be utf-8");
    assert!(verify_stdout_text.contains("source_fixed_file_manifest=present\n"));
    assert!(verify_stdout_text.contains("source_program_archive=present\n"));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_source_generated_key_directory_round_trips_eth_block_public_values() {
    let dir = temp_dir("prove-witness-source-generated-eth-block");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_bytes(
        &source_path,
        "public eth_block_hash_u32_be[8];\n\
         public eth_parent_hash_u32_be[8];\n\
         public eth_beneficiary_u32_be[5];\n\
         public eth_state_root_u32_be[8];\n\
         public eth_receipts_root_u32_be[8];\n\
         public eth_logs_bloom_u32_be[64];\n\
         public eth_difficulty_u32_be[8];\n\
         public eth_block_number_u32_le[2];\n\
         public eth_block_timestamp_u32_le[2];\n\
         public eth_extra_data_len;\n\
         public eth_extra_data_u32_be[8];\n\
         public eth_gas_limit_u32_le[2];\n\
         public eth_gas_used_u32_le[2];\n\
         public eth_base_fee_per_gas_present;\n\
         public eth_base_fee_per_gas_u32_be[8];\n\
         public eth_mix_hash_u32_be[8];\n\
         public eth_nonce_u32_be[2];\n\
         public eth_ommers_hash_u32_be[8];\n\
         public eth_transactions_root_u32_be[8];\n\
         public eth_withdrawals_root_present;\n\
         public eth_withdrawals_root_u32_be[8];\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );

    let mut generate_stdout = Vec::new();
    let mut generate_stderr = Vec::new();
    let generate_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut generate_stdout,
        &mut generate_stderr,
    );
    assert_eq!(
        generate_code,
        0,
        "{}",
        String::from_utf8_lossy(&generate_stderr)
    );
    assert!(generate_stderr.is_empty());
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");

    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let trace_path = dir.join("trace.bin");
    let block_input_path = dir.join("block.input");
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    let block_input_hash = eth_block_input_bytes_digest(&encoded_block_input);
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&trace_path, sample_trace_bytes(23));
    write_bytes(&block_input_path, &encoded_block_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let generated_bytes =
        fs::read(&generated_public_values_path).expect("generated public values should read");
    let generated_public_values =
        parse_public_values(&generated_bytes).expect("generated public values should parse");
    assert_eq!(
        generated_public_values,
        public_values_from_eth_block_input(setup_hash, &block_input)
    );
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains("source_fixed_file_manifest=present\n"));
    assert!(stdout_text.contains("source_program_archive=present\n"));
    assert!(stdout_text.contains("public_inputs_generated=eth_block_input\n"));
    assert!(stdout_text.contains(&format!(
        "eth_block_input_hash={}\n",
        format_hash(&block_input_hash)
    )));

    let proof_path = output_dir.join("proof.bin");
    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            generated_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );

    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    let verify_stdout_text =
        String::from_utf8(verify_stdout).expect("verify stdout should be utf-8");
    assert!(verify_stdout_text.contains("source_fixed_file_manifest=present\n"));
    assert!(verify_stdout_text.contains("source_program_archive=present\n"));
    assert!(verify_stdout_text.contains("eth_block_input_match=ok\n"));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_source_generated_all_units_preserves_bound_inputs() {
    let dir = temp_dir("prove-witness-source-generated-all-units-bound-inputs");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_bytes(
        &source_path,
        "public eth_block_hash_u32_be[8];\n\
         public eth_parent_hash_u32_be[8];\n\
         public eth_beneficiary_u32_be[5];\n\
         public eth_state_root_u32_be[8];\n\
         public eth_receipts_root_u32_be[8];\n\
         public eth_logs_bloom_u32_be[64];\n\
         public eth_difficulty_u32_be[8];\n\
         public eth_block_number_u32_le[2];\n\
         public eth_block_timestamp_u32_le[2];\n\
         public eth_extra_data_len;\n\
         public eth_extra_data_u32_be[8];\n\
         public eth_gas_limit_u32_le[2];\n\
         public eth_gas_used_u32_le[2];\n\
         public eth_base_fee_per_gas_present;\n\
         public eth_base_fee_per_gas_u32_be[8];\n\
         public eth_mix_hash_u32_be[8];\n\
         public eth_nonce_u32_be[2];\n\
         public eth_ommers_hash_u32_be[8];\n\
         public eth_transactions_root_u32_be[8];\n\
         public eth_withdrawals_root_present;\n\
         public eth_withdrawals_root_u32_be[8];\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
         }\n\
         airtemplate UnitB() {\n\
             col witness values[2];\n\
         }\n\
         airgroup GroupA { UnitA(); UnitB(); }\n\
         col fixed main.left = [5, 1];",
    );

    let mut generate_stdout = Vec::new();
    let mut generate_stderr = Vec::new();
    let generate_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut generate_stdout,
        &mut generate_stderr,
    );
    assert_eq!(
        generate_code,
        0,
        "{}",
        String::from_utf8_lossy(&generate_stderr)
    );
    assert!(generate_stderr.is_empty());
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let unit_count = u32::try_from(catalog.units.len()).expect("unit count should fit");
    assert!(unit_count > 1);
    let stored_witnesses = unit_count.to_string();

    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let bundle_path = dir.join("trace-bundle.bin");
    let block_input_path = dir.join("block.input");
    let program_path = dir.join("program.bin");
    let constraint_digest_path = dir.join("constraint.digest");
    let root_path = dir.join("root.bin");
    let cache_path = dir.join("program_image.cache");
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&bundle_path, sample_trace_bundle_bytes(unit_count, 29));
    write_bytes(&block_input_path, &encoded_block_input);
    write_bytes(&program_path, b"source-generated-program");
    write_bytes(&constraint_digest_path, setup_hash);
    write_bytes(
        &root_path,
        encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![11, 12, 13, 14]))
            .expect("root should encode"),
    );
    write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
        program_path: &program_path,
        guest_image_path: &guest_image,
        constraint_digest_path: &constraint_digest_path,
        root_path: &root_path,
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
        output_path: &cache_path,
    })
    .expect("cache should write");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--all-units",
            "--stored-witnesses",
            stored_witnesses.as_str(),
            "--trace-bundle",
            bundle_path.to_str().expect("bundle path should be utf-8"),
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let generated_bytes =
        fs::read(&generated_public_values_path).expect("generated public values should read");
    let generated_public_values =
        parse_public_values(&generated_bytes).expect("generated public values should parse");
    assert_eq!(
        generated_public_values,
        public_values_from_eth_block_input(setup_hash, &block_input)
    );
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains(&format!("units={unit_count}\n")));
    assert!(stdout_text.contains("source_fixed_file_manifest=present\n"));
    assert!(stdout_text.contains("source_program_archive=present\n"));
    assert!(stdout_text.contains("public_inputs_generated=eth_block_input\n"));
    assert!(stdout_text.contains("program_image_cache_gpu_mode=cuda\n"));
    assert!(stdout_text.contains("eth_block_input="));

    let proof_path = output_dir.join("proof.bin");
    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            generated_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );

    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    let verify_stdout_text =
        String::from_utf8(verify_stdout).expect("verify stdout should be utf-8");
    assert!(verify_stdout_text.contains("source_fixed_file_manifest=present\n"));
    assert!(verify_stdout_text.contains("source_program_archive=present\n"));
    assert!(verify_stdout_text.contains("program_image_cache_match=ok\n"));
    assert!(verify_stdout_text.contains("eth_block_input_match=ok\n"));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_source_generated_all_units_packs_eth_block_public_outputs() {
    let dir = temp_dir("prove-witness-source-generated-packed-eth-outputs");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_bytes(
        &source_path,
        "public rom_root[4];\n\
         public inputs[64];\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
         }\n\
         airtemplate UnitB() {\n\
             col witness values[2];\n\
         }\n\
         airgroup GroupA { UnitA(); UnitB(); }\n\
         col fixed main.left = [5, 1];",
    );

    let mut generate_stdout = Vec::new();
    let mut generate_stderr = Vec::new();
    let generate_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut generate_stdout,
        &mut generate_stderr,
    );
    assert_eq!(
        generate_code,
        0,
        "{}",
        String::from_utf8_lossy(&generate_stderr)
    );
    assert!(generate_stderr.is_empty());
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let unit_count = u32::try_from(catalog.units.len()).expect("unit count should fit");
    assert!(unit_count > 1);
    let stored_witnesses = unit_count.to_string();

    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let bundle_path = dir.join("trace-bundle.bin");
    let block_input_path = dir.join("block.input");
    let program_path = dir.join("program.bin");
    let constraint_digest_path = dir.join("constraint.digest");
    let root_path = dir.join("root.bin");
    let cache_path = dir.join("program_image.cache");
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&bundle_path, sample_trace_bundle_bytes(unit_count, 31));
    write_bytes(&block_input_path, &encoded_block_input);
    write_bytes(&program_path, b"source-generated-program");
    write_bytes(&constraint_digest_path, setup_hash);
    write_bytes(
        &root_path,
        encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![11, 12, 13, 14]))
            .expect("root should encode"),
    );
    write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
        program_path: &program_path,
        guest_image_path: &guest_image,
        constraint_digest_path: &constraint_digest_path,
        root_path: &root_path,
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
        output_path: &cache_path,
    })
    .expect("cache should write");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--all-units",
            "--stored-witnesses",
            stored_witnesses.as_str(),
            "--trace-bundle",
            bundle_path.to_str().expect("bundle path should be utf-8"),
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let generated_bytes =
        fs::read(&generated_public_values_path).expect("generated public values should read");
    let generated_public_values =
        parse_public_values(&generated_bytes).expect("generated public values should parse");
    assert_eq!(generated_public_values.setup_hash, setup_hash);
    assert_eq!(generated_public_values.values.len(), 2);
    assert_eq!(generated_public_values.values[0].name, "rom_root");
    assert_eq!(
        generated_public_values.values[0].elements,
        vec![11, 12, 13, 14]
    );
    assert_eq!(generated_public_values.values[1].name, "inputs");
    let mut expected_inputs = block_input
        .block_hash
        .chunks_exact(4)
        .map(|chunk| {
            u64::from(u32::from_le_bytes(
                chunk.try_into().expect("chunk is 4 bytes"),
            ))
        })
        .collect::<Vec<_>>();
    expected_inputs.resize(64, 0);
    assert_eq!(generated_public_values.values[1].elements, expected_inputs);
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains(&format!("units={unit_count}\n")));
    assert!(stdout_text.contains("public_inputs_generated=eth_block_input\n"));
    assert!(stdout_text.contains("public_input_values=2\n"));
    assert!(stdout_text.contains("public_input_fields=68\n"));
    assert!(stdout_text.contains("program_image_cache_gpu_mode=cuda\n"));
    assert!(stdout_text.contains("eth_block_input="));

    let proof_path = output_dir.join("proof.bin");
    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            generated_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );

    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    let verify_stdout_text =
        String::from_utf8(verify_stdout).expect("verify stdout should be utf-8");
    assert!(verify_stdout_text.contains("program_image_cache_match=ok\n"));
    assert!(verify_stdout_text.contains("eth_block_input_match=ok\n"));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn runs_prove_witness_commitments_from_trace_bundle() {
    let dir = temp_dir("prove-witness-trace-bundle");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_proof_group_and_unit_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let bundle_path = dir.join("trace-bundle.bin");
    let unit_values_path = dir.join("unit_values.bin");
    let proof_values_path = dir.join("proof_values.bin");
    let group_values_path = dir.join("group_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [17_u8]);
    write_field_words(&unit_values_path, &[101, 201, 202, 203]);
    write_field_words(&proof_values_path, &[51, 52, 53]);
    write_field_words(&group_values_path, &[61, 62, 63]);
    let bundle_bytes = sample_trace_bundle_bytes(4, 17);
    write_bytes(&bundle_path, &bundle_bytes);

    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--trace-bundle",
            bundle_path.to_str().expect("bundle path should be utf-8"),
            "--aggregate",
            "--save-outputs",
            "--unit-values",
            unit_values_path
                .to_str()
                .expect("unit values path should be utf-8"),
            "--proof-values",
            proof_values_path
                .to_str()
                .expect("proof values path should be utf-8"),
            "--group-values",
            group_values_path
                .to_str()
                .expect("group values path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let proof_bytes = fs::read(output_dir.join("proof.bin")).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let witness_ids = proof
        .segments
        .iter()
        .filter(|segment| {
            segment.id >= WITNESS_COMMITMENT_SEGMENT_BASE_ID
                && segment.id < WITNESS_COMMITMENT_SEGMENT_BASE_ID + 2
        })
        .map(|segment| segment.id)
        .collect::<Vec<_>>();
    assert_eq!(
        witness_ids,
        vec![
            WITNESS_COMMITMENT_SEGMENT_BASE_ID,
            WITNESS_COMMITMENT_SEGMENT_BASE_ID + 1,
        ]
    );
    assert!(output_dir.join("unit-0.witness-segment").exists());
    assert!(output_dir.join("unit-1.witness-segment").exists());
    let stdout = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout.starts_with("status=ok\npass=full\n"));
    assert!(stdout.contains("unit_index=0\n"));
    assert!(stdout.contains("unit_index=1\n"));
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_trace_bundle_missing_unit_for_aggregate_witness_runs() {
    let dir = temp_dir("prove-witness-trace-bundle-missing-unit");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_proof_group_and_unit_value(&dir);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let bundle_path = dir.join("trace-bundle.bin");
    let unit_values_path = dir.join("unit_values.bin");
    let proof_values_path = dir.join("proof_values.bin");
    let group_values_path = dir.join("group_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [17_u8]);
    write_field_words(&unit_values_path, &[101, 201, 202, 203]);
    write_field_words(&proof_values_path, &[51, 52, 53]);
    write_field_words(&group_values_path, &[61, 62, 63]);
    let bundle_bytes = sample_trace_bundle_bytes(1, 17);
    write_bytes(&bundle_path, &bundle_bytes);

    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&sample_public_values(setup_hash))
            .expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--trace-bundle",
            bundle_path.to_str().expect("bundle path should be utf-8"),
            "--aggregate",
            "--save-outputs",
            "--unit-values",
            unit_values_path
                .to_str()
                .expect("unit values path should be utf-8"),
            "--proof-values",
            proof_values_path
                .to_str()
                .expect("proof values path should be utf-8"),
            "--group-values",
            group_values_path
                .to_str()
                .expect("group values path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove witness failed: trace bundle is missing unit 1\n"
    );
}

#[test]
fn rejects_trace_bytes_for_all_unit_witness_runs() {
    let dir = temp_dir("prove-witness-trace-bytes-all-units");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let trace_path = dir.join("trace.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            "--all-units",
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove witness failed: --trace-bytes requires a single-unit witness run\n"
    );
}

#[test]
fn saves_prove_witness_commitment_outputs_when_requested() {
    let dir = temp_dir("prove-witness-save");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [11_u8]);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan {
            input_data: Some(input_data.clone()),
            partition_count: 1,
            partition_ids: vec![0],
            worker_index: 0,
        }),
        options: ProveRunOptions {
            save_outputs: true,
            ..ProveRunOptions::default_for_output(output_dir.clone())
        },
        gpu: GpuRunOptions::default(),
    };
    let plan = derive_prove_execution_plan(
        &catalog,
        request,
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library.clone()),
            guest_image: guest_image.clone(),
            public_inputs: Some(public_values_path.clone()),
        },
    )
    .expect("execution plan should derive");
    let output = run_prove_witness_commitments(&plan, 0).expect("witness commitments should run");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--save-outputs",
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert!(stderr.is_empty());
    for commitment in output.stage_commitments().commitments() {
        let root_path = output_dir.join(format!(
            "unit-0-stage-{}.witness-root",
            commitment.stage_index()
        ));
        let tree_path = output_dir.join(format!(
            "unit-0-stage-{}.witness-tree",
            commitment.stage_index()
        ));
        let mut expected_root = Vec::new();
        for value in commitment.root() {
            expected_root.extend_from_slice(&value.to_le_bytes());
        }
        assert_eq!(
            fs::read(&root_path).expect("root output should read"),
            expected_root
        );
        assert_eq!(
            fs::read(&tree_path).expect("tree output should read"),
            commitment.tree_bytes()
        );
    }
    let expected_segment =
        build_witness_commitment_segment(&output).expect("witness segment should build");
    let segment_path = output_dir.join("unit-0.witness-segment");
    let segment_bytes = fs::read(&segment_path).expect("segment output should read");
    assert_eq!(expected_segment.id, WITNESS_COMMITMENT_SEGMENT_BASE_ID);
    assert_eq!(segment_bytes, expected_segment.data);
    assert_eq!(
        parse_witness_commitment_segment(&segment_bytes)
            .expect("segment output should parse")
            .stages
            .len(),
        output.stage_commitments().stage_count()
    );
    let proof_bytes = fs::read(output_dir.join("proof.bin")).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    assert_eq!(proof.setup_hash, setup_hash);
    assert_eq!(
        proof.public_values_hash,
        public_values_digest(&public_values).expect("digest should compute")
    );
    assert_eq!(proof.segments.len(), 5);
    assert_has_no_contribution_segment(&proof);
    assert_eq!(proof.segments[0].id, PCS_MATERIAL_MANIFEST_SEGMENT_ID);
    let manifest = parse_pcs_material_manifest_segment(&proof.segments[0].data)
        .expect("material manifest should parse");
    assert_eq!(manifest.units.len(), catalog.units.len());
    for (index, (manifest_unit, catalog_unit)) in
        manifest.units.iter().zip(catalog.units.iter()).enumerate()
    {
        let material = catalog_unit
            .pcs_material
            .as_ref()
            .expect("material should be loaded");
        assert_eq!(manifest_unit.unit_index, index as u32);
        assert_eq!(manifest_unit.plan_digest, material.plan_digest);
        assert_eq!(
            manifest_unit.fixed_column_digest,
            material.fixed_column_digest
        );
        assert_eq!(
            manifest_unit.constant_tree_digest,
            material.constant_tree_digest
        );
        assert_eq!(
            manifest_unit.constant_tree_root,
            material.constant_tree_root
        );
        assert_eq!(manifest_unit.fixed_byte_count, material.fixed_byte_count);
        assert_eq!(
            manifest_unit.constant_tree_byte_count,
            material.constant_tree_byte_count
        );
        assert_eq!(manifest_unit.leaf_byte_count, material.leaf_byte_count);
        assert_eq!(manifest_unit.node_byte_count, material.node_byte_count);
    }
    let public_values_hash = public_values_digest(&public_values).expect("digest should compute");
    let expected_query_segment = build_pcs_query_plan_segment(
        &plan.run_plan.schedule,
        public_values_hash,
        &proof.segments[0],
        std::slice::from_ref(&expected_segment),
    )
    .expect("query segment should build");
    let query_plan =
        parse_pcs_query_plan_segment(&proof.segments[1].data).expect("query plan should parse");
    assert_eq!(proof.segments[1].id, PCS_QUERY_PLAN_SEGMENT_ID);
    assert_eq!(proof.segments[1], expected_query_segment);
    assert_eq!(query_plan.units.len(), 1);
    assert_eq!(
        query_plan.units[0].queries.len(),
        plan.run_plan.schedule.units[0].query_count as usize
    );
    let expected_constant_opening_segment =
        build_constant_opening_segment(&catalog, &plan.run_plan.schedule, &proof.segments[1])
            .expect("constant opening segment should build");
    let constant_opening = parse_constant_opening_segment(&proof.segments[2].data)
        .expect("constant opening segment should parse");
    assert_eq!(proof.segments[2].id, CONSTANT_OPENING_SEGMENT_ID);
    assert_eq!(proof.segments[2], expected_constant_opening_segment);
    assert_eq!(constant_opening.units.len(), 1);
    assert_eq!(
        constant_opening.units[0].queries.len(),
        query_plan.units[0].queries.len()
    );
    let expected_opening_segment =
        build_witness_opening_segment(&plan.run_plan.schedule, &proof.segments[1], &output)
            .expect("opening segment should build");
    let opening = parse_witness_opening_segment(&proof.segments[3].data)
        .expect("opening segment should parse");
    assert_eq!(proof.segments[3].id, WITNESS_OPENING_SEGMENT_ID);
    assert_eq!(proof.segments[3], expected_opening_segment);
    assert_eq!(opening.units.len(), 1);
    assert_eq!(
        opening.units[0].queries.len(),
        query_plan.units[0].queries.len()
    );
    assert_eq!(proof.segments[4], expected_segment);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn writes_prove_witness_proof_without_save_outputs() {
    let dir = temp_dir("prove-witness-proof-only");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let setup_hash_hex = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);

    let public_values = sample_public_values(setup_hash);
    let public_values_hash = public_values_digest(&public_values).expect("digest should compute");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan {
            input_data: Some(input_data.clone()),
            partition_count: 1,
            partition_ids: vec![0],
            worker_index: 0,
        }),
        options: ProveRunOptions::default_for_output(output_dir.clone()),
        gpu: GpuRunOptions::default(),
    };
    let plan = derive_prove_execution_plan(
        &catalog,
        request,
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library.clone()),
            guest_image: guest_image.clone(),
            public_inputs: Some(public_values_path.clone()),
        },
    )
    .expect("execution plan should derive");
    let output = run_prove_witness_commitments(&plan, 0).expect("witness commitments should run");
    let mut expected_stages = String::new();
    for commitment in output.stage_commitments().commitments() {
        let root = commitment
            .root()
            .iter()
            .map(|value| value.to_u64().to_string())
            .collect::<Vec<_>>()
            .join(",");
        expected_stages.push_str(&format!(
            "stage_{}_root={root}\nstage_{}_tree_bytes={}\n",
            commitment.stage_index(),
            commitment.stage_index(),
            commitment.tree_bytes().len()
        ));
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let proof_bytes = fs::read(output_dir.join("proof.bin")).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    assert!(!output_dir.join("unit-0.witness-segment").exists());
    assert!(!output_dir.join("unit-0-stage-0.witness-root").exists());
    assert!(!output_dir.join("unit-0-stage-0.witness-tree").exists());
    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data={}\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={setup_hash_hex}\npublic_inputs={}\npublic_inputs_hash={}\npublic_input_values=1\npublic_input_fields=1\nunit_index=0\ninput_bytes=1\ntrace_rows=2\ntrace_columns=2\nstage_count=2\n{}",
            input_data.display(),
            output_dir.display(),
            public_values_path.display(),
            format_hash(&public_values_hash),
            expected_stages
        )
    );
    assert!(stderr.is_empty());
    assert_eq!(proof.setup_hash, setup_hash);
    assert_eq!(proof.segments.len(), 5);
    assert_has_no_contribution_segment(&proof);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn runs_direct_prove_for_setup_directory() {
    let dir = temp_dir("direct-prove");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(
        &public_values_path,
        encode_public_values(&sample_public_values(setup_hash))
            .expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let proof_path = output_dir.join("proof.bin");
    let proof = fs::read(&proof_path)
        .ok()
        .and_then(|bytes| parse_proof_artifact(&bytes).ok());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert!(stderr.is_empty());
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\npass=full\n"));
    assert_eq!(
        proof.expect("proof output should parse").setup_hash,
        setup_hash
    );
}

#[cfg(not(feature = "cuda"))]
#[test]
fn prove_witness_rejects_gpu_preallocate_without_cuda() {
    if lzvm_prover::gpu_setup_available() {
        return;
    }

    let dir = temp_dir("prove-witness-gpu-preallocate-unavailable");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&trace_path, [1_u8, 2, 3, 4]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--gpu-preallocate",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove witness failed: prover GPU setup is unavailable\n"
    );
}

#[test]
fn runs_prove_witness_commitments_with_minimal_memory() {
    let dir = temp_dir("prove-witness-minimal-memory");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    write_bytes(&guest_image, sample_guest_image());
    let trace_bytes = sample_trace_bytes(0);
    write_bytes(&trace_path, &trace_bytes);

    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan {
            input_data: None,
            partition_count: 1,
            partition_ids: vec![0],
            worker_index: 0,
        }),
        options: ProveRunOptions {
            minimal_memory: true,
            ..ProveRunOptions::default_for_output(output_dir.clone())
        },
        gpu: GpuRunOptions::default(),
    };
    let plan = derive_prove_execution_plan(
        &catalog,
        request,
        ProveExecutionInputArtifacts {
            witness_library: None,
            guest_image: guest_image.clone(),
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let backend = TraceBytesBackend::new(trace_bytes);
    let output =
        run_prove_witness_commitments_with_trace_backend(&plan, 0, Default::default(), &backend)
            .expect("witness commitments should run");
    let mut expected_stages = String::new();
    for commitment in output.commitments().stage_commitments().commitments() {
        let root = commitment
            .root()
            .iter()
            .map(|value| value.to_u64().to_string())
            .collect::<Vec<_>>()
            .join(",");
        expected_stages.push_str(&format!(
            "stage_{}_root={root}\nstage_{}_tree_bytes={}\n",
            commitment.stage_index(),
            commitment.stage_index(),
            commitment.tree_bytes().len()
        ));
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--minimal-memory",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=true\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={setup_hash}\nunit_index=0\ninput_bytes=0\ntrace_rows=2\ntrace_columns=2\nstage_count=2\n{}",
            output_dir.display(),
            expected_stages
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn runs_prove_witness_commitments_with_unpacked_trace() {
    let dir = temp_dir("prove-witness-unpacked-trace");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let material_bytes = pcs_material_byte_count(&catalog);
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let trace_path = dir.join("trace.bin");
    write_bytes(&guest_image, sample_guest_image());
    let trace_bytes = sample_trace_bytes(0);
    write_bytes(&trace_path, &trace_bytes);

    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan {
            input_data: None,
            partition_count: 1,
            partition_ids: vec![0],
            worker_index: 0,
        }),
        options: ProveRunOptions::default_for_output(output_dir.clone()),
        gpu: GpuRunOptions {
            pack_trace: false,
            ..Default::default()
        },
    };
    let plan = derive_prove_execution_plan(
        &catalog,
        request,
        ProveExecutionInputArtifacts {
            witness_library: None,
            guest_image: guest_image.clone(),
            public_inputs: None,
        },
    )
    .expect("execution plan should derive");
    let backend = TraceBytesBackend::new(trace_bytes);
    let output =
        run_prove_witness_commitments_with_trace_backend(&plan, 0, Default::default(), &backend)
            .expect("witness commitments should run");
    let mut expected_stages = String::new();
    for commitment in output.commitments().stage_commitments().commitments() {
        let root = commitment
            .root()
            .iter()
            .map(|value| value.to_u64().to_string())
            .collect::<Vec<_>>()
            .join(",");
        expected_stages.push_str(&format!(
            "stage_{}_root={root}\nstage_{}_tree_bytes={}\n",
            commitment.stage_index(),
            commitment.stage_index(),
            commitment.tree_bytes().len()
        ));
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--no-pack-trace",
            "--trace-bytes",
            trace_path.to_str().expect("trace path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=false\nsetup_hash={setup_hash}\nunit_index=0\ninput_bytes=0\ntrace_rows=2\ntrace_columns=2\nstage_count=2\n{}",
            output_dir.display(),
            expected_stages
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn embeds_program_image_cache_segment_in_prove_witness_proof_output() {
    let dir = temp_dir("prove-witness-program-image-cache-segment");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let public_values_path = dir.join("public_values.bin");
    let program_path = dir.join("program.bin");
    let other_program_path = dir.join("other-program.bin");
    let constraint_digest_path = dir.join("constraint.digest");
    let root_path = dir.join("root.bin");
    let cache_path = dir.join("program_image.cache");
    let other_cache_path = dir.join("other-program_image.cache");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(
        &public_values_path,
        encode_public_values(&sample_public_values(setup_hash))
            .expect("public values should encode"),
    );
    write_bytes(&program_path, b"packed-program");
    write_bytes(&other_program_path, b"other-packed-program");
    write_bytes(&constraint_digest_path, setup_hash);
    write_bytes(
        &root_path,
        encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![11, 12, 13, 14]))
            .expect("root should encode"),
    );
    write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
        program_path: &program_path,
        guest_image_path: &guest_image,
        constraint_digest_path: &constraint_digest_path,
        root_path: &root_path,
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
        output_path: &cache_path,
    })
    .expect("cache should write");
    write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
        program_path: &other_program_path,
        guest_image_path: &guest_image,
        constraint_digest_path: &constraint_digest_path,
        root_path: &root_path,
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
        output_path: &other_cache_path,
    })
    .expect("other cache should write");
    let expected_cache =
        read_program_image_commitment_cache_file(&cache_path).expect("cache should read");
    let expected_cache_segment =
        encode_program_image_cache_segment(&expected_cache).expect("cache segment should encode");
    let expected_cache_segment_hash = program_image_cache_segment_digest(&expected_cache_segment);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PROGRAM_IMAGE_CACHE_SEGMENT_ID)
        .expect("program image cache segment should be present");
    let parsed_cache =
        parse_program_image_cache_segment(&segment.data).expect("cache segment should parse");
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("setup path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    let verify_stdout_text = String::from_utf8(verify_stdout).expect("stdout should be utf-8");
    let mut proof_verify_stdout = Vec::new();
    let mut proof_verify_stderr = Vec::new();
    let proof_verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            dir.to_str().expect("setup path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut proof_verify_stdout,
        &mut proof_verify_stderr,
    );
    let proof_verify_stdout_text =
        String::from_utf8(proof_verify_stdout).expect("stdout should be utf-8");
    let mut mismatch_stdout = Vec::new();
    let mut mismatch_stderr = Vec::new();
    let mismatch_code = run_cli(
        &[
            "verify",
            "proof",
            "--program-image-cache",
            other_cache_path
                .to_str()
                .expect("cache path should be utf-8"),
            dir.to_str().expect("setup path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut mismatch_stdout,
        &mut mismatch_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert!(stdout_text.contains(&format!(
        "program_image_cache_constraint_system_digest={}\n",
        format_hash(&expected_cache.constraint_system_digest)
    )));
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    assert!(verify_stdout_text.contains("program_image_caches=1\n"));
    assert!(verify_stdout_text.contains(&format!(
        "program_image_cache_segment_hash={}\n",
        format_hash(&expected_cache_segment_hash)
    )));
    assert!(verify_stdout_text.contains(&format!(
        "program_image_cache_program_digest={}\n",
        format_hash(&expected_cache.program_digest)
    )));
    assert!(verify_stdout_text.contains(&format!(
        "program_image_cache_source_image_digest={}\n",
        format_hash(&expected_cache.source_image_digest)
    )));
    assert!(verify_stdout_text.contains(&format!(
        "program_image_cache_constraint_system_digest={}\n",
        format_hash(&expected_cache.constraint_system_digest)
    )));
    assert!(verify_stdout_text.contains("program_image_cache_tree_root=11,12,13,14\n"));
    assert!(verify_stdout_text.contains("program_image_cache_trace_rows=1024\n"));
    assert!(verify_stdout_text.contains("program_image_cache_trace_columns=17\n"));
    assert!(verify_stdout_text.contains("program_image_cache_blowup_factor=8\n"));
    assert!(verify_stdout_text.contains("program_image_cache_arity=4\n"));
    assert!(verify_stdout_text.contains("program_image_cache_gpu_mode=cuda\n"));
    assert_eq!(
        proof_verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&proof_verify_stderr)
    );
    assert!(proof_verify_stderr.is_empty());
    assert!(proof_verify_stdout_text.contains(&format!(
        "program_image_cache_program_digest={}\n",
        format_hash(&expected_cache.program_digest)
    )));
    assert!(proof_verify_stdout_text.contains("program_image_cache_match=ok\n"));
    assert_eq!(mismatch_code, 1);
    assert!(mismatch_stdout.is_empty());
    assert_eq!(
        String::from_utf8(mismatch_stderr).expect("mismatch stderr should be utf-8"),
        "verify proof failed: program image cache proof segment mismatch\n"
    );
    assert_eq!(parsed_cache, expected_cache);
}

#[test]
fn embeds_program_image_cache_and_eth_block_input_segments_in_prove_witness_proof_output() {
    let dir = temp_dir("prove-witness-program-image-cache-and-eth-block-input");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let program_path = dir.join("program.bin");
    let constraint_digest_path = dir.join("constraint.digest");
    let root_path = dir.join("root.bin");
    let cache_path = dir.join("program_image.cache");
    let block_input_path = dir.join("block.input");
    let receipt_item = sample_receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let receipts_rlp = rlp_list(&[receipt_item]);
    let block_rlp = sample_block_rlp_with_receipts_root(receipt_build.root);
    let block_input = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect("block input should build");
    let block_input_bytes =
        encode_eth_block_input(&block_input).expect("block input should encode");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&program_path, b"packed-program");
    write_bytes(&constraint_digest_path, setup_hash);
    write_bytes(
        &root_path,
        encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![11, 12, 13, 14]))
            .expect("root should encode"),
    );
    write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
        program_path: &program_path,
        guest_image_path: &guest_image,
        constraint_digest_path: &constraint_digest_path,
        root_path: &root_path,
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
        output_path: &cache_path,
    })
    .expect("cache should write");
    let expected_cache =
        read_program_image_commitment_cache_file(&cache_path).expect("cache should read");
    write_bytes(&block_input_path, &block_input_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    let proof_path = output_dir.join("proof.bin");
    let public_values_path = output_dir.join("eth-block-public-values.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let cache_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PROGRAM_IMAGE_CACHE_SEGMENT_ID)
        .expect("program image cache segment should be present");
    let parsed_cache =
        parse_program_image_cache_segment(&cache_segment.data).expect("cache segment should parse");
    let block_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
        .expect("ETH block input segment should be present");
    let parsed_block_input = parse_eth_block_input_segment(&block_segment.data)
        .expect("block input segment should parse");
    let generated_public_values_bytes =
        fs::read(&public_values_path).expect("public values should read");
    let generated_public_values =
        parse_public_values(&generated_public_values_bytes).expect("public values should parse");
    let expected_public_values = public_values_from_eth_block_input(setup_hash, &block_input);

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(generated_public_values, expected_public_values);
    assert_eq!(parsed_cache, expected_cache);
    assert_eq!(parsed_block_input, block_input);
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains("public_inputs_generated=eth_block_input\n"));
    assert!(stdout_text.contains("program_image_cache_gpu_mode=cuda\n"));
    assert!(stdout_text.contains("eth_block_input_bytes="));
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    let verify_stdout_text =
        String::from_utf8(verify_stdout).expect("verify stdout should be utf-8");
    assert!(verify_stdout_text.contains("program_image_caches=1\n"));
    assert!(verify_stdout_text.contains("eth_block_inputs=1\n"));
    assert!(verify_stdout_text.contains("program_image_cache_match=ok\n"));
    assert!(verify_stdout_text.contains("eth_block_input_match=ok\n"));
}

#[test]
fn embeds_eth_block_input_segment_in_prove_witness_proof_output() {
    let dir = temp_dir("prove-witness-eth-block-input-segment");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let public_values_path = dir.join("public_values.bin");
    let mismatched_public_values_path = dir.join("mismatched_public_values.bin");
    let mismatched_output_dir = dir.join("mismatched-proof-out");
    let mismatched_proof_path = dir.join("mismatched-public-proof.bin");
    let block_input_path = dir.join("block.input");
    let receipt_item = sample_receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let receipts_rlp = rlp_list(&[receipt_item]);
    let block_rlp = sample_block_rlp_with_receipts_root(receipt_build.root);
    let block_input = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect("block input should build");
    let block_input_bytes =
        encode_eth_block_input(&block_input).expect("block input should encode");
    let block_input_hash = eth_block_input_bytes_digest(&block_input_bytes);
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let other_block_input =
        build_eth_block_input(&sample_block_rlp_variant()).expect("block input should build");
    let other_block_input_bytes =
        encode_eth_block_input(&other_block_input).expect("block input should encode");
    let other_block_input_path = dir.join("other-block.input");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    let public_values = public_values_from_eth_block_input(setup_hash, &block_input);
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );
    let mismatched_public_values =
        public_values_from_eth_block_input(setup_hash, &other_block_input);
    write_bytes(
        &mismatched_public_values_path,
        encode_public_values(&mismatched_public_values).expect("public values should encode"),
    );
    let mismatched_proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&mismatched_public_values)
            .expect("digest should compute"),
        segments: vec![ProofSegment {
            id: ETH_BLOCK_INPUT_SEGMENT_ID,
            data: encode_eth_block_input_segment(&block_input).expect("segment should encode"),
        }],
    };
    write_bytes(
        &mismatched_proof_path,
        encode_proof_artifact(&mismatched_proof).expect("proof should encode"),
    );
    write_bytes(&block_input_path, &block_input_bytes);
    write_bytes(&other_block_input_path, &other_block_input_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    let mut public_values_prove_stdout = Vec::new();
    let mut public_values_prove_stderr = Vec::new();
    let public_values_prove_code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            mismatched_output_dir
                .to_str()
                .expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            mismatched_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut public_values_prove_stdout,
        &mut public_values_prove_stderr,
    );

    let proof_bytes = fs::read(output_dir.join("proof.bin")).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
        .expect("ETH block input segment should be present");
    let parsed_input =
        parse_eth_block_input_segment(&segment.data).expect("block input segment should parse");
    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let proof_path = output_dir.join("proof.bin");
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    let mut preflight_stdout = Vec::new();
    let mut preflight_stderr = Vec::new();
    let preflight_code = run_cli(
        &[
            "verify",
            "preflight",
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut preflight_stdout,
        &mut preflight_stderr,
    );
    let mut setup_preflight_stdout = Vec::new();
    let mut setup_preflight_stderr = Vec::new();
    let setup_preflight_code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut setup_preflight_stdout,
        &mut setup_preflight_stderr,
    );
    let mut mismatch_stdout = Vec::new();
    let mut mismatch_stderr = Vec::new();
    let mismatch_code = run_cli(
        &[
            "verify",
            "proof",
            "--eth-block-input",
            other_block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut mismatch_stdout,
        &mut mismatch_stderr,
    );
    let mut public_mismatch_stdout = Vec::new();
    let mut public_mismatch_stderr = Vec::new();
    let public_mismatch_code = run_cli(
        &[
            "verify",
            "proof",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            mismatched_proof_path
                .to_str()
                .expect("proof path should be utf-8"),
            mismatched_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut public_mismatch_stdout,
        &mut public_mismatch_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(
        public_values_prove_code,
        1,
        "{}",
        String::from_utf8_lossy(&public_values_prove_stderr)
    );
    assert_eq!(
        String::from_utf8(public_values_prove_stderr)
            .expect("public values prove stderr should be utf-8"),
        "prove witness failed: ETH block public value mismatch: eth_block_hash_u32_be\n"
    );
    assert!(public_values_prove_stdout.is_empty());
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    let verify_stdout_text =
        String::from_utf8(verify_stdout).expect("verify stdout should be utf-8");
    assert!(verify_stdout_text.contains(&format!(
        "eth_block_inputs=1\neth_block_input_hash={}\neth_block_input_match=ok\neth_block_input_bytes={}\neth_block_rlp_bytes={}\neth_block_hash={}\neth_parent_hash={}\neth_ommers_hash={}\neth_beneficiary={}\neth_state_root={}\neth_receipts_root={}\neth_logs_bloom={}\neth_difficulty={}\neth_block_number={}\neth_block_timestamp={}\neth_extra_data={}\neth_gas_limit={}\neth_gas_used={}\neth_base_fee_per_gas={}\neth_mix_hash={}\neth_nonce={}\neth_transactions_root={}\neth_transaction_trie_preimages={}\neth_transaction_count=1\neth_legacy_transactions=1\neth_typed_transactions=0\neth_receipts=present\neth_receipts_rlp_bytes={}\neth_receipt_trie_preimages={}\neth_receipt_count=1\neth_legacy_receipts=1\neth_typed_receipts=0\n",
        format_hash(&block_input_hash),
        block_input_bytes.len(),
        block_input.block_rlp.len(),
        format_hash(&block_input.block_hash),
        format_hash(&block_input.parent_hash),
        format_hash(&block_input.ommers_hash),
        format_hex(&block_input.beneficiary),
        format_hash(&block_input.state_root),
        format_hash(&block_input.receipts_root),
        format_hex(&block_input.logs_bloom),
        format_u256(&block_input.difficulty),
        block_input.block_number,
        block_input.timestamp,
        format_hex(&block_input.extra_data),
        block_input.gas_limit,
        block_input.gas_used,
        format_optional_u256(block_input.base_fee_per_gas.as_ref()),
        format_hash(&block_input.mix_hash),
        format_hex(&block_input.nonce),
        format_hash(&block_input.transactions_root),
        block_input.transactions.hash_preimages.len(),
        receipts_rlp.len(),
        receipt_build.hash_preimages.len()
    )));
    assert_eq!(
        preflight_code,
        0,
        "{}",
        String::from_utf8_lossy(&preflight_stderr)
    );
    assert!(preflight_stderr.is_empty());
    let preflight_stdout_text =
        String::from_utf8(preflight_stdout).expect("preflight stdout should be utf-8");
    assert!(preflight_stdout_text.contains(&format!(
        "eth_block_inputs=1\neth_block_input_hash={}\neth_block_input_bytes={}\neth_block_rlp_bytes={}\neth_block_hash={}\neth_parent_hash={}\neth_ommers_hash={}\neth_beneficiary={}\neth_state_root={}\neth_receipts_root={}\neth_logs_bloom={}\neth_difficulty={}\neth_block_number={}\neth_block_timestamp={}\neth_extra_data={}\neth_gas_limit={}\neth_gas_used={}\neth_base_fee_per_gas={}\neth_mix_hash={}\neth_nonce={}\neth_transactions_root={}\neth_transaction_trie_preimages={}\neth_transaction_count=1\neth_legacy_transactions=1\neth_typed_transactions=0\neth_receipts=present\neth_receipts_rlp_bytes={}\neth_receipt_trie_preimages={}\neth_receipt_count=1\neth_legacy_receipts=1\neth_typed_receipts=0\n",
        format_hash(&block_input_hash),
        block_input_bytes.len(),
        block_input.block_rlp.len(),
        format_hash(&block_input.block_hash),
        format_hash(&block_input.parent_hash),
        format_hash(&block_input.ommers_hash),
        format_hex(&block_input.beneficiary),
        format_hash(&block_input.state_root),
        format_hash(&block_input.receipts_root),
        format_hex(&block_input.logs_bloom),
        format_u256(&block_input.difficulty),
        block_input.block_number,
        block_input.timestamp,
        format_hex(&block_input.extra_data),
        block_input.gas_limit,
        block_input.gas_used,
        format_optional_u256(block_input.base_fee_per_gas.as_ref()),
        format_hash(&block_input.mix_hash),
        format_hex(&block_input.nonce),
        format_hash(&block_input.transactions_root),
        block_input.transactions.hash_preimages.len(),
        receipts_rlp.len(),
        receipt_build.hash_preimages.len()
    )));
    assert_eq!(
        setup_preflight_code,
        0,
        "{}",
        String::from_utf8_lossy(&setup_preflight_stderr)
    );
    assert!(setup_preflight_stderr.is_empty());
    let setup_preflight_stdout_text =
        String::from_utf8(setup_preflight_stdout).expect("setup preflight stdout should be utf-8");
    assert!(setup_preflight_stdout_text.contains(&format!(
        "eth_block_inputs=1\neth_block_input_hash={}\neth_block_input_bytes={}\neth_block_rlp_bytes={}\neth_block_hash={}\neth_parent_hash={}\neth_ommers_hash={}\neth_beneficiary={}\neth_state_root={}\neth_receipts_root={}\neth_logs_bloom={}\neth_difficulty={}\neth_block_number={}\neth_block_timestamp={}\neth_extra_data={}\neth_gas_limit={}\neth_gas_used={}\neth_base_fee_per_gas={}\neth_mix_hash={}\neth_nonce={}\neth_transactions_root={}\neth_transaction_trie_preimages={}\neth_transaction_count=1\neth_legacy_transactions=1\neth_typed_transactions=0\neth_receipts=present\neth_receipts_rlp_bytes={}\neth_receipt_trie_preimages={}\neth_receipt_count=1\neth_legacy_receipts=1\neth_typed_receipts=0\n",
        format_hash(&block_input_hash),
        block_input_bytes.len(),
        block_input.block_rlp.len(),
        format_hash(&block_input.block_hash),
        format_hash(&block_input.parent_hash),
        format_hash(&block_input.ommers_hash),
        format_hex(&block_input.beneficiary),
        format_hash(&block_input.state_root),
        format_hash(&block_input.receipts_root),
        format_hex(&block_input.logs_bloom),
        format_u256(&block_input.difficulty),
        block_input.block_number,
        block_input.timestamp,
        format_hex(&block_input.extra_data),
        block_input.gas_limit,
        block_input.gas_used,
        format_optional_u256(block_input.base_fee_per_gas.as_ref()),
        format_hash(&block_input.mix_hash),
        format_hex(&block_input.nonce),
        format_hash(&block_input.transactions_root),
        block_input.transactions.hash_preimages.len(),
        receipts_rlp.len(),
        receipt_build.hash_preimages.len()
    )));
    assert_eq!(mismatch_code, 1);
    assert_eq!(
        String::from_utf8(mismatch_stderr).expect("mismatch stderr should be utf-8"),
        "verify proof failed: ETH block input proof segment mismatch\n"
    );
    assert!(mismatch_stdout.is_empty());
    assert_eq!(public_mismatch_code, 1);
    assert_eq!(
        String::from_utf8(public_mismatch_stderr).expect("public mismatch stderr should be utf-8"),
        "verify proof failed: ETH block public value mismatch: eth_block_hash_u32_be\n"
    );
    assert!(public_mismatch_stdout.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains(&format!("public_inputs={}\n", public_values_path.display())));
    assert!(stdout_text.contains("public_input_values=21\npublic_input_fields=170\n"));
    assert_eq!(parsed_input.block_rlp, block_rlp);
    assert_eq!(parsed_input.block_hash, block_input.block_hash);
    assert_eq!(parsed_input.transactions.hash_preimages.len(), 1);
    assert_eq!(
        parsed_input
            .receipts
            .as_ref()
            .expect("receipts should be present")
            .hash_preimages
            .len(),
        receipt_build.hash_preimages.len()
    );
    assert_eq!(
        parse_eth_block_input(&block_input_bytes)
            .expect("block input should parse")
            .block_hash,
        parsed_input.block_hash
    );
}

#[test]
fn prove_witness_uses_eth_block_input_as_default_witness_input() {
    let dir = temp_dir("prove-witness-eth-default-input");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let block_input_path = dir.join("block.input");
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&block_input_path, &encoded_block_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains(&format!("input_data={}\n", block_input_path.display())));
    assert!(stdout_text.contains(&format!("input_bytes={}\n", encoded_block_input.len())));
}

#[test]
fn prove_witness_generates_eth_block_public_values_when_missing() {
    let dir = temp_dir("prove-witness-eth-public-values");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let block_input_path = dir.join("block.input");
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let proof_path = output_dir.join("proof.bin");
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    let block_input_hash = eth_block_input_bytes_digest(&encoded_block_input);
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&block_input_path, &encoded_block_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let generated_bytes =
        fs::read(&generated_public_values_path).expect("generated public values should read");
    let generated_public_values =
        parse_public_values(&generated_bytes).expect("generated public values should parse");
    assert_eq!(
        generated_public_values,
        public_values_from_eth_block_input(setup_hash, &block_input)
    );
    let proof_bytes = fs::read(&proof_path).expect("proof should be written");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof should parse");
    assert_eq!(
        proof.public_values_hash,
        public_values_digest(&generated_public_values).expect("digest should compute")
    );
    let segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
        .expect("ETH block input segment should be present");
    let parsed_input =
        parse_eth_block_input_segment(&segment.data).expect("block input segment should parse");
    assert_eq!(parsed_input.block_hash, block_input.block_hash);
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains(&format!(
        "public_inputs={}\n",
        generated_public_values_path.display()
    )));
    assert!(stdout_text.contains(&format!(
        "eth_block_input_hash={}\n",
        format_hash(&block_input_hash)
    )));
    assert!(stdout_text.contains("public_input_values=21\npublic_input_fields=170\n"));

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            generated_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    assert!(String::from_utf8(verify_stdout)
        .expect("verify stdout should be utf-8")
        .contains("eth_block_input_match=ok\n"));
}

#[test]
fn prove_witness_uses_eth_public_input_as_default_witness_input() {
    let dir = temp_dir("prove-witness-eth-public-default-input");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let public_input_path = dir.join("public.bin");
    let generated_block_input_path = output_dir.join("eth-block.input");
    let public_input = sample_public_block_bytes_with_matching_roots();
    let public_block = parse_eth_public_block_prefix(&public_input).expect("block should parse");
    let block_input = build_eth_block_input(&public_block.block_rlp()).expect("input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&public_input_path, &public_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-public-input",
            public_input_path
                .to_str()
                .expect("public input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains(&format!(
        "input_data={}\n",
        generated_block_input_path.display()
    )));
    assert!(stdout_text.contains(&format!("input_bytes={}\n", encoded_block_input.len())));
    assert!(stdout_text.contains("eth_block_input_generated=eth_public_input\n"));
}

#[test]
fn proves_and_verifies_eth_public_input_directly() {
    let dir = temp_dir("prove-verify-eth-public-input");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let public_input_path = dir.join("public.bin");
    let generated_block_input_path = output_dir.join("eth-block.input");
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let proof_path = output_dir.join("proof.bin");
    let public_input = sample_public_block_bytes_with_matching_roots();
    let public_block = parse_eth_public_block_prefix(&public_input).expect("block should parse");
    let block_input = build_eth_block_input(&public_block.block_rlp()).expect("input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    let block_input_hash = eth_block_input_bytes_digest(&encoded_block_input);
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&public_input_path, &public_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-public-input",
            public_input_path
                .to_str()
                .expect("public input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let generated_block_input_bytes =
        fs::read(&generated_block_input_path).expect("generated block input should read");
    let generated_block_input =
        parse_eth_block_input(&generated_block_input_bytes).expect("block input should parse");
    assert_eq!(generated_block_input, block_input);
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains(&format!(
        "eth_public_input={}\n",
        public_input_path.display()
    )));
    assert!(stdout_text.contains("eth_block_input_generated=eth_public_input\n"));
    assert!(stdout_text.contains("public_inputs_generated=eth_block_input\n"));
    assert!(stdout_text.contains(&format!(
        "eth_block_input={}\n",
        generated_block_input_path.display()
    )));
    assert!(stdout_text.contains(&format!(
        "eth_block_input_hash={}\n",
        format_hash(&block_input_hash)
    )));

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--eth-public-input",
            public_input_path
                .to_str()
                .expect("public input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            generated_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    assert!(String::from_utf8(verify_stdout)
        .expect("verify stdout should be utf-8")
        .contains("eth_block_input_match=ok\n"));
}

#[test]
fn setup_preflight_verifies_eth_public_input_directly() {
    let dir = temp_dir("setup-preflight-eth-public-input");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let public_input_path = dir.join("public.bin");
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let proof_path = output_dir.join("proof.bin");
    let public_input = sample_public_block_bytes_with_matching_roots();
    let public_block = parse_eth_public_block_prefix(&public_input).expect("block should parse");
    let block_input = build_eth_block_input(&public_block.block_rlp()).expect("input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&public_input_path, &public_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-public-input",
            public_input_path
                .to_str()
                .expect("public input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "setup-preflight",
            "--eth-public-input",
            public_input_path
                .to_str()
                .expect("public input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            generated_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    assert!(String::from_utf8(verify_stdout)
        .expect("verify stdout should be utf-8")
        .contains("eth_block_input_match=ok\n"));
}

#[test]
fn proves_and_verifies_allowed_trailing_eth_public_input_directly() {
    let dir = temp_dir("prove-verify-eth-public-input-allow-trailing");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let public_input_path = dir.join("public.bin");
    let generated_block_input_path = output_dir.join("eth-block.input");
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let proof_path = output_dir.join("proof.bin");
    let public_input = sample_public_block_bytes_with_matching_roots();
    let public_block = parse_eth_public_block_prefix(&public_input).expect("block should parse");
    let block_input = build_eth_block_input(&public_block.block_rlp()).expect("input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let mut public_input_with_tail = public_input;
    public_input_with_tail.extend_from_slice(b"tail");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&public_input_path, public_input_with_tail);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-public-input",
            public_input_path
                .to_str()
                .expect("public input path should be utf-8"),
            "--eth-public-input-allow-trailing",
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let generated_block_input_bytes =
        fs::read(&generated_block_input_path).expect("generated block input should read");
    let generated_block_input =
        parse_eth_block_input(&generated_block_input_bytes).expect("block input should parse");
    assert_eq!(generated_block_input, block_input);
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains("eth_block_input_generated=eth_public_input\n"));
    assert!(stdout_text.contains("public_inputs_generated=eth_block_input\n"));

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--eth-public-input",
            public_input_path
                .to_str()
                .expect("public input path should be utf-8"),
            "--eth-public-input-allow-trailing",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            generated_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    assert!(String::from_utf8(verify_stdout)
        .expect("verify stdout should be utf-8")
        .contains("eth_block_input_match=ok\n"));
}

#[test]
fn verify_proof_binding_reports_eth_block_withdrawal_count() {
    let dir = temp_dir("verify-proof-eth-withdrawal-count");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let block_input_path = dir.join("block.input");
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let proof_path = output_dir.join("proof.bin");
    let withdrawal_item = sample_withdrawal_item();
    let withdrawals = vec![parse_rlp(&withdrawal_item).expect("withdrawal should parse")];
    let withdrawal_build = withdrawals_trie_build(&withdrawals);
    let block_rlp = sample_block_rlp_with_withdrawals(withdrawal_build.root, vec![withdrawal_item]);
    let block_input = build_eth_block_input(&block_rlp).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&block_input_path, &encoded_block_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            generated_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    let verify_stdout_text =
        String::from_utf8(verify_stdout).expect("verify stdout should be utf-8");
    assert!(verify_stdout_text.contains("eth_block_input_match=ok\n"));
    assert!(verify_stdout_text.contains(&format!(
        "eth_withdrawals=present\neth_withdrawals_root={}\neth_withdrawal_count=1\n",
        format_hash(&withdrawal_build.root)
    )));
    assert!(verify_stdout_text.contains(&format!(
        "eth_withdrawal_trie_preimages={}\n",
        withdrawal_build.hash_preimages.len()
    )));
}

#[test]
fn verify_proof_binding_reports_eth_block_extra_field_counts() {
    let dir = temp_dir("verify-proof-eth-extra-field-counts");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let block_input_path = dir.join("block.input");
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let proof_path = output_dir.join("proof.bin");
    let block_rlp = sample_block_rlp_with_extra_fields();
    let block_input = build_eth_block_input(&block_rlp).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&block_input_path, &encoded_block_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            generated_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    let verify_stdout_text =
        String::from_utf8(verify_stdout).expect("verify stdout should be utf-8");
    assert!(verify_stdout_text.contains("eth_block_input_match=ok\n"));
    assert!(verify_stdout_text.contains("eth_extra_header_fields=1\neth_extra_body_fields=1\n"));
}

#[test]
fn verify_proof_reports_eth_block_extra_field_counts_from_proof_segment() {
    let dir = temp_dir("verify-proof-segment-eth-extra-field-counts");
    let _ = fs::remove_dir_all(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let block_input_path = dir.join("block.input");
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let proof_path = output_dir.join("proof.bin");
    let block_rlp = sample_block_rlp_with_extra_fields();
    let block_input = build_eth_block_input(&block_rlp).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&block_input_path, &encoded_block_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            generated_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    let verify_stdout_text =
        String::from_utf8(verify_stdout).expect("verify stdout should be utf-8");
    assert!(verify_stdout_text.contains("eth_extra_header_fields=1\neth_extra_body_fields=1\n"));
}

#[test]
fn prove_witness_all_units_round_trips_generated_eth_block_public_values_and_program_image_cache() {
    let dir = temp_dir("prove-witness-all-units-eth-public-values");
    let _ = fs::remove_dir_all(&dir);
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let bundle_path = dir.join("trace-bundle.bin");
    let block_input_path = dir.join("block.input");
    let program_path = dir.join("program.bin");
    let constraint_digest_path = dir.join("constraint.digest");
    let root_path = dir.join("root.bin");
    let cache_path = dir.join("program_image.cache");
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    let block_input_hash = eth_block_input_bytes_digest(&encoded_block_input);
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&bundle_path, sample_trace_bundle_bytes(4, 17));
    write_bytes(&program_path, b"packed-program");
    write_bytes(&constraint_digest_path, setup_hash);
    write_bytes(
        &root_path,
        encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![11, 12, 13, 14]))
            .expect("root should encode"),
    );
    write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
        program_path: &program_path,
        guest_image_path: &guest_image,
        constraint_digest_path: &constraint_digest_path,
        root_path: &root_path,
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
        output_path: &cache_path,
    })
    .expect("cache should write");
    let expected_cache =
        read_program_image_commitment_cache_file(&cache_path).expect("cache should read");
    let expected_cache_segment =
        encode_program_image_cache_segment(&expected_cache).expect("cache segment should encode");
    let expected_cache_segment_hash = program_image_cache_segment_digest(&expected_cache_segment);
    write_bytes(&block_input_path, &encoded_block_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--all-units",
            "--trace-bundle",
            bundle_path.to_str().expect("bundle path should be utf-8"),
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let generated_bytes =
        fs::read(&generated_public_values_path).expect("generated public values should read");
    let generated_public_values =
        parse_public_values(&generated_bytes).expect("generated public values should parse");
    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof should parse");
    let program_image_cache_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PROGRAM_IMAGE_CACHE_SEGMENT_ID)
        .expect("program image cache segment should be present");
    let parsed_cache = parse_program_image_cache_segment(&program_image_cache_segment.data)
        .expect("cache segment should parse");
    let eth_block_input_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
        .expect("ETH block input segment should be present");
    let parsed_block_input = parse_eth_block_input_segment(&eth_block_input_segment.data)
        .expect("block input segment should parse");
    assert_eq!(
        generated_public_values,
        public_values_from_eth_block_input(setup_hash, &block_input)
    );
    assert_eq!(parsed_cache, expected_cache);
    assert_eq!(parsed_block_input, block_input);
    assert_eq!(
        proof.public_values_hash,
        public_values_digest(&generated_public_values).expect("digest should compute")
    );
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains(&format!(
        "public_inputs={}\n",
        generated_public_values_path.display()
    )));
    assert!(stdout_text.contains("public_inputs_generated=eth_block_input\n"));
    assert!(stdout_text.contains("program_image_cache_gpu_mode=cuda\n"));
    assert!(stdout_text.contains("eth_block_input="));
    assert!(stdout_text.contains(&format!(
        "eth_block_input_hash={}\n",
        format_hash(&block_input_hash)
    )));
    assert!(stdout_text.contains(&format!(
        "program_image_cache_segment_hash={}\n",
        format_hash(&expected_cache_segment_hash)
    )));
    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            generated_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    let verify_stdout_text =
        String::from_utf8(verify_stdout).expect("verify stdout should be utf-8");
    assert!(verify_stdout_text.contains("program_image_caches=1\n"));
    assert!(verify_stdout_text.contains("eth_block_inputs=1\n"));
    assert!(verify_stdout_text.contains("program_image_cache_match=ok\n"));
    assert!(verify_stdout_text.contains("eth_block_input_match=ok\n"));
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn proves_all_units_and_verifies_eth_public_input_directly() {
    let dir = temp_dir("prove-witness-all-units-eth-public-input");
    let _ = fs::remove_dir_all(&dir);
    let public_input = sample_public_block_bytes_with_matching_roots();
    let public_block = parse_eth_public_block_prefix(&public_input).expect("block should parse");
    let block_input = build_eth_block_input(&public_block.block_rlp()).expect("input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let bundle_path = dir.join("trace-bundle.bin");
    let public_input_path = dir.join("public.bin");
    let generated_block_input_path = output_dir.join("eth-block.input");
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    let block_input_hash = eth_block_input_bytes_digest(&encoded_block_input);
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&bundle_path, sample_trace_bundle_bytes(4, 29));
    write_bytes(&public_input_path, &public_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--all-units",
            "--trace-bundle",
            bundle_path.to_str().expect("bundle path should be utf-8"),
            "--eth-public-input",
            public_input_path
                .to_str()
                .expect("public input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let generated_block_input_bytes =
        fs::read(&generated_block_input_path).expect("generated block input should read");
    let generated_block_input =
        parse_eth_block_input(&generated_block_input_bytes).expect("block input should parse");
    let generated_public_values_bytes =
        fs::read(&generated_public_values_path).expect("public values should read");
    let generated_public_values =
        parse_public_values(&generated_public_values_bytes).expect("public values should parse");
    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof should parse");
    let block_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
        .expect("ETH block input segment should be present");
    let parsed_block_input =
        parse_eth_block_input_segment(&block_segment.data).expect("block input should parse");
    assert_eq!(generated_block_input, block_input);
    assert_eq!(parsed_block_input, block_input);
    assert_eq!(
        generated_public_values,
        public_values_from_eth_block_input(setup_hash, &block_input)
    );
    assert_eq!(
        proof.public_values_hash,
        public_values_digest(&generated_public_values).expect("digest should compute")
    );
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains(&format!(
        "public_inputs={}\n",
        generated_public_values_path.display()
    )));
    assert!(stdout_text.contains("public_inputs_generated=eth_block_input\n"));
    assert!(stdout_text.contains(&format!(
        "eth_public_input={}\n",
        public_input_path.display()
    )));
    assert!(stdout_text.contains("eth_block_input_generated=eth_public_input\n"));
    assert!(stdout_text.contains(&format!(
        "eth_block_input={}\n",
        generated_block_input_path.display()
    )));
    assert!(stdout_text.contains(&format!(
        "eth_block_input_hash={}\n",
        format_hash(&block_input_hash)
    )));

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--eth-public-input",
            public_input_path
                .to_str()
                .expect("public input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            generated_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    let verify_stdout_text =
        String::from_utf8(verify_stdout).expect("verify stdout should be utf-8");
    assert!(verify_stdout_text.contains("eth_block_inputs=1\n"));
    assert!(verify_stdout_text.contains("eth_block_input_match=ok\n"));
}

#[test]
fn prove_witness_final_wrap_round_trips_generated_eth_block_public_values() {
    let dir = temp_dir("prove-witness-final-wrap-eth-public-values");
    let _ = fs::remove_dir_all(&dir);
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let bundle_path = dir.join("trace-bundle.bin");
    let block_input_path = dir.join("block.input");
    let generated_public_values_path = output_dir.join("eth-block-public-values.bin");
    let encoded_block_input =
        encode_eth_block_input(&block_input).expect("block input should encode");
    let block_input_hash = eth_block_input_bytes_digest(&encoded_block_input);
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);
    write_bytes(&bundle_path, sample_trace_bundle_bytes(4, 23));
    write_bytes(&block_input_path, &encoded_block_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--aggregate",
            "--final-wrap",
            "--trace-bundle",
            bundle_path.to_str().expect("bundle path should be utf-8"),
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let generated_bytes =
        fs::read(&generated_public_values_path).expect("generated public values should read");
    let generated_public_values =
        parse_public_values(&generated_bytes).expect("generated public values should parse");
    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof should parse");
    let block_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
        .expect("ETH block input segment should be present");
    let parsed_block_input =
        parse_eth_block_input_segment(&block_segment.data).expect("block input should parse");
    assert_eq!(
        generated_public_values,
        public_values_from_eth_block_input(setup_hash, &block_input)
    );
    assert_eq!(parsed_block_input, block_input);
    assert_eq!(
        proof.public_values_hash,
        public_values_digest(&generated_public_values).expect("digest should compute")
    );
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains("aggregate=true\n"));
    assert!(stdout_text.contains("final_wrap=true\n"));
    assert!(stdout_text.contains("public_inputs_generated=eth_block_input\n"));
    assert!(stdout_text.contains(&format!(
        "eth_block_input_hash={}\n",
        format_hash(&block_input_hash)
    )));
    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "proof",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            generated_public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert!(verify_stderr.is_empty());
    let verify_stdout_text =
        String::from_utf8(verify_stdout).expect("verify stdout should be utf-8");
    assert!(verify_stdout_text.contains("eth_block_inputs=1\n"));
    assert!(verify_stdout_text.contains("eth_block_input_match=ok\n"));
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_rejects_mismatched_eth_block_public_values_before_witness_loading() {
    let dir = temp_dir("prove-witness-eth-public-values-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("missing-witness.so");
    let guest_image = dir.join("guest.elf");
    let block_input_path = dir.join("block.input");
    let public_values_path = dir.join("mismatched-public-values.bin");
    let other_block_input =
        build_eth_block_input(&sample_block_rlp_variant()).expect("block input should build");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(
        &block_input_path,
        encode_eth_block_input(&block_input).expect("block input should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values_from_eth_block_input(
            setup_hash,
            &other_block_input,
        ))
        .expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove witness failed: ETH block public value mismatch: eth_block_hash_u32_be\n"
    );
}

#[test]
fn prove_witness_rejects_mismatched_program_image_cache_public_values_before_witness_loading() {
    let dir = temp_dir("prove-witness-program-image-public-values-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_execution_ready_setup_directory_with_public_values(
        &dir,
        &eth_block_public_values_with_rom_root([0; 32], &block_input, [0, 0, 0, 0]),
    );
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("missing-witness.so");
    let guest_image = dir.join("guest.elf");
    let block_input_path = dir.join("block.input");
    let public_values_path = dir.join("mismatched-public-values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(
        &block_input_path,
        encode_eth_block_input(&block_input).expect("block input should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&eth_block_public_values_with_rom_root(
            setup_hash,
            &block_input,
            [99, 98, 97, 96],
        ))
        .expect("public values should encode"),
    );
    let cache_path =
        write_sample_program_image_cache(&dir, &guest_image, setup_hash, [11, 12, 13, 14]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--no-verify-outputs",
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove witness failed: program image cache tree root does not match public value: rom_root\n"
    );
}

#[test]
fn prove_witness_rejects_eth_block_public_values_with_wrong_setup_hash_before_witness_loading() {
    let dir = temp_dir("prove-witness-eth-public-values-setup-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("missing-witness.so");
    let guest_image = dir.join("guest.elf");
    let block_input_path = dir.join("block.input");
    let public_values_path = dir.join("wrong-setup-public-values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(
        &block_input_path,
        encode_eth_block_input(&block_input).expect("block input should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values_from_eth_block_input(
            [0x99; 32],
            &block_input,
        ))
        .expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove witness failed: public inputs setup hash mismatch\n"
    );
}

#[test]
fn writes_eth_block_public_values_from_setup_directory() {
    let dir = temp_dir("eth-block-public-values-setup-dir");
    let _ = fs::remove_dir_all(&dir);
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_execution_ready_setup_directory_with_eth_block_public_values(&dir, &block_input);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let setup_hash_hex = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let block_input_path = dir.join("block.input");
    let public_values_path = dir.join("eth-public-values.bin");
    let block_input_bytes =
        encode_eth_block_input(&block_input).expect("block input should encode");
    write_bytes(&block_input_path, &block_input_bytes);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-block-public-values",
            "--setup-dir",
            dir.to_str().expect("setup path should be utf-8"),
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let encoded = fs::read(&public_values_path).expect("public values should read");
    let parsed = parse_public_values(&encoded).expect("public values should parse");
    let public_values_hash =
        public_values_digest(&parsed).expect("public values digest should compute");
    assert_eq!(parsed.setup_hash, setup_hash);
    assert_eq!(
        parsed,
        public_values_from_eth_block_input(setup_hash, &block_input)
    );
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npublic_values={}\nbytes={}\nblock_input={}\nblock_input_bytes={}\nblock_input_hash={}\nblock_rlp_bytes={}\nsetup_hash={}\npublic_values_hash={}\nvalues=21\npublic_value_fields=170\nblock_hash={}\nparent_hash={}\nommers_hash={}\nbeneficiary={}\nstate_root={}\nreceipts_root={}\nlogs_bloom={}\ndifficulty=01\nblock_number=2\ntimestamp=101\nextra_data=6c7a766d\ngas_limit=1000000\ngas_used=900000\nbase_fee_per_gas=absent\nmix_hash={}\nnonce={}\ntransactions_root={}\ntransaction_trie_preimages=1\ntransaction_count=1\nlegacy_transactions=1\ntyped_transactions=0\nreceipts=absent\nwithdrawals=absent\n",
            public_values_path.display(),
            encoded.len(),
            block_input_path.display(),
            block_input_bytes.len(),
            format_hash(&eth_block_input_bytes_digest(&block_input_bytes)),
            block_input.block_rlp.len(),
            setup_hash_hex,
            format_hash(&public_values_hash),
            format_hash(&block_input.block_hash),
            format_hash(&block_input.parent_hash),
            format_hash(&block_input.ommers_hash),
            format_hex(&block_input.beneficiary),
            format_hash(&block_input.state_root),
            format_hash(&block_input.receipts_root),
            format_hex(&block_input.logs_bloom),
            format_hash(&block_input.mix_hash),
            format_hex(&block_input.nonce),
            format_hash(&block_input.transactions_root)
        )
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn writes_packed_eth_block_public_values_from_setup_directory_and_program_image_cache() {
    let dir = temp_dir("eth-block-public-values-packed-setup-dir");
    let _ = fs::remove_dir_all(&dir);
    let source_path = dir.join("source").join("main.pil");
    write_bytes(
        &source_path,
        "public rom_root[4];\n\
         public inputs[64];\n\
         airtemplate UnitA() {\n\
             col witness values[2];\n\
         }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];",
    );
    let mut generate_stdout = Vec::new();
    let mut generate_stderr = Vec::new();
    let generate_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut generate_stdout,
        &mut generate_stderr,
    );
    assert_eq!(
        generate_code,
        0,
        "{}",
        String::from_utf8_lossy(&generate_stderr)
    );
    assert!(generate_stderr.is_empty());

    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let setup_hash_hex = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    let block_input_bytes =
        encode_eth_block_input(&block_input).expect("block input should encode");
    let block_input_path = dir.join("block.input");
    let public_values_path = dir.join("eth-public-values.bin");
    let guest_image = dir.join("guest.elf");
    let program_path = dir.join("program.bin");
    let constraint_digest_path = dir.join("constraint.digest");
    let root_path = dir.join("root.bin");
    let cache_path = dir.join("program_image.cache");
    write_bytes(&block_input_path, &block_input_bytes);
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&program_path, b"source-generated-program");
    write_bytes(&constraint_digest_path, setup_hash);
    write_bytes(
        &root_path,
        encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![11, 12, 13, 14]))
            .expect("root should encode"),
    );
    write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
        program_path: &program_path,
        guest_image_path: &guest_image,
        constraint_digest_path: &constraint_digest_path,
        root_path: &root_path,
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
        output_path: &cache_path,
    })
    .expect("cache should write");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-block-public-values",
            "--setup-dir",
            dir.to_str().expect("setup path should be utf-8"),
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let encoded = fs::read(&public_values_path).expect("public values should read");
    let parsed = parse_public_values(&encoded).expect("public values should parse");
    assert_eq!(parsed.setup_hash, setup_hash);
    assert_eq!(parsed.values.len(), 2);
    assert_eq!(parsed.values[0].name, "rom_root");
    assert_eq!(parsed.values[0].elements, vec![11, 12, 13, 14]);
    assert_eq!(parsed.values[1].name, "inputs");
    let mut expected_inputs = block_input
        .block_hash
        .chunks_exact(4)
        .map(|chunk| {
            u64::from(u32::from_le_bytes(
                chunk.try_into().expect("chunk is 4 bytes"),
            ))
        })
        .collect::<Vec<_>>();
    expected_inputs.resize(64, 0);
    assert_eq!(parsed.values[1].elements, expected_inputs);
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains(&format!("setup_hash={setup_hash_hex}\n")));
    assert!(stdout_text.contains("values=2\n"));
    assert!(stdout_text.contains("public_value_fields=68\n"));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_eth_block_public_values_from_mismatched_setup_metadata() {
    let dir = temp_dir("eth-block-public-values-metadata-mismatch");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let block_input_path = dir.join("block.input");
    let public_values_path = dir.join("eth-public-values.bin");
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    write_bytes(
        &block_input_path,
        encode_eth_block_input(&block_input).expect("block input should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-block-public-values",
            "--setup-dir",
            dir.to_str().expect("setup path should be utf-8"),
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "eth block public values failed: unsupported ETH block public metadata: block_number\n"
    );
}

#[test]
fn builds_witness_proof_core_for_multiple_units() {
    let dir = temp_dir("prove-witness-core-multi");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);

    let public_values = sample_public_values(setup_hash);
    let public_values_hash = public_values_digest(&public_values).expect("hash should compute");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan {
            input_data: Some(input_data),
            partition_count: 1,
            partition_ids: vec![0],
            worker_index: 0,
        }),
        options: ProveRunOptions::default_for_output(output_dir),
        gpu: GpuRunOptions::default(),
    };
    let plan = derive_prove_execution_plan(
        &catalog,
        request,
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: Some(public_values_path),
        },
    )
    .expect("execution plan should derive");
    let first = run_prove_witness_commitments(&plan, 0).expect("first unit should run");
    let second = run_prove_witness_commitments(&plan, 1).expect("second unit should run");

    let proof = build_witness_proof_core_artifact(
        &catalog,
        &plan.run_plan.schedule,
        public_values_hash,
        &[&first, &second],
    )
    .expect("core proof artifact should build");
    let encoded = encode_proof_artifact(&proof).expect("proof should encode");
    let proof = parse_proof_artifact(&encoded).expect("proof should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let witness_ids = proof
        .segments
        .iter()
        .filter(|segment| segment.id < PCS_MATERIAL_MANIFEST_SEGMENT_ID)
        .map(|segment| segment.id)
        .collect::<Vec<_>>();
    assert_eq!(
        witness_ids,
        vec![
            WITNESS_COMMITMENT_SEGMENT_BASE_ID,
            WITNESS_COMMITMENT_SEGMENT_BASE_ID + 1
        ]
    );
    let query_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_QUERY_PLAN_SEGMENT_ID)
        .expect("query segment should be present");
    let query_plan =
        parse_pcs_query_plan_segment(&query_segment.data).expect("query segment should parse");
    assert_eq!(
        query_plan
            .units
            .iter()
            .map(|unit| unit.unit_index)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    let constant_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == CONSTANT_OPENING_SEGMENT_ID)
        .expect("constant opening segment should be present");
    let constant_opening = parse_constant_opening_segment(&constant_segment.data)
        .expect("constant opening segment should parse");
    assert_eq!(constant_opening.units.len(), 2);
    let witness_opening_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == WITNESS_OPENING_SEGMENT_ID)
        .expect("witness opening segment should be present");
    let witness_opening = parse_witness_opening_segment(&witness_opening_segment.data)
        .expect("witness opening segment should parse");
    assert_eq!(
        witness_opening
            .units
            .iter()
            .map(|unit| unit.unit_index)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
}

#[test]
fn builds_witness_proof_artifact_for_multiple_units_with_unit_values() {
    let dir = temp_dir("prove-witness-artifact-multi-values");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_unit_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [7_u8]);

    let public_values = sample_public_values(setup_hash);
    let public_values_hash = public_values_digest(&public_values).expect("hash should compute");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan {
            input_data: Some(input_data),
            partition_count: 1,
            partition_ids: vec![0],
            worker_index: 0,
        }),
        options: ProveRunOptions::default_for_output(output_dir),
        gpu: GpuRunOptions::default(),
    };
    let plan = derive_prove_execution_plan(
        &catalog,
        request,
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: Some(public_values_path),
        },
    )
    .expect("execution plan should derive");
    let first = run_prove_witness_commitments(&plan, 0).expect("first unit should run");
    let second = run_prove_witness_commitments(&plan, 1).expect("second unit should run");

    let packed_unit_values = vec![
        Felt::from_u64(101),
        Felt::from_u64(201),
        Felt::from_u64(202),
        Felt::from_u64(203),
    ];
    let unit_values = vec![
        ProveUnitValues {
            unit_index: 0,
            unit_value_map: plan.run_plan.schedule.units[0].unit_value_map.clone(),
            packed_values: packed_unit_values.clone(),
        },
        ProveUnitValues {
            unit_index: 1,
            unit_value_map: plan.run_plan.schedule.units[1].unit_value_map.clone(),
            packed_values: packed_unit_values.clone(),
        },
    ];

    let proof = build_witness_proof_artifact(
        &catalog,
        &plan.run_plan.schedule,
        public_values_hash,
        &[&first, &second],
        &[],
        &[],
        &unit_values,
    )
    .expect("proof artifact should build");
    let encoded = encode_proof_artifact(&proof).expect("proof should encode");
    let proof = parse_proof_artifact(&encoded).expect("proof should parse");

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
    assert_eq!(unit_values.units[0].values, vec![101, 201, 202, 203]);
    assert_eq!(unit_values.units[1].values, vec![101, 201, 202, 203]);
    validate_setup_preflight(&catalog, &proof, &public_values)
        .expect("setup preflight should validate");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn builds_witness_proof_artifact_for_multiple_units_with_tail_segments() {
    let dir = temp_dir("prove-witness-artifact-multi-tail");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_proof_group_and_unit_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [9_u8]);

    let public_values = sample_public_values(setup_hash);
    let public_values_hash = public_values_digest(&public_values).expect("hash should compute");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let request = ProveRunRequest {
        pass: ProvePassRequest::Full(ProvePartitionPlan {
            input_data: Some(input_data),
            partition_count: 1,
            partition_ids: vec![0],
            worker_index: 0,
        }),
        options: ProveRunOptions::default_for_output(output_dir),
        gpu: GpuRunOptions::default(),
    };
    let plan = derive_prove_execution_plan(
        &catalog,
        request,
        ProveExecutionInputArtifacts {
            witness_library: Some(witness_library),
            guest_image,
            public_inputs: Some(public_values_path),
        },
    )
    .expect("execution plan should derive");
    let first = run_prove_witness_commitments(&plan, 0).expect("first unit should run");
    let second = run_prove_witness_commitments(&plan, 1).expect("second unit should run");

    let proof_values = vec![Felt::from_u64(51), Felt::from_u64(52), Felt::from_u64(53)];
    let group_values = vec![Ext3::from_u64s([61, 62, 63])];
    let packed_unit_values = vec![
        Felt::from_u64(101),
        Felt::from_u64(201),
        Felt::from_u64(202),
        Felt::from_u64(203),
    ];
    let unit_values = vec![
        ProveUnitValues {
            unit_index: 0,
            unit_value_map: plan.run_plan.schedule.units[0].unit_value_map.clone(),
            packed_values: packed_unit_values.clone(),
        },
        ProveUnitValues {
            unit_index: 1,
            unit_value_map: plan.run_plan.schedule.units[1].unit_value_map.clone(),
            packed_values: packed_unit_values.clone(),
        },
    ];

    let proof = build_witness_proof_artifact(
        &catalog,
        &plan.run_plan.schedule,
        public_values_hash,
        &[&first, &second],
        &proof_values,
        &group_values,
        &unit_values,
    )
    .expect("proof artifact should build");
    let encoded = encode_proof_artifact(&proof).expect("proof should encode");
    let proof = parse_proof_artifact(&encoded).expect("proof should parse");

    let proof_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_PROOF_VALUES_SEGMENT_ID)
        .expect("proof values segment should exist");
    let proof_values = parse_pcs_proof_values_segment(&proof_values_segment.data)
        .expect("proof values should parse");
    assert_eq!(proof_values.values, vec![[51, 52, 53]]);

    let group_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == GROUP_VALUES_SEGMENT_ID)
        .expect("group values segment should exist");
    let group_values =
        parse_group_values_segment(&group_values_segment.data).expect("group values should parse");
    assert_eq!(group_values.values, vec![[61, 62, 63]]);

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
    assert_eq!(unit_values.units[0].values, vec![101, 201, 202, 203]);
    assert_eq!(unit_values.units[1].values, vec![101, 201, 202, 203]);
    validate_setup_preflight(&catalog, &proof, &public_values)
        .expect("setup preflight should validate");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn runs_prove_witness_for_aggregate_when_requested() {
    let dir = temp_dir("prove-witness-all-units");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_proof_group_and_unit_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let unit_values_path = dir.join("unit_values.bin");
    let proof_values_path = dir.join("proof_values.bin");
    let group_values_path = dir.join("group_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [17_u8]);
    write_field_words(&unit_values_path, &[101, 201, 202, 203]);
    write_field_words(&proof_values_path, &[51, 52, 53]);
    write_field_words(&group_values_path, &[61, 62, 63]);

    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--aggregate",
            "--save-outputs",
            "--unit-values",
            unit_values_path
                .to_str()
                .expect("unit values path should be utf-8"),
            "--proof-values",
            proof_values_path
                .to_str()
                .expect("proof values path should be utf-8"),
            "--group-values",
            group_values_path
                .to_str()
                .expect("group values path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let proof_bytes = fs::read(output_dir.join("proof.bin")).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let witness_ids = proof
        .segments
        .iter()
        .filter(|segment| {
            segment.id >= WITNESS_COMMITMENT_SEGMENT_BASE_ID
                && segment.id < WITNESS_COMMITMENT_SEGMENT_BASE_ID + 2
        })
        .map(|segment| segment.id)
        .collect::<Vec<_>>();
    assert_eq!(
        witness_ids,
        vec![
            WITNESS_COMMITMENT_SEGMENT_BASE_ID,
            WITNESS_COMMITMENT_SEGMENT_BASE_ID + 1
        ]
    );
    assert!(output_dir.join("unit-0.witness-segment").exists());
    assert!(output_dir.join("unit-1.witness-segment").exists());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn runs_prove_witness_contributions_with_compact_artifact() {
    let dir = temp_dir("prove-witness-contributions-compact");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_proof_group_and_unit_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let unit_values_path = dir.join("unit_values.bin");
    let proof_values_path = dir.join("proof_values.bin");
    let group_values_path = dir.join("group_values.bin");
    let public_values_path = dir.join("public_values.bin");
    let challenge_values_path = output_dir.join("challenge_values_segment.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [17_u8]);
    write_field_words(&unit_values_path, &[101, 201, 202, 203]);
    write_field_words(&proof_values_path, &[51, 52, 53]);
    write_field_words(&group_values_path, &[61, 62, 63]);
    write_bytes(
        &public_values_path,
        encode_public_values(&sample_public_values(setup_hash))
            .expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--contributions",
            "--save-outputs",
            "--unit-values",
            unit_values_path
                .to_str()
                .expect("unit values path should be utf-8"),
            "--proof-values",
            proof_values_path
                .to_str()
                .expect("proof values path should be utf-8"),
            "--group-values",
            group_values_path
                .to_str()
                .expect("group values path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("pass=contributions\n"));
    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let segment_ids = proof
        .segments
        .iter()
        .map(|segment| segment.id)
        .collect::<Vec<_>>();
    assert!(segment_ids.contains(&CONTRIBUTION_SEGMENT_ID));
    assert!(segment_ids.contains(&PCS_PROOF_VALUES_SEGMENT_ID));
    assert!(!segment_ids.contains(&PCS_MATERIAL_MANIFEST_SEGMENT_ID));
    assert!(!segment_ids.contains(&PCS_QUERY_PLAN_SEGMENT_ID));
    assert!(!segment_ids.contains(&CONSTANT_OPENING_SEGMENT_ID));
    assert!(!segment_ids.contains(&WITNESS_OPENING_SEGMENT_ID));
    assert!(!segment_ids.iter().any(|segment_id| {
        *segment_id >= WITNESS_COMMITMENT_SEGMENT_BASE_ID
            && *segment_id < WITNESS_COMMITMENT_SEGMENT_BASE_ID + 16
    }));

    assert_external_contribution_challenge_verifies(
        &dir,
        &public_values_path,
        &proof_path,
        &challenge_values_path,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn prove_witness_contribution_challenges_preserve_bound_program_image_and_eth_block_input() {
    let dir = temp_dir("prove-witness-contribution-bound-inputs");
    let _ = fs::remove_dir_all(&dir);
    let block_input = build_eth_block_input(&sample_block_rlp_with_extra_fields())
        .expect("block input should build");
    write_execution_ready_setup_directory_with_public_values(
        &dir,
        &eth_block_public_values_with_rom_root([0; 32], &block_input, [11, 12, 13, 14]),
    );
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let block_input_path = dir.join("block.input");
    let public_values_path = dir.join("public_values.bin");
    let challenge_segment_path = dir.join("challenge_values_segment.bin");
    let block_input_bytes =
        encode_eth_block_input(&block_input).expect("block input should encode");
    let block_input_hash = eth_block_input_bytes_digest(&block_input_bytes);
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [17_u8]);
    write_bytes(&block_input_path, &block_input_bytes);
    write_bytes(
        &public_values_path,
        encode_public_values(&eth_block_public_values_with_rom_root(
            setup_hash,
            &block_input,
            [11, 12, 13, 14],
        ))
        .expect("public values should encode"),
    );
    let cache_path =
        write_sample_program_image_cache(&dir, &guest_image, setup_hash, [11, 12, 13, 14]);
    let cache = read_program_image_commitment_cache_file(&cache_path).expect("cache should read");
    let cache_segment =
        encode_program_image_cache_segment(&cache).expect("cache segment should encode");
    let cache_segment_hash = program_image_cache_segment_digest(&cache_segment);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--contributions",
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("pass=contributions\n"));
    assert!(stdout.contains("program_image_cache_gpu_mode=cuda\n"));
    assert!(stdout.contains(&format!(
        "eth_block_input_hash={}\n",
        format_hash(&block_input_hash)
    )));

    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let segment_ids = proof
        .segments
        .iter()
        .map(|segment| segment.id)
        .collect::<Vec<_>>();
    assert!(segment_ids.contains(&CONTRIBUTION_SEGMENT_ID));
    assert!(segment_ids.contains(&PROGRAM_IMAGE_CACHE_SEGMENT_ID));
    assert!(segment_ids.contains(&ETH_BLOCK_INPUT_SEGMENT_ID));

    let mut writer_stdout = Vec::new();
    let mut writer_stderr = Vec::new();
    let writer_code = run_cli(
        &[
            "prove",
            "write-contribution-challenges",
            dir.to_str().expect("setup path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
            challenge_segment_path
                .to_str()
                .expect("challenge path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
        ],
        &mut writer_stdout,
        &mut writer_stderr,
    );
    assert_eq!(
        writer_code,
        0,
        "{}",
        String::from_utf8_lossy(&writer_stderr)
    );
    assert!(writer_stderr.is_empty());
    let writer_stdout = String::from_utf8(writer_stdout).expect("writer stdout should be utf-8");
    assert!(writer_stdout.contains("program_image_caches=1\n"));
    assert!(writer_stdout.contains(&format!(
        "program_image_cache_segment_hash={}\n",
        format_hash(&cache_segment_hash)
    )));
    assert!(writer_stdout.contains("eth_block_inputs=1\n"));
    assert!(writer_stdout.contains(&format!(
        "eth_block_input_hash={}\n",
        format_hash(&block_input_hash)
    )));
    assert!(writer_stdout.contains("challenge_values=1\n"));

    let mut challenge_stdout = Vec::new();
    let mut challenge_stderr = Vec::new();
    let challenge_code = run_cli(
        &[
            "verify",
            "contribution-challenge",
            dir.to_str().expect("setup path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
            challenge_segment_path
                .to_str()
                .expect("challenge path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
        ],
        &mut challenge_stdout,
        &mut challenge_stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        challenge_code,
        0,
        "{}",
        String::from_utf8_lossy(&challenge_stderr)
    );
    assert!(challenge_stderr.is_empty());
    let challenge_stdout =
        String::from_utf8(challenge_stdout).expect("challenge stdout should be utf-8");
    assert!(challenge_stdout.contains("program_image_caches=1\n"));
    assert!(challenge_stdout.contains(&format!(
        "program_image_cache_segment_hash={}\n",
        format_hash(&cache_segment_hash)
    )));
    assert!(challenge_stdout.contains("eth_block_inputs=1\n"));
    assert!(challenge_stdout.contains(&format!(
        "eth_block_input_hash={}\n",
        format_hash(&block_input_hash)
    )));
    assert!(challenge_stdout.contains("challenge_values=1\n"));
}

fn run_prove_witness_with_aggregate_modifier(
    dir_name: &str,
    modifier: &str,
) -> (i32, String, String) {
    run_prove_witness_with_aggregate_modifiers(dir_name, &[modifier])
}

fn run_prove_witness_with_aggregate_modifiers(
    dir_name: &str,
    modifiers: &[&str],
) -> (i32, String, String) {
    let dir = temp_dir(dir_name);
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_proof_group_and_unit_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let unit_values_path = dir.join("unit_values.bin");
    let proof_values_path = dir.join("proof_values.bin");
    let group_values_path = dir.join("group_values.bin");
    let public_values_path = dir.join("public_values.bin");
    let challenge_values_path = output_dir.join("challenge_values_segment.bin");
    let public_values = sample_public_values(setup_hash);
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [17_u8]);
    write_field_words(&unit_values_path, &[101, 201, 202, 203]);
    write_field_words(&proof_values_path, &[51, 52, 53]);
    write_field_words(&group_values_path, &[61, 62, 63]);
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut args = vec!["prove", "witness", "--aggregate"];
    args.extend_from_slice(modifiers);
    args.extend_from_slice(&[
        "--save-outputs",
        "--unit-values",
        unit_values_path
            .to_str()
            .expect("unit values path should be utf-8"),
        "--proof-values",
        proof_values_path
            .to_str()
            .expect("proof values path should be utf-8"),
        "--group-values",
        group_values_path
            .to_str()
            .expect("group values path should be utf-8"),
        "--input-data",
        input_data.to_str().expect("input path should be utf-8"),
        dir.to_str().expect("path should be utf-8"),
        output_dir.to_str().expect("output path should be utf-8"),
        witness_library
            .to_str()
            .expect("witness path should be utf-8"),
        guest_image.to_str().expect("guest path should be utf-8"),
        public_values_path
            .to_str()
            .expect("public values path should be utf-8"),
    ]);
    let code = run_cli(&args, &mut stdout, &mut stderr);
    if code == 0 {
        let proof_path = output_dir.join("proof.bin");
        let proof_bytes = fs::read(&proof_path).expect("proof output should read");
        let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
        if modifiers.contains(&"--remote-aggregation") {
            let segment_ids = proof
                .segments
                .iter()
                .map(|segment| segment.id)
                .collect::<Vec<_>>();
            assert!(segment_ids.contains(&CONTRIBUTION_SEGMENT_ID));
            assert!(!segment_ids.contains(&PCS_MATERIAL_MANIFEST_SEGMENT_ID));
            assert!(!segment_ids.contains(&PCS_QUERY_PLAN_SEGMENT_ID));
            assert!(!segment_ids.contains(&CONSTANT_OPENING_SEGMENT_ID));
            assert!(!segment_ids.contains(&WITNESS_OPENING_SEGMENT_ID));
            assert!(!segment_ids.iter().any(|segment_id| {
                *segment_id >= WITNESS_COMMITMENT_SEGMENT_BASE_ID
                    && *segment_id < WITNESS_COMMITMENT_SEGMENT_BASE_ID + 16
            }));

            assert_external_contribution_challenge_verifies(
                &dir,
                &public_values_path,
                &proof_path,
                &challenge_values_path,
            );
        } else {
            let witness_ids = proof
                .segments
                .iter()
                .filter(|segment| {
                    segment.id >= WITNESS_COMMITMENT_SEGMENT_BASE_ID
                        && segment.id < WITNESS_COMMITMENT_SEGMENT_BASE_ID + 2
                })
                .map(|segment| segment.id)
                .collect::<Vec<_>>();
            assert_eq!(
                witness_ids,
                vec![
                    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
                    WITNESS_COMMITMENT_SEGMENT_BASE_ID + 1
                ]
            );
            validate_setup_preflight(&catalog, &proof, &public_values)
                .expect("setup preflight should validate");
        }
        assert!(output_dir.join("unit-0.witness-segment").exists());
        assert!(output_dir.join("unit-1.witness-segment").exists());
    }
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
    (
        code,
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        String::from_utf8(stderr).expect("stderr should be utf-8"),
    )
}

#[test]
fn runs_prove_witness_with_final_wrap() {
    let (code, stdout, stderr) =
        run_prove_witness_with_aggregate_modifier("prove-witness-final-wrap", "--final-wrap");

    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty());
    assert!(stdout.contains("aggregate=true\n"));
    assert!(stdout.contains("final_wrap=true\n"));
}

#[test]
fn runs_prove_witness_with_remote_aggregation() {
    let (code, stdout, stderr) = run_prove_witness_with_aggregate_modifier(
        "prove-witness-remote-aggregation",
        "--remote-aggregation",
    );

    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty());
    assert!(stdout.contains("aggregate=true\n"));
    assert!(stdout.contains("remote_aggregation=true\n"));
}

#[test]
fn rejects_prove_witness_with_too_few_stored_witnesses() {
    let (code, stdout, stderr) = run_prove_witness_with_aggregate_modifiers(
        "prove-witness-stored-witnesses",
        &["--stored-witnesses", "1"],
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        stderr,
        "prove witness failed: stored witness limit 1 is lower than required all-unit witness outputs 4\n"
    );
}

#[test]
fn runs_prove_witness_for_aggregate_with_unit_values_segment() {
    let dir = temp_dir("prove-witness-aggregate-unit-values-segment");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_proof_group_and_unit_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let unit_values_segment_path = dir.join("unit_values_segment.bin");
    let proof_values_path = dir.join("proof_values.bin");
    let group_values_path = dir.join("group_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [19_u8]);
    write_bytes(
        &unit_values_segment_path,
        encode_unit_values_segment(&UnitValuesSegment {
            units: vec![
                UnitValuesUnitSegment {
                    unit_index: 0,
                    values: vec![101, 201, 202, 203],
                },
                UnitValuesUnitSegment {
                    unit_index: 1,
                    values: vec![111, 211, 212, 213],
                },
            ],
        })
        .expect("unit values segment should encode"),
    );
    write_field_words(&proof_values_path, &[51, 52, 53]);
    write_field_words(&group_values_path, &[61, 62, 63]);

    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--aggregate",
            "--save-outputs",
            "--unit-values-segment",
            unit_values_segment_path
                .to_str()
                .expect("unit values segment path should be utf-8"),
            "--proof-values",
            proof_values_path
                .to_str()
                .expect("proof values path should be utf-8"),
            "--group-values",
            group_values_path
                .to_str()
                .expect("group values path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let proof_bytes = fs::read(output_dir.join("proof.bin")).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
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
    assert_eq!(unit_values.units[0].values, vec![101, 201, 202, 203]);
    assert_eq!(unit_values.units[1].values, vec![111, 211, 212, 213]);
    assert!(output_dir.join("unit-0.witness-segment").exists());
    assert!(output_dir.join("unit-1.witness-segment").exists());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn runs_prove_witness_for_aggregate_with_proof_and_group_segments() {
    let dir = temp_dir("prove-witness-aggregate-proof-group-segments");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_proof_group_and_unit_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let unit_values_segment_path = dir.join("unit_values_segment.bin");
    let proof_values_segment_path = dir.join("proof_values_segment.bin");
    let group_values_segment_path = dir.join("group_values_segment.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [21_u8]);
    write_bytes(
        &unit_values_segment_path,
        encode_unit_values_segment(&UnitValuesSegment {
            units: vec![
                UnitValuesUnitSegment {
                    unit_index: 0,
                    values: vec![101, 201, 202, 203],
                },
                UnitValuesUnitSegment {
                    unit_index: 1,
                    values: vec![111, 211, 212, 213],
                },
            ],
        })
        .expect("unit values segment should encode"),
    );
    write_bytes(
        &proof_values_segment_path,
        encode_pcs_proof_values_segment(&PcsProofValuesSegment {
            values: vec![[51, 52, 53]],
        })
        .expect("proof values segment should encode"),
    );
    write_bytes(
        &group_values_segment_path,
        encode_group_values_segment(&GroupValuesSegment {
            values: vec![[61, 62, 63]],
        })
        .expect("group values segment should encode"),
    );

    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--aggregate",
            "--save-outputs",
            "--unit-values-segment",
            unit_values_segment_path
                .to_str()
                .expect("unit values segment path should be utf-8"),
            "--proof-values-segment",
            proof_values_segment_path
                .to_str()
                .expect("proof values segment path should be utf-8"),
            "--group-values-segment",
            group_values_segment_path
                .to_str()
                .expect("group values segment path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let proof_bytes = fs::read(output_dir.join("proof.bin")).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");

    let proof_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_PROOF_VALUES_SEGMENT_ID)
        .expect("proof values segment should exist");
    let proof_values = parse_pcs_proof_values_segment(&proof_values_segment.data)
        .expect("proof values should parse");
    assert_eq!(proof_values.values, vec![[51, 52, 53]]);

    let group_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == GROUP_VALUES_SEGMENT_ID)
        .expect("group values segment should exist");
    let group_values =
        parse_group_values_segment(&group_values_segment.data).expect("group values should parse");
    assert_eq!(group_values.values, vec![[61, 62, 63]]);

    let unit_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == UNIT_VALUES_SEGMENT_ID)
        .expect("unit values segment should exist");
    let unit_values =
        parse_unit_values_segment(&unit_values_segment.data).expect("unit values should parse");
    assert_eq!(unit_values.units.len(), 2);
    assert_eq!(unit_values.units[0].values, vec![101, 201, 202, 203]);
    assert_eq!(unit_values.units[1].values, vec![111, 211, 212, 213]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn runs_prove_witness_for_aggregate_with_transcript_fri_outputs() {
    let dir = temp_dir("prove-witness-aggregate-fri");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_fri_quotient(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let evaluation_values_path = dir.join("evaluation_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [23_u8]);
    write_field_words(&evaluation_values_path, &[30, 31, 32, 40, 41, 42]);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_hash = public_values_digest(&public_values).expect("digest should compute");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--aggregate",
            "--save-outputs",
            "--evaluation-values",
            evaluation_values_path
                .to_str()
                .expect("evaluation path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains(&format!("public_inputs={}\n", public_values_path.display())));
    assert!(stdout_text.contains(&format!(
        "public_inputs_hash={}\n",
        format_hash(&public_values_hash)
    )));
    assert!(stdout_text.contains("public_input_values=1\npublic_input_fields=1\n"));
    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    assert_eq!(proof.segments.len(), 11);
    assert_has_no_contribution_segment(&proof);
    assert!(proof
        .segments
        .iter()
        .any(|segment| segment.id == PCS_QUERY_NONCE_SEGMENT_ID));

    let evaluation_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_EVALUATION_SEGMENT_ID)
        .expect("evaluation segment should exist");
    let evaluations = parse_pcs_evaluation_segment(&evaluation_segment.data)
        .expect("evaluation segment should parse");
    assert_eq!(evaluations.units.len(), 4);
    for unit in &evaluations.units {
        assert_eq!(unit.values, vec![[30, 31, 32], [40, 41, 42]]);
    }

    let fri_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID)
        .expect("FRI segment should exist");
    let fri = parse_pcs_fri_opening_segment(&fri_segment.data).expect("FRI segment should parse");
    assert_eq!(fri.units.len(), 4);

    let query_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_QUERY_PLAN_SEGMENT_ID)
        .expect("query segment should exist");
    let query_plan =
        parse_pcs_query_plan_segment(&query_segment.data).expect("query segment should parse");
    assert_eq!(query_plan.units.len(), 4);

    let witness_ids = proof
        .segments
        .iter()
        .filter(|segment| {
            segment.id >= WITNESS_COMMITMENT_SEGMENT_BASE_ID
                && segment.id < WITNESS_COMMITMENT_SEGMENT_BASE_ID + 4
        })
        .map(|segment| segment.id)
        .collect::<Vec<_>>();
    assert_eq!(
        witness_ids,
        vec![
            WITNESS_COMMITMENT_SEGMENT_BASE_ID,
            WITNESS_COMMITMENT_SEGMENT_BASE_ID + 1,
            WITNESS_COMMITMENT_SEGMENT_BASE_ID + 2,
            WITNESS_COMMITMENT_SEGMENT_BASE_ID + 3
        ]
    );
    for unit_index in 0..4 {
        assert!(output_dir
            .join(format!("unit-{unit_index}.witness-segment"))
            .exists());
    }

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert_eq!(
        String::from_utf8(verify_stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(11, &public_values_path)
    );
    assert!(verify_stderr.is_empty());

    let mut proof_verify_stdout = Vec::new();
    let mut proof_verify_stderr = Vec::new();
    let proof_verify_code = run_cli(
        &[
            "verify",
            "proof",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut proof_verify_stdout,
        &mut proof_verify_stderr,
    );
    assert_eq!(
        proof_verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&proof_verify_stderr)
    );
    assert_eq!(
        String::from_utf8(proof_verify_stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(11, &public_values_path)
    );
    assert!(proof_verify_stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn runs_prove_witness_for_aggregate_with_evaluation_values_segment() {
    let dir = temp_dir("prove-witness-aggregate-evaluation-segment");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_fri_quotient(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let evaluation_values_segment_path = dir.join("evaluation_values_segment.bin");
    let expected_evaluations = vec![
        PcsEvaluationUnitSegment {
            unit_index: 0,
            values: vec![[30, 31, 32], [40, 41, 42]],
        },
        PcsEvaluationUnitSegment {
            unit_index: 1,
            values: vec![[50, 51, 52], [60, 61, 62]],
        },
        PcsEvaluationUnitSegment {
            unit_index: 2,
            values: vec![[70, 71, 72], [80, 81, 82]],
        },
        PcsEvaluationUnitSegment {
            unit_index: 3,
            values: vec![[90, 91, 92], [100, 101, 102]],
        },
    ];
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [29_u8]);
    write_bytes(
        &evaluation_values_segment_path,
        encode_pcs_evaluation_segment(&PcsEvaluationSegment {
            units: expected_evaluations.clone(),
        })
        .expect("evaluation segment should encode"),
    );
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--aggregate",
            "--save-outputs",
            "--evaluation-values-segment",
            evaluation_values_segment_path
                .to_str()
                .expect("evaluation segment path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    assert_eq!(proof.segments.len(), 11);
    assert_has_no_contribution_segment(&proof);

    let evaluation_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_EVALUATION_SEGMENT_ID)
        .expect("evaluation segment should exist");
    let evaluations = parse_pcs_evaluation_segment(&evaluation_segment.data)
        .expect("evaluation segment should parse");
    assert_eq!(evaluations.units, expected_evaluations);

    let fri_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID)
        .expect("FRI segment should exist");
    let fri = parse_pcs_fri_opening_segment(&fri_segment.data).expect("FRI segment should parse");
    assert_eq!(fri.units.len(), 4);

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert_eq!(
        String::from_utf8(verify_stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(11, &public_values_path)
    );
    assert!(verify_stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn runs_prove_witness_for_aggregate_fri_with_unit_values_segment() {
    let dir = temp_dir("prove-witness-aggregate-fri-unit-values-segment");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_fri_quotient_and_unit_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let evaluation_values_segment_path = dir.join("evaluation_values_segment.bin");
    let unit_values_segment_path = dir.join("unit_values_segment.bin");
    let expected_evaluations = vec![
        PcsEvaluationUnitSegment {
            unit_index: 0,
            values: vec![[30, 31, 32], [40, 41, 42]],
        },
        PcsEvaluationUnitSegment {
            unit_index: 1,
            values: vec![[50, 51, 52], [60, 61, 62]],
        },
        PcsEvaluationUnitSegment {
            unit_index: 2,
            values: vec![[70, 71, 72], [80, 81, 82]],
        },
        PcsEvaluationUnitSegment {
            unit_index: 3,
            values: vec![[90, 91, 92], [100, 101, 102]],
        },
    ];
    let expected_unit_values = UnitValuesSegment {
        units: vec![
            UnitValuesUnitSegment {
                unit_index: 0,
                values: vec![101, 201, 202, 203],
            },
            UnitValuesUnitSegment {
                unit_index: 1,
                values: vec![111, 211, 212, 213],
            },
            UnitValuesUnitSegment {
                unit_index: 2,
                values: vec![121, 221, 222, 223],
            },
            UnitValuesUnitSegment {
                unit_index: 3,
                values: vec![131, 231, 232, 233],
            },
        ],
    };
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [31_u8]);
    write_bytes(
        &evaluation_values_segment_path,
        encode_pcs_evaluation_segment(&PcsEvaluationSegment {
            units: expected_evaluations.clone(),
        })
        .expect("evaluation segment should encode"),
    );
    write_bytes(
        &unit_values_segment_path,
        encode_unit_values_segment(&expected_unit_values).expect("unit values should encode"),
    );
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--aggregate",
            "--save-outputs",
            "--evaluation-values-segment",
            evaluation_values_segment_path
                .to_str()
                .expect("evaluation segment path should be utf-8"),
            "--unit-values-segment",
            unit_values_segment_path
                .to_str()
                .expect("unit values segment path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    assert_eq!(proof.segments.len(), 12);
    assert_has_no_contribution_segment(&proof);

    let unit_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == UNIT_VALUES_SEGMENT_ID)
        .expect("unit values segment should exist");
    let unit_values =
        parse_unit_values_segment(&unit_values_segment.data).expect("unit values should parse");
    assert_eq!(unit_values, expected_unit_values);

    let evaluation_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_EVALUATION_SEGMENT_ID)
        .expect("evaluation segment should exist");
    let evaluations = parse_pcs_evaluation_segment(&evaluation_segment.data)
        .expect("evaluation segment should parse");
    assert_eq!(evaluations.units, expected_evaluations);

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert_eq!(
        String::from_utf8(verify_stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(12, &public_values_path)
    );
    assert!(verify_stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn saves_prove_witness_transcript_fri_outputs_when_requested() {
    let dir = temp_dir("prove-witness-save-fri");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_fri_quotient(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let evaluation_values_path = dir.join("evaluation_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [11_u8]);
    write_field_words(&evaluation_values_path, &[30, 31, 32, 40, 41, 42]);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--save-outputs",
            "--evaluation-values",
            evaluation_values_path
                .to_str()
                .expect("evaluation path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert!(stderr.is_empty());
    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    assert_eq!(proof.segments.len(), 8);
    assert_has_no_contribution_segment(&proof);
    assert!(proof
        .segments
        .iter()
        .any(|segment| segment.id == PCS_EVALUATION_SEGMENT_ID));
    assert!(proof
        .segments
        .iter()
        .any(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID));
    assert!(proof
        .segments
        .iter()
        .any(|segment| segment.id == PCS_QUERY_NONCE_SEGMENT_ID));
    let evaluation_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_EVALUATION_SEGMENT_ID)
        .expect("evaluation segment should exist");
    let evaluations = parse_pcs_evaluation_segment(&evaluation_segment.data)
        .expect("evaluation segment should parse");
    let material_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_MATERIAL_MANIFEST_SEGMENT_ID)
        .expect("material segment should exist");
    let material = parse_pcs_material_manifest_segment(&material_segment.data)
        .expect("material segment should parse");
    let witness_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == WITNESS_COMMITMENT_SEGMENT_BASE_ID)
        .expect("witness segment should exist");
    let witness = parse_witness_commitment_segment(&witness_segment.data)
        .expect("witness segment should parse");
    let query_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_QUERY_PLAN_SEGMENT_ID)
        .expect("query segment should exist");
    let query_plan =
        parse_pcs_query_plan_segment(&query_segment.data).expect("query segment should parse");
    let fri_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID)
        .expect("FRI segment should exist");
    let fri = parse_pcs_fri_opening_segment(&fri_segment.data).expect("FRI segment should parse");
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let challenges = derive_pcs_transcript_challenges_from_segments(PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material.units[0],
        public_values: &[Felt::from_u64(12_345)],
        unit_values: &[],
        witness: &witness,
        evaluations: &evaluations.units[0],
        fri: &fri.units[0],
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: &[],
    })
    .expect("transcript challenges should derive");
    assert!(verify_fri_opening_folds(
        &schedule.units[0],
        PcsFriOpeningFoldRequest {
            unit_index: 0,
            query_rows: &query_plan.units[0].queries,
            challenges: &challenges,
            fri: &fri.units[0],
        },
    )
    .expect("FRI folds should verify"));
    let constant_opening = parse_constant_opening_segment(
        &proof
            .segments
            .iter()
            .find(|segment| segment.id == CONSTANT_OPENING_SEGMENT_ID)
            .expect("constant opening segment should exist")
            .data,
    )
    .expect("constant opening segment should parse");
    let witness_opening = parse_witness_opening_segment(
        &proof
            .segments
            .iter()
            .find(|segment| segment.id == WITNESS_OPENING_SEGMENT_ID)
            .expect("witness opening segment should exist")
            .data,
    )
    .expect("witness opening segment should parse");
    let query_outputs = evaluate_verifier_unit_queries(
        &schedule.units[0],
        VerifierUnitQueryEvalRequest {
            unit_index: 0,
            challenges: &challenges,
            proof_values: &[],
            constant_unit: &constant_opening.units[0],
            witness_unit: &witness_opening.units[0],
            evaluations: &evaluations.units[0],
            code: &catalog.units[0].metadata.verifier.query,
            publics: &[Felt::from_u64(12_345)],
        },
    )
    .expect("query outputs should evaluate");
    assert!(verify_query_outputs_against_fri_opening(
        &schedule.units[0],
        VerifierFriComparisonRequest {
            unit_index: 0,
            query_rows: &query_plan.units[0].queries,
            query_outputs: &query_outputs,
            fri: &fri.units[0],
        },
    )
    .expect("query outputs should match FRI opening"));
    assert_eq!(
        evaluations.units[0].values,
        vec![[30, 31, 32], [40, 41, 42]]
    );

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert_eq!(
        String::from_utf8(verify_stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(8, &public_values_path)
    );
    assert!(verify_stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_prove_witness_transcript_fri_outputs_without_evaluation_values() {
    let dir = temp_dir("prove-witness-save-fri-missing-evals");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_fri_quotient(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [11_u8]);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--save-outputs",
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove witness failed: missing evaluation values for unit 0: expected 2\n"
    );
}

#[test]
fn saves_prove_witness_proof_values_when_requested() {
    let dir = temp_dir("prove-witness-save-proof-values");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_proof_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let proof_values_path = dir.join("proof_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [13_u8]);
    write_field_words(&proof_values_path, &[51, 52, 53]);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--save-outputs",
            "--proof-values",
            proof_values_path
                .to_str()
                .expect("proof values path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert!(stderr.is_empty());
    let proof_bytes = fs::read(output_dir.join("proof.bin")).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let proof_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_PROOF_VALUES_SEGMENT_ID)
        .expect("proof values segment should exist");
    let proof_values = parse_pcs_proof_values_segment(&proof_values_segment.data)
        .expect("proof values segment should parse");
    assert_eq!(proof_values.values, vec![[51, 52, 53]]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn saves_prove_witness_proof_values_segment_when_requested() {
    let dir = temp_dir("prove-witness-save-proof-values-segment");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_proof_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let proof_values_segment_path = dir.join("proof_values_segment.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [13_u8]);
    write_bytes(
        &proof_values_segment_path,
        encode_pcs_proof_values_segment(&PcsProofValuesSegment {
            values: vec![[51, 52, 53]],
        })
        .expect("proof values segment should encode"),
    );
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--save-outputs",
            "--proof-values-segment",
            proof_values_segment_path
                .to_str()
                .expect("proof values segment path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let proof_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == PCS_PROOF_VALUES_SEGMENT_ID)
        .expect("proof values segment should exist");
    let proof_values = parse_pcs_proof_values_segment(&proof_values_segment.data)
        .expect("proof values segment should parse");
    assert_eq!(proof_values.values, vec![[51, 52, 53]]);

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert_eq!(
        String::from_utf8(verify_stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(6, &public_values_path)
    );
    assert!(verify_stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn saves_prove_witness_group_values_when_requested() {
    let dir = temp_dir("prove-witness-save-group-values");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory_with_group_value(&dir);
    let group_value = [61, 62, 63];
    write_global_constraint_program(
        &dir,
        GlobalConstraintProgram {
            entries: vec![GlobalConstraintEntry {
                destination_dimension: 3,
                destination_id: 0,
                temp1_count: 0,
                temp3_count: 1,
                ops_count: 1,
                ops_offset: 0,
                args_count: 6,
                args_offset: 0,
                source_line: "group residual".to_owned(),
            }],
            ops: vec![2],
            args: vec![1, 0, 5, 0, 2, 0],
            numbers: group_value.to_vec(),
        },
    );
    run_generate_key_command(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let group_values_path = dir.join("group_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [13_u8]);
    write_field_words(&group_values_path, &group_value);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--save-outputs",
            "--group-values",
            group_values_path
                .to_str()
                .expect("group values path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let group_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == GROUP_VALUES_SEGMENT_ID)
        .expect("group values segment should exist");
    let group_values = parse_group_values_segment(&group_values_segment.data)
        .expect("group values segment should parse");
    assert_eq!(group_values.values, vec![group_value]);

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert_eq!(
        String::from_utf8(verify_stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(6, &public_values_path)
    );
    assert!(verify_stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn saves_prove_witness_unit_values_when_requested() {
    let dir = temp_dir("prove-witness-save-unit-values");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_unit_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let unit_values_path = dir.join("unit_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [13_u8]);
    write_field_words(&unit_values_path, &[101, 201, 202, 203]);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--save-outputs",
            "--unit-values",
            unit_values_path
                .to_str()
                .expect("unit values path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let proof_bytes = fs::read(output_dir.join("proof.bin")).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let unit_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == UNIT_VALUES_SEGMENT_ID)
        .expect("unit values segment should exist");
    let unit_values =
        parse_unit_values_segment(&unit_values_segment.data).expect("unit values should parse");
    assert_eq!(unit_values.units.len(), 1);
    assert_eq!(unit_values.units[0].unit_index, 0);
    assert_eq!(unit_values.units[0].values, vec![101, 201, 202, 203]);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn saves_prove_witness_unit_values_segment_when_requested() {
    let dir = temp_dir("prove-witness-save-unit-values-segment");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_unit_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let unit_values_segment_path = dir.join("unit_values_segment.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [13_u8]);
    write_bytes(
        &unit_values_segment_path,
        encode_unit_values_segment(&UnitValuesSegment {
            units: vec![UnitValuesUnitSegment {
                unit_index: 0,
                values: vec![101, 201, 202, 203],
            }],
        })
        .expect("unit values segment should encode"),
    );
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--save-outputs",
            "--unit-values-segment",
            unit_values_segment_path
                .to_str()
                .expect("unit values segment path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let proof_path = output_dir.join("proof.bin");
    let proof_bytes = fs::read(&proof_path).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let unit_values_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == UNIT_VALUES_SEGMENT_ID)
        .expect("unit values segment should exist");
    let unit_values =
        parse_unit_values_segment(&unit_values_segment.data).expect("unit values should parse");
    assert_eq!(unit_values.units.len(), 1);
    assert_eq!(unit_values.units[0].unit_index, 0);
    assert_eq!(unit_values.units[0].values, vec![101, 201, 202, 203]);

    let mut verify_stdout = Vec::new();
    let mut verify_stderr = Vec::new();
    let verify_code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut verify_stdout,
        &mut verify_stderr,
    );
    assert_eq!(
        verify_code,
        0,
        "{}",
        String::from_utf8_lossy(&verify_stderr)
    );
    assert_eq!(
        String::from_utf8(verify_stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(6, &public_values_path)
    );
    assert!(verify_stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_combined_unit_values_inputs() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--unit-values",
            "unit-values.bin",
            "--unit-values-segment",
            "unit-values-segment.bin",
            "setup",
            "out",
            "witness.so",
            "guest.elf",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove witness failed: cannot combine --unit-values and --unit-values-segment\n"
    );
}

#[test]
fn passes_proof_values_to_witness_regular_constraints() {
    let dir = temp_dir("prove-witness-proof-value-constraint");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_proof_value_constraint(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let proof_values_path = dir.join("proof_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [13_u8]);
    write_field_words(&proof_values_path, &[14, 52, 53]);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--save-outputs",
            "--proof-values",
            proof_values_path
                .to_str()
                .expect("proof values path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
}

#[test]
fn passes_unit_values_to_witness_regular_constraints() {
    let dir = temp_dir("prove-witness-unit-value-constraint");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_unit_value_constraint(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let unit_values_path = dir.join("unit_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [13_u8]);
    write_field_words(&unit_values_path, &[14, 201, 202, 203]);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--save-outputs",
            "--unit-values",
            unit_values_path
                .to_str()
                .expect("unit values path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
}

#[test]
fn passes_challenge_values_to_witness_regular_constraints() {
    let dir = temp_dir("prove-witness-challenge-constraint");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_challenge_constraint(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let challenge_values_path = dir.join("challenge_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [13_u8]);
    write_field_words(&challenge_values_path, &[14, 52, 53]);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--save-outputs",
            "--challenge-values",
            challenge_values_path
                .to_str()
                .expect("challenge values path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
}

#[test]
fn passes_challenge_values_segment_to_witness_regular_constraints() {
    let dir = temp_dir("prove-witness-challenge-constraint-segment");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_challenge_constraint(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let challenge_values_segment_path = dir.join("challenge_values_segment.bin");
    let challenge_values_segment = encode_challenge_values_segment(&ChallengeValuesSegment {
        values: vec![[14, 52, 53]],
    })
    .expect("challenge values segment should encode");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [13_u8]);
    write_bytes(&challenge_values_segment_path, &challenge_values_segment);
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--save-outputs",
            "--no-verify-outputs",
            "--challenge-values-segment",
            challenge_values_segment_path
                .to_str()
                .expect("challenge values segment path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    let proof_bytes = fs::read(output_dir.join("proof.bin")).expect("proof output should read");
    let proof = parse_proof_artifact(&proof_bytes).expect("proof output should parse");
    let proof_challenge_segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == CHALLENGE_VALUES_SEGMENT_ID)
        .expect("challenge values segment should be embedded");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(proof_challenge_segment.data, challenge_values_segment);
}

#[test]
fn rejects_prove_witness_proof_output_with_mismatched_public_inputs() {
    let dir = temp_dir("prove-witness-bad-public-inputs");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [11_u8]);
    write_bytes(
        &public_values_path,
        encode_public_values(&sample_public_values([0x99; 32]))
            .expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--save-outputs",
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "prove witness failed: prove execution plan public inputs setup hash mismatch: {}\n",
            public_values_path.display()
        )
    );
}

#[test]
fn rejects_prove_witness_with_mismatched_program_image_cache_source_digest() {
    let dir = temp_dir("prove-witness-program-image-cache-mismatch");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let cache_guest_image = dir.join("cache-guest.elf");
    let program_path = dir.join("program.bin");
    let constraint_digest_path = dir.join("constraint.digest");
    let root_path = dir.join("root.bin");
    let cache_path = dir.join("program_image.cache");
    let guest_image_bytes = sample_guest_image();
    let mut cache_guest_image_bytes = sample_guest_image();
    cache_guest_image_bytes[63] = 1;
    write_bytes(&guest_image, guest_image_bytes);
    write_bytes(&cache_guest_image, cache_guest_image_bytes);
    write_bytes(&program_path, b"packed-program");
    write_bytes(&constraint_digest_path, setup_hash);
    write_bytes(
        &root_path,
        encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![11, 12, 13, 14]))
            .expect("root should encode"),
    );
    write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
        program_path: &program_path,
        guest_image_path: &cache_guest_image,
        constraint_digest_path: &constraint_digest_path,
        root_path: &root_path,
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cpu,
        output_path: &cache_path,
    })
    .expect("cache should write");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "prove witness failed: program image cache guest image digest mismatch at {}\n",
            cache_path.display()
        )
    );
}

#[test]
fn rejects_prove_witness_with_mismatched_program_image_cache_setup_hash() {
    let dir = temp_dir("prove-witness-program-image-cache-setup-hash-mismatch");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let program_path = dir.join("program.bin");
    let constraint_digest_path = dir.join("constraint.digest");
    let root_path = dir.join("root.bin");
    let cache_path = dir.join("program_image.cache");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&program_path, b"packed-program");
    write_bytes(&constraint_digest_path, [0x44_u8; 32]);
    write_bytes(
        &root_path,
        encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![11, 12, 13, 14]))
            .expect("root should encode"),
    );
    write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
        program_path: &program_path,
        guest_image_path: &guest_image,
        constraint_digest_path: &constraint_digest_path,
        root_path: &root_path,
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cpu,
        output_path: &cache_path,
    })
    .expect("cache should write");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "prove witness failed: program image cache setup hash mismatch at {}\n",
            cache_path.display()
        )
    );
}

#[test]
fn prove_inputs_rejects_duplicate_program_image_cache() {
    let dir = temp_dir("prove-inputs-duplicate-program-image-cache");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    let program_path = dir.join("program.bin");
    let other_program_path = dir.join("other-program.bin");
    let constraint_digest_path = dir.join("constraint.digest");
    let root_path = dir.join("root.bin");
    let cache_path = dir.join("program_image.cache");
    let other_cache_path = dir.join("other-program_image.cache");
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&program_path, b"packed-program");
    write_bytes(&other_program_path, b"other-packed-program");
    write_bytes(&constraint_digest_path, setup_hash);
    write_bytes(
        &root_path,
        encode_verification_key_binary(&VerificationKeyRoot::FieldElements(vec![11, 12, 13, 14]))
            .expect("root should encode"),
    );
    write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
        program_path: &program_path,
        guest_image_path: &guest_image,
        constraint_digest_path: &constraint_digest_path,
        root_path: &root_path,
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
        output_path: &cache_path,
    })
    .expect("cache should write");
    write_program_image_commitment_cache_file(ProgramImageCommitmentCacheFileRequest {
        program_path: &other_program_path,
        guest_image_path: &guest_image,
        constraint_digest_path: &constraint_digest_path,
        root_path: &root_path,
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cuda,
        output_path: &other_cache_path,
    })
    .expect("other cache should write");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--program-image-cache",
            cache_path.to_str().expect("cache path should be utf-8"),
            "--program-image-cache",
            other_cache_path
                .to_str()
                .expect("cache path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove inputs failed: duplicate --program-image-cache option\n"
    );
}

#[test]
fn rejects_prove_inputs_with_invalid_guest_image() {
    let dir = temp_dir("prove-inputs-invalid-guest");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    write_bytes(&witness_library, sample_witness_library());
    write_bytes(&guest_image, b"not-an-elf");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "prove inputs failed: prove execution plan guest image is invalid: {}: invalid guest image magic\n",
            guest_image.display()
        )
    );
}

#[test]
fn rejects_prove_inputs_with_invalid_witness_library() {
    let dir = temp_dir("prove-inputs-invalid-witness");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = dir.join("libwitness.so");
    let guest_image = dir.join("guest.elf");
    write_bytes(&witness_library, b"not-an-elf");
    write_bytes(&guest_image, sample_guest_image());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            dir.to_str().expect("path should be utf-8"),
            output_dir.to_str().expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "prove inputs failed: prove execution plan witness library is invalid: {}: invalid witness library magic\n",
            witness_library.display()
        )
    );
}

#[test]
fn runs_setup_aware_verify_preflight() {
    let dir = temp_dir("verify-setup-preflight");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values_hash =
        public_values_digest(&sample_public_values(setup_hash)).expect("digest should compute");
    let (proof_path, public_values_path) =
        write_proof_pair_with_material(&dir, setup_hash, &catalog);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits=4\nsegments=5\npublic_values=1\npublic_values_hash={}\npublic_value_fields=1\n",
            format_hash(&public_values_hash)
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn runs_setup_aware_verify_preflight_with_source_generated_key_directory() {
    let dir = temp_dir("verify-setup-preflight-source-generated");
    let _ = fs::remove_dir_all(&dir);
    write_source_key_inputs(&dir);
    let source_path = dir.join("main.pil");
    write_bytes(
        &source_path,
        "public block_number;\n\
         airtemplate UnitA() { }\n\
         airgroup GroupA { UnitA(); }\n\
         col fixed main.left = [5, 1];\n\
         col fixed main.right = [9, 9];",
    );

    let mut generate_stdout = Vec::new();
    let mut generate_stderr = Vec::new();
    let generate_code = run_cli(
        &[
            "setup",
            "generate-key",
            "--source",
            source_path.to_str().expect("source path should be utf-8"),
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut generate_stdout,
        &mut generate_stderr,
    );
    assert_eq!(
        generate_code,
        0,
        "{}",
        String::from_utf8_lossy(&generate_stderr)
    );
    assert!(generate_stderr.is_empty());
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values_hash =
        public_values_digest(&sample_public_values(setup_hash)).expect("digest should compute");
    let (proof_path, public_values_path) =
        write_proof_pair_with_material(&dir, setup_hash, &catalog);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits=4\nsegments=5\npublic_values=1\npublic_values_hash={}\npublic_value_fields=1\nsource_fixed_file_manifest=present\nsource_fixed_file_manifest_entries=0\nsource_program_archive=present\nsource_program_archive_sources=1\nsource_program_archive_edges=0\n",
            format_hash(&public_values_hash)
        )
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_unbound_pcs_fri_opening() {
    let dir = temp_dir("verify-setup-preflight-unbound-fri-opening");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let fri_segment = sample_pcs_fri_opening_segment(&schedule, &proof.segments[1], 0);
    proof.segments.push(fri_segment);
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: PCS FRI opening segment requires transcript query inputs\n"
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn validates_setup_aware_verify_preflight_with_transcript_query_plan() {
    let dir = temp_dir("verify-setup-preflight-transcript-query");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_value_fields = public_values
        .values
        .iter()
        .flat_map(|entry| entry.elements.iter().copied().map(Felt::from_u64))
        .collect::<Vec<_>>();
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let material_segment =
        build_pcs_material_manifest_segment(&schedule).expect("material segment should build");
    let material = parse_pcs_material_manifest_segment(&material_segment.data)
        .expect("material segment should parse")
        .units[0]
        .clone();
    let witness_segment = sample_witness_proof_segment(&schedule, 0);
    let witness = parse_witness_commitment_segment(&witness_segment.data)
        .expect("witness segment should parse");
    let evaluation_segment = sample_pcs_evaluation_segment(0);
    let evaluations = parse_pcs_evaluation_segment(&evaluation_segment.data)
        .expect("evaluation segment should parse")
        .units[0]
        .clone();
    let fri_unit = sample_folded_pcs_fri_opening_template(
        &schedule,
        &material,
        &public_value_fields,
        &witness,
        &evaluations,
        0,
    );
    let transcript_inputs = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &public_value_fields,
        unit_values: &[],
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri_unit,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: &[],
    };
    let nonce_segment =
        build_pcs_query_nonce_segment_from_transcript_segments(&schedule, transcript_inputs)
            .expect("nonce segment should build");
    let query_segment = build_pcs_query_plan_segment_from_transcript_segments(
        &schedule,
        std::slice::from_ref(&witness_segment),
        transcript_inputs,
        &nonce_segment,
    )
    .expect("query segment should build");
    let constant_opening_segment =
        build_constant_opening_segment(&catalog, &schedule, &query_segment)
            .expect("constant opening segment should build");
    let opening_segment = sample_witness_opening_segment(&schedule, &query_segment, 0);
    let fri_segment = sample_folded_pcs_fri_opening_segment(&schedule, &query_segment, 0, fri_unit);
    let proof = ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            material_segment,
            query_segment,
            constant_opening_segment,
            opening_segment,
            witness_segment,
            evaluation_segment,
            fri_segment,
            nonce_segment,
        ],
    };
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(8, &public_values_path)
    );
    assert!(stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn validates_setup_aware_verify_preflight_with_proof_values() {
    let dir = temp_dir("verify-setup-preflight-proof-values");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, segment_count) =
        write_proof_value_query_preflight_fixture(&dir, Some(vec![[51, 52, 53]]));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(segment_count, &public_values_path)
    );
    assert!(stderr.is_empty());

    let report = validate_setup_preflight_from_files(&dir, &proof_path, &public_values_path)
        .expect("file-based setup preflight should validate");
    assert_eq!(report.unit_count, 4);
    assert_eq!(report.segment_count, segment_count);
    assert_eq!(report.public_value_count, 1);
    assert_eq!(report.public_value_field_count, 1);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_setup_aware_verify_preflight_with_invalid_contribution_segment() {
    let dir = temp_dir("verify-setup-preflight-invalid-contribution");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, _) =
        write_proof_value_query_preflight_fixture(&dir, Some(vec![[51, 52, 53]]));
    let proof_bytes = fs::read(&proof_path).expect("proof should read");
    let mut proof = parse_proof_artifact(&proof_bytes).expect("proof should parse");
    proof.segments.push(ProofSegment {
        id: CONTRIBUTION_SEGMENT_ID,
        data: vec![0, 0, 0, 0],
    });
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: invalid contribution segment: invalid contribution segment magic\n"
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_setup_aware_verify_preflight_with_missing_contribution_challenge() {
    let dir = temp_dir("verify-setup-preflight-missing-contribution-challenge");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let material_segment =
        build_pcs_material_manifest_segment(&schedule).expect("material segment should build");
    let witness_segment = sample_witness_proof_segment(&schedule, 0);
    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );
    let contribution_segment = build_contribution_segment(&entries)
        .expect("contribution segment should build")
        .expect("contribution segment should exist");
    let query_segment = build_pcs_query_plan_segment(
        &schedule,
        public_values_digest(&public_values).expect("digest should compute"),
        &material_segment,
        std::slice::from_ref(&witness_segment),
    )
    .expect("query segment should build");
    let constant_opening_segment =
        build_constant_opening_segment(&catalog, &schedule, &query_segment)
            .expect("constant opening segment should build");
    let opening_segment = sample_witness_opening_segment(&schedule, &query_segment, 0);
    let proof = ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            material_segment,
            query_segment,
            constant_opening_segment,
            opening_segment,
            witness_segment,
            contribution_segment,
        ],
    };
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: missing contribution challenge values\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_mismatched_contribution_challenge() {
    let dir = temp_dir("verify-setup-preflight-bad-contribution-challenge");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_value_fields =
        public_values_as_fields(&public_values).expect("public values should flatten");
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let material_segment =
        build_pcs_material_manifest_segment(&schedule).expect("material segment should build");
    let witness_segment = sample_witness_proof_segment(&schedule, 0);
    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );
    let contribution_segment = build_contribution_segment(&entries)
        .expect("contribution segment should build")
        .expect("contribution segment should exist");
    let expected_challenge = derive_global_challenge_from_contributions(
        &catalog.layout.global_info,
        &public_value_fields,
        &[],
        &entries,
    )
    .expect("challenge should derive");
    let bad_challenge_segment = ProofSegment {
        id: CHALLENGE_VALUES_SEGMENT_ID,
        data: encode_challenge_values_segment(&ChallengeValuesSegment {
            values: vec![[
                expected_challenge.c0.to_u64() + 1,
                expected_challenge.c1.to_u64(),
                expected_challenge.c2.to_u64(),
            ]],
        })
        .expect("challenge values segment should encode"),
    };
    let query_segment = build_pcs_query_plan_segment_with_bindings(
        &schedule,
        public_values_digest(&public_values).expect("digest should compute"),
        &material_segment,
        std::slice::from_ref(&witness_segment),
        std::slice::from_ref(&bad_challenge_segment),
    )
    .expect("query segment should build");
    let constant_opening_segment =
        build_constant_opening_segment(&catalog, &schedule, &query_segment)
            .expect("constant opening segment should build");
    let opening_segment = sample_witness_opening_segment(&schedule, &query_segment, 0);
    let proof = ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            material_segment,
            query_segment,
            constant_opening_segment,
            opening_segment,
            witness_segment,
            contribution_segment,
            bad_challenge_segment,
        ],
    };
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: contribution challenge values mismatch\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_unexpected_proof_values_segment() {
    let dir = temp_dir("verify-setup-preflight-unexpected-proof-values");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    proof
        .segments
        .push(sample_pcs_proof_values_segment(vec![[1, 2, 3]]));
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: unexpected PCS proof values segment\n"
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_setup_aware_verify_preflight_with_unexpected_group_values_segment() {
    let dir = temp_dir("verify-setup-preflight-unexpected-group-values");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    proof
        .segments
        .push(sample_group_values_segment(vec![[1, 2, 3]]));
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: unexpected group values segment\n"
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_setup_aware_verify_preflight_with_unexpected_unit_values_segment() {
    let dir = temp_dir("verify-setup-preflight-unexpected-unit-values");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    proof.segments.push(sample_unit_values_segment(0, vec![1]));
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: unexpected unit values segment for unit 0\n"
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_setup_aware_verify_preflight_with_missing_declared_proof_values() {
    let dir = temp_dir("verify-setup-preflight-missing-declared-proof-values");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_proof_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let proof = sample_proof_with_material(&public_values, &catalog);
    let (proof_path, public_values_path) = write_preflight_artifacts(&dir, &proof, &public_values);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: missing PCS proof values segment\n"
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_setup_aware_verify_preflight_with_missing_declared_group_values() {
    let dir = temp_dir("verify-setup-preflight-missing-declared-group-values");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory_with_group_value(&dir);
    run_generate_key_command(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let proof = sample_proof_with_material(&public_values, &catalog);
    let (proof_path, public_values_path) = write_preflight_artifacts(&dir, &proof, &public_values);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: missing group values segment\n"
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn validates_setup_aware_verify_proof_with_proof_values() {
    let dir = temp_dir("verify-proof-with-proof-values");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, segment_count) =
        write_proof_value_query_preflight_fixture(&dir, Some(vec![[51, 52, 53]]));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "proof",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(segment_count, &public_values_path)
    );
    assert!(stderr.is_empty());

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn validates_setup_aware_verify_preflight_with_unit_values() {
    let dir = temp_dir("verify-setup-preflight-unit-values");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, segment_count) =
        write_unit_value_query_preflight_fixture(&dir, Some(vec![101, 201, 202, 203]));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(segment_count, &public_values_path)
    );
    assert!(stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn validates_setup_aware_verify_preflight_with_global_constraints() {
    let dir = temp_dir("verify-setup-preflight-global-constraint");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, segment_count) =
        write_global_constraint_preflight_fixture(&dir, [51, 52, 53]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(segment_count, &public_values_path)
    );
    assert!(stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn validates_setup_aware_verify_preflight_with_global_hints() {
    let dir = temp_dir("verify-setup-preflight-global-hint");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, segment_count) =
        write_global_hint_preflight_fixture(&dir, Some(vec![[51, 52, 53]]));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(segment_count, &public_values_path)
    );
    assert!(stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn validates_setup_aware_verify_preflight_with_challenge_global_constraints() {
    let dir = temp_dir("verify-setup-preflight-challenge-global-constraint");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, segment_count) =
        write_challenge_global_constraint_preflight_fixture(&dir);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(segment_count, &public_values_path)
    );
    assert!(stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn validates_setup_aware_verify_preflight_with_group_value_global_constraints() {
    let dir = temp_dir("verify-setup-preflight-group-value-global-constraint");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, segment_count) =
        write_group_value_global_constraint_preflight_fixture(&dir, [61, 62, 63]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        expected_setup_verify_stdout(segment_count, &public_values_path)
    );
    assert!(stderr.is_empty());
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_setup_aware_verify_preflight_with_bad_global_constraint() {
    let dir = temp_dir("verify-setup-preflight-bad-global-constraint");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, _) =
        write_global_constraint_preflight_fixture(&dir, [50, 52, 53]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: global constraint 0 is not satisfied\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_missing_global_hint_proof_values() {
    let dir = temp_dir("verify-setup-preflight-missing-global-hint-proof-values");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, _) = write_global_hint_preflight_fixture(&dir, None);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: missing PCS proof values segment\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_unbalanced_global_lookup_hints() {
    let dir = temp_dir("verify-setup-preflight-unbalanced-global-lookup-hints");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, _) = write_global_lookup_hint_preflight_fixture(&dir);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: unbalanced lookup bus 7 tuple 11 has net weight 1\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_missing_proof_values() {
    let dir = temp_dir("verify-setup-preflight-missing-proof-values");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, _) = write_proof_value_query_preflight_fixture(&dir, None);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: missing PCS proof values segment\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_missing_unit_values() {
    let dir = temp_dir("verify-setup-preflight-missing-unit-values");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, _) = write_unit_value_query_preflight_fixture(&dir, None);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: missing unit values segment\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_wrong_proof_value_count() {
    let dir = temp_dir("verify-setup-preflight-bad-proof-value-count");
    let _ = fs::remove_dir_all(&dir);
    let (proof_path, public_values_path, _) =
        write_proof_value_query_preflight_fixture(&dir, Some(vec![[51, 52, 53], [1, 2, 3]]));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: PCS proof values segment count mismatch: expected 1, found 2\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_bad_fri_fold_chain() {
    let dir = temp_dir("verify-setup-preflight-bad-fri-fold");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_value_fields = public_values
        .values
        .iter()
        .flat_map(|entry| entry.elements.iter().copied().map(Felt::from_u64))
        .collect::<Vec<_>>();
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let material_segment =
        build_pcs_material_manifest_segment(&schedule).expect("material segment should build");
    let material = parse_pcs_material_manifest_segment(&material_segment.data)
        .expect("material segment should parse")
        .units[0]
        .clone();
    let witness_segment = sample_witness_proof_segment(&schedule, 0);
    let witness = parse_witness_commitment_segment(&witness_segment.data)
        .expect("witness segment should parse");
    let evaluation_segment = sample_pcs_evaluation_segment(0);
    let evaluations = parse_pcs_evaluation_segment(&evaluation_segment.data)
        .expect("evaluation segment should parse")
        .units[0]
        .clone();
    let fri_unit = sample_stable_pcs_fri_opening_unit(&schedule, &[0], 0);
    let transcript_inputs = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &public_value_fields,
        unit_values: &[],
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri_unit,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: &[],
    };
    let nonce_segment =
        build_pcs_query_nonce_segment_from_transcript_segments(&schedule, transcript_inputs)
            .expect("nonce segment should build");
    let query_segment = build_pcs_query_plan_segment_from_transcript_segments(
        &schedule,
        std::slice::from_ref(&witness_segment),
        transcript_inputs,
        &nonce_segment,
    )
    .expect("query segment should build");
    let constant_opening_segment =
        build_constant_opening_segment(&catalog, &schedule, &query_segment)
            .expect("constant opening segment should build");
    let opening_segment = sample_witness_opening_segment(&schedule, &query_segment, 0);
    let fri_segment = sample_stable_pcs_fri_opening_segment(&schedule, &query_segment, 0);
    let proof = ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            material_segment,
            query_segment,
            constant_opening_segment,
            opening_segment,
            witness_segment,
            evaluation_segment,
            fri_segment,
            nonce_segment,
        ],
    };
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: PCS FRI opening segment mismatch for unit 0\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_bad_query_output() {
    let dir = temp_dir("verify-setup-preflight-bad-query-output");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_value_fields = public_values
        .values
        .iter()
        .flat_map(|entry| entry.elements.iter().copied().map(Felt::from_u64))
        .collect::<Vec<_>>();
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let material_segment =
        build_pcs_material_manifest_segment(&schedule).expect("material segment should build");
    let material = parse_pcs_material_manifest_segment(&material_segment.data)
        .expect("material segment should parse")
        .units[0]
        .clone();
    let witness_segment = sample_witness_proof_segment(&schedule, 0);
    let witness = parse_witness_commitment_segment(&witness_segment.data)
        .expect("witness segment should parse");
    let evaluation_segment = sample_pcs_evaluation_segment(0);
    let evaluations = parse_pcs_evaluation_segment(&evaluation_segment.data)
        .expect("evaluation segment should parse")
        .units[0]
        .clone();
    let fri_unit = sample_folded_pcs_fri_opening_template_with_values(
        &schedule,
        &material,
        &public_value_fields,
        &witness,
        &evaluations,
        0,
        [11, 12, 13],
    );
    let transcript_inputs = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &public_value_fields,
        unit_values: &[],
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri_unit,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: &[],
    };
    let nonce_segment =
        build_pcs_query_nonce_segment_from_transcript_segments(&schedule, transcript_inputs)
            .expect("nonce segment should build");
    let query_segment = build_pcs_query_plan_segment_from_transcript_segments(
        &schedule,
        std::slice::from_ref(&witness_segment),
        transcript_inputs,
        &nonce_segment,
    )
    .expect("query segment should build");
    let constant_opening_segment =
        build_constant_opening_segment(&catalog, &schedule, &query_segment)
            .expect("constant opening segment should build");
    let opening_segment = sample_witness_opening_segment(&schedule, &query_segment, 0);
    let fri_segment = sample_folded_pcs_fri_opening_segment_with_values(
        &schedule,
        &query_segment,
        0,
        fri_unit,
        [11, 12, 13],
    );
    let proof = ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            material_segment,
            query_segment,
            constant_opening_segment,
            opening_segment,
            witness_segment,
            evaluation_segment,
            fri_segment,
            nonce_segment,
        ],
    };
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: PCS FRI opening segment mismatch for unit 0\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_wrong_evaluation_value_count() {
    let dir = temp_dir("verify-setup-preflight-bad-evaluation-count");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let public_value_fields = public_values
        .values
        .iter()
        .flat_map(|entry| entry.elements.iter().copied().map(Felt::from_u64))
        .collect::<Vec<_>>();
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let material_segment =
        build_pcs_material_manifest_segment(&schedule).expect("material segment should build");
    let material = parse_pcs_material_manifest_segment(&material_segment.data)
        .expect("material segment should parse")
        .units[0]
        .clone();
    let witness_segment = sample_witness_proof_segment(&schedule, 0);
    let witness = parse_witness_commitment_segment(&witness_segment.data)
        .expect("witness segment should parse");
    let evaluation_segment = sample_pcs_evaluation_segment_with_values(0, vec![[31, 32, 33]]);
    let evaluations = parse_pcs_evaluation_segment(&evaluation_segment.data)
        .expect("evaluation segment should parse")
        .units[0]
        .clone();
    let fri_unit = sample_stable_pcs_fri_opening_unit(&schedule, &[0], 0);
    let transcript_inputs = PcsTranscriptSegmentInputs {
        unit_index: 0,
        unit: &schedule.units[0],
        material: &material,
        public_values: &public_value_fields,
        unit_values: &[],
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri_unit,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
        binding_segments: &[],
    };
    let nonce_segment =
        build_pcs_query_nonce_segment_from_transcript_segments(&schedule, transcript_inputs)
            .expect("nonce segment should build");
    let query_segment = build_pcs_query_plan_segment_from_transcript_segments(
        &schedule,
        std::slice::from_ref(&witness_segment),
        transcript_inputs,
        &nonce_segment,
    )
    .expect("query segment should build");
    let constant_opening_segment =
        build_constant_opening_segment(&catalog, &schedule, &query_segment)
            .expect("constant opening segment should build");
    let opening_segment = sample_witness_opening_segment(&schedule, &query_segment, 0);
    let fri_segment = sample_stable_pcs_fri_opening_segment(&schedule, &query_segment, 0);
    let proof = ProofArtifact {
        setup_hash: public_values.setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            material_segment,
            query_segment,
            constant_opening_segment,
            opening_segment,
            witness_segment,
            evaluation_segment,
            fri_segment,
            nonce_segment,
        ],
    };
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: PCS evaluation segment value count mismatch for unit 0\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_mismatched_pcs_material_manifest() {
    let dir = temp_dir("verify-setup-preflight-bad-material");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    proof.segments[0].data[16] ^= 0x01;
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: PCS material manifest mismatch for unit 0\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_mismatched_pcs_query_plan() {
    let dir = temp_dir("verify-setup-preflight-bad-query");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    proof.segments[1].data[20] ^= 0x01;
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: PCS query plan segment mismatch\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_mismatched_program_image_cache_setup_hash() {
    let dir = temp_dir("verify-setup-preflight-bad-program-image-cache");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    let cache = ProgramImageCommitmentCache {
        program_digest: [0x11; 32],
        source_image_digest: [0x12; 32],
        constraint_system_digest: [0x44; 32],
        tree_root: [11, 12, 13, 14],
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cpu,
    };
    proof.segments.push(ProofSegment {
        id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
        data: encode_program_image_cache_segment(&cache).expect("cache segment should encode"),
    });
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: program image cache setup hash mismatch\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_mismatched_constant_opening() {
    let dir = temp_dir("verify-setup-preflight-bad-constant-opening");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    proof.segments[2].data[36] ^= 0x01;
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: constant opening segment mismatch for unit 0\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_unbound_bad_pcs_fri_opening() {
    let dir = temp_dir("verify-setup-preflight-bad-fri-opening");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    let schedule = derive_prove_schedule(&catalog).expect("schedule should derive");
    let mut fri_segment = sample_pcs_fri_opening_segment(&schedule, &proof.segments[1], 0);
    let mut opening =
        parse_pcs_fri_opening_segment(&fri_segment.data).expect("FRI opening should parse");
    opening.units[0].layers[0].queries[0].values[0][0] ^= 1;
    fri_segment.data = encode_pcs_fri_opening_segment(&opening).expect("FRI opening should encode");
    proof.segments.push(fri_segment);
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: PCS FRI opening segment requires transcript query inputs\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_mismatched_witness_opening() {
    let dir = temp_dir("verify-setup-preflight-bad-opening");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    proof.segments[3].data[32] ^= 0x01;
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: witness opening segment mismatch for unit 0\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_invalid_witness_segment() {
    let dir = temp_dir("verify-setup-preflight-bad-witness");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let proof = sample_proof_with_material_and_bad_witness(&public_values, &catalog);
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: invalid witness commitment segment for unit 0: invalid witness commitment segment magic\n"
    );
}

#[test]
fn rejects_setup_aware_verify_preflight_with_mismatched_setup_catalog() {
    let dir = temp_dir("verify-setup-preflight-bad-setup");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let (proof_path, public_values_path) = write_proof_pair(&dir, [0x88; 32]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            dir.to_str().expect("path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: setup catalog fingerprint mismatch\n"
    );
}

#[test]
fn verifies_contribution_challenge_from_proof_artifact() {
    let dir = temp_dir("verify-contribution");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should parse");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);
    assert_eq!(catalog.layout.global_info.stage_one_proof_value_count(), 0);

    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );
    let public_fields =
        public_values_as_fields(&public_values).expect("public values should flatten");
    let expected_challenge = derive_global_challenge_from_contributions(
        &catalog.layout.global_info,
        &public_fields,
        &[],
        &entries,
    )
    .expect("challenge should derive");
    let contribution_segment = build_contribution_segment(&entries)
        .expect("contribution segment should build")
        .expect("contribution segment should exist");
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            contribution_segment,
            sample_challenge_values_segment(expected_challenge.to_u64s()),
        ],
    };
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let public_values_hash = public_values_digest(&public_values).expect("digest should compute");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "contribution",
            dir.to_str().expect("setup path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert!(stderr.is_empty());
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nsegments=2\npublic_values=1\npublic_values_hash={}\npublic_value_fields=1\nproof_values=0\ncontributions=2\ncontribution_challenge={},{},{}\n",
            format_hash(&public_values_hash),
            expected_challenge.c0.to_u64(),
            expected_challenge.c1.to_u64(),
            expected_challenge.c2.to_u64()
        )
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_verify_contribution_without_embedded_challenge_values() {
    let dir = temp_dir("verify-contribution-missing-challenge");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should parse");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);

    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![build_contribution_segment(&entries)
            .expect("contribution segment should build")
            .expect("contribution segment should exist")],
    };
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "contribution",
            dir.to_str().expect("setup path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify contribution failed: missing contribution challenge values\n"
    );
}

#[test]
fn verifies_contribution_reports_bound_program_image_and_eth_block_input() {
    let dir = temp_dir("verify-contribution-bound-inputs");
    let _ = fs::remove_dir_all(&dir);
    let block_input = build_eth_block_input(&sample_block_rlp_with_extra_fields())
        .expect("block input should build");
    write_setup_directory_with_public_values(&dir, &eth_block_public_values_metadata(&block_input));
    let catalog = read_key_directory_catalog(&dir).expect("catalog should parse");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = public_values_from_eth_block_input(setup_hash, &block_input);
    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );
    let cache = ProgramImageCommitmentCache {
        program_digest: [0x11; 32],
        source_image_digest: [0x12; 32],
        constraint_system_digest: setup_hash,
        tree_root: [11, 12, 13, 14],
        trace_row_count: 1024,
        trace_column_count: 17,
        blowup_factor: 8,
        merkle_tree_arity: 4,
        gpu_mode: ProgramImageGpuMode::Cpu,
    };
    let cache_segment_data =
        encode_program_image_cache_segment(&cache).expect("cache segment should encode");
    let cache_segment_hash = program_image_cache_segment_digest(&cache_segment_data);
    let block_segment_data =
        encode_eth_block_input_segment(&block_input).expect("block segment should encode");
    let block_segment_hash = eth_block_input_bytes_digest(&block_segment_data);
    let mut proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            build_contribution_segment(&entries)
                .expect("contribution segment should build")
                .expect("contribution segment should exist"),
            ProofSegment {
                id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
                data: cache_segment_data,
            },
            ProofSegment {
                id: ETH_BLOCK_INPUT_SEGMENT_ID,
                data: block_segment_data,
            },
        ],
    };
    let public_fields =
        public_values_as_fields(&public_values).expect("public values should flatten");
    let expected_challenge = derive_global_challenge_from_proof_segments(
        &catalog.layout.global_info,
        &public_fields,
        &[],
        &proof.segments,
    )
    .expect("challenge should derive");
    proof.segments.push(sample_challenge_values_segment(
        expected_challenge.to_u64s(),
    ));
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    let challenge_values_path = dir.join("challenge_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "contribution",
            dir.to_str().expect("setup path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let mut writer_stdout = Vec::new();
    let mut writer_stderr = Vec::new();
    let writer_code = run_cli(
        &[
            "prove",
            "write-contribution-challenges",
            dir.to_str().expect("setup path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
            challenge_values_path
                .to_str()
                .expect("challenge path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
        ],
        &mut writer_stdout,
        &mut writer_stderr,
    );

    let mut challenge_stdout = Vec::new();
    let mut challenge_stderr = Vec::new();
    let challenge_code = run_cli(
        &[
            "verify",
            "contribution-challenge",
            dir.to_str().expect("setup path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
            challenge_values_path
                .to_str()
                .expect("challenge path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
        ],
        &mut challenge_stdout,
        &mut challenge_stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert_eq!(
        writer_code,
        0,
        "{}",
        String::from_utf8_lossy(&writer_stderr)
    );
    assert!(writer_stderr.is_empty());
    let writer_stdout_text =
        String::from_utf8(writer_stdout).expect("writer stdout should be utf-8");
    assert_eq!(
        challenge_code,
        0,
        "{}",
        String::from_utf8_lossy(&challenge_stderr)
    );
    assert!(challenge_stderr.is_empty());
    let challenge_stdout_text =
        String::from_utf8(challenge_stdout).expect("challenge stdout should be utf-8");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(stdout_text.contains("program_image_caches=1\n"));
    assert!(stdout_text.contains(&format!(
        "program_image_cache_segment_hash={}\n",
        format_hash(&cache_segment_hash)
    )));
    assert!(stdout_text.contains(&format!(
        "program_image_cache_program_digest={}\n",
        format_hash(&cache.program_digest)
    )));
    assert!(stdout_text.contains(&format!(
        "program_image_cache_source_image_digest={}\n",
        format_hash(&cache.source_image_digest)
    )));
    assert!(stdout_text.contains(&format!(
        "program_image_cache_constraint_system_digest={}\n",
        format_hash(&setup_hash)
    )));
    assert!(stdout_text.contains("program_image_cache_tree_root=11,12,13,14\n"));
    assert!(stdout_text.contains("program_image_cache_trace_rows=1024\n"));
    assert!(stdout_text.contains("program_image_cache_trace_columns=17\n"));
    assert!(stdout_text.contains("program_image_cache_blowup_factor=8\n"));
    assert!(stdout_text.contains("program_image_cache_arity=4\n"));
    assert!(stdout_text.contains("program_image_cache_gpu_mode=cpu\n"));
    assert!(stdout_text.contains("eth_block_inputs=1\n"));
    assert!(stdout_text.contains(&format!(
        "eth_block_input_hash={}\n",
        format_hash(&block_segment_hash)
    )));
    assert!(stdout_text.contains(&format!(
        "eth_block_input_bytes={}\n",
        proof
            .segments
            .iter()
            .find(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
            .expect("block segment should exist")
            .data
            .len()
    )));
    assert!(stdout_text.contains(&format!(
        "eth_block_rlp_bytes={}\n",
        block_input.block_rlp.len()
    )));
    assert!(stdout_text.contains("eth_extra_header_fields=1\neth_extra_body_fields=1\n"));
    assert_eth_block_binding_summary(
        &stdout_text,
        &block_input,
        &block_segment_hash,
        proof
            .segments
            .iter()
            .find(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
            .expect("block segment should exist")
            .data
            .len(),
    );
    assert_eth_block_binding_summary(
        &writer_stdout_text,
        &block_input,
        &block_segment_hash,
        proof
            .segments
            .iter()
            .find(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
            .expect("block segment should exist")
            .data
            .len(),
    );
    assert_eth_block_binding_summary(
        &challenge_stdout_text,
        &block_input,
        &block_segment_hash,
        proof
            .segments
            .iter()
            .find(|segment| segment.id == ETH_BLOCK_INPUT_SEGMENT_ID)
            .expect("block segment should exist")
            .data
            .len(),
    );
    assert!(stdout_text.contains(&format!(
        "contribution_challenge={},{},{}\n",
        expected_challenge.c0.to_u64(),
        expected_challenge.c1.to_u64(),
        expected_challenge.c2.to_u64()
    )));
}

#[test]
fn rejects_verify_contribution_with_mismatched_embedded_challenge() {
    let dir = temp_dir("verify-contribution-mismatched-challenge");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should parse");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);

    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );
    let contribution_segment = build_contribution_segment(&entries)
        .expect("contribution segment should build")
        .expect("contribution segment should exist");
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            contribution_segment,
            sample_challenge_values_segment([1, 2, 3]),
        ],
    };
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "contribution",
            dir.to_str().expect("setup path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify contribution failed: contribution challenge values mismatch\n"
    );
}

#[test]
fn rejects_verify_contribution_with_unexpected_segment() {
    let dir = temp_dir("verify-contribution-unexpected-segment");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should parse");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);
    assert_eq!(catalog.layout.global_info.stage_one_proof_value_count(), 0);

    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );
    let contribution_segment = build_contribution_segment(&entries)
        .expect("contribution segment should build")
        .expect("contribution segment should exist");
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            contribution_segment,
            ProofSegment {
                id: 99_999,
                data: vec![1],
            },
        ],
    };
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "contribution",
            dir.to_str().expect("setup path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify contribution failed: unexpected contribution proof segment id 99999\n"
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_verify_contribution_with_irrelevant_setup_segment() {
    let dir = temp_dir("verify-contribution-irrelevant-setup-segment");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should parse");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);

    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );
    let mut proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            build_contribution_segment(&entries)
                .expect("contribution segment should build")
                .expect("contribution segment should exist"),
            ProofSegment {
                id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
                data: vec![1],
            },
        ],
    };
    let public_fields =
        public_values_as_fields(&public_values).expect("public values should flatten");
    let expected_challenge = derive_global_challenge_from_proof_segments(
        &catalog.layout.global_info,
        &public_fields,
        &[],
        &proof.segments,
    )
    .expect("challenge should derive");
    proof.segments.push(sample_challenge_values_segment(
        expected_challenge.to_u64s(),
    ));
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "contribution",
            dir.to_str().expect("setup path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        format!(
            "verify contribution failed: unexpected contribution proof segment id {PCS_MATERIAL_MANIFEST_SEGMENT_ID}\n"
        )
    );
}

#[test]
fn verifies_contribution_reports_packed_proof_value_fields() {
    let dir = temp_dir("verify-contribution-proof-values");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory_with_proof_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should parse");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);

    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );
    let contribution_segment = build_contribution_segment(&entries)
        .expect("contribution segment should build")
        .expect("contribution segment should exist");
    let proof_values_segment = sample_pcs_proof_values_segment(vec![[51, 52, 53]]);
    let mut proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![contribution_segment, proof_values_segment],
    };

    let public_fields =
        public_values_as_fields(&public_values).expect("public values should flatten");
    let expected_challenge = derive_global_challenge_from_proof_segments(
        &catalog.layout.global_info,
        &public_fields,
        &[Felt::from_u64(51), Felt::from_u64(52), Felt::from_u64(53)],
        &proof.segments,
    )
    .expect("challenge should derive");
    proof.segments.push(sample_challenge_values_segment(
        expected_challenge.to_u64s(),
    ));
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "contribution",
            dir.to_str().expect("setup path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains("proof_values=3\n"));
    assert!(stdout_text.contains(&format!(
        "contribution_challenge={},{},{}\n",
        expected_challenge.c0.to_u64(),
        expected_challenge.c1.to_u64(),
        expected_challenge.c2.to_u64()
    )));
}

#[test]
fn verifies_contribution_challenge_from_multiple_proof_artifacts() {
    let dir = temp_dir("verify-contribution-set");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should parse");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);
    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );
    let public_fields =
        public_values_as_fields(&public_values).expect("public values should flatten");
    let public_values_hash = public_values_digest(&public_values).expect("digest should compute");
    let expected_challenge = derive_global_challenge_from_contributions(
        &catalog.layout.global_info,
        &public_fields,
        &[],
        &entries,
    )
    .expect("challenge should derive");

    let proof_a = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            build_contribution_segment(&[entries[0].clone()])
                .expect("contribution segment should build")
                .expect("contribution segment should exist"),
            sample_challenge_values_segment(expected_challenge.to_u64s()),
        ],
    };
    let proof_b = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            build_contribution_segment(&[entries[1].clone()])
                .expect("contribution segment should build")
                .expect("contribution segment should exist"),
            sample_challenge_values_segment(expected_challenge.to_u64s()),
        ],
    };
    let proof_a_path = dir.join("proof-a.bin");
    let proof_b_path = dir.join("proof-b.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_a_path,
        encode_proof_artifact(&proof_a).expect("proof should encode"),
    );
    write_bytes(
        &proof_b_path,
        encode_proof_artifact(&proof_b).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "contribution-set",
            dir.to_str().expect("setup path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
            proof_a_path.to_str().expect("proof path should be utf-8"),
            proof_b_path.to_str().expect("proof path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert!(stderr.is_empty());
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nproofs=2\nsegments=4\npublic_values=1\npublic_values_hash={}\npublic_value_fields=1\nproof_values=0\ncontributions=2\ncontribution_challenge={},{},{}\n",
            format_hash(&public_values_hash),
            expected_challenge.c0.to_u64(),
            expected_challenge.c1.to_u64(),
            expected_challenge.c2.to_u64()
        )
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_verify_contribution_set_without_embedded_challenge_values() {
    let dir = temp_dir("verify-contribution-set-missing-challenge");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should parse");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);
    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );

    let proof_a = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![build_contribution_segment(&[entries[0].clone()])
            .expect("contribution segment should build")
            .expect("contribution segment should exist")],
    };
    let proof_b = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![build_contribution_segment(&[entries[1].clone()])
            .expect("contribution segment should build")
            .expect("contribution segment should exist")],
    };
    let proof_a_path = dir.join("proof-a.bin");
    let proof_b_path = dir.join("proof-b.bin");
    let public_values_path = dir.join("public_values.bin");
    write_bytes(
        &proof_a_path,
        encode_proof_artifact(&proof_a).expect("proof should encode"),
    );
    write_bytes(
        &proof_b_path,
        encode_proof_artifact(&proof_b).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "contribution-set",
            dir.to_str().expect("setup path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
            proof_a_path.to_str().expect("proof path should be utf-8"),
            proof_b_path.to_str().expect("proof path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify contribution-set failed: missing contribution challenge values\n"
    );
}

#[test]
fn writes_contribution_challenge_segment_from_multiple_proof_artifacts() {
    let dir = temp_dir("write-contribution-challenges");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should parse");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);
    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );

    let proof_a = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![build_contribution_segment(&[entries[0].clone()])
            .expect("contribution segment should build")
            .expect("contribution segment should exist")],
    };
    let proof_b = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![build_contribution_segment(&[entries[1].clone()])
            .expect("contribution segment should build")
            .expect("contribution segment should exist")],
    };
    let proof_a_path = dir.join("proof-a.bin");
    let proof_b_path = dir.join("proof-b.bin");
    let public_values_path = dir.join("public_values.bin");
    let challenge_segment_path = dir.join("challenge_values_segment.bin");
    write_bytes(
        &proof_a_path,
        encode_proof_artifact(&proof_a).expect("proof should encode"),
    );
    write_bytes(
        &proof_b_path,
        encode_proof_artifact(&proof_b).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let public_fields =
        public_values_as_fields(&public_values).expect("public values should flatten");
    let public_values_hash = public_values_digest(&public_values).expect("digest should compute");
    let expected_challenge = derive_global_challenge_from_contributions(
        &catalog.layout.global_info,
        &public_fields,
        &[],
        &entries,
    )
    .expect("challenge should derive");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "write-contribution-challenges",
            dir.to_str().expect("setup path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
            challenge_segment_path
                .to_str()
                .expect("challenge path should be utf-8"),
            proof_a_path.to_str().expect("proof path should be utf-8"),
            proof_b_path.to_str().expect("proof path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert!(stderr.is_empty());
    let challenge_bytes = fs::read(&challenge_segment_path).expect("challenge output should read");
    let challenge_segment =
        parse_challenge_values_segment(&challenge_bytes).expect("challenge output should parse");
    assert_eq!(
        challenge_segment.values,
        vec![[
            expected_challenge.c0.to_u64(),
            expected_challenge.c1.to_u64(),
            expected_challenge.c2.to_u64(),
        ]]
    );
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nproofs=2\nsegments=2\npublic_values=1\npublic_values_hash={}\npublic_value_fields=1\nproof_values=0\ncontributions=2\nchallenge_values=1\ncontribution_challenge={},{},{}\nbytes_written={}\noutput={}\n",
            format_hash(&public_values_hash),
            expected_challenge.c0.to_u64(),
            expected_challenge.c1.to_u64(),
            expected_challenge.c2.to_u64(),
            challenge_bytes.len(),
            challenge_segment_path.display()
        )
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn verifies_contribution_challenge_segment_from_multiple_proof_artifacts() {
    let dir = temp_dir("verify-contribution-challenge");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should parse");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);
    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );

    let proof_a = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![build_contribution_segment(&[entries[0].clone()])
            .expect("contribution segment should build")
            .expect("contribution segment should exist")],
    };
    let proof_b = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![build_contribution_segment(&[entries[1].clone()])
            .expect("contribution segment should build")
            .expect("contribution segment should exist")],
    };
    let proof_a_path = dir.join("proof-a.bin");
    let proof_b_path = dir.join("proof-b.bin");
    let public_values_path = dir.join("public_values.bin");
    let challenge_segment_path = dir.join("challenge_values_segment.bin");
    let tampered_challenge_segment_path = dir.join("tampered_challenge_values_segment.bin");
    write_bytes(
        &proof_a_path,
        encode_proof_artifact(&proof_a).expect("proof should encode"),
    );
    write_bytes(
        &proof_b_path,
        encode_proof_artifact(&proof_b).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );

    let public_fields =
        public_values_as_fields(&public_values).expect("public values should flatten");
    let public_values_hash = public_values_digest(&public_values).expect("digest should compute");
    let expected_challenge = derive_global_challenge_from_contributions(
        &catalog.layout.global_info,
        &public_fields,
        &[],
        &entries,
    )
    .expect("challenge should derive");
    write_bytes(
        &challenge_segment_path,
        encode_challenge_values_segment(&ChallengeValuesSegment {
            values: vec![[
                expected_challenge.c0.to_u64(),
                expected_challenge.c1.to_u64(),
                expected_challenge.c2.to_u64(),
            ]],
        })
        .expect("challenge values segment should encode"),
    );
    write_bytes(
        &tampered_challenge_segment_path,
        encode_challenge_values_segment(&ChallengeValuesSegment {
            values: vec![[
                expected_challenge.c0.to_u64() + 1,
                expected_challenge.c1.to_u64(),
                expected_challenge.c2.to_u64(),
            ]],
        })
        .expect("challenge values segment should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "contribution-challenge",
            dir.to_str().expect("setup path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
            challenge_segment_path
                .to_str()
                .expect("challenge path should be utf-8"),
            proof_a_path.to_str().expect("proof path should be utf-8"),
            proof_b_path.to_str().expect("proof path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nproofs=2\nsegments=2\npublic_values=1\npublic_values_hash={}\npublic_value_fields=1\nproof_values=0\ncontributions=2\nchallenge_values=1\ncontribution_challenge={},{},{}\n",
            format_hash(&public_values_hash),
            expected_challenge.c0.to_u64(),
            expected_challenge.c1.to_u64(),
            expected_challenge.c2.to_u64()
        )
    );

    let mut tampered_stdout = Vec::new();
    let mut tampered_stderr = Vec::new();
    let tampered_code = run_cli(
        &[
            "verify",
            "contribution-challenge",
            dir.to_str().expect("setup path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
            tampered_challenge_segment_path
                .to_str()
                .expect("challenge path should be utf-8"),
            proof_a_path.to_str().expect("proof path should be utf-8"),
            proof_b_path.to_str().expect("proof path should be utf-8"),
        ],
        &mut tampered_stdout,
        &mut tampered_stderr,
    );

    assert_eq!(tampered_code, 1);
    assert!(tampered_stdout.is_empty());
    assert_eq!(
        String::from_utf8(tampered_stderr).expect("stderr should be utf-8"),
        "verify contribution-challenge failed: contribution challenge values mismatch\n"
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_contribution_challenge_verification_with_mismatched_embedded_challenge() {
    let dir = temp_dir("verify-contribution-challenge-embedded-mismatch");
    let _ = fs::remove_dir_all(&dir);
    write_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should parse");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("catalog digest should compute");
    let public_values = sample_public_values(setup_hash);
    let entries = sample_contribution_entries(
        catalog
            .layout
            .global_info
            .lattice_size
            .expect("lattice size should exist") as usize,
    );
    let public_fields =
        public_values_as_fields(&public_values).expect("public values should flatten");
    let expected_challenge = derive_global_challenge_from_contributions(
        &catalog.layout.global_info,
        &public_fields,
        &[],
        &entries,
    )
    .expect("challenge should derive");

    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![
            build_contribution_segment(&entries)
                .expect("contribution segment should build")
                .expect("contribution segment should exist"),
            ProofSegment {
                id: CHALLENGE_VALUES_SEGMENT_ID,
                data: encode_challenge_values_segment(&ChallengeValuesSegment {
                    values: vec![[1, 2, 3]],
                })
                .expect("challenge values segment should encode"),
            },
        ],
    };
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public_values.bin");
    let challenge_segment_path = dir.join("challenge_values_segment.bin");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );
    write_bytes(
        &challenge_segment_path,
        encode_challenge_values_segment(&ChallengeValuesSegment {
            values: vec![[
                expected_challenge.c0.to_u64(),
                expected_challenge.c1.to_u64(),
                expected_challenge.c2.to_u64(),
            ]],
        })
        .expect("challenge values segment should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "contribution-challenge",
            dir.to_str().expect("setup path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
            challenge_segment_path
                .to_str()
                .expect("challenge path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify contribution-challenge failed: contribution challenge values mismatch\n"
    );
}

#[test]
fn rounds_trip_contribution_challenge_through_witness_run() {
    let dir = temp_dir("round-trip-contribution-challenge");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory_with_proof_group_and_unit_value(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let contribution_output_dir = dir.join("contribution-out");
    let full_output_dir = dir.join("full-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let unit_values_path = dir.join("unit_values.bin");
    let proof_values_path = dir.join("proof_values.bin");
    let group_values_path = dir.join("group_values.bin");
    let public_values_path = dir.join("public_values.bin");
    let challenge_segment_path = dir.join("challenge_values_segment.bin");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [23_u8]);
    write_field_words(&unit_values_path, &[101, 201, 202, 203]);
    write_field_words(&proof_values_path, &[51, 52, 53]);
    write_field_words(&group_values_path, &[61, 62, 63]);
    write_bytes(
        &public_values_path,
        encode_public_values(&sample_public_values(setup_hash))
            .expect("public values should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--contributions",
            "--aggregate",
            "--save-outputs",
            "--unit-values",
            unit_values_path
                .to_str()
                .expect("unit values path should be utf-8"),
            "--proof-values",
            proof_values_path
                .to_str()
                .expect("proof values path should be utf-8"),
            "--group-values",
            group_values_path
                .to_str()
                .expect("group values path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            contribution_output_dir
                .to_str()
                .expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let contribution_proof_path = contribution_output_dir.join("proof.bin");
    let contribution_proof = parse_proof_artifact(
        &fs::read(&contribution_proof_path).expect("contribution proof should read"),
    )
    .expect("contribution proof should parse");
    assert!(contribution_proof
        .segments
        .iter()
        .any(|segment| segment.id == CONTRIBUTION_SEGMENT_ID));

    let mut writer_stdout = Vec::new();
    let mut writer_stderr = Vec::new();
    let writer_code = run_cli(
        &[
            "prove",
            "write-contribution-challenges",
            dir.to_str().expect("setup path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public path should be utf-8"),
            challenge_segment_path
                .to_str()
                .expect("challenge path should be utf-8"),
            contribution_proof_path
                .to_str()
                .expect("proof path should be utf-8"),
        ],
        &mut writer_stdout,
        &mut writer_stderr,
    );
    assert_eq!(
        writer_code,
        0,
        "{}",
        String::from_utf8_lossy(&writer_stderr)
    );
    assert!(writer_stderr.is_empty());
    assert!(challenge_segment_path.exists());

    let mut full_stdout = Vec::new();
    let mut full_stderr = Vec::new();
    let full_code = run_cli(
        &[
            "prove",
            "witness",
            "--aggregate",
            "--save-outputs",
            "--challenge-values-segment",
            challenge_segment_path
                .to_str()
                .expect("challenge path should be utf-8"),
            "--unit-values",
            unit_values_path
                .to_str()
                .expect("unit values path should be utf-8"),
            "--proof-values",
            proof_values_path
                .to_str()
                .expect("proof values path should be utf-8"),
            "--group-values",
            group_values_path
                .to_str()
                .expect("group values path should be utf-8"),
            "--input-data",
            input_data.to_str().expect("input path should be utf-8"),
            dir.to_str().expect("path should be utf-8"),
            full_output_dir
                .to_str()
                .expect("output path should be utf-8"),
            witness_library
                .to_str()
                .expect("witness path should be utf-8"),
            guest_image.to_str().expect("guest path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut full_stdout,
        &mut full_stderr,
    );
    assert_eq!(full_code, 0, "{}", String::from_utf8_lossy(&full_stderr));
    assert!(full_stderr.is_empty());
    let full_proof = parse_proof_artifact(
        &fs::read(full_output_dir.join("proof.bin")).expect("full proof should read"),
    )
    .expect("full proof should parse");
    assert!(full_proof
        .segments
        .iter()
        .any(|segment| segment.id == CONTRIBUTION_SEGMENT_ID));
    assert!(full_proof
        .segments
        .iter()
        .any(|segment| segment.id == CHALLENGE_VALUES_SEGMENT_ID));
    assert!(full_proof
        .segments
        .iter()
        .any(|segment| segment.id == CONSTANT_OPENING_SEGMENT_ID));
    assert!(full_proof
        .segments
        .iter()
        .any(|segment| segment.id == WITNESS_OPENING_SEGMENT_ID));
    assert!(full_proof
        .segments
        .iter()
        .any(|segment| segment.id == PCS_QUERY_PLAN_SEGMENT_ID));
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn reports_usage_for_missing_setup_directory() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["setup", "validate"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup validate <setup-dir>\n"
    );
}

#[test]
fn reports_usage_for_missing_fingerprint_directory() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["setup", "fingerprint"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup fingerprint <setup-dir>\n"
    );
}

#[test]
fn reports_usage_for_missing_prove_schedule_directory() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["prove", "schedule"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm prove schedule <setup-dir>\n"
    );
}

#[test]
fn reports_usage_for_missing_prove_plan_paths() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["prove", "plan"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm prove plan [options] <setup-dir> <output-dir>\n  --contributions\n  --internal-contributions <count>\n"
    );
}

#[test]
fn reports_usage_for_missing_prove_input_paths() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["prove", "inputs"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm prove inputs [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]\n       lzvm prove inputs --trace-bytes <trace-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n       lzvm prove inputs --trace-bundle <bundle-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n       lzvm prove inputs --guest-pc-trace <instruction-limit> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n  --eth-block-input <block-input>\n  --eth-public-input <public-input>\n  --eth-public-input-allow-trailing\n  --program-image-cache <cache-bin>\n  --trace-bytes <trace-bin>\n  --trace-bundle <bundle-bin>\n  --guest-pc-trace <instruction-limit>\n"
    );
}

#[test]
fn reports_usage_for_missing_prove_witness_paths() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["prove", "witness"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm prove witness [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]\n       lzvm prove witness --trace-bytes <trace-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n       lzvm prove witness --trace-bundle <bundle-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n       lzvm prove witness --guest-pc-trace <instruction-limit> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n  --eth-block-input <block-input>\n  --eth-public-input <public-input>\n  --eth-public-input-allow-trailing\n  --program-image-cache <cache-bin>\n  --trace-bytes <trace-bin>\n  --trace-bundle <bundle-bin>\n  --guest-pc-trace <instruction-limit>\n"
    );
}

#[test]
fn prove_inputs_rejects_missing_eth_block_input_value_before_next_option() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--eth-block-input",
            "--trace-bytes",
            "trace.bin",
            "setup",
            "out",
            "guest.elf",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove inputs failed: missing --eth-block-input value\n"
    );
}

#[test]
fn prove_inputs_rejects_missing_program_image_cache_value_before_next_option() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "inputs",
            "--program-image-cache",
            "--eth-block-input",
            "block.input",
            "setup",
            "out",
            "witness.so",
            "guest.elf",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove inputs failed: missing --program-image-cache value\n"
    );
}

#[test]
fn prove_witness_rejects_missing_eth_block_input_value_before_next_option() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-block-input",
            "--trace-bytes",
            "trace.bin",
            "setup",
            "out",
            "guest.elf",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove witness failed: missing --eth-block-input value\n"
    );
}

#[test]
fn prove_witness_rejects_missing_eth_public_input_value_before_next_option() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--eth-public-input",
            "--trace-bytes",
            "trace.bin",
            "setup",
            "out",
            "guest.elf",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove witness failed: missing --eth-public-input value\n"
    );
}

#[test]
fn prove_witness_rejects_missing_program_image_cache_value_before_next_option() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "witness",
            "--program-image-cache",
            "--eth-block-input",
            "block.input",
            "setup",
            "out",
            "witness.so",
            "guest.elf",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove witness failed: missing --program-image-cache value\n"
    );
}

#[test]
fn reports_usage_for_missing_setup_preflight_inputs() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["verify", "setup-preflight"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm verify setup-preflight [--eth-block-input <block-input>] [--eth-public-input <public-input>] [--eth-public-input-allow-trailing] [--program-image-cache <cache-bin>] <setup-dir> <proof-bin> <public-values>\n"
    );
}

#[test]
fn verify_setup_preflight_rejects_missing_eth_public_input_value_before_next_option() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "setup-preflight",
            "--eth-public-input",
            "--program-image-cache",
            "cache.bin",
            "setup",
            "proof.bin",
            "public.bin",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify setup-preflight failed: missing --eth-public-input value\n"
    );
}

#[test]
fn verify_preflight_verifies_eth_block_input_binding() {
    let dir = temp_dir("verify-preflight-eth-block-binding");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let block_input = build_eth_block_input(&sample_block_rlp()).expect("block input should build");
    let block_input_bytes =
        encode_eth_block_input(&block_input).expect("block input should encode");
    let setup_hash = [7; 32];
    let public_values = public_values_from_eth_block_input(setup_hash, &block_input);
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![ProofSegment {
            id: ETH_BLOCK_INPUT_SEGMENT_ID,
            data: encode_eth_block_input_segment(&block_input).expect("segment should encode"),
        }],
    };
    let block_input_path = dir.join("block.input");
    let public_values_path = dir.join("public-values.bin");
    let proof_path = dir.join("proof.bin");
    write_bytes(&block_input_path, &block_input_bytes);
    write_bytes(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    );
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "preflight",
            "--eth-block-input",
            block_input_path
                .to_str()
                .expect("block input path should be utf-8"),
            proof_path.to_str().expect("proof path should be utf-8"),
            public_values_path
                .to_str()
                .expect("public values path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains("eth_block_inputs=1\n"));
    assert!(stdout_text.contains("eth_block_input_match=ok\n"));
}

#[test]
fn reports_usage_for_missing_verify_proof_inputs() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["verify", "proof"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm verify proof [--eth-block-input <block-input>] [--eth-public-input <public-input>] [--eth-public-input-allow-trailing] [--program-image-cache <cache-bin>] <setup-dir> <proof-bin> <public-values>\n"
    );
}

#[test]
fn verify_proof_rejects_unknown_options() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "proof",
            "--unknown",
            "setup",
            "proof.bin",
            "public.bin",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify proof failed: unknown option --unknown\n"
    );
}

#[test]
fn verify_proof_rejects_missing_eth_block_input_value_before_next_option() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "proof",
            "--eth-block-input",
            "--program-image-cache",
            "cache.bin",
            "setup",
            "proof.bin",
            "public.bin",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify proof failed: missing --eth-block-input value\n"
    );
}

#[test]
fn verify_proof_rejects_missing_eth_public_input_value_before_next_option() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "proof",
            "--eth-public-input",
            "--program-image-cache",
            "cache.bin",
            "setup",
            "proof.bin",
            "public.bin",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify proof failed: missing --eth-public-input value\n"
    );
}

#[test]
fn verify_proof_rejects_missing_program_image_cache_value_before_next_option() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "verify",
            "proof",
            "--program-image-cache",
            "--eth-block-input",
            "block.input",
            "setup",
            "proof.bin",
            "public.bin",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "verify proof failed: missing --program-image-cache value\n"
    );
}

fn format_hash(hash: &[u8; 32]) -> String {
    format_hex(hash)
}

fn assert_eth_block_binding_summary(
    stdout_text: &str,
    block_input: &EthBlockInput,
    block_segment_hash: &[u8; 32],
    block_segment_bytes: usize,
) {
    assert!(stdout_text.contains("eth_block_inputs=1\n"));
    assert!(stdout_text.contains(&format!(
        "eth_block_input_hash={}\n",
        format_hash(block_segment_hash)
    )));
    assert!(stdout_text.contains(&format!("eth_block_input_bytes={block_segment_bytes}\n")));
    assert!(stdout_text.contains(&format!(
        "eth_block_rlp_bytes={}\n",
        block_input.block_rlp.len()
    )));
    assert!(stdout_text.contains(&format!(
        "eth_block_hash={}\n",
        format_hash(&block_input.block_hash)
    )));
    assert!(stdout_text.contains(&format!(
        "eth_parent_hash={}\n",
        format_hash(&block_input.parent_hash)
    )));
    assert!(stdout_text.contains(&format!(
        "eth_ommers_hash={}\n",
        format_hash(&block_input.ommers_hash)
    )));
    assert!(stdout_text.contains(&format!(
        "eth_beneficiary={}\n",
        format_hex(&block_input.beneficiary)
    )));
    assert!(stdout_text.contains(&format!(
        "eth_state_root={}\n",
        format_hash(&block_input.state_root)
    )));
    assert!(stdout_text.contains(&format!(
        "eth_receipts_root={}\n",
        format_hash(&block_input.receipts_root)
    )));
    assert!(stdout_text.contains(&format!(
        "eth_logs_bloom={}\n",
        format_hex(&block_input.logs_bloom)
    )));
    assert!(stdout_text.contains(&format!(
        "eth_difficulty={}\n",
        format_u256(&block_input.difficulty)
    )));
    assert!(stdout_text.contains(&format!("eth_block_number={}\n", block_input.block_number)));
    assert!(stdout_text.contains(&format!("eth_block_timestamp={}\n", block_input.timestamp)));
    assert!(stdout_text.contains(&format!(
        "eth_extra_data={}\n",
        format_hex(&block_input.extra_data)
    )));
    assert!(stdout_text.contains(&format!("eth_gas_limit={}\n", block_input.gas_limit)));
    assert!(stdout_text.contains(&format!("eth_gas_used={}\n", block_input.gas_used)));
    assert!(stdout_text.contains(&format!(
        "eth_base_fee_per_gas={}\n",
        format_optional_u256(block_input.base_fee_per_gas.as_ref())
    )));
    assert!(stdout_text.contains(&format!(
        "eth_mix_hash={}\n",
        format_hash(&block_input.mix_hash)
    )));
    assert!(stdout_text.contains(&format!("eth_nonce={}\n", format_hex(&block_input.nonce))));
    assert!(stdout_text.contains(&format!(
        "eth_transactions_root={}\n",
        format_hash(&block_input.transactions_root)
    )));
    assert!(stdout_text.contains(&format!(
        "eth_transaction_trie_preimages={}\n",
        block_input.transactions.hash_preimages.len()
    )));
    match (&block_input.receipts, &block_input.receipts_rlp) {
        (Some(receipts), Some(receipts_rlp)) => {
            assert!(stdout_text.contains("eth_receipts=present\n"));
            assert!(
                stdout_text.contains(&format!("eth_receipts_rlp_bytes={}\n", receipts_rlp.len()))
            );
            assert!(stdout_text.contains(&format!(
                "eth_receipt_trie_preimages={}\n",
                receipts.hash_preimages.len()
            )));
        }
        (None, None) => assert!(stdout_text.contains("eth_receipts=absent\n")),
        _ => panic!("receipt fixture should be internally consistent"),
    }
    match (&block_input.withdrawals_root, &block_input.withdrawals) {
        (Some(root), Some(withdrawals)) => {
            assert!(stdout_text.contains("eth_withdrawals=present\n"));
            assert!(stdout_text.contains(&format!("eth_withdrawals_root={}\n", format_hash(root))));
            assert!(stdout_text.contains(&format!(
                "eth_withdrawal_trie_preimages={}\n",
                withdrawals.hash_preimages.len()
            )));
        }
        (None, None) => assert!(stdout_text.contains("eth_withdrawals=absent\n")),
        _ => panic!("withdrawal fixture should be internally consistent"),
    }
}

fn format_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn format_u256(bytes: &[u8; 32]) -> String {
    match bytes.iter().position(|byte| *byte != 0) {
        Some(index) => format_hex(&bytes[index..]),
        None => "0".to_owned(),
    }
}

fn format_optional_u256(value: Option<&[u8; 32]>) -> String {
    match value {
        Some(bytes) => format_u256(bytes),
        None => "absent".to_owned(),
    }
}

fn expected_setup_verify_stdout(segment_count: usize, public_values_path: &Path) -> String {
    let bytes = fs::read(public_values_path).expect("public values should read");
    let public_values = parse_public_values(&bytes).expect("public values should parse");
    let public_values_hash = public_values_digest(&public_values).expect("digest should compute");
    let public_value_fields = public_values
        .values
        .iter()
        .map(|entry| entry.elements.len())
        .sum::<usize>();
    format!(
        "status=ok\nunits=4\nsegments={segment_count}\npublic_values={}\npublic_values_hash={}\npublic_value_fields={public_value_fields}\n",
        public_values.values.len(),
        format_hash(&public_values_hash)
    )
}
