use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_opening_segment_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/OpeningSegmentBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean opening segment binding should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");
    let fri_segment_path = crate_root.join("../lzvm-artifacts/src/pcs_fri_segment.rs");
    let fri_segment_source =
        std::fs::read_to_string(&fri_segment_path).expect("FRI segment source should read");
    let fri_parse_body =
        rust_function_body(&fri_segment_source, "pub fn parse_pcs_fri_opening_segment");
    let fri_validate_body =
        rust_function_body(&fri_segment_source, "fn validate_pcs_fri_opening_segment");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.OpeningSegmentBinding"),
        "top-level Lean module should import opening segment binding"
    );
    assert!(
        lean_source.contains("RuntimeOpeningSegmentBindingValidation")
            && lean_source.contains("RuntimeOpeningSegmentBindingBoundContract")
            && lean_source.contains("RuntimeOpeningEvidence")
            && lean_source.contains("RuntimeFriOpeningSegmentParserBoundary")
            && lean_source.contains("RuntimeFriOpeningSegmentParserContract")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean opening segment binding should expose checked soundness and verifier core projection"
    );
    assert!(
        fri_parse_body.contains("PCS_FRI_OPENING_V1_VERSION | PCS_FRI_OPENING_V2_VERSION")
            && fri_parse_body.contains("PcsFriOpeningSegmentError::UnsupportedVersion { version }")
            && fri_parse_body.contains("reader.require_items(final_count, EXTENSION_BYTES)?")
            && fri_parse_body.contains("reader.require_items(last_level_count, ROOT_BYTES)?")
            && fri_parse_body.contains("reader.require_items(value_count, EXTENSION_BYTES)?")
            && fri_parse_body.contains("reader.require_items(sibling_count, ROOT_BYTES)?")
            && fri_parse_body.contains("validate_pcs_fri_opening_segment(&out)?"),
        "Rust FRI opening parser should validate supported version, walk counted payloads, and re-run semantic validation"
    );
    assert!(
        fri_validate_body.matches("Felt::from_canonical(word).map_err").count() == 5
            && fri_validate_body.contains("unit.final_polynomial.iter().enumerate()")
            && fri_validate_body.contains("PcsFriOpeningSegmentError::FinalPolynomialValueNonCanonical")
            && fri_validate_body.contains("layer.root.iter().copied().enumerate()")
            && fri_validate_body.contains("PcsFriOpeningSegmentError::LayerRootNonCanonical")
            && fri_validate_body.contains("layer.last_level.iter().enumerate()")
            && fri_validate_body.contains("PcsFriOpeningSegmentError::LastLevelRootNonCanonical")
            && fri_validate_body.contains("query.values.iter().enumerate()")
            && fri_validate_body.contains("PcsFriOpeningSegmentError::QueryValueNonCanonical")
            && fri_validate_body.contains("level.siblings.iter().enumerate()")
            && fri_validate_body.contains("PcsFriOpeningSegmentError::SiblingRootNonCanonical"),
        "Rust FRI opening parser should keep version, root, last-level, and field-canonicality checks represented by Lean"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_opening_segment_binding_fri_segments_valid_parser_contract",
            "runtime_opening_segment_binding_evidence_implies_bound_contract",
            "runtime_opening_segment_binding_evidence_implies_query_plan_bound",
            "runtime_opening_segment_binding_evidence_implies_fri_opening_checks",
            "runtime_opening_segment_binding_evidence_implies_pcs_and_fri",
            "runtime_opening_segment_binding_checked_acceptance_query_plan_bound",
            "runtime_opening_segment_binding_checked_acceptance_bound_contract",
            "runtime_opening_segment_binding_checked_acceptance_fri_opening_checks",
            "runtime_opening_segment_binding_checked_acceptance_fri_parser_contract",
            "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_without_assumptions",
            "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri",
            "runtime_opening_segment_binding_evidence_implies_opening_bound_contract",
            "runtime_opening_segment_binding_checked_acceptance_opening_bound_contract",
            "runtime_opening_segment_binding_checked_acceptance_opening_pcs_fri_contract",
            "runtime_opening_segment_binding_checked_acceptance_bound_pcs_fri_contract",
            "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
            "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
            "runtime_opening_segment_binding_checked_acceptance_sound_from_hash_concrete_opening",
            "runtime_opening_segment_binding_checked_acceptance_sound_from_concrete_nary_merkle",
            "runtime_opening_segment_binding_checked_acceptance_sound",
            "runtime_opening_segment_binding_checked_acceptance_verifier_core_contract",
            "runtime_opening_segment_binding_checked_acceptance_opening_and_core_contract",
            "runtime_opening_segment_binding_checked_acceptance_full_soundness_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_evidence_implies_query_plan_bound",
        &[
            "RuntimeOpeningSegmentBindingEvidence",
            "validation.queryPlanBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_query_plan_bound",
        &[
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "validation.queryPlanBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_query_plan_bound",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_query_plan_bound",
        &["validation.openingSegmentBindingAcceptedImpliesQueryPlanBound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_evidence_implies_fri_opening_checks",
        &[
            "validation.friOpeningSegmentsValid artifact publicInput proof",
            "validation.friFoldsValid artifact publicInput proof",
            "validation.verifierQueryOutputsValid artifact publicInput proof",
            "validation.openingValidation.friOpeningBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_fri_segments_valid_parser_contract",
        &[
            "RuntimeFriOpeningSegmentParserBoundary system validation",
            "validation.friOpeningSegmentsValid artifact publicInput proof",
            "RuntimeFriOpeningSegmentParserContract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_fri_segments_valid_parser_contract",
        &[
            "boundary.friOpeningSegmentsValidImpliesSupportedEncodingVersion",
            "boundary.friOpeningSegmentsValidImpliesFinalPolynomialValuesCanonical",
            "boundary.friOpeningSegmentsValidImpliesQueryValuesCanonical",
            "boundary.friOpeningSegmentsValidImpliesLayerDigestRootsCanonical",
            "boundary.friOpeningSegmentsValidImpliesLastLevelDigestRootsCanonical",
            "boundary.friOpeningSegmentsValidImpliesSiblingDigestRootsCanonical",
            "boundary.friOpeningSegmentsValidImpliesSegmentLayoutWalked",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_fri_opening_checks",
        &[
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "validation.friOpeningSegmentsValid artifact publicInput proof",
            "validation.friFoldsValid artifact publicInput proof",
            "validation.verifierQueryOutputsValid artifact publicInput proof",
            "validation.openingValidation.friOpeningBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_fri_parser_contract",
        &[
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "RuntimeFriOpeningSegmentParserBoundary system validation",
            "RuntimeFriOpeningSegmentParserContract",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_fri_parser_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_fri_parser_contract",
        &[
            "validation.openingSegmentBindingAcceptedImpliesFriOpeningSegmentsValid",
            "runtime_opening_segment_binding_fri_segments_valid_parser_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_evidence_implies_pcs_and_fri",
        &[
            "RuntimeOpeningSegmentBindingEvidence",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_without_assumptions",
        &[
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_without_assumptions",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_without_assumptions",
        &["runtime_opening_segment_binding_evidence_implies_pcs_and_fri"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri",
        &[
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_evidence_implies_opening_bound_contract",
        &[
            "RuntimeOpeningSegmentBindingEvidence",
            "RuntimeOpeningBoundContract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_opening_bound_contract",
        &[
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "RuntimeOpeningBoundContract",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_opening_bound_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_opening_pcs_fri_contract",
        &[
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "RuntimeOpeningCheckedAcceptance",
            "RuntimeOpeningBoundContract",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_opening_pcs_fri_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_bound_pcs_fri_contract",
        &[
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "RuntimeOpeningSegmentBindingBoundContract",
            "RuntimeOpeningBoundContract",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_bound_pcs_fri_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_bound_pcs_fri_contract",
        &[
            "runtime_opening_segment_binding_checked_acceptance_bound_contract",
            "runtime_opening_segment_binding_checked_acceptance_opening_bound_contract",
            "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        &[
            "HashCollisionResistanceAssumption",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        &[
            "runtime_opening_segment_binding_checked_acceptance_opening",
            "runtime_opening_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "assumptions.crypto.hashCollisionResistance",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
        &["HashCollisionResistanceAssumption"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
        &[
            "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
            "assumptions.crypto.hashCollisionResistance",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_sound_from_hash_concrete_opening",
        &[
            "AssumptionBundle system",
            "HashCollisionResistanceAssumption",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "RuntimeOpeningSegmentBindingEvidence",
            "RuntimeOpeningEvidence",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_sound_from_hash_concrete_opening",
        &[
            "runtime_opening_segment_binding_checked_acceptance_evidence",
            "runtime_opening_segment_binding_checked_acceptance_opening",
            "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_hash_concrete_opening",
            "runtime_opening_checked_acceptance_bound_pcs_fri_contract_from_hash_concrete_opening",
            "runtime_soundness_checked_acceptance_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_sound_from_hash_concrete_opening",
        &[
            "runtime_opening_checked_acceptance_sound",
            "runtime_opening_segment_binding_checked_acceptance_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_sound_from_concrete_nary_merkle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "assumptions.crypto.hashCollisionResistance",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "RuntimeOpeningSegmentBindingEvidence",
            "RuntimeOpeningEvidence",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_sound_from_concrete_nary_merkle",
        &[
            "runtime_opening_segment_binding_checked_acceptance_sound_from_hash_concrete_opening",
            "assumptions.crypto.hashCollisionResistance",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_sound_from_concrete_nary_merkle",
        &["runtime_opening_segment_binding_checked_acceptance_sound\n"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_verifier_core_contract",
        &[
            "openingSegmentBindingAcceptedImpliesOpeningAccepted",
            "runtime_opening_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_verifier_core_contract",
        &[
            "runtime_opening_segment_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_opening_and_core_contract",
        &[
            "RuntimeOpeningSegmentBindingBoundContract",
            "RuntimeOpeningEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_opening_and_core_contract",
        &[
            "runtime_opening_segment_binding_checked_acceptance_evidence",
            "runtime_opening_checked_acceptance_evidence",
            "runtime_opening_segment_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_opening_and_core_contract",
        &[
            "runtime_opening_segment_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_full_soundness_contract",
        &[
            "RuntimeOpeningSegmentBindingBoundContract",
            "RuntimeOpeningEvidence",
            "RuntimeOpeningBoundContract",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_full_soundness_contract",
        &[
            "runtime_opening_segment_binding_checked_acceptance_evidence",
            "runtime_opening_segment_binding_checked_acceptance_opening",
            "runtime_opening_checked_acceptance_full_soundness_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_full_soundness_contract",
        &[
            "runtime_opening_segment_binding_checked_acceptance_sound",
            "runtime_opening_checked_acceptance_sound",
        ],
    );
}

fn rust_function_body<'a>(source: &'a str, start: &str) -> &'a str {
    let start_index = source
        .find(start)
        .unwrap_or_else(|| panic!("source should contain {start}"));
    let open_index = source[start_index..]
        .find('{')
        .map(|offset| start_index + offset)
        .unwrap_or_else(|| panic!("{start} should have an opening brace"));
    let close_index = matching_closing_brace(source, open_index)
        .unwrap_or_else(|| panic!("{start} should have a matching closing brace"));
    &source[start_index..=close_index]
}

fn matching_closing_brace(source: &str, open_index: usize) -> Option<usize> {
    let mut depth = 0_usize;
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
