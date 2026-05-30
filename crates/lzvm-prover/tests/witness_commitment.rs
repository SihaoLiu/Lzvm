use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, WitnessCommitmentSegment, WitnessCommitmentStageSegment,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
#[cfg(not(feature = "cuda"))]
use lzvm_field::DomainError;
use lzvm_field::{coset_extend_evaluations, poseidon2_hash_16, poseidon2_hash_8, Felt};
#[cfg(feature = "cuda")]
use lzvm_prover::witness_commitment::extend_witness_stage_leaves_with_cuda;
use lzvm_prover::witness_commitment::{
    commit_witness_stage_leaves, commit_witness_trace_stages,
    commit_witness_trace_stages_with_workers, decode_witness_stage_leaf_values,
    extend_witness_trace_stage_values, open_witness_stage_commitment,
    verify_witness_stage_opening_root, WitnessStageCommitmentError, WitnessStageOpeningError,
    WitnessTraceCommitmentError,
};
use lzvm_prover::witness_commitment::{extend_witness_stage_leaves, WitnessStageLeafError};
use lzvm_prover::witness_commitment::{
    load_witness_commitment_segments, LoadWitnessCommitmentSegmentsError,
};
use lzvm_prover::witness_layout::{derive_witness_trace_layout, WitnessTraceLayoutError};
use lzvm_prover::witness_trace::parse_witness_trace;
use lzvm_prover::ProveUnitSchedule;

fn encode_values(values: &[u64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 8);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

fn sample_unit(rows: u64, stage_commit_widths: Vec<u32>) -> ProveUnitSchedule {
    let mut transcript_root_challenge_draws = vec![1; stage_commit_widths.len()];
    if let Some(first) = transcript_root_challenge_draws.first_mut() {
        *first = 2;
    }
    ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: Some(0),
        unit_id: Some(0),
        group_name: Some("group-a".to_owned()),
        unit_name: Some("unit-a".to_owned()),
        base_domain_bits: 1,
        extended_domain_bits: 2,
        base_domain_size: rows,
        extended_domain_size: 4,
        blowup_factor: 2,
        query_count: 2,
        proof_of_work_bits: 0,
        merkle_tree_arity: 2,
        last_level_verification: 0,
        transcript_arity: Some(2),
        hash_commits: true,
        transcript_root_challenge_draws,
        challenge_count: 6,
        evaluation_value_count: 2,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 2,
        constant_width: 1,
        stage_commit_widths,
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0],
        fri_layers: vec![PcsFriLayer {
            input_bits: 2,
            output_bits: 1,
            folding_factor: 2,
        }],
        final_layer_bits: 1,
        fixed_bytes: 16,
        constant_tree_root: None,
        pcs_material_bytes: None,
        pcs_material_plan_digest: None,
        pcs_material_fixed_column_digest: None,
        pcs_material_constant_tree_digest: None,
        pcs_material_constant_tree_root: None,
        pcs_material_fixed_byte_count: None,
        pcs_material_constant_tree_byte_count: None,
        pcs_material_leaf_byte_count: None,
        pcs_material_node_byte_count: None,
    }
}

fn decode_words(bytes: &[u8]) -> Vec<u64> {
    bytes
        .chunks_exact(8)
        .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("chunk length checked")))
        .collect()
}

fn encode_digest_words(out: &mut Vec<u64>, digest: [Felt; 4]) {
    out.extend(digest.into_iter().map(|value| value.to_u64()));
}

fn parent_hash(left: [Felt; 4], right: [Felt; 4]) -> [Felt; 4] {
    let state = poseidon2_hash_8([
        left[0], left[1], left[2], left[3], right[0], right[1], right[2], right[3],
    ]);
    [state[0], state[1], state[2], state[3]]
}

