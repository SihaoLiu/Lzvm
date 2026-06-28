use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_batch_opening_binding_tracks_runtime_batch_helpers() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/BatchOpeningBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean batch opening binding source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");
    let opening_path = crate_root.join("src/witness_opening.rs");
    let opening_source =
        std::fs::read_to_string(&opening_path).expect("witness opening source should read");
    let tree_path = crate_root.join("src/witness_commitment/tree.rs");
    let tree_source =
        std::fs::read_to_string(&tree_path).expect("witness commitment tree source should read");

    assert!(
        lean_source.contains("RuntimeBatchWitnessOpeningRowsValidation")
            && lean_source.contains("perRowWitnessOpeningRowsBound")
            && lean_source.contains("def RuntimeBatchWitnessOpeningRowsBoundContract")
            && lean_source.contains("RuntimeOpeningSegmentBindingEvidence")
            && lean_source.contains("RuntimeOpeningEvidence")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean batch opening binding should expose per-row batch opening soundness evidence"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_batch_witness_opening_rows_checked_acceptance_evidence",
            "runtime_batch_witness_opening_rows_evidence_implies_opening_segment_evidence",
            "runtime_batch_witness_opening_rows_evidence_implies_opening_evidence",
            "runtime_batch_witness_opening_rows_evidence_implies_bound_contract",
            "runtime_batch_witness_opening_rows_checked_acceptance_opening_segment_evidence",
            "runtime_batch_witness_opening_rows_checked_acceptance_opening_evidence",
            "runtime_batch_witness_opening_rows_checked_acceptance_bound_contract",
            "runtime_batch_witness_opening_rows_checked_acceptance_sound_from_hash_concrete_opening",
            "runtime_batch_witness_opening_rows_checked_acceptance_sound_from_concrete_nary_merkle",
            "runtime_batch_witness_opening_rows_checked_acceptance_sound",
            "runtime_batch_witness_opening_rows_checked_acceptance_verifier_core_contract",
            "runtime_batch_witness_opening_rows_checked_acceptance_bound_and_core_contract",
            "runtime_batch_witness_opening_rows_checked_acceptance_opening_and_core_contract",
            "runtime_batch_witness_opening_rows_checked_acceptance_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_batch_witness_opening_rows_checked_acceptance_bound_and_core_contract",
        &[
            "RuntimeBatchWitnessOpeningRowsBoundContract",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_batch_witness_opening_rows_checked_acceptance_opening_and_core_contract",
        &[
            "RuntimeOpeningEvidence",
            "RuntimeBatchWitnessOpeningRowsBoundContract",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_batch_witness_opening_rows_checked_acceptance_evidence_core_and_sound",
        &[
            "RuntimeBatchWitnessOpeningRowsEvidence",
            "RuntimeOpeningEvidence",
            "RuntimeBatchWitnessOpeningRowsBoundContract",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_batch_witness_opening_rows_checked_acceptance_evidence_core_and_sound",
        &[
            "runtime_batch_witness_opening_rows_checked_acceptance_sound",
            "runtime_batch_witness_opening_rows_checked_acceptance_opening_and_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_batch_witness_opening_rows_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_batch_witness_opening_rows_checked_acceptance_sound_from_hash_concrete_opening",
        &[
            "HashCollisionResistanceAssumption",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "RuntimeBatchWitnessOpeningRowsEvidence",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_batch_witness_opening_rows_checked_acceptance_sound_from_hash_concrete_opening",
        &["runtime_opening_segment_binding_checked_acceptance_sound_from_hash_concrete_opening"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_batch_witness_opening_rows_checked_acceptance_sound_from_hash_concrete_opening",
        &["runtime_opening_segment_binding_checked_acceptance_sound\n"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_batch_witness_opening_rows_checked_acceptance_sound_from_concrete_nary_merkle",
        &["runtime_batch_witness_opening_rows_checked_acceptance_sound_from_hash_concrete_opening"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_batch_witness_opening_rows_checked_acceptance_verifier_core_contract",
        &[
            "batchWitnessOpeningRowsAcceptedImpliesOpeningSegmentAccepted",
            "runtime_opening_segment_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_batch_witness_opening_rows_checked_acceptance_verifier_core_contract",
        &[
            "runtime_batch_witness_opening_rows_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.BatchOpeningBinding"),
        "top-level Lean module should import batch opening binding"
    );
    assert!(
        opening_source.contains("open_witness_stage_commitment_batches_with_source_devices_timing")
            && opening_source.contains("WitnessStageOpeningBatchRequest")
            && opening_source.contains("open_witness_stage_commitments("),
        "runtime witness opening builder should call batch stage opening helpers"
    );
    assert!(
        tree_source.contains("open_witness_stage_commitments_with_source_device_timing")
            && tree_source.contains("open_compact_batch_on_demand_with_source_device"),
        "runtime witness commitment tree should expose compact batch opening helpers"
    );
}
