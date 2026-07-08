use std::path::Path;

use lzvm_artifacts::challenge_values_segment::CHALLENGE_VALUES_SEGMENT_ID;
use lzvm_artifacts::constant_opening_segment::CONSTANT_OPENING_SEGMENT_ID;
use lzvm_artifacts::contribution_segment::CONTRIBUTION_SEGMENT_ID;
use lzvm_artifacts::eth_block_input_segment::ETH_BLOCK_INPUT_SEGMENT_ID;
use lzvm_artifacts::group_values_segment::GROUP_VALUES_SEGMENT_ID;
use lzvm_artifacts::guest_input_segment::FRAMED_GUEST_INPUT_SEGMENT_ID;
use lzvm_artifacts::pcs_evaluation_segment::PCS_EVALUATION_SEGMENT_ID;
use lzvm_artifacts::pcs_fri_segment::PCS_FRI_OPENING_SEGMENT_ID;
use lzvm_artifacts::pcs_material_segment::PCS_MATERIAL_MANIFEST_SEGMENT_ID;
use lzvm_artifacts::pcs_nonce_segment::PCS_QUERY_NONCE_SEGMENT_ID;
use lzvm_artifacts::pcs_proof_values_segment::PCS_PROOF_VALUES_SEGMENT_ID;
use lzvm_artifacts::pcs_query_segment::PCS_QUERY_PLAN_SEGMENT_ID;
use lzvm_artifacts::program_image_segment::PROGRAM_IMAGE_CACHE_SEGMENT_ID;
use lzvm_artifacts::trace_constraint_segment::TRACE_CONSTRAINT_SEGMENT_ID;
use lzvm_artifacts::unit_values_segment::UNIT_VALUES_SEGMENT_ID;
use lzvm_artifacts::witness_opening_segment::WITNESS_OPENING_SEGMENT_ID;
use lzvm_artifacts::witness_segment::WITNESS_COMMITMENT_SEGMENT_BASE_ID;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_proof_segment_ids_track_runtime_allowlist() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_root_path = crate_root.join("../../lean/Lzvm.lean");
    let lean_root_source =
        std::fs::read_to_string(&lean_root_path).expect("top-level Lean source should read");
    let lean_path = crate_root.join("../../lean/Lzvm/ProofSegmentIds.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean proof segment ID source should read");
    let rust_path = crate_root.join("src/proof_segment_ids.rs");
    let rust_source =
        std::fs::read_to_string(&rust_path).expect("Rust proof segment ID source should read");

    assert!(
        lean_binding::contains_import(&lean_root_source, "Lzvm.ProofSegmentIds"),
        "top-level Lean module should import concrete proof segment ID allowlist"
    );
    assert!(
        lean_source.contains("def IsAllowedProofSegmentId")
            && lean_source.contains("def isFixedProofSegmentIdBool")
            && lean_source.contains("def AllProofSegmentIdsAllowed")
            && lean_source.contains("def ProofSegmentIdsAllowed")
            && lean_source.contains("theorem witness_commitment_base_id_allowed")
            && lean_source.contains("theorem empty_proof_segment_ids_allowed")
            && lean_source.contains("theorem all_proof_segment_ids_allowed_cons")
            && lean_source.contains("theorem first_unknown_fixed_proof_segment_id_not_allowed")
            && lean_source.contains("theorem unknown_fixed_proof_segment_id_not_allowed")
            && lean_source.contains("theorem fixed_proof_segment_ids_nodup")
            && lean_source
                .contains("theorem witness_commitment_segment_range_disjoint_fixed_segment_ids"),
        "Lean should expose concrete allowed and rejected proof segment ID facts"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "fixed_proof_segment_ids_nodup",
            "fixed_proof_segment_ids_length",
            "empty_proof_segment_ids_allowed",
            "proof_segment_ids_allowed_iff_all_list_ids_allowed",
            "all_proof_segment_ids_allowed_cons",
            "fixed_proof_segment_ids_are_at_or_above_manifest",
            "witness_commitment_segment_range_disjoint_fixed_segment_ids",
            "witness_commitment_base_id_allowed",
            "last_witness_commitment_id_allowed",
            "pcs_material_manifest_segment_id_allowed",
            "pcs_query_plan_segment_id_allowed",
            "witness_opening_segment_id_allowed",
            "constant_opening_segment_id_allowed",
            "pcs_fri_opening_segment_id_allowed",
            "pcs_query_nonce_segment_id_allowed",
            "pcs_evaluation_segment_id_allowed",
            "pcs_proof_values_segment_id_allowed",
            "group_values_segment_id_allowed",
            "unit_values_segment_id_allowed",
            "program_image_cache_segment_id_allowed",
            "contribution_segment_id_allowed",
            "challenge_values_segment_id_allowed",
            "eth_block_input_segment_id_allowed",
            "trace_constraint_segment_id_allowed",
            "framed_guest_input_segment_id_allowed",
            "reserved_proof_segment_id_not_allowed",
            "unknown_fixed_proof_segment_id_not_allowed",
            "first_unknown_fixed_proof_segment_id_not_allowed",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "fixed_proof_segment_ids_length",
        &["fixedProofSegmentIds.length = 16"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "proof_segment_ids_allowed_iff_all_list_ids_allowed",
        &["ProofSegmentIdsAllowed proof", "id ∈ proof.segmentIds"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "all_proof_segment_ids_allowed_cons",
        &["unfold AllProofSegmentIdsAllowed", "simp"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "fixed_proof_segment_ids_are_at_or_above_manifest",
        &["IsFixedProofSegmentId id -> pcsMaterialManifestSegmentId <= id"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "witness_commitment_segment_range_disjoint_fixed_segment_ids",
        &[
            "fixed_proof_segment_ids_are_at_or_above_manifest",
            "Nat.not_lt_of_ge fixedLower witness.right",
        ],
    );
    for theorem in [
        "witness_commitment_base_id_allowed",
        "last_witness_commitment_id_allowed",
    ] {
        lean_binding::assert_theorem_body_contains(&lean_source, theorem, &["left"]);
    }
    for theorem in [
        "pcs_material_manifest_segment_id_allowed",
        "pcs_query_plan_segment_id_allowed",
        "witness_opening_segment_id_allowed",
        "constant_opening_segment_id_allowed",
        "pcs_fri_opening_segment_id_allowed",
        "pcs_query_nonce_segment_id_allowed",
        "pcs_evaluation_segment_id_allowed",
        "pcs_proof_values_segment_id_allowed",
        "group_values_segment_id_allowed",
        "unit_values_segment_id_allowed",
        "program_image_cache_segment_id_allowed",
        "contribution_segment_id_allowed",
        "challenge_values_segment_id_allowed",
        "eth_block_input_segment_id_allowed",
        "trace_constraint_segment_id_allowed",
        "framed_guest_input_segment_id_allowed",
    ] {
        lean_binding::assert_theorem_body_contains(&lean_source, theorem, &["right", "decide"]);
    }
    for theorem in [
        "reserved_proof_segment_id_not_allowed",
        "unknown_fixed_proof_segment_id_not_allowed",
        "first_unknown_fixed_proof_segment_id_not_allowed",
    ] {
        lean_binding::assert_theorem_prefix_contains(
            &lean_source,
            theorem,
            &["Not (IsAllowedProofSegmentId"],
        );
        lean_binding::assert_theorem_body_contains(&lean_source, theorem, &["decide"]);
    }
    assert!(
        rust_source.contains(
            "(WITNESS_COMMITMENT_SEGMENT_BASE_ID..PCS_MATERIAL_MANIFEST_SEGMENT_ID).contains(&id)"
        ) && rust_source.contains("const FIXED_PROOF_SEGMENT_IDS: &[u32] = &[")
            && rust_source.contains("FIXED_PROOF_SEGMENT_IDS.contains(&id)"),
        "Rust proof segment ID helper should retain the witness range and fixed-ID allowlist"
    );

    for expected in EXPECTED_SEGMENT_IDS {
        assert!(
            lean_source.contains(&format!(
                "def {} : Nat := {}",
                expected.lean_name, expected.value
            )),
            "Lean proof segment ID {} should match runtime value {}",
            expected.lean_name,
            expected.value
        );
        assert!(
            rust_source.contains(expected.rust_name),
            "Rust proof segment ID helper should include {}",
            expected.rust_name
        );
    }

    let mut actual_fixed_lean_names = lean_fixed_proof_segment_ids(&lean_source);
    actual_fixed_lean_names.sort_unstable();
    let mut expected_fixed_lean_names = EXPECTED_SEGMENT_IDS
        .iter()
        .skip(1)
        .map(|expected| expected.lean_name)
        .collect::<Vec<_>>();
    expected_fixed_lean_names.sort_unstable();
    assert_eq!(
        actual_fixed_lean_names, expected_fixed_lean_names,
        "Lean fixed proof segment ID list should exactly match runtime fixed segment IDs"
    );

    let mut actual_fixed_rust_names = rust_fixed_proof_segment_ids(&rust_source);
    actual_fixed_rust_names.sort_unstable();
    let mut expected_fixed_rust_names = EXPECTED_SEGMENT_IDS
        .iter()
        .skip(1)
        .map(|expected| expected.rust_name)
        .collect::<Vec<_>>();
    expected_fixed_rust_names.sort_unstable();
    assert_eq!(
        actual_fixed_rust_names, expected_fixed_rust_names,
        "Rust fixed proof segment ID list should exactly match runtime fixed segment IDs"
    );
}

fn lean_fixed_proof_segment_ids(source: &str) -> Vec<&str> {
    let (_, after_def) = source
        .split_once("def fixedProofSegmentIds : List Nat :=")
        .expect("Lean source should define fixedProofSegmentIds");
    let (body, _) = after_def
        .split_once("\ndef IsWitnessCommitmentSegmentId")
        .expect("fixedProofSegmentIds should precede witness segment ID predicate");

    body.split(|ch: char| ch == '[' || ch == ']' || ch == ',' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect()
}

fn rust_fixed_proof_segment_ids(source: &str) -> Vec<&str> {
    let (_, after_def) = source
        .split_once("const FIXED_PROOF_SEGMENT_IDS: &[u32] = &[")
        .expect("Rust source should define FIXED_PROOF_SEGMENT_IDS");
    let (body, _) = after_def
        .split_once("\n];")
        .expect("FIXED_PROOF_SEGMENT_IDS should close as a slice literal");

    body.split(|ch: char| ch == '[' || ch == ']' || ch == ',' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect()
}

struct ExpectedSegmentId {
    lean_name: &'static str,
    rust_name: &'static str,
    value: u32,
}

const EXPECTED_SEGMENT_IDS: &[ExpectedSegmentId] = &[
    ExpectedSegmentId {
        lean_name: "witnessCommitmentSegmentBaseId",
        rust_name: "WITNESS_COMMITMENT_SEGMENT_BASE_ID",
        value: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
    },
    ExpectedSegmentId {
        lean_name: "pcsMaterialManifestSegmentId",
        rust_name: "PCS_MATERIAL_MANIFEST_SEGMENT_ID",
        value: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "pcsQueryPlanSegmentId",
        rust_name: "PCS_QUERY_PLAN_SEGMENT_ID",
        value: PCS_QUERY_PLAN_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "witnessOpeningSegmentId",
        rust_name: "WITNESS_OPENING_SEGMENT_ID",
        value: WITNESS_OPENING_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "constantOpeningSegmentId",
        rust_name: "CONSTANT_OPENING_SEGMENT_ID",
        value: CONSTANT_OPENING_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "pcsFriOpeningSegmentId",
        rust_name: "PCS_FRI_OPENING_SEGMENT_ID",
        value: PCS_FRI_OPENING_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "pcsQueryNonceSegmentId",
        rust_name: "PCS_QUERY_NONCE_SEGMENT_ID",
        value: PCS_QUERY_NONCE_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "pcsEvaluationSegmentId",
        rust_name: "PCS_EVALUATION_SEGMENT_ID",
        value: PCS_EVALUATION_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "pcsProofValuesSegmentId",
        rust_name: "PCS_PROOF_VALUES_SEGMENT_ID",
        value: PCS_PROOF_VALUES_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "groupValuesSegmentId",
        rust_name: "GROUP_VALUES_SEGMENT_ID",
        value: GROUP_VALUES_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "unitValuesSegmentId",
        rust_name: "UNIT_VALUES_SEGMENT_ID",
        value: UNIT_VALUES_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "programImageCacheSegmentId",
        rust_name: "PROGRAM_IMAGE_CACHE_SEGMENT_ID",
        value: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "contributionSegmentId",
        rust_name: "CONTRIBUTION_SEGMENT_ID",
        value: CONTRIBUTION_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "challengeValuesSegmentId",
        rust_name: "CHALLENGE_VALUES_SEGMENT_ID",
        value: CHALLENGE_VALUES_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "ethBlockInputSegmentId",
        rust_name: "ETH_BLOCK_INPUT_SEGMENT_ID",
        value: ETH_BLOCK_INPUT_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "traceConstraintSegmentId",
        rust_name: "TRACE_CONSTRAINT_SEGMENT_ID",
        value: TRACE_CONSTRAINT_SEGMENT_ID,
    },
    ExpectedSegmentId {
        lean_name: "framedGuestInputSegmentId",
        rust_name: "FRAMED_GUEST_INPUT_SEGMENT_ID",
        value: FRAMED_GUEST_INPUT_SEGMENT_ID,
    },
];
