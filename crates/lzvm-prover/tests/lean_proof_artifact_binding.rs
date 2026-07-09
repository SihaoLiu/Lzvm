use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_proof_artifact_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/ProofArtifactBinding.lean");
    let lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean proof artifact binding source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");
    let proof_artifact_path = crate_root.join("src/proof_artifact.rs");
    let proof_artifact_source =
        std::fs::read_to_string(&proof_artifact_path).expect("proof artifact source should read");
    let proof_preflight_path = crate_root.join("src/proof_preflight.rs");
    let proof_preflight_source =
        std::fs::read_to_string(&proof_preflight_path).expect("proof preflight source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.ProofArtifactBinding"),
        "top-level Lean module should import proof artifact binding"
    );
    assert!(
        lean_binding::contains_import(&lean_source, "Lzvm.ProofSegmentIds"),
        "proof artifact binding should import the concrete proof segment ID allowlist"
    );
    assert!(
        lean_source.contains("RuntimeProofArtifactBindingValidation")
            && lean_source.contains("def RuntimeProofArtifactBindingValidationAgreement")
            && lean_source.contains("RuntimeConformanceValidationAgreement")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("RuntimeProofArtifactBindingStructuralObligations")
            && lean_source.contains("proofContainerCanonical")
            && lean_source.contains("proofMetadataCanonical")
            && lean_source.contains("proofSegmentsPresent")
            && lean_source.contains("proofSegmentPayloadsNonempty")
            && lean_source.contains("proofSegmentIdsAllowed")
            && lean_source.contains("proofSegmentIdsUnique")
            && lean_source.contains("proofUnitValuesTraceIdentityCoverage")
            && lean_source.contains("RuntimeProofArtifactFinalized")
            && lean_source.contains("RuntimeProofArtifactConcreteSegmentIdBinding")
            && lean_source.contains("RuntimeProofArtifactConcreteSegmentIdsAllowed")
            && lean_source.contains("ProofSegmentIdsAllowed proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean proof artifact binding should expose checked soundness and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_proof_artifact_binding_checked_acceptance_runtime_accepted",
            "runtime_proof_artifact_binding_checked_acceptance_runtime_evidence",
            "runtime_proof_artifact_binding_checked_acceptance_container_canonical",
            "runtime_proof_artifact_binding_checked_acceptance_metadata_canonical",
            "runtime_proof_artifact_binding_checked_acceptance_segments_present",
            "runtime_proof_artifact_binding_checked_acceptance_segment_payloads_nonempty",
            "runtime_proof_artifact_binding_checked_acceptance_segment_ids_allowed",
            "runtime_proof_artifact_binding_checked_acceptance_segment_ids_unique",
            "runtime_proof_artifact_binding_checked_acceptance_unit_values_trace_identity_coverage",
            "runtime_proof_artifact_binding_checked_acceptance_structural_obligations",
            "runtime_proof_artifact_binding_validation_agreement_segment_ids_allowed",
            "runtime_proof_artifact_concrete_segment_id_binding_of_agreement_left",
            "runtime_proof_artifact_concrete_segment_id_binding_of_agreement_right",
            "runtime_proof_artifact_binding_checked_acceptance_concrete_segment_ids_allowed",
            "runtime_proof_artifact_binding_checked_acceptance_runtime_shape_contract",
            "runtime_proof_artifact_finalized_concrete_segment_ids_allowed",
            "runtime_proof_artifact_finalized_from_checked_acceptance",
            "runtime_proof_artifact_finalized_structural_obligations",
            "runtime_proof_artifact_finalized_checked_acceptance",
            "runtime_proof_artifact_binding_checked_acceptance_soundness_obligations",
            "runtime_proof_artifact_binding_checked_acceptance_sound",
            "runtime_proof_artifact_binding_checked_acceptance_full_contract",
            "runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract",
            "runtime_proof_artifact_binding_checked_acceptance_evidence_core_and_sound",
            "runtime_proof_artifact_binding_checked_acceptance_accepts_evidence_core_and_sound",
            "runtime_proof_artifact_binding_checked_acceptance_audited_core_contract",
            "runtime_proof_artifact_binding_checked_acceptance_concrete_core_sound_contract",
            "runtime_proof_artifact_finalized_full_contract",
            "runtime_proof_artifact_finalized_verifier_core_contract",
            "runtime_proof_artifact_finalized_evidence_core_and_sound",
            "runtime_proof_artifact_finalized_accepts_evidence_core_and_sound",
            "runtime_proof_artifact_finalized_audited_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_container_canonical",
        &["bindingAcceptedImpliesProofContainerCanonical"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_metadata_canonical",
        &["bindingAcceptedImpliesProofMetadataCanonical"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_segments_present",
        &["bindingAcceptedImpliesProofSegmentsPresent"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_segment_payloads_nonempty",
        &["bindingAcceptedImpliesProofSegmentPayloadsNonempty"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_segment_ids_allowed",
        &["bindingAcceptedImpliesProofSegmentIdsAllowed"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_segment_ids_unique",
        &["bindingAcceptedImpliesProofSegmentIdsUnique"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_unit_values_trace_identity_coverage",
        &["bindingAcceptedImpliesProofUnitValuesTraceIdentityCoverage"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_structural_obligations",
        &[
            "bindingAcceptedImpliesProofContainerCanonical",
            "bindingAcceptedImpliesProofMetadataCanonical",
            "bindingAcceptedImpliesProofSegmentsPresent",
            "bindingAcceptedImpliesProofSegmentPayloadsNonempty",
            "bindingAcceptedImpliesProofSegmentIdsAllowed",
            "bindingAcceptedImpliesProofSegmentIdsUnique",
            "bindingAcceptedImpliesProofUnitValuesTraceIdentityCoverage",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_proof_artifact_binding_validation_agreement_segment_ids_allowed",
        &[
            "RuntimeProofArtifactBindingValidationAgreement left right",
            "left.proofSegmentIdsAllowed artifact publicInput proof <->",
            "right.proofSegmentIdsAllowed artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_validation_agreement_segment_ids_allowed",
        &["rcases agreement"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_proof_artifact_concrete_segment_id_binding_of_agreement_left",
        &[
            "RuntimeProofArtifactBindingValidationAgreement left right",
            "RuntimeProofArtifactConcreteSegmentIdBinding right",
            "RuntimeProofArtifactConcreteSegmentIdBinding left",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_concrete_segment_id_binding_of_agreement_left",
        &[
            "binding.proofSegmentIdsAllowedImpliesConcrete",
            "runtime_proof_artifact_binding_validation_agreement_segment_ids_allowed",
            ".mp leftAllowed",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_proof_artifact_concrete_segment_id_binding_of_agreement_right",
        &[
            "RuntimeProofArtifactBindingValidationAgreement left right",
            "RuntimeProofArtifactConcreteSegmentIdBinding left",
            "RuntimeProofArtifactConcreteSegmentIdBinding right",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_concrete_segment_id_binding_of_agreement_right",
        &[
            "binding.proofSegmentIdsAllowedImpliesConcrete",
            "runtime_proof_artifact_binding_validation_agreement_segment_ids_allowed",
            ".mpr rightAllowed",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_concrete_segment_ids_allowed",
        &[
            "proofSegmentIdsAllowedImpliesConcrete",
            "bindingAcceptedImpliesProofSegmentIdsAllowed",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_concrete_segment_ids_allowed",
        &["segmentIdView"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_runtime_shape_contract",
        &[
            "RuntimeProofArtifactConcreteSegmentIdBinding validation",
            "RuntimeProofArtifactBindingStructuralObligations",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_runtime_shape_contract",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_structural_obligations",
            "runtime_proof_artifact_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_finalized_concrete_segment_ids_allowed",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_concrete_segment_ids_allowed",
            "finalized.left",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_finalized_from_checked_acceptance",
        &[
            "And.intro accepted",
            "runtime_proof_artifact_binding_checked_acceptance_structural_obligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_finalized_structural_obligations",
        &["finalized.right"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_finalized_checked_acceptance",
        &["finalized.left"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_runtime_evidence",
        &[
            "RuntimeProofArtifactBindingCheckedAcceptance",
            "RuntimeArtifactEvidence",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_runtime_evidence",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_evidence",
            "runtime_proof_artifact_binding_evidence_implies_runtime_evidence",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_soundness_obligations",
        &[
            "RuntimeProofArtifactBindingCheckedAcceptance",
            "RuntimeArtifactSoundnessObligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_soundness_obligations",
        &["runtime_proof_artifact_binding_checked_acceptance_obligations"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_sound",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_obligations",
            "abstract_verifier_sound_with_semantic_evidence",
        ],
    );
    lean_binding::assert_theorem_body_omits_identifier(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_sound",
        "abstract_verifier_sound",
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_full_contract",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_sound",
            "runtime_proof_artifact_binding_checked_acceptance_structural_obligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract",
        &["runtime_proof_artifact_binding_checked_acceptance_obligations"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
            "abstract_verifier_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_evidence_core_and_sound",
        &[
            "RuntimeProofArtifactBindingCheckedAcceptance",
            "RuntimeProofArtifactBindingEvidence",
            "RuntimeProofArtifactBindingStructuralObligations",
            "RuntimeArtifactEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_evidence_core_and_sound",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_full_contract",
            "runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "RuntimeProofArtifactBindingCheckedAcceptance",
            "system.accepts publicInput proof",
            "RuntimeProofArtifactBindingEvidence",
            "RuntimeProofArtifactBindingStructuralObligations",
            "RuntimeArtifactEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_runtime_accepted",
            "runtime_artifact_checked_acceptance_accepts_evidence_core_and_sound",
            "runtime_proof_artifact_binding_checked_acceptance_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_full_contract",
            "runtime_artifact_checked_acceptance_evidence_core_and_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_audited_core_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeProofArtifactBindingEvidence",
            "RuntimeProofArtifactBindingStructuralObligations",
            "RuntimeArtifactEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "assumption_bundle_carries_required_semantic_evidence",
            "runtime_proof_artifact_binding_checked_acceptance_full_contract",
            "runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_evidence",
            "runtime_proof_artifact_binding_checked_acceptance_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_concrete_core_sound_contract",
        &[
            "RuntimeProofArtifactBindingCheckedAcceptance",
            "RuntimeProofArtifactConcreteSegmentIdBinding validation",
            "RuntimeProofArtifactBindingEvidence",
            "RuntimeProofArtifactBindingStructuralObligations",
            "RuntimeArtifactEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_concrete_core_sound_contract",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_evidence_core_and_sound",
            "runtime_proof_artifact_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    for identifier in [
        "runtime_proof_artifact_binding_checked_acceptance_sound",
        "sound_witness_implies_verifier_core_contract",
        "abstract_verifier_sound",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "runtime_proof_artifact_binding_checked_acceptance_concrete_core_sound_contract",
            identifier,
        );
    }
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_finalized_full_contract",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_sound",
            "finalized.left",
            "finalized.right",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_finalized_verifier_core_contract",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract",
            "finalized.left",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_proof_artifact_finalized_verifier_core_contract",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
            "abstract_verifier_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_proof_artifact_finalized_evidence_core_and_sound",
        &[
            "RuntimeProofArtifactFinalized",
            "RuntimeProofArtifactBindingEvidence",
            "RuntimeProofArtifactBindingStructuralObligations",
            "RuntimeArtifactEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_finalized_evidence_core_and_sound",
        &[
            "runtime_proof_artifact_finalized_full_contract",
            "runtime_proof_artifact_finalized_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_proof_artifact_finalized_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_proof_artifact_finalized_accepts_evidence_core_and_sound",
        &[
            "RuntimeProofArtifactFinalized",
            "system.accepts publicInput proof",
            "RuntimeProofArtifactBindingEvidence",
            "RuntimeProofArtifactBindingStructuralObligations",
            "RuntimeArtifactEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_finalized_accepts_evidence_core_and_sound",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_accepts_evidence_core_and_sound",
            "runtime_proof_artifact_finalized_evidence_core_and_sound",
            "checkedContract.left",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_proof_artifact_finalized_accepts_evidence_core_and_sound",
        &[
            "runtime_proof_artifact_finalized_full_contract",
            "runtime_artifact_checked_acceptance_evidence_core_and_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_proof_artifact_finalized_audited_core_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeProofArtifactFinalized",
            "RuntimeProofArtifactBindingEvidence",
            "RuntimeProofArtifactBindingStructuralObligations",
            "RuntimeArtifactEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_finalized_audited_core_contract",
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "assumption_bundle_carries_required_semantic_evidence",
            "runtime_proof_artifact_finalized_full_contract",
            "runtime_proof_artifact_finalized_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_proof_artifact_finalized_audited_core_contract",
        &[
            "assumption_bundle_carries_required_evidence",
            "runtime_proof_artifact_finalized_evidence_core_and_sound",
        ],
    );
    assert!(
        proof_artifact_source.contains("fn finish_proof_artifact(proof: ProofArtifact)")
            && proof_artifact_source.contains("validate_proof_artifact(&proof)"),
        "Rust proof artifact builders should finalize constructed artifacts through invariant validation"
    );
    assert!(
        proof_artifact_source.contains("use crate::proof_segment_ids::unexpected_proof_segment_id;")
            && proof_artifact_source.contains("unexpected_proof_segment_id(&proof.segments)"),
        "Rust proof artifact finalization should reject proof segment IDs outside the checked runtime set"
    );
    assert!(
        proof_preflight_source.contains("pub fn validate_proof_artifact_runtime_shape(")
            && proof_preflight_source.contains("validate_proof_artifact(proof)")
            && proof_preflight_source.contains("unexpected_proof_segment_id(&proof.segments)")
            && proof_preflight_source.contains("pub fn read_checked_proof_artifact_file(")
            && proof_preflight_source.contains("validate_proof_artifact_runtime_shape(&proof)?"),
        "Rust proof preflight should expose a checked proof reader matching the Lean runtime shape contract"
    );
    assert_production_proof_artifact_literals_are_finalized(&proof_artifact_source);
}

fn assert_production_proof_artifact_literals_are_finalized(source: &str) {
    let production_source = source.split("\n#[cfg(test)]").next().unwrap_or(source);
    let mut search_start = 0;
    let mut literal_count = 0;

    while let Some(offset) = production_source[search_start..].find("ProofArtifact {") {
        let literal_start = search_start + offset;
        literal_count += 1;

        let prefix = production_source[..literal_start].trim_end();
        if prefix.ends_with("finish_proof_artifact(") {
            search_start = literal_start + "ProofArtifact {".len();
            continue;
        }

        let line_start = production_source[..literal_start]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        let declaration = production_source[line_start..literal_start].trim();
        assert!(
            declaration == "let proof =",
            "ProofArtifact literal on line {} should be assigned to proof before finalization",
            line_number(production_source, literal_start)
        );

        let literal_open = production_source[literal_start..]
            .find('{')
            .map(|index| literal_start + index)
            .expect("ProofArtifact literal should have an opening brace");
        let literal_end = matching_closing_brace(production_source, literal_open)
            .expect("ProofArtifact literal should have a matching closing brace");
        let function_end =
            next_function_start(production_source, literal_end).unwrap_or(production_source.len());
        let function_tail = &production_source[literal_end..function_end];

        assert!(
            function_tail.contains("finish_proof_artifact(proof)"),
            "ProofArtifact literal on line {} should flow through finish_proof_artifact(proof)",
            line_number(production_source, literal_start)
        );

        search_start = literal_end + 1;
    }

    assert!(
        literal_count > 0,
        "proof artifact source should construct production ProofArtifact values"
    );
}

fn matching_closing_brace(source: &str, open_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, ch) in source[open_index..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_index + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn next_function_start(source: &str, after: usize) -> Option<usize> {
    let mut cursor = after;
    for line in source[after..].split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub(crate) fn ")
        {
            return Some(cursor);
        }
        cursor += line.len();
    }
    None
}

fn line_number(source: &str, index: usize) -> usize {
    source[..index]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}
