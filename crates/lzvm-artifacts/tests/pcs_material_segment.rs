use lzvm_artifacts::pcs_material_segment::{
    encode_pcs_material_manifest_segment, parse_pcs_material_manifest_segment,
    PcsMaterialManifestSegment, PcsMaterialManifestSegmentError, PcsMaterialManifestUnit,
};

fn sample_segment() -> PcsMaterialManifestSegment {
    PcsMaterialManifestSegment {
        units: vec![
            PcsMaterialManifestUnit {
                unit_index: 0,
                plan_digest: [1; 32],
                fixed_column_digest: [2; 32],
                constant_tree_digest: [3; 32],
                constant_tree_root: [4, 5, 6, 7],
                fixed_byte_count: 64,
                constant_tree_byte_count: 224,
                leaf_byte_count: 64,
                node_byte_count: 160,
            },
            PcsMaterialManifestUnit {
                unit_index: 1,
                plan_digest: [8; 32],
                fixed_column_digest: [9; 32],
                constant_tree_digest: [10; 32],
                constant_tree_root: [11, 12, 13, 14],
                fixed_byte_count: 128,
                constant_tree_byte_count: 448,
                leaf_byte_count: 128,
                node_byte_count: 320,
            },
        ],
    }
}

fn segment_header(unit_count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"pms0");
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, unit_count);
    bytes
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn encodes_and_parses_pcs_material_manifest_segments() {
    let encoded = encode_pcs_material_manifest_segment(&sample_segment())
        .expect("manifest segment should encode");
    let parsed =
        parse_pcs_material_manifest_segment(&encoded).expect("manifest segment should parse");

    assert_eq!(&encoded[0..4], b"pms0");
    assert_eq!(parsed, sample_segment());
}

#[test]
fn rejects_empty_pcs_material_manifest_segments() {
    let segment = PcsMaterialManifestSegment { units: Vec::new() };

    assert!(matches!(
        encode_pcs_material_manifest_segment(&segment),
        Err(PcsMaterialManifestSegmentError::EmptyUnits)
    ));
}

#[test]
fn rejects_duplicate_pcs_material_manifest_units() {
    let mut segment = sample_segment();
    segment.units[1].unit_index = 0;

    assert!(matches!(
        encode_pcs_material_manifest_segment(&segment),
        Err(PcsMaterialManifestSegmentError::DuplicateUnitIndex { unit_index: 0 })
    ));
}

#[test]
fn rejects_truncated_pcs_material_manifest_segments() {
    let result = parse_pcs_material_manifest_segment(b"pms0\x01\0");

    assert!(matches!(
        result,
        Err(PcsMaterialManifestSegmentError::UnexpectedEof {
            needed: 8,
            available: 6
        })
    ));
}

#[test]
fn rejects_unit_count_that_exceeds_remaining_units() {
    assert!(matches!(
        parse_pcs_material_manifest_segment(&segment_header(1)),
        Err(PcsMaterialManifestSegmentError::LengthOverflow)
    ));
}
