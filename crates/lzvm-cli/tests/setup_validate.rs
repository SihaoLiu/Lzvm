use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lzvm_artifacts::constant_opening_segment::{
    parse_constant_opening_segment, CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::constraint_program::{
    encode_global_constraint_program, GlobalConstraintEntry, GlobalConstraintProgram,
};
use lzvm_artifacts::expression_program::{
    encode_expression_program, ExpressionEntry, ExpressionProgram,
};
use lzvm_artifacts::guest_image::parse_guest_image;
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
use lzvm_artifacts::pcs_proof_values_segment::{
    encode_pcs_proof_values_segment, parse_pcs_proof_values_segment, PcsProofValuesSegment,
    PCS_PROOF_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::{parse_pcs_query_plan_segment, PCS_QUERY_PLAN_SEGMENT_ID};
use lzvm_artifacts::proof::{
    encode_proof_artifact, parse_proof_artifact, ProofArtifact, ProofSegment,
};
use lzvm_artifacts::public_values::{
    encode_public_values_json, public_values_digest, PublicValueEntry, PublicValues,
};
use lzvm_artifacts::verification_key::{encode_verification_key_binary, VerificationKeyRoot};
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
use lzvm_cli::run_cli;
use lzvm_field::{poseidon2_hash_16, Ext3, Felt};
use lzvm_prover::pcs_fri::verify_fri_fold;
use lzvm_prover::pcs_transcript::{
    derive_pcs_transcript_challenges_from_segments, PcsTranscriptSegmentInputs,
};
use lzvm_prover::{
    build_constant_opening_segment, build_pcs_material_manifest_segment,
    build_pcs_query_nonce_segment_from_transcript_segments, build_pcs_query_plan_segment,
    build_pcs_query_plan_segment_from_transcript_segments, build_witness_commitment_segment,
    build_witness_opening_segment, derive_prove_execution_plan, derive_prove_schedule,
    run_prove_witness_commitments, GpuRunOptions, ProveExecutionInputArtifacts, ProvePartitionPlan,
    ProvePassRequest, ProveRunOptions, ProveRunRequest,
};

fn sample_global_info_json() -> &'static str {
    r#"{
        "name": "sample-program",
        "air_groups": ["group-a"],
        "airs": [[{"name": "unit-a", "num_rows": 2}]],
        "curve": "None",
        "latticeSize": 368,
        "aggTypes": [[]],
        "nPublics": 0,
        "numChallenges": [1],
        "numProofValues": [],
        "publicsMap": [],
        "transcriptArity": 4
    }"#
}

fn sample_global_info_json_with_proof_value() -> &'static str {
    r#"{
        "name": "sample-program",
        "air_groups": ["group-a"],
        "airs": [[{"name": "unit-a", "num_rows": 2}]],
        "curve": "None",
        "latticeSize": 368,
        "aggTypes": [[]],
        "nPublics": 0,
        "numChallenges": [1],
        "numProofValues": [1],
        "proofValuesMap": [
            {"name": "proof-a", "stage": 2}
        ],
        "publicsMap": [],
        "transcriptArity": 4
    }"#
}

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 1,
        "nConstants": 2,
        "nPublics": 0,
        "nConstraints": 0,
        "qDeg": 3,
        "openingPoints": [0],
        "mapSectionsN": {
            "const": 2,
            "cm1": 1,
            "cm2": 1
        },
        "constPolsMap": [
            {"stage": 0, "name": "main.left", "dim": 1, "polsMapId": 0, "stageId": 0},
            {"stage": 0, "name": "main.right", "dim": 1, "polsMapId": 1, "stageId": 1}
        ],
        "challengesMap": [{}, {}, {}],
        "evMap": [{}, {}],
        "boundaries": [],
        "starkStruct": {
            "nBits": 1,
            "nBitsExt": 2,
            "nQueries": 1,
            "steps": [
                {"nBits": 2},
                {"nBits": 1}
            ],
            "hashCommits": true,
            "lastLevelVerification": 2,
            "powBits": 0,
            "merkleTreeArity": 4,
            "verificationHashType": "GL",
            "transcriptArity": 4,
            "merkleTreeCustom": true
        }
    }"#
}

