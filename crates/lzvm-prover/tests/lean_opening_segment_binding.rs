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
    let constant_opening_path = crate_root.join("src/constant_opening.rs");
    let constant_opening_source = std::fs::read_to_string(&constant_opening_path)
        .expect("constant opening source should read");
    let witness_opening_path = crate_root.join("src/witness_opening.rs");
    let witness_opening_source =
        std::fs::read_to_string(&witness_opening_path).expect("witness opening source should read");
    let pcs_evaluation_path = crate_root.join("src/pcs_evaluation.rs");
    let pcs_evaluation_source =
        std::fs::read_to_string(&pcs_evaluation_path).expect("PCS evaluation source should read");
    let verifier_query_path = crate_root.join("src/verifier_query.rs");
    let verifier_query_source =
        std::fs::read_to_string(&verifier_query_path).expect("verifier query source should read");
    let fri_segment_path = crate_root.join("../lzvm-artifacts/src/pcs_fri_segment.rs");
    let fri_segment_source =
        std::fs::read_to_string(&fri_segment_path).expect("FRI segment source should read");
    let constant_opening_query_units_body = rust_function_body(
        &constant_opening_source,
        "pub(crate) fn validate_constant_opening_units_match_query_units_from_segment",
    );
    let witness_opening_query_units_body = rust_function_body(
        &witness_opening_source,
        "pub(crate) fn validate_witness_opening_units_match_query_units_from_segment",
    );
    let pcs_evaluation_query_units_body = rust_function_body(
        &pcs_evaluation_source,
        "pub(crate) fn validate_pcs_evaluation_units_match_query_units_from_segment",
    );
    let verifier_query_identity_body = rust_function_body(
        &verifier_query_source,
        "fn validate_verifier_query_unit_identities_match_query_units",
    );
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
            && lean_source.contains("RuntimeOpeningSegmentExactIdentityContract")
            && lean_source.contains("RuntimeFriOpeningSegmentParserBoundary")
            && lean_source.contains("RuntimeFriOpeningSegmentParserContract")
            && lean_source.contains("RuntimeFriFoldTraceIdentityContract")
            && lean_source.contains("openingUnitTraceIdentityCoverageExact")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean opening segment binding should expose checked soundness and verifier core projection"
    );
    assert!(
        constant_opening_query_units_body.matches("BTreeSet").count() >= 2
            && constant_opening_query_units_body.contains("opening_identities.insert(identity)")
            && constant_opening_query_units_body.contains("LoadConstantOpeningUnitError::UnexpectedUnit")
            && constant_opening_query_units_body.contains("LoadConstantOpeningUnitError::MissingUnit")
            && witness_opening_query_units_body.matches("BTreeSet").count() >= 2
            && witness_opening_query_units_body.contains("opening_identities.insert(identity)")
            && witness_opening_query_units_body.contains("LoadWitnessOpeningUnitError::UnexpectedUnit")
            && witness_opening_query_units_body.contains("LoadWitnessOpeningUnitError::MissingUnit")
            && pcs_evaluation_query_units_body.matches("BTreeSet").count() >= 2
            && pcs_evaluation_query_units_body.contains("evaluation_identities.insert(identity)")
            && pcs_evaluation_query_units_body.contains("LoadPcsEvaluationUnitError::UnexpectedUnit")
            && pcs_evaluation_query_units_body.contains("LoadPcsEvaluationUnitError::MissingUnit")
            && verifier_query_identity_body.contains("opening_identities.insert(identity)")
            && verifier_query_identity_body.contains("challenge_identities.insert(identity)")
            && verifier_query_identity_body.contains("missing_query_identity_index"),
        "Rust query artifact validators should enforce exact identity coverage for opened units and verifier inputs"
    );
    assert!(
        fri_parse_body.contains("PCS_FRI_OPENING_V1_VERSION | PCS_FRI_OPENING_V2_VERSION")
            && fri_parse_body.contains("PcsFriOpeningSegmentError::UnsupportedVersion { version }")
            && fri_parse_body.contains("reader.require_items(unit_count, unit_header_bytes)?")
            && fri_parse_body.contains("reader.require_items(final_count, EXTENSION_BYTES)?")
            && fri_parse_body.contains("reader.require_items(layer_count, LAYER_HEADER_BYTES)?")
            && fri_parse_body.contains("reader.require_items(last_level_count, ROOT_BYTES)?")
            && fri_parse_body.contains("reader.require_items(query_count, QUERY_HEADER_BYTES)?")
            && fri_parse_body.contains("reader.require_items(value_count, EXTENSION_BYTES)?")
            && fri_parse_body.contains("reader.require_items(level_count, LEVEL_HEADER_BYTES)?")
            && fri_parse_body.contains("reader.require_items(sibling_count, ROOT_BYTES)?")
            && fri_parse_body.contains("reader.finish()?")
            && fri_parse_body.contains("validate_pcs_fri_opening_segment(&out)?"),
        "Rust FRI opening parser should validate supported version, consume the complete counted payload, and re-run semantic validation"
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
            "runtime_opening_segment_binding_evidence_implies_fri_fold_trace_identity_contract",
            "runtime_opening_segment_binding_checked_acceptance_trace_identity_coverage_exact",
            "runtime_opening_segment_binding_checked_acceptance_exact_identity_contract",
            "runtime_opening_segment_binding_evidence_implies_pcs_and_fri",
            "runtime_opening_segment_binding_checked_acceptance_query_plan_bound",
            "runtime_opening_segment_binding_checked_acceptance_bound_contract",
            "runtime_opening_segment_binding_checked_acceptance_fri_opening_checks",
            "runtime_opening_segment_binding_checked_acceptance_fri_fold_trace_identity_contract",
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
            "runtime_opening_segment_binding_checked_acceptance_full_soundness_with_fri_parser_contract",
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
        "runtime_opening_segment_binding_evidence_implies_fri_fold_trace_identity_contract",
        &[
            "RuntimeOpeningSegmentBindingEvidence",
            "RuntimeFriFoldTraceIdentityContract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_evidence_implies_fri_fold_trace_identity_contract",
        &[
            "rcases evidence with",
            "queryPlanBound",
            "traceIdentitiesMatch",
            "friFoldsValid",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_segment_binding_evidence_implies_fri_fold_trace_identity_contract",
        &["evidence.right.right.right.right.right.left"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_trace_identity_coverage_exact",
        &[
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "validation.openingUnitTraceIdentityCoverageExact",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_trace_identity_coverage_exact",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_trace_identity_coverage_exact",
        &["validation.openingSegmentBindingAcceptedImpliesTraceIdentityCoverageExact"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_exact_identity_contract",
        &[
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "RuntimeOpeningSegmentExactIdentityContract",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_exact_identity_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_exact_identity_contract",
        &[
            "validation.openingSegmentBindingAcceptedImpliesQueryPlanBound",
            "validation.openingSegmentBindingAcceptedImpliesTraceIdentitiesMatch",
            "runtime_opening_segment_binding_checked_acceptance_trace_identity_coverage_exact",
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
        "runtime_opening_segment_binding_checked_acceptance_fri_fold_trace_identity_contract",
        &[
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "RuntimeFriFoldTraceIdentityContract",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_fri_fold_trace_identity_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_fri_fold_trace_identity_contract",
        &[
            "runtime_opening_segment_binding_checked_acceptance_evidence",
            "runtime_opening_segment_binding_evidence_implies_fri_fold_trace_identity_contract",
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
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_full_soundness_with_fri_parser_contract",
        &[
            "AssumptionBundle system",
            "RuntimeFriOpeningSegmentParserBoundary system validation",
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "RuntimeOpeningSegmentBindingBoundContract",
            "RuntimeOpeningEvidence",
            "RuntimeOpeningBoundContract",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "RuntimeFriOpeningSegmentParserContract",
            "RuntimeFriFoldTraceIdentityContract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_full_soundness_with_fri_parser_contract",
        &[
            "runtime_opening_segment_binding_checked_acceptance_full_soundness_contract",
            "runtime_opening_segment_binding_checked_acceptance_fri_parser_contract",
            "runtime_opening_segment_binding_checked_acceptance_fri_fold_trace_identity_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_full_soundness_with_fri_parser_contract",
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
