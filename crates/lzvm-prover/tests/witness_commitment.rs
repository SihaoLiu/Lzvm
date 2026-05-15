use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_field::{coset_extend_evaluations, DomainError, Felt};
use lzvm_prover::witness_commitment::{extend_witness_stage_leaves, WitnessStageLeafError};
use lzvm_prover::witness_layout::derive_witness_trace_layout;
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
        transcript_arity: Some(2),
        constant_width: 1,
        stage_commit_widths,
        opening_points: vec![0],
        fri_layers: vec![PcsFriLayer {
            input_bits: 2,
            output_bits: 1,
            folding_factor: 2,
        }],
        final_layer_bits: 1,
        fixed_bytes: 16,
        constant_tree_root: None,
    }
}

fn decode_words(bytes: &[u8]) -> Vec<u64> {
    bytes
        .chunks_exact(8)
        .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("chunk length checked")))
        .collect()
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
fn rejects_witness_stage_extension_domain_mismatches() {
    let unit = sample_unit(2, vec![1]);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let trace = parse_witness_trace(&encode_values(&[5, 1]), 2, 1).expect("trace should parse");
    let stage = layout.stage_trace(&trace, 1).expect("stage should extract");

    assert!(matches!(
        extend_witness_stage_leaves(&stage, 2, 3),
        Err(WitnessStageLeafError::Domain(DomainError::LengthMismatch {
            expected: 4,
            found: 2
        }))
    ));
}
