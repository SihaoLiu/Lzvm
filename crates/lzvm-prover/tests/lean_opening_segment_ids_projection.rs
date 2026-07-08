use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_opening_segment_ids_project_to_concrete_allowlist() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let segment_ids_path = crate_root.join("../../lean/Lzvm/OpeningSegmentBinding/SegmentIds.lean");
    let segment_ids_source = std::fs::read_to_string(&segment_ids_path)
        .expect("Lean opening segment IDs source should read");

    assert!(
        lean_binding::contains_import(&segment_ids_source, "Lzvm.ProofSegmentIds"),
        "opening segment IDs module should import the concrete proof segment allowlist"
    );
    lean_binding::assert_theorem_declarations(
        &segment_ids_source,
        &["runtime_opening_segment_binding_checked_acceptance_proof_segment_ids_allowed"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &segment_ids_source,
        "runtime_opening_segment_binding_checked_acceptance_proof_segment_ids_allowed",
        &[
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "ProofSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &segment_ids_source,
        "runtime_opening_segment_binding_checked_acceptance_proof_segment_ids_allowed",
        &["runtime_opening_segment_binding_checked_acceptance_concrete_segment_ids_allowed"],
    );
    lean_binding::assert_theorem_body_omits(
        &segment_ids_source,
        "runtime_opening_segment_binding_checked_acceptance_proof_segment_ids_allowed",
        &["AssumptionBundle", "RuntimeOpeningEvidence"],
    );
}
