use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_material_segment::{
    encode_pcs_material_manifest_segment, parse_pcs_material_manifest_segment,
    PcsMaterialManifestSegment, PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_prover::pcs_material_manifest::{
    validate_pcs_material_manifest_segments, ValidatePcsMaterialManifestSegmentsError,
};
use lzvm_prover::{build_pcs_material_manifest_segment, ProveSchedule, ProveUnitSchedule};

#[test]
fn validates_pcs_material_manifest_segments_from_schedule() {
    let schedule = sample_schedule();
    let segment = build_pcs_material_manifest_segment(&schedule).expect("segment should build");

    validate_pcs_material_manifest_segments(&schedule, &[segment])
        .expect("segment should validate");
}

#[test]
fn rejects_missing_pcs_material_manifest_segment() {
    let error = validate_pcs_material_manifest_segments(&sample_schedule(), &[])
        .expect_err("segment should be present");

    assert_eq!(
        error,
        ValidatePcsMaterialManifestSegmentsError::MissingSegment
    );
}

#[test]
fn rejects_duplicate_pcs_material_manifest_segments() {
    let schedule = sample_schedule();
    let segment = build_pcs_material_manifest_segment(&schedule).expect("segment should build");

    let error = validate_pcs_material_manifest_segments(&schedule, &[segment.clone(), segment])
        .expect_err("duplicate material manifest segments should reject");

    assert_eq!(error.to_string(), "duplicate PCS material manifest segment");
}

#[test]
fn rejects_pcs_material_manifest_mismatches() {
    let schedule = sample_schedule();
    let segment = build_pcs_material_manifest_segment(&schedule).expect("segment should build");
    let mut manifest =
        parse_pcs_material_manifest_segment(&segment.data).expect("segment should parse");
    manifest.units[0].fixed_byte_count += 1;

    let error = validate_pcs_material_manifest_segments(
        &schedule,
        &[encode_pcs_material_manifest_proof_segment(&manifest)],
    )
    .expect_err("manifest should match schedule material");

    assert_eq!(
        error,
        ValidatePcsMaterialManifestSegmentsError::UnitMismatch { unit_index: 0 }
    );
}

#[test]
fn rejects_pcs_material_manifest_unit_index_out_of_range() {
    let schedule = sample_schedule();
    let segment = build_pcs_material_manifest_segment(&schedule).expect("segment should build");
    let mut manifest =
        parse_pcs_material_manifest_segment(&segment.data).expect("segment should parse");
    manifest.units[0].unit_index = 1;

    let error = validate_pcs_material_manifest_segments(
        &schedule,
        &[encode_pcs_material_manifest_proof_segment(&manifest)],
    )
    .expect_err("unit index should stay inside the schedule");

    assert_eq!(
        error,
        ValidatePcsMaterialManifestSegmentsError::UnitMismatch { unit_index: 1 }
    );
}

#[test]
fn rejects_pcs_material_manifest_unit_order_mismatch() {
    let schedule = sample_schedule_with_two_units();
    let segment = build_pcs_material_manifest_segment(&schedule).expect("segment should build");
    let mut manifest =
        parse_pcs_material_manifest_segment(&segment.data).expect("segment should parse");
    manifest.units.swap(0, 1);

    let error = validate_pcs_material_manifest_segments(
        &schedule,
        &[encode_pcs_material_manifest_proof_segment(&manifest)],
    )
    .expect_err("unit indices should follow schedule order");

    assert_eq!(
        error,
        ValidatePcsMaterialManifestSegmentsError::UnitMismatch { unit_index: 0 }
    );
}

fn encode_pcs_material_manifest_proof_segment(
    manifest: &PcsMaterialManifestSegment,
) -> ProofSegment {
    ProofSegment {
        id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
        data: encode_pcs_material_manifest_segment(manifest).expect("segment should encode"),
    }
}

fn sample_schedule() -> ProveSchedule {
    ProveSchedule {
        setup_hash: [0; 32],
        unit_count: 1,
        total_fixed_bytes: 64,
        total_pcs_material_bytes: 224,
        pcs_material_unit_count: 1,
        total_query_count: 2,
        max_extended_domain_bits: 2,
        units: vec![sample_unit()],
    }
}

fn sample_schedule_with_two_units() -> ProveSchedule {
    let mut second_unit = sample_unit();
    second_unit.group_id = Some(1);
    second_unit.unit_id = Some(1);
    second_unit.group_name = Some("group-b".to_owned());
    second_unit.unit_name = Some("unit-b".to_owned());

    ProveSchedule {
        setup_hash: [0; 32],
        unit_count: 2,
        total_fixed_bytes: 128,
        total_pcs_material_bytes: 448,
        pcs_material_unit_count: 2,
        total_query_count: 4,
        max_extended_domain_bits: 2,
        units: vec![sample_unit(), second_unit],
    }
}

fn sample_unit() -> ProveUnitSchedule {
    ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: Some(0),
        unit_id: Some(0),
        group_name: Some("group-a".to_owned()),
        unit_name: Some("unit-a".to_owned()),
        base_domain_bits: 1,
        extended_domain_bits: 2,
        base_domain_size: 2,
        extended_domain_size: 4,
        blowup_factor: 2,
        query_count: 2,
        proof_of_work_bits: 0,
        merkle_tree_arity: 2,
        last_level_verification: 0,
        transcript_arity: Some(2),
        hash_commits: false,
        transcript_root_challenge_draws: vec![1],
        challenge_count: 1,
        evaluation_value_count: 1,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 1,
        constant_width: 1,
        stage_commit_widths: vec![1],
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
        fixed_bytes: 64,
        constant_tree_root: None,
        pcs_material_bytes: Some(224),
        pcs_material_plan_digest: Some([1; 32]),
        pcs_material_fixed_column_digest: Some([2; 32]),
        pcs_material_constant_tree_digest: Some([3; 32]),
        pcs_material_constant_tree_root: Some([4, 5, 6, 7]),
        pcs_material_fixed_byte_count: Some(64),
        pcs_material_constant_tree_byte_count: Some(224),
        pcs_material_leaf_byte_count: Some(64),
        pcs_material_node_byte_count: Some(160),
    }
}