fn manual_expected_tree_words(leaves: &[u8]) -> Vec<u64> {
    let leaf_words = decode_words(leaves);
    let rows = leaf_words
        .chunks_exact(2)
        .map(|row| {
            [
                Felt::from_canonical(row[0]).expect("canonical"),
                Felt::from_canonical(row[1]).expect("canonical"),
                Felt::ZERO,
                Felt::ZERO,
            ]
        })
        .collect::<Vec<_>>();
    let parent_left = parent_hash(rows[0], rows[1]);
    let parent_right = parent_hash(rows[2], rows[3]);
    let root = parent_hash(parent_left, parent_right);

    let mut expected = leaf_words;
    for row in rows {
        encode_digest_words(&mut expected, row);
    }
    encode_digest_words(&mut expected, parent_left);
    encode_digest_words(&mut expected, parent_right);
    encode_digest_words(&mut expected, root);
    expected
}

fn manual_expected_wide_arity4_tree_words(leaves: &[u8]) -> Vec<u64> {
    const ROW_WIDTH: usize = 5;

    let leaf_words = decode_words(leaves);
    let rows = leaf_words
        .chunks_exact(ROW_WIDTH)
        .map(|row| {
            let mut state = [Felt::ZERO; 16];
            for (value, raw) in state.iter_mut().zip(row) {
                *value = Felt::from_canonical(*raw).expect("canonical");
            }
            let state = poseidon2_hash_16(state);
            [state[0], state[1], state[2], state[3]]
        })
        .collect::<Vec<_>>();
    let root_state = poseidon2_hash_16([
        rows[0][0], rows[0][1], rows[0][2], rows[0][3], rows[1][0], rows[1][1], rows[1][2],
        rows[1][3], rows[2][0], rows[2][1], rows[2][2], rows[2][3], rows[3][0], rows[3][1],
        rows[3][2], rows[3][3],
    ]);
    let root = [root_state[0], root_state[1], root_state[2], root_state[3]];

    let mut expected = leaf_words;
    for row in rows {
        encode_digest_words(&mut expected, row);
    }
    encode_digest_words(&mut expected, root);
    expected
}

#[test]
fn loads_witness_commitment_segments_sorted_by_unit() {
    let units = vec![sample_unit(2, vec![1]), sample_unit(2, vec![2])];
    let segments = vec![
        witness_commitment_proof_segment(1, &units[1]),
        witness_commitment_proof_segment(0, &units[0]),
    ];

    let loaded = load_witness_commitment_segments(&units, &segments).expect("segments should load");

    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].id, WITNESS_COMMITMENT_SEGMENT_BASE_ID);
    assert_eq!(loaded[1].id, WITNESS_COMMITMENT_SEGMENT_BASE_ID + 1);
}

#[test]
fn rejects_missing_witness_commitment_segments() {
    let units = vec![sample_unit(2, vec![1])];

    let error = load_witness_commitment_segments(&units, &[])
        .expect_err("at least one witness commitment should be present");

    assert_eq!(error, LoadWitnessCommitmentSegmentsError::MissingSegment);
}

#[test]
fn rejects_duplicate_witness_commitment_segments() {
    let units = vec![sample_unit(2, vec![1])];
    let segment = witness_commitment_proof_segment(0, &units[0]);

    let error = load_witness_commitment_segments(&units, &[segment.clone(), segment])
        .expect_err("duplicate segment should be rejected");

    assert_eq!(
        error.to_string(),
        "duplicate witness commitment segment for unit 0"
    );
}

#[test]
fn rejects_unexpected_witness_commitment_segments() {
    let units = vec![sample_unit(2, vec![1])];
    let expected_segment = witness_commitment_proof_segment(0, &units[0]);
    let unexpected_segment = witness_commitment_proof_segment_with_payload_unit(1, 1, &units[0]);

    let error = load_witness_commitment_segments(&units, &[expected_segment, unexpected_segment])
        .expect_err("unexpected segment should be rejected");

    assert_eq!(
        error.to_string(),
        "unexpected witness commitment segment for unit 1"
    );
}

#[test]
fn rejects_witness_commitment_unit_mismatches() {
    let units = vec![sample_unit(2, vec![1])];
    let segment = witness_commitment_proof_segment_with_payload_unit(0, 1, &units[0]);

    let error = load_witness_commitment_segments(&units, &[segment])
        .expect_err("segment id and payload unit should match");

    assert_eq!(
        error,
        LoadWitnessCommitmentSegmentsError::UnitMismatch { unit_index: 0 }
    );
}