fn sample_verifier_info_json_with_proof_value() -> &'static str {
    r#"{
        "qVerifier": {
            "tmpUsed": 1,
            "code": [
                {
                    "op": "copy",
                    "dest": {"type": "tmp", "id": 0, "dim": 3},
                    "src": [{"type": "number", "value": "1", "dim": 1}]
                }
            ]
        },
        "queryVerifier": {
            "expId": 7,
            "stage": 2,
            "tmpUsed": 1,
            "line": "query-expression",
            "code": [
                {
                    "op": "copy",
                    "dest": {"type": "tmp", "id": 0, "dim": 3},
                    "src": [{"type": "proofvalue", "id": 0, "dim": 3}]
                }
            ]
        }
    }"#
}

fn sample_expression_info_json() -> &'static str {
    r#"{
        "hintsInfo": [],
        "expressionsCode": [
            {
                "expId": 7,
                "stage": 2,
                "line": "query-expression",
                "tmpUsed": 0,
                "code": []
            }
        ],
        "constraints": []
    }"#
}

fn sample_verifier_info_json() -> &'static str {
    r#"{
        "qVerifier": {
            "tmpUsed": 1,
            "code": [
                {
                    "op": "copy",
                    "dest": {"type": "tmp", "id": 0, "dim": 3},
                    "src": [{"type": "number", "value": "1", "dim": 1}]
                }
            ]
        },
        "queryVerifier": {
            "expId": 7,
            "stage": 2,
            "tmpUsed": 1,
            "line": "query-expression",
            "code": [
                {
                    "op": "copy",
                    "dest": {"type": "tmp", "id": 0, "dim": 3},
                    "src": [{"type": "eval", "id": 0, "dim": 3}]
                }
            ]
        }
    }"#
}

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
                vec![[0_u64; 4]; unit.merkle_tree_arity.pow(unit.last_level_verification) as usize];
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
            let last_level = vec![
                leaf_digest;
                unit.merkle_tree_arity.pow(unit.last_level_verification) as usize
            ];
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
    let unit = &schedule.units[unit_index];
    assert_eq!(unit.fri_layers.len(), 1);
    let layer = &unit.fri_layers[0];
    let values = vec![query_value; layer.folding_factor as usize];
    let leaf_digest = sample_fri_value_digest(&values);
    let last_level =
        vec![leaf_digest; unit.merkle_tree_arity.pow(unit.last_level_verification) as usize];
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
        witness,
        evaluations,
        fri: &template,
        root_challenge_draws: &unit.transcript_root_challenge_draws,
        evaluation_challenge_draws: unit.transcript_evaluation_challenge_draws,
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

fn write_text(path: &Path, value: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, value).expect("fixture file should be written");
}

fn write_bytes(path: &Path, value: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, value).expect("fixture file should be written");
}

fn write_field_words(path: &Path, values: &[u64]) {
    let mut bytes = Vec::with_capacity(values.len() * 8);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    write_bytes(path, bytes);
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

fn write_global_files_with_info(root: &Path, global_info: &str) {
    fs::create_dir_all(root).expect("fixture root should be created");
    fs::write(root.join("pilout.globalInfo.json"), global_info)
        .expect("global metadata should be written");
    fs::write(root.join("pilout.globalConstraints.json"), "{}")
        .expect("global constraints metadata should be written");
    let constraints = encode_global_constraint_program(&GlobalConstraintProgram {
        entries: vec![],
        ops: vec![],
        args: vec![],
        numbers: vec![],
    })
    .expect("global constraints should encode");
    fs::write(root.join("pilout.globalConstraints.bin"), constraints)
        .expect("global constraints program should be written");
}

fn write_global_constraint_program(root: &Path, program: GlobalConstraintProgram) {
    let constraints =
        encode_global_constraint_program(&program).expect("global constraints should encode");
    fs::write(root.join("pilout.globalConstraints.bin"), constraints)
        .expect("global constraints program should be written");
}

fn write_global_files(root: &Path) {
    write_global_files_with_info(root, sample_global_info_json());
}

fn write_unit_files_with_verifier_info(unit: &KeyUnitPaths, verifier_info: &str) {
    if let Some(path) = unit.setup_info() {
        write_text(&path, sample_setup_info_json());
    }
    if let Some(path) = unit.expression_info() {
        write_text(&path, sample_expression_info_json());
    }
    if let Some(path) = unit.verifier_info() {
        write_text(&path, verifier_info);
    }

    let program =
        encode_expression_program(&sample_expression_program()).expect("program should encode");
    if let Some(path) = unit.expression_program() {
        write_bytes(&path, &program);
    }
    if let Some(path) = unit.verifier_program() {
        write_bytes(&path, &program);
    }

    write_text(&unit.verification_key_json(), "[1,2,3,4]");
    let root = VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4]);
    write_bytes(
        &unit.verification_key_binary(),
        encode_verification_key_binary(&root).expect("verification key should encode"),
    );
    write_bytes(&unit.fixed_columns, sample_raw_fixed_columns());
}

