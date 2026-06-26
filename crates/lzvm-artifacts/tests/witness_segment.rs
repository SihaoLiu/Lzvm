use lzvm_artifacts::pcs_material_segment::PCS_MATERIAL_MANIFEST_SEGMENT_ID;
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, parse_witness_commitment_segment,
    witness_commitment_segment_id, witness_commitment_segment_identity, WitnessCommitmentSegment,
    WitnessCommitmentSegmentError, WitnessCommitmentSegmentIdError,
    WitnessCommitmentSegmentIdentity, WitnessCommitmentStageSegment,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};

const NON_CANONICAL_FIELD: u64 = 0xffff_ffff_0000_0001;
const HEADER_BYTES: usize = 40;
const STAGE_BYTES: usize = 4 + 4 + 4 * 8 + 8 + 32;
const FIRST_STAGE_ROOT_OFFSET: usize = HEADER_BYTES + 8;

fn sample_segment() -> WitnessCommitmentSegment {
    WitnessCommitmentSegment {
        unit_index: 0,
        input_byte_count: 64,
        trace_rows: 16,
        trace_columns: 4,
        stages: vec![WitnessCommitmentStageSegment {
            stage_index: 0,
            arity: 4,
            root: [1, 2, 3, 4],
            tree_byte_count: 128,
            tree_digest: [5; 32],
        }],
    }
}

fn segment_header(stage_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"wcs0");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 0);
    push_u64(&mut bytes, 64);
    push_u64(&mut bytes, 16);
    push_u64(&mut bytes, 4);
    push_u32(&mut bytes, stage_count);
    bytes
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn witness_commitment_segment_ids() {
    let unit_count = 66;
    let base_identity = WitnessCommitmentSegmentIdentity {
        unit_index: 3,
        trace_instance_index: 0,
    };
    let base_id = witness_commitment_segment_id(unit_count, base_identity)
        .expect("instance zero id should encode");
    assert_eq!(base_id, WITNESS_COMMITMENT_SEGMENT_BASE_ID + 3);
    assert_eq!(
        witness_commitment_segment_identity(unit_count, base_id)
            .expect("instance zero id should decode"),
        Some(base_identity)
    );

    let later_identity = WitnessCommitmentSegmentIdentity {
        unit_index: 3,
        trace_instance_index: 2,
    };
    let later_id = witness_commitment_segment_id(unit_count, later_identity)
        .expect("nonzero instance id should encode");
    assert_eq!(
        later_id,
        WITNESS_COMMITMENT_SEGMENT_BASE_ID + 2 * unit_count + 3
    );
    assert_eq!(
        witness_commitment_segment_identity(unit_count, later_id)
            .expect("nonzero instance id should decode"),
        Some(later_identity)
    );

    assert_eq!(
        witness_commitment_segment_identity(unit_count, WITNESS_COMMITMENT_SEGMENT_BASE_ID - 1)
            .expect("foreign id should decode"),
        None
    );
    assert_eq!(
        witness_commitment_segment_identity(unit_count, PCS_MATERIAL_MANIFEST_SEGMENT_ID)
            .expect("non-witness id should decode"),
        None
    );
    assert_eq!(
        witness_commitment_segment_id(
            66,
            WitnessCommitmentSegmentIdentity {
                unit_index: 0,
                trace_instance_index: 150,
            },
        ),
        Err(WitnessCommitmentSegmentIdError::SegmentIdOverflow)
    );
    assert_eq!(
        witness_commitment_segment_id(
            0,
            WitnessCommitmentSegmentIdentity {
                unit_index: 0,
                trace_instance_index: 0,
            },
        ),
        Err(WitnessCommitmentSegmentIdError::EmptyUnitSet)
    );
    assert_eq!(
        witness_commitment_segment_id(
            unit_count,
            WitnessCommitmentSegmentIdentity {
                unit_index: unit_count,
                trace_instance_index: 0,
            },
        ),
        Err(WitnessCommitmentSegmentIdError::UnitIndexOutOfRange {
            unit_index: unit_count,
            unit_count,
        })
    );
    assert_eq!(
        witness_commitment_segment_id(
            u32::MAX,
            WitnessCommitmentSegmentIdentity {
                unit_index: 0,
                trace_instance_index: u32::MAX,
            },
        ),
        Err(WitnessCommitmentSegmentIdError::SegmentIdOverflow)
    );
}

#[test]
fn rejects_non_canonical_witness_commitment_stage_roots() {
    let mut segment = sample_segment();
    segment.stages[0].root[2] = NON_CANONICAL_FIELD;

    let err = encode_witness_commitment_segment(&segment).expect_err("stage root should reject");

    assert_eq!(
        err.to_string(),
        "witness commitment unit 0 stage 0 root word 2 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_non_canonical_witness_commitment_stage_roots_when_parsing() {
    let mut encoded =
        encode_witness_commitment_segment(&sample_segment()).expect("segment should encode");
    encoded[FIRST_STAGE_ROOT_OFFSET + 8..FIRST_STAGE_ROOT_OFFSET + 16]
        .copy_from_slice(&NON_CANONICAL_FIELD.to_le_bytes());

    let err = parse_witness_commitment_segment(&encoded).expect_err("stage root should reject");

    assert_eq!(
        err.to_string(),
        "witness commitment unit 0 stage 0 root word 1 is non-canonical: non-canonical field element: 18446744069414584321"
    );
}

#[test]
fn rejects_truncated_witness_commitment_segments() {
    let result = parse_witness_commitment_segment(b"wcs0\x01\0");

    assert!(matches!(
        result,
        Err(WitnessCommitmentSegmentError::UnexpectedEof {
            needed: 8,
            available: 6
        })
    ));
}

#[test]
fn rejects_stage_count_that_exceeds_remaining_stage_records() {
    assert!(matches!(
        parse_witness_commitment_segment(&segment_header(1)),
        Err(WitnessCommitmentSegmentError::UnexpectedEof {
            needed,
            available: HEADER_BYTES
        }) if needed == HEADER_BYTES + STAGE_BYTES
    ));
}

#[test]
fn rejects_truncated_witness_commitment_stage_payload() {
    let mut encoded =
        encode_witness_commitment_segment(&sample_segment()).expect("segment should encode");
    encoded.pop();

    assert!(matches!(
        parse_witness_commitment_segment(&encoded),
        Err(WitnessCommitmentSegmentError::UnexpectedEof {
            needed,
            available
        }) if needed == HEADER_BYTES + STAGE_BYTES
            && available == HEADER_BYTES + STAGE_BYTES - 1
    ));
}