#[test]
fn rejects_witness_commitment_row_count_mismatches() {
    let units = vec![sample_unit(2, vec![1])];
    let mut segment = sample_witness_commitment_segment(0, &units[0]);
    segment.trace_rows = 3;

    let error = load_witness_commitment_segments(
        &units,
        &[encode_witness_commitment_proof_segment(0, &segment)],
    )
    .expect_err("segment row count should match schedule");

    assert_eq!(
        error,
        LoadWitnessCommitmentSegmentsError::RowCountMismatch { unit_index: 0 }
    );
}

#[test]
fn extends_witness_stage_values_into_row_major_leaves() {
    let unit = sample_unit(2, vec![2]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace =
        parse_witness_trace(&encode_values(&[5, 9, 1, 9]), 2, 2).expect("trace should parse");
    let stage = layout.stage_trace(&trace, 1).expect("stage should extract");

    let leaves =
        extend_witness_stage_leaves(&stage, 1, 2).expect("witness stage leaves should extend");

    let column_0 = coset_extend_evaluations(&[Felt::from_u64(5), Felt::from_u64(1)], 1, 2)
        .expect("column should extend");
    let column_1 = coset_extend_evaluations(&[Felt::from_u64(9), Felt::from_u64(9)], 1, 2)
        .expect("column should extend");
    let expected = (0..4)
        .flat_map(|row| [column_0[row].to_u64(), column_1[row].to_u64()])
        .collect::<Vec<_>>();

    assert_eq!(leaves.stage_index(), 1);
    assert_eq!(leaves.source_row_count(), 2);
    assert_eq!(leaves.extended_row_count(), 4);
    assert_eq!(leaves.column_count(), 2);
    assert_eq!(decode_words(leaves.bytes()), expected);
}

#[test]
fn decodes_extended_witness_stage_values_for_expression_inputs() {
    let unit = sample_unit(2, vec![2]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace =
        parse_witness_trace(&encode_values(&[5, 9, 1, 9]), 2, 2).expect("trace should parse");
    let stage = layout.stage_trace(&trace, 1).expect("stage should extract");
    let leaves =
        extend_witness_stage_leaves(&stage, 1, 2).expect("witness stage leaves should extend");

    let values = decode_witness_stage_leaf_values(&leaves).expect("extended values should decode");

    let column_0 = coset_extend_evaluations(&[Felt::from_u64(5), Felt::from_u64(1)], 1, 2)
        .expect("column should extend");
    let column_1 = coset_extend_evaluations(&[Felt::from_u64(9), Felt::from_u64(9)], 1, 2)
        .expect("column should extend");
    let expected = (0..4)
        .flat_map(|row| [column_0[row], column_1[row]])
        .collect::<Vec<_>>();

    assert_eq!(values, expected);
}

#[test]
fn commits_witness_stage_leaves_with_the_native_tree_layout() {
    let unit = sample_unit(2, vec![2]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace =
        parse_witness_trace(&encode_values(&[5, 9, 1, 9]), 2, 2).expect("trace should parse");
    let stage = layout.stage_trace(&trace, 1).expect("stage should extract");
    let leaves =
        extend_witness_stage_leaves(&stage, 1, 2).expect("witness stage leaves should extend");

    let commitment = commit_witness_stage_leaves(&leaves, 2).expect("witness stage should commit");
    let expected_words = manual_expected_tree_words(leaves.bytes());
    let root_words = &expected_words[expected_words.len() - 4..];

    assert_eq!(commitment.stage_index(), 1);
    assert_eq!(commitment.arity(), 2);
    assert_eq!(decode_words(commitment.tree_bytes()), expected_words);
    assert_eq!(
        commitment.root(),
        [
            Felt::from_canonical(root_words[0]).expect("canonical"),
            Felt::from_canonical(root_words[1]).expect("canonical"),
            Felt::from_canonical(root_words[2]).expect("canonical"),
            Felt::from_canonical(root_words[3]).expect("canonical"),
        ]
    );
}

#[test]
fn commits_wide_witness_stage_leaves_with_arity4_hashing() {
    let unit = sample_unit(2, vec![5]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace = parse_witness_trace(&encode_values(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]), 2, 5)
        .expect("trace should parse");
    let stage = layout.stage_trace(&trace, 1).expect("stage should extract");
    let leaves =
        extend_witness_stage_leaves(&stage, 1, 2).expect("witness stage leaves should extend");

    let commitment = commit_witness_stage_leaves(&leaves, 4).expect("witness stage should commit");
    let expected_words = manual_expected_wide_arity4_tree_words(leaves.bytes());
    let root_words = &expected_words[expected_words.len() - 4..];

    assert_eq!(commitment.stage_index(), 1);
    assert_eq!(commitment.arity(), 4);
    assert_eq!(decode_words(commitment.tree_bytes()), expected_words);
    assert_eq!(
        commitment.root(),
        [
            Felt::from_canonical(root_words[0]).expect("canonical"),
            Felt::from_canonical(root_words[1]).expect("canonical"),
            Felt::from_canonical(root_words[2]).expect("canonical"),
            Felt::from_canonical(root_words[3]).expect("canonical"),
        ]
    );
}

#[test]
fn opens_and_verifies_witness_stage_commitments() {
    let unit = sample_unit(2, vec![2]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace =
        parse_witness_trace(&encode_values(&[5, 9, 1, 9]), 2, 2).expect("trace should parse");
    let stage = layout.stage_trace(&trace, 1).expect("stage should extract");
    let leaves =
        extend_witness_stage_leaves(&stage, 1, 2).expect("witness stage leaves should extend");
    let commitment = commit_witness_stage_leaves(&leaves, 2).expect("witness stage should commit");

    let opening = open_witness_stage_commitment(&commitment, 2, 4, 2)
        .expect("witness stage opening should build");
    let expected_values = decode_words(leaves.bytes())[4..6]
        .iter()
        .map(|value| Felt::from_canonical(*value).expect("canonical"))
        .collect::<Vec<_>>();
    let mut bad_root = commitment.root();
    bad_root[0] = bad_root[0] + Felt::ONE;

    assert_eq!(opening.row_index(), 2);
    assert_eq!(opening.values(), expected_values);
    assert_eq!(opening.siblings().len(), 2);
    assert!(
        verify_witness_stage_opening_root(commitment.root(), commitment.arity(), &opening)
            .expect("opening should verify")
    );
    assert!(
        !verify_witness_stage_opening_root(bad_root, commitment.arity(), &opening)
            .expect("opening should check")
    );
}

#[test]
fn rejects_witness_stage_openings_outside_the_domain() {
    let unit = sample_unit(2, vec![2]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace =
        parse_witness_trace(&encode_values(&[5, 9, 1, 9]), 2, 2).expect("trace should parse");
    let stage = layout.stage_trace(&trace, 1).expect("stage should extract");
    let leaves =
        extend_witness_stage_leaves(&stage, 1, 2).expect("witness stage leaves should extend");
    let commitment = commit_witness_stage_leaves(&leaves, 2).expect("witness stage should commit");

    assert!(matches!(
        open_witness_stage_commitment(&commitment, 4, 4, 2),
        Err(WitnessStageOpeningError::RowOutOfRange {
            row_index: 4,
            row_count: 4
        })
    ));
}

#[test]
fn commits_all_witness_trace_stages_from_the_unit_schedule() {
    let unit = sample_unit(2, vec![2, 1]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace = parse_witness_trace(&encode_values(&[5, 9, 11, 1, 9, 13]), 2, 3)
        .expect("trace should parse");

    let commitments =
        commit_witness_trace_stages(&trace, &unit).expect("trace stages should commit");

    assert_eq!(commitments.stage_count(), 2);
    assert_eq!(commitments.commitments()[0].stage_index(), 1);
    assert_eq!(commitments.commitments()[1].stage_index(), 2);

    for stage_index in 1..=2 {
        let stage = layout
            .stage_trace(&trace, stage_index)
            .expect("stage should extract");
        let leaves = extend_witness_stage_leaves(&stage, 1, 2).expect("stage leaves should extend");
        let expected =
            commit_witness_stage_leaves(&leaves, 2).expect("stage should commit directly");

        assert_eq!(&commitments.commitments()[stage_index - 1], &expected);
    }
}

#[test]
fn worker_count_preserves_witness_trace_commitments() {
    let unit = sample_unit(2, vec![2, 1]);
    let trace = parse_witness_trace(&encode_values(&[5, 9, 11, 1, 9, 13]), 2, 3)
        .expect("trace should parse");

    let serial =
        commit_witness_trace_stages(&trace, &unit).expect("serial trace stages should commit");
    let threaded = commit_witness_trace_stages_with_workers(&trace, &unit, 2)
        .expect("threaded trace stages should commit");

    assert_eq!(threaded, serial);
    assert_eq!(threaded.stage_count(), 2);
    assert_eq!(threaded.commitments()[0].stage_index(), 1);
    assert_eq!(threaded.commitments()[1].stage_index(), 2);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_coset_extension_matches_cpu_for_witness_stage_leaves() {
    let unit = sample_unit(2, vec![2]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace =
        parse_witness_trace(&encode_values(&[5, 9, 1, 9]), 2, 2).expect("trace should parse");
    let stage = layout.stage_trace(&trace, 1).expect("stage should extract");

    let cuda = extend_witness_stage_leaves_with_cuda(&stage, 1, 2)
        .expect("cuda stage leaves should extend");
    let actual = decode_witness_stage_leaf_values(&cuda).expect("leaves should decode");
    let column_0 = coset_extend_evaluations(&[Felt::from_u64(5), Felt::from_u64(1)], 1, 2)
        .expect("first column should extend");
    let column_1 = coset_extend_evaluations(&[Felt::from_u64(9), Felt::from_u64(9)], 1, 2)
        .expect("second column should extend");
    let expected = (0..4)
        .flat_map(|row| [column_0[row], column_1[row]])
        .collect::<Vec<_>>();

    assert_eq!(actual, expected);
}

#[test]
fn extends_all_witness_trace_stages_for_expression_inputs() {
    let unit = sample_unit(2, vec![2, 1]);
    let trace = parse_witness_trace(&encode_values(&[5, 9, 11, 1, 9, 13]), 2, 3)
        .expect("trace should parse");

    let stages =
        extend_witness_trace_stage_values(&trace, &unit).expect("trace stages should extend");

    let stage_1_column_0 = coset_extend_evaluations(&[Felt::from_u64(5), Felt::from_u64(1)], 1, 2)
        .expect("column should extend");
    let stage_1_column_1 = coset_extend_evaluations(&[Felt::from_u64(9), Felt::from_u64(9)], 1, 2)
        .expect("column should extend");
    let expected_stage_1 = (0..4)
        .flat_map(|row| [stage_1_column_0[row], stage_1_column_1[row]])
        .collect::<Vec<_>>();
    let expected_stage_2 =
        coset_extend_evaluations(&[Felt::from_u64(11), Felt::from_u64(13)], 1, 2)
            .expect("column should extend");

    assert_eq!(stages.len(), 2);
    assert_eq!(stages[0].stage_index(), 1);
    assert_eq!(stages[0].source_row_count(), 2);
    assert_eq!(stages[0].extended_row_count(), 4);
    assert_eq!(stages[0].column_count(), 2);
    assert_eq!(stages[0].values(), expected_stage_1);
    assert_eq!(stages[1].stage_index(), 2);
    assert_eq!(stages[1].source_row_count(), 2);
    assert_eq!(stages[1].extended_row_count(), 4);
    assert_eq!(stages[1].column_count(), 1);
    assert_eq!(stages[1].values(), expected_stage_2);
}

#[test]
fn rejects_unsupported_witness_stage_commitment_arities() {
    let unit = sample_unit(2, vec![1]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace = parse_witness_trace(&encode_values(&[5, 1]), 2, 1).expect("trace should parse");
    let stage = layout.stage_trace(&trace, 1).expect("stage should extract");
    let leaves =
        extend_witness_stage_leaves(&stage, 1, 2).expect("witness stage leaves should extend");

    assert!(matches!(
        commit_witness_stage_leaves(&leaves, 3),
        Err(WitnessStageCommitmentError::UnsupportedArity { arity: 3 })
    ));
}

#[test]
fn rejects_trace_commitment_shape_mismatches() {
    let unit = sample_unit(2, vec![2]);
    let trace = parse_witness_trace(&encode_values(&[5, 9]), 1, 2).expect("trace should parse");

    assert!(matches!(
        commit_witness_trace_stages(&trace, &unit),
        Err(WitnessTraceCommitmentError::Layout(
            WitnessTraceLayoutError::TraceShapeMismatch {
                expected_rows: 2,
                expected_columns: 2,
                found_rows: 1,
                found_columns: 2
            }
        ))
    ));
}

#[test]
fn rejects_trace_commitments_with_unsupported_arities() {
    let mut unit = sample_unit(2, vec![1]);
    unit.merkle_tree_arity = 3;
    let trace = parse_witness_trace(&encode_values(&[5, 1]), 2, 1).expect("trace should parse");

    assert!(matches!(
        commit_witness_trace_stages(&trace, &unit),
        Err(WitnessTraceCommitmentError::StageCommitment(
            WitnessStageCommitmentError::UnsupportedArity { arity: 3 }
        ))
    ));
}

#[test]
fn rejects_witness_stage_extension_domain_mismatches() {
    let unit = sample_unit(2, vec![1]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace = parse_witness_trace(&encode_values(&[5, 1]), 2, 1).expect("trace should parse");
    let stage = layout.stage_trace(&trace, 1).expect("stage should extract");

    #[cfg(feature = "cuda")]
    assert!(matches!(
        extend_witness_stage_leaves(&stage, 2, 3),
        Err(WitnessStageLeafError::Accel(
            lzvm_accel::AccelError::InvalidDomain { bits: 2, len: 2 }
        ))
    ));

    #[cfg(not(feature = "cuda"))]
    assert!(matches!(
        extend_witness_stage_leaves(&stage, 2, 3),
        Err(WitnessStageLeafError::Domain(DomainError::LengthMismatch {
            expected: 4,
            found: 2
        }))
    ));
}

fn witness_commitment_proof_segment(unit_index: u32, unit: &ProveUnitSchedule) -> ProofSegment {
    witness_commitment_proof_segment_with_payload_unit(unit_index, unit_index, unit)
}

fn witness_commitment_proof_segment_with_payload_unit(
    segment_unit_index: u32,
    payload_unit_index: u32,
    unit: &ProveUnitSchedule,
) -> ProofSegment {
    let segment = sample_witness_commitment_segment(payload_unit_index, unit);
    encode_witness_commitment_proof_segment(segment_unit_index, &segment)
}

fn encode_witness_commitment_proof_segment(
    segment_unit_index: u32,
    segment: &WitnessCommitmentSegment,
) -> ProofSegment {
    ProofSegment {
        id: WITNESS_COMMITMENT_SEGMENT_BASE_ID + segment_unit_index,
        data: encode_witness_commitment_segment(segment).expect("segment should encode"),
    }
}

fn sample_witness_commitment_segment(
    unit_index: u32,
    unit: &ProveUnitSchedule,
) -> WitnessCommitmentSegment {
    WitnessCommitmentSegment {
        unit_index,
        input_byte_count: 8,
        trace_rows: unit.base_domain_size,
        trace_columns: unit
            .stage_commit_widths
            .iter()
            .map(|width| u64::from(*width))
            .sum(),
        stages: unit
            .stage_commit_widths
            .iter()
            .enumerate()
            .map(|(index, _)| WitnessCommitmentStageSegment {
                stage_index: u32::try_from(index + 1).expect("stage index should fit"),
                arity: unit.merkle_tree_arity,
                root: [
                    u64::try_from(index + 1).expect("root seed should fit"),
                    0,
                    0,
                    0,
                ],
                tree_byte_count: 32,
                tree_digest: [u8::try_from(index + 1).expect("digest seed should fit"); 32],
            })
            .collect(),
    }
}