fn write_unit_files(unit: &KeyUnitPaths) {
    write_unit_files_with_verifier_info(unit, sample_verifier_info_json());
}

fn write_setup_directory(root: &Path) {
    write_global_files(root);
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files(unit);
    }
}

fn write_setup_directory_with_proof_value(root: &Path) {
    write_global_files_with_info(root, sample_global_info_json_with_proof_value());
    let layout = read_key_directory_layout(root).expect("layout should parse");
    for unit in &layout.units {
        write_unit_files_with_verifier_info(unit, sample_verifier_info_json_with_proof_value());
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

fn write_execution_ready_setup_directory(root: &Path) {
    write_setup_directory(root);
    let root = root.to_str().expect("path should be utf-8");
    run_setup_command(&["setup", "write-base-directory", "--derive-verkey", root]);
    run_setup_command(&["setup", "write-pcs-directory", root]);
    run_setup_command(&["setup", "write-pcs-material-directory", root]);
}

fn write_execution_ready_setup_directory_with_proof_value(root: &Path) {
    write_setup_directory_with_proof_value(root);
    let root = root.to_str().expect("path should be utf-8");
    run_setup_command(&["setup", "write-base-directory", "--derive-verkey", root]);
    run_setup_command(&["setup", "write-pcs-directory", root]);
    run_setup_command(&["setup", "write-pcs-material-directory", root]);
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
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri_unit,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
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
    let public_values_path = root.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
    );
    (proof_path, public_values_path, segment_count)
}

fn write_global_constraint_preflight_fixture(
    root: &Path,
    proof_value: [u64; 3],
) -> (PathBuf, PathBuf, usize) {
    write_execution_ready_setup_directory_with_proof_value(root);
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
    let catalog = read_key_directory_catalog(root).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let public_values = sample_public_values(setup_hash);
    let mut proof = sample_proof_with_material(&public_values, &catalog);
    proof
        .segments
        .push(sample_pcs_proof_values_segment(vec![proof_value]));
    let segment_count = proof.segments.len();
    let proof_path = root.join("proof.bin");
    let public_values_path = root.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri_unit,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
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
    let public_values_path = root.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
    let public_values_path = root.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
    let public_values_path = root.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
    );
    (proof_path, public_values_path)
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        "status=ok\nunits=4\nglobal_constraints=0\nfixed_bytes=128\n"
    );
    assert!(stderr.is_empty());
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!("status=ok\nunits=4\nfingerprint={expected}\n")
    );
    assert!(stderr.is_empty());
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits=4\nfixed_bytes=128\npcs_material_units=0\npcs_material_bytes=0\nqueries=4\nmax_extended_domain_bits=2\nsetup_hash={expected}\n"
        )
    );
    assert!(stderr.is_empty());
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
            "--final-wrap",
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
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=0\npcs_material_bytes=0\nqueries=4\nmax_extended_domain_bits=2\npartitions=4\npartition_ids=1,3\nworker=2\ninput_data={}\naggregate=true\nremote_aggregation=false\nfinal_wrap=true\nverify_outputs=true\nsave_outputs=true\nminimal_memory=false\noutput={}\ngpu_preallocate=true\ngpu_streams=8\nwitness_thread_pools=2\nstored_witnesses=3\npack_trace=false\nsetup_hash={expected}\n",
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
fn prints_prove_inputs_for_setup_directory() {
    let dir = temp_dir("prove-inputs");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
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

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npass=full\nunits=4\nfixed_bytes=128\npcs_material_units=4\npcs_material_bytes={material_bytes}\nqueries=4\nmax_extended_domain_bits=2\npartitions=1\npartition_ids=0\nworker=0\ninput_data=none\naggregate=false\nremote_aggregation=false\nfinal_wrap=false\nverify_outputs=true\nsave_outputs=false\nminimal_memory=false\noutput={}\ngpu_preallocate=false\ngpu_streams=20\nwitness_thread_pools=4\nstored_witnesses=4\npack_trace=true\nsetup_hash={expected}\nwitness_library={}\nwitness_library_bytes=64\nwitness_library_machine=62\nwitness_library_digest={}\nguest_image={}\nguest_image_bytes=64\nguest_image_machine=243\nguest_image_entry=2147483648\nguest_image_digest={}\npublic_inputs={}\n",
            output_dir.display(),
            witness_library.display(),
            format_hash(&witness_library_info.digest),
            guest_image.display(),
            format_hash(&guest_image_info.digest),
            public_inputs.display()
        )
    );
    assert!(stderr.is_empty());
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
            witness_library: witness_library.clone(),
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
    let public_values_path = dir.join("public_values.json");
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
            witness_library: witness_library.clone(),
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
    let public_values_path = dir.join("public_values.json");
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
fn rejects_prove_witness_proof_output_with_mismatched_public_inputs() {
    let dir = temp_dir("prove-witness-bad-public-inputs");
    let _ = fs::remove_dir_all(&dir);
    write_execution_ready_setup_directory(&dir);
    let output_dir = dir.join("proof-out");
    let witness_library = build_shared_library(&dir, "witness", sample_witness_source());
    let guest_image = dir.join("guest.elf");
    let input_data = dir.join("input.bin");
    let public_values_path = dir.join("public_values.json");
    write_bytes(&guest_image, sample_guest_image());
    write_bytes(&input_data, [11_u8]);
    write_text(
        &public_values_path,
        &encode_public_values_json(&sample_public_values([0x99; 32]))
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
        "prove witness failed: public inputs setup hash mismatch\n"
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
        "status=ok\nunits=4\nsegments=5\npublic_values=1\n"
    );
    assert!(stderr.is_empty());
}

#[test]
fn validates_setup_aware_verify_preflight_with_pcs_fri_opening() {
    let dir = temp_dir("verify-setup-preflight-fri-opening");
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
    let public_values_path = dir.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        "status=ok\nunits=4\nsegments=6\npublic_values=1\n"
    );
    assert!(stderr.is_empty());
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
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri_unit,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
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
    let public_values_path = dir.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        "status=ok\nunits=4\nsegments=8\npublic_values=1\n"
    );
    assert!(stderr.is_empty());
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!("status=ok\nunits=4\nsegments={segment_count}\npublic_values=1\n")
    );
    assert!(stderr.is_empty());
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!("status=ok\nunits=4\nsegments={segment_count}\npublic_values=1\n")
    );
    assert!(stderr.is_empty());
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!("status=ok\nunits=4\nsegments={segment_count}\npublic_values=1\n")
    );
    assert!(stderr.is_empty());
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
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri_unit,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
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
    let public_values_path = dir.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri_unit,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
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
    let public_values_path = dir.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
        witness: &witness,
        evaluations: &evaluations,
        fri: &fri_unit,
        root_challenge_draws: &schedule.units[0].transcript_root_challenge_draws,
        evaluation_challenge_draws: schedule.units[0].transcript_evaluation_challenge_draws,
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
    let public_values_path = dir.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
    let public_values_path = dir.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
    let public_values_path = dir.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
    let public_values_path = dir.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
fn rejects_setup_aware_verify_preflight_with_mismatched_pcs_fri_opening() {
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
    let public_values_path = dir.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
    let public_values_path = dir.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
    let public_values_path = dir.join("public_values.json");
    write_bytes(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    );
    write_text(
        &public_values_path,
        &encode_public_values_json(&public_values).expect("public values should encode"),
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
        "usage: lzvm prove plan [options] <setup-dir> <output-dir>\n"
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
        "usage: lzvm prove inputs [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]\n"
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
        "usage: lzvm verify setup-preflight <setup-dir> <proof-bin> <public-values-json>\n"
    );
}

fn format_hash(hash: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in hash {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
