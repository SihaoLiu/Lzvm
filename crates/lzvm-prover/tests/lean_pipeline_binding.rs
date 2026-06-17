use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_pipeline_binding_exports_required_external_source_soundness() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/PipelineBinding.lean");
    let pipeline_source =
        std::fs::read_to_string(&lean_path).expect("Lean pipeline binding source should read");
    let core_path = crate_root.join("../../lean/Lzvm/PipelineBinding/Core.lean");
    let core_source =
        std::fs::read_to_string(&core_path).expect("Lean pipeline core binding source should read");
    let accepts_path = crate_root.join("../../lean/Lzvm/PipelineBinding/Accepts.lean");
    let accepts_source = std::fs::read_to_string(&accepts_path)
        .expect("Lean pipeline accepts binding source should read");
    let contracts_path = crate_root.join("../../lean/Lzvm/PipelineBinding/Contracts.lean");
    let contracts_source = std::fs::read_to_string(&contracts_path)
        .expect("Lean pipeline binding contracts source should read");
    let obligations_path = crate_root.join("../../lean/Lzvm/PipelineBinding/Obligations.lean");
    let obligations_source = std::fs::read_to_string(&obligations_path)
        .expect("Lean pipeline obligations source should read");
    let audited_path = crate_root.join("../../lean/Lzvm/PipelineBinding/Audited.lean");
    let audited_source =
        std::fs::read_to_string(&audited_path).expect("Lean pipeline audited source should read");
    let lean_source = format!(
        "{core_source}\n{pipeline_source}\n{obligations_source}\n{audited_source}\n{accepts_source}\n{contracts_source}"
    );
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");
    let setup_preflight_path = crate_root.join("src/setup_preflight.rs");
    let setup_preflight_source =
        std::fs::read_to_string(&setup_preflight_path).expect("setup preflight source should read");
    let proof_artifact_path = crate_root.join("src/proof_artifact.rs");
    let proof_artifact_source =
        std::fs::read_to_string(&proof_artifact_path).expect("proof artifact source should read");
    let model_path = crate_root.join("../../lean/Lzvm/Model.lean");
    let model_source = std::fs::read_to_string(&model_path).expect("Lean model source should read");
    let conformance_path = crate_root.join("../../lean/Lzvm/Conformance.lean");
    let conformance_source =
        std::fs::read_to_string(&conformance_path).expect("Lean conformance source should read");
    let external_source_path = crate_root.join("../../lean/Lzvm/ExternalSource.lean");
    let external_source_source = std::fs::read_to_string(&external_source_path)
        .expect("Lean external source source should read");
    let trace_constraint_path = crate_root.join("../../lean/Lzvm/TraceConstraintValidation.lean");
    let trace_constraint_source = std::fs::read_to_string(&trace_constraint_path)
        .expect("Lean trace constraint source should read");
    let runtime_soundness_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness.lean");
    let runtime_soundness_source = std::fs::read_to_string(&runtime_soundness_path)
        .expect("Lean runtime soundness source should read");

    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_pipeline_binding_required_external_source_sound",
            "runtime_pipeline_binding_checked_acceptance_pipeline_evidence",
            "runtime_pipeline_binding_checked_acceptance_sound",
            "runtime_pipeline_binding_evidence_implies_transcript_bound",
            "runtime_pipeline_binding_evidence_implies_public_input_bound",
            "runtime_pipeline_binding_checked_acceptance_transcript_bound",
            "runtime_pipeline_binding_checked_acceptance_public_input_bound",
            "runtime_pipeline_binding_evidence_implies_pcs_and_fri",
            "runtime_pipeline_binding_checked_acceptance_query_plan_pcs_and_fri",
            "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_without_assumptions",
            "runtime_pipeline_binding_checked_acceptance_pcs_and_fri",
            "runtime_pipeline_binding_evidence_implies_core_obligations",
            "runtime_pipeline_binding_evidence_implies_execution_obligations",
            "runtime_pipeline_binding_evidence_implies_runtime_soundness_evidence",
            "runtime_pipeline_binding_evidence_implies_runtime_artifact_evidence",
            "runtime_pipeline_binding_evidence_implies_runtime_artifact_core_contract",
            "runtime_pipeline_binding_evidence_implies_external_source_requirements",
            "runtime_pipeline_binding_evidence_implies_seeded_query_plan_contract",
            "runtime_pipeline_binding_checked_acceptance_query_opening_evidence",
            "runtime_pipeline_binding_checked_acceptance_query_opening_contract",
            "runtime_pipeline_binding_checked_acceptance_seeded_query_plan_contract",
            "runtime_pipeline_binding_checked_acceptance_seed_binds_witness_tree_digests",
            "runtime_pipeline_binding_checked_acceptance_seeded_fri_opening_requirements_checked",
            "runtime_pipeline_binding_checked_acceptance_challenge_transcript_payload_contract",
            "runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance",
            "runtime_pipeline_binding_checked_acceptance_opening_segment_evidence",
            "runtime_pipeline_binding_checked_acceptance_opening_segment_bound_contract",
            "runtime_pipeline_binding_checked_acceptance_opening_bound_contract",
            "runtime_pipeline_binding_checked_acceptance_constant_opening_bound_from_concrete_nary_merkle",
            "runtime_pipeline_binding_checked_acceptance_witness_opening_bound_from_concrete_nary_merkle",
            "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
            "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
            "runtime_pipeline_binding_checked_acceptance_runtime_soundness_evidence_from_opening_checks",
            "runtime_pipeline_binding_checked_acceptance_challenge_query_opening_contract",
            "runtime_pipeline_binding_checked_acceptance_challenge_query_opening_core_contract",
            "runtime_pipeline_binding_checked_acceptance_full_soundness_contract",
            "runtime_pipeline_binding_checked_acceptance_trace_conformance_contract",
            "runtime_pipeline_compact_digest_merkle_observation_eq_full_state",
            "runtime_pipeline_binding_checked_acceptance_compact_digest_merkle_contract",
            "runtime_pipeline_binding_checked_acceptance_audited_assumptions",
            "runtime_pipeline_binding_checked_acceptance_verifier_sound_witness",
            "runtime_pipeline_binding_checked_acceptance_verifier_core_contract",
            "runtime_pipeline_binding_checked_acceptance_execution_obligations",
            "runtime_pipeline_binding_checked_acceptance_runtime_soundness_contract",
            "runtime_pipeline_binding_checked_acceptance_runtime_soundness_accepts_contract",
            "runtime_pipeline_binding_checked_acceptance_runtime_artifact_evidence",
            "runtime_pipeline_binding_checked_acceptance_runtime_artifact_soundness_obligations",
            "runtime_pipeline_binding_checked_acceptance_trace_artifact_soundness_obligations",
            "runtime_pipeline_binding_checked_acceptance_accepts_full_soundness_contract",
            "runtime_pipeline_binding_checked_acceptance_proof_system_sound",
            "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract",
            "runtime_pipeline_binding_checked_acceptance_audited_proof_system_contract",
            "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract",
            "runtime_pipeline_binding_evidence_audited_core_contract",
            "runtime_pipeline_binding_checked_acceptance_audited_accepts_sound_witness_contract",
            "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract",
            "runtime_pipeline_binding_checked_acceptance_audited_concrete_opening_contract",
            "runtime_pipeline_checked_acceptance_audited_concrete_opening_core_contract",
            "runtime_pipeline_binding_checked_acceptance_hash_concrete_opening_core_contract",
            "runtime_pipeline_binding_required_external_source_verifier_core_contract",
            "runtime_pipeline_binding_required_external_source_full_soundness_contract",
            "runtime_pipeline_binding_required_external_source_proof_system_full_soundness_contract",
            "runtime_pipeline_binding_required_external_source_audited_proof_system_contract",
            "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract",
            "runtime_pipeline_required_external_source_concrete_opening_core_contract",
            "runtime_pipeline_binding_required_external_source_audited_accepts_sound_witness_contract",
            "runtime_pipeline_binding_required_external_source_audited_pcs_accepts_sound_witness_contract",
            "runtime_pipeline_binding_required_external_source_audited_pcs_fri_witness_contract",
            "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract",
            "runtime_pipeline_binding_required_external_source_audited_seeded_query_requirements_contract",
        ],
    );
    assert!(
        pipeline_source.contains("import Lzvm.PipelineBinding.Core"),
        "Lean pipeline binding module should import the core pipeline binding module"
    );
    assert!(
        pipeline_source.contains("import Lzvm.PipelineBinding.Obligations")
            && pipeline_source.contains("import Lzvm.PipelineBinding.Audited"),
        "Lean pipeline binding module should re-export split obligation and audited contracts"
    );
    assert!(
        top_level_source.contains("import Lzvm.PipelineBinding.Contracts"),
        "top-level Lean module should import pipeline binding contracts"
    );
    assert!(
        top_level_source.contains("import Lzvm.PipelineBinding.Accepts"),
        "top-level Lean module should import pipeline binding accepts contracts"
    );
    assert!(theorem_prefix(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_runtime_artifact_evidence"
    )
    .contains("(validation : RuntimePipelineBindingValidation system)"));
    assert!(theorem_prefix(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_pipeline_evidence"
    )
    .contains("(validation : RuntimePipelineBindingValidation system)"));
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_pipeline_evidence"
        )
        .contains("(assumptions : AssumptionBundle system)"),
        "pipeline evidence projection should require the audited assumption bundle"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_runtime_artifact_evidence"
        )
        .contains("AssumptionBundle"),
        "runtime artifact evidence projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_evidence_implies_runtime_artifact_core_contract"
        )
        .contains("RuntimeArtifactEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_evidence_implies_runtime_artifact_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof"),
        "pipeline evidence should package runtime artifact evidence with verifier core obligations"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_evidence_implies_runtime_artifact_core_contract"
        )
        .contains("AssumptionBundle"),
        "pipeline evidence artifact-core projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_evidence_implies_external_source_requirements"
        )
        .contains("RuntimePipelineBindingEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_evidence_implies_external_source_requirements"
            )
            .contains("ExternalSourceOpeningRequirement")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_evidence_implies_external_source_requirements"
            )
            .contains("runtime_pipeline_trace_source_validation validation")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_evidence_implies_external_source_requirements"
            )
            .contains("runtime_pipeline_opening_source_validation validation"),
        "pipeline evidence should expose both trace and opening external-source requirements"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_evidence_implies_external_source_requirements"
        )
        .contains("AssumptionBundle"),
        "pipeline external-source requirement projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_evidence_implies_seeded_query_plan_contract"
        )
        .contains("RuntimePipelineBindingEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_evidence_implies_seeded_query_plan_contract"
            )
            .contains("RuntimeQueryPlanBindingSeededContract"),
        "pipeline evidence should retain seeded query-plan witness digest and FRI-opening obligations"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_evidence_implies_seeded_query_plan_contract"
        )
        .contains("AssumptionBundle"),
        "pipeline seeded query-plan projection should not require cryptographic assumptions"
    );
    for theorem in [
        "runtime_pipeline_binding_evidence_implies_transcript_bound",
        "runtime_pipeline_binding_evidence_implies_public_input_bound",
        "runtime_pipeline_binding_evidence_implies_pcs_and_fri",
        "runtime_pipeline_binding_evidence_implies_runtime_artifact_core_contract",
        "runtime_pipeline_binding_evidence_implies_external_source_requirements",
        "runtime_pipeline_binding_evidence_implies_execution_obligations",
        "runtime_pipeline_binding_evidence_implies_runtime_soundness_evidence",
        "runtime_pipeline_binding_evidence_implies_runtime_artifact_evidence",
    ] {
        lean_binding::assert_theorem_body_omits(&lean_source, theorem, &[".right.right.right"]);
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_query_plan_pcs_and_fri",
        &[
            "RuntimePipelineBindingCheckedAcceptance",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_query_plan_pcs_and_fri",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_without_assumptions",
        &[
            "RuntimePipelineBindingCheckedAcceptance",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_without_assumptions",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_without_assumptions",
        &["runtime_pipeline_binding_checked_acceptance_query_plan_pcs_and_fri"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_required_external_source_audited_seeded_query_requirements_contract",
        &[
            "RuntimePipelineBindingCheckedAcceptance",
            "requiresExternalSource",
            "validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests",
            "validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_required_external_source_audited_seeded_query_requirements_contract",
        &[
            "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract",
            "seedBinds",
            "seededFriOpeningChecked",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_runtime_soundness_evidence_from_opening_checks",
        &[
            "AssumptionBundle system",
            "RuntimePipelineBindingCheckedAcceptance",
            "RuntimeSoundnessEvidence",
            "(runtime_pipeline_runtime_soundness_validation validation)",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_runtime_soundness_evidence_from_opening_checks",
        &[
            "runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance",
            "runtime_opening_segment_binding_checked_acceptance_opening",
            "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_opening_checks",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_runtime_soundness_evidence_from_opening_checks",
        &[
            "runtime_pipeline_binding_checked_acceptance_sound",
            "runtime_pipeline_binding_evidence_implies_runtime_soundness_evidence",
            "runtime_soundness_checked_acceptance_evidence",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_runtime_soundness_evidence_from_hash_concrete_opening",
        &[
            "AssumptionBundle system",
            "HashCollisionResistanceAssumption",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "RuntimePipelineBindingCheckedAcceptance",
            "RuntimeSoundnessEvidence",
            "(runtime_pipeline_runtime_soundness_validation validation)",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_runtime_soundness_evidence_from_hash_concrete_opening",
        &[
            "runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance",
            "runtime_opening_segment_binding_checked_acceptance_opening",
            "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_hash_concrete_opening",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_runtime_soundness_evidence_from_hash_concrete_opening",
        &[
            "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_opening_checks",
            "runtime_pipeline_binding_checked_acceptance_sound",
            "runtime_pipeline_binding_evidence_implies_runtime_soundness_evidence",
        ],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_runtime_soundness_accepts_contract"
        )
        .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_runtime_soundness_accepts_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_runtime_soundness_accepts_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_runtime_soundness_accepts_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "runtime soundness accepts contract should expose verifier acceptance and runtime evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_runtime_artifact_soundness_obligations"
        )
        .contains("(assumptions : AssumptionBundle system)"),
        "runtime artifact soundness obligations should require the audited assumption bundle"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_trace_artifact_soundness_obligations"
        )
        .contains("(assumptions : AssumptionBundle system)")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_trace_artifact_soundness_obligations"
            )
            .contains("requiresExternalSource")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_trace_artifact_soundness_obligations"
            )
            .contains("RuntimeTraceConstraintSoundnessObligations"),
        "trace artifact soundness obligations should expose the trace contract with audited assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_accepts_full_soundness_contract"
        )
        .contains("(assumptions : AssumptionBundle system)"),
        "accepts plus full soundness contract should require the audited assumption bundle"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_accepts_full_soundness_contract"
        )
        .contains("system.accepts publicInput proof"),
        "accepts plus full soundness contract should expose verifier acceptance"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_proof_system_sound"
        )
        .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_sound"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_sound"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline checked acceptance should expose model-wide proof-system soundness and the accepted proof witness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_audited_accepts_sound_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_accepts_sound_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_accepts_sound_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_accepts_sound_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline checked acceptance should expose compact audited acceptance and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_audited_accepts_sound_witness_contract"
        )
        .contains("RuntimePipelineBindingEvidence"),
        "compact audited pipeline acceptance contract should not force callers to unpack full pipeline evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract"
        )
        .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("RuntimePipelineBindingEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline checked acceptance should package model soundness with the accepted full soundness contract"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_audited_proof_system_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_contract"
            )
            .contains("RuntimePipelineBindingEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline checked acceptance should package audited crypto assumptions with proof-system soundness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
            )
            .contains("validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
            )
            .contains(
                "validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked"
            )
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
            )
            .contains("system.traceConsistent publicInput proof trace")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline contracts should expose compact proof-system, binding, execution, verifier core, and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
        )
        .contains("RuntimePipelineBindingEvidence")
            && !theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations"),
        "compact proof-system core contract should not force callers to unpack full pipeline evidence or artifact obligations"
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_evidence_audited_core_contract",
        &[
            "RuntimePipelineBindingEvidence",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_pipeline_binding_evidence_audited_core_contract",
        &["RuntimePipelineBindingCheckedAcceptance"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_evidence_audited_core_contract",
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "runtime_pipeline_binding_evidence_implies_pcs_and_fri",
            "runtime_pipeline_binding_evidence_implies_core_obligations",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_pipeline_binding_evidence_audited_core_contract",
        &["abstract_verifier_sound"],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains(
                "validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked"
            )
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline checked acceptance should expose compact audited binding, seeded PCS/FRI, verifier core, and witness evidence"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract",
        &[
            "runtime_pipeline_binding_checked_acceptance_seed_binds_witness_tree_digests",
            "runtime_pipeline_binding_checked_acceptance_seeded_fri_opening_requirements_checked",
        ],
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
        )
        .contains("RuntimePipelineBindingEvidence"),
        "compact audited pipeline binding contract should not force callers to unpack full pipeline evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_seeded_query_plan_contract"
        )
        .contains("RuntimeQueryPlanBindingSeededContract"),
        "pipeline checked acceptance should project seeded query-plan constraints"
    );
    assert!(
        {
            let prefix = theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_seed_binds_witness_tree_digests",
            );
            prefix.contains(
                "validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests",
            ) && prefix.contains("artifact")
                && prefix.contains("publicInput")
                && prefix.contains("proof")
        },
        "pipeline checked acceptance should expose the seeded query seed witness-tree digest binding"
    );
    assert!(
        {
            let prefix = theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_seeded_fri_opening_requirements_checked",
            );
            prefix.contains(
                "validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked",
            ) && prefix.contains("artifact")
                && prefix.contains("publicInput")
                && prefix.contains("proof")
        },
        "pipeline checked acceptance should expose the seeded FRI opening requirement check"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_seed_binds_witness_tree_digests",
        &[
            "runtime_pipeline_binding_checked_acceptance_seeded_query_plan_contract",
            "runtime_query_plan_binding_seeded_contract_implies_seed_binds_witness_tree_digests",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_seeded_fri_opening_requirements_checked",
        &[
            "runtime_pipeline_binding_checked_acceptance_seeded_query_plan_contract",
            "runtime_query_plan_binding_seeded_contract_implies_seeded_fri_opening_requirements_checked",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_challenge_transcript_payload_contract",
        &[
            "RuntimeChallengeSegmentBindingEvidence",
            "RuntimeTranscriptBindingEvidence",
            "RuntimeArtifactEvidence",
            "RuntimeTranscriptBindingPayloadContract",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_challenge_transcript_payload_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_challenge_transcript_payload_contract",
        &[
            "runtime_pipeline_binding_checked_acceptance_query_plan",
            "runtime_query_plan_binding_checked_acceptance_challenge",
            "runtime_challenge_segment_binding_checked_acceptance_transcript_payload_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_challenge_transcript_payload_contract",
        &["abstract_verifier_sound"],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_challenge_query_opening_core_contract"
        )
        .contains("RuntimeChallengeSegmentBindingEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_challenge_query_opening_core_contract"
            )
            .contains("RuntimeTranscriptBindingEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_challenge_query_opening_core_contract"
            )
            .contains("RuntimeQueryPlanBindingBoundContract")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_challenge_query_opening_core_contract"
            )
            .contains("RuntimeOpeningSegmentBindingBoundContract")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_challenge_query_opening_core_contract"
            )
            .contains("RuntimeOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_challenge_query_opening_core_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_challenge_query_opening_core_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_challenge_query_opening_core_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_challenge_query_opening_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof"),
        "pipeline checked acceptance should expose challenge, transcript, query, opening, and verifier core evidence"
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_opening_bound_contract",
        &["RuntimeOpeningBoundContract"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_opening_bound_contract",
        &[
            "runtime_opening_segment_binding_checked_acceptance_opening_bound_contract",
            "runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_constant_opening_bound_from_concrete_nary_merkle",
        &[
            "AssumptionBundle system",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimePipelineBindingCheckedAcceptance",
            "constantOpeningsBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_constant_opening_bound_from_concrete_nary_merkle",
        &[
            "runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance",
            "runtime_opening_segment_binding_checked_acceptance_opening",
            "runtime_constant_opening_nary_checked_acceptance_constant_bound_from_bundle",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_witness_opening_bound_from_concrete_nary_merkle",
        &[
            "AssumptionBundle system",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimePipelineBindingCheckedAcceptance",
            "witnessOpeningsBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_witness_opening_bound_from_concrete_nary_merkle",
        &[
            "runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance",
            "runtime_opening_segment_binding_checked_acceptance_opening",
            "runtime_witness_opening_nary_checked_acceptance_witness_bound_from_bundle",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
        &[
            "AssumptionBundle system",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimePipelineBindingCheckedAcceptance",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
        &[
            "runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance",
            "runtime_opening_segment_binding_checked_acceptance_opening",
            "runtime_opening_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
        &["runtime_pipeline_binding_checked_acceptance_query_plan_pcs_and_fri"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        &[
            "HashCollisionResistanceAssumption",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimePipelineBindingCheckedAcceptance",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        &[
            "runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance",
            "runtime_opening_segment_binding_checked_acceptance_opening",
            "runtime_opening_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        &["runtime_pipeline_binding_checked_acceptance_query_plan_pcs_and_fri"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_audited_concrete_opening_contract",
        &[
            "AssumptionBundle system",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimePipelineBindingCheckedAcceptance",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "ProofSystemSound system",
            "system.accepts publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_audited_concrete_opening_contract",
        &[
            "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
            "runtime_pipeline_binding_checked_acceptance_audited_accepts_sound_witness_contract",
            "runtime_pipeline_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_audited_concrete_opening_contract",
        &["runtime_pipeline_binding_checked_acceptance_pcs_and_fri\n"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_checked_acceptance_audited_concrete_opening_core_contract",
        &[
            "AssumptionBundle system",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimePipelineBindingCheckedAcceptance",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "ProofSystemSound system",
            "system.accepts publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_checked_acceptance_audited_concrete_opening_core_contract",
        &[
            "runtime_pipeline_binding_checked_acceptance_audited_concrete_opening_contract",
            "runtime_pipeline_binding_checked_acceptance_execution_obligations",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_pipeline_checked_acceptance_audited_concrete_opening_core_contract",
        &["runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_hash_concrete_opening_core_contract",
        &[
            "AssumptionBundle system",
            "HashCollisionResistanceAssumption",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "hashAssumptions",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "RuntimePipelineBindingCheckedAcceptance",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "ProofSystemSound system",
            "system.accepts publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_hash_concrete_opening_core_contract",
        &[
            "runtime_query_plan_binding_checked_acceptance_seeded_hash_concrete_opening_and_core_contract",
            "runtime_pipeline_binding_checked_acceptance_audited_accepts_sound_witness_contract",
            "runtime_pipeline_binding_checked_acceptance_execution_obligations",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_hash_concrete_opening_core_contract",
        &[
            "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
            "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
            "runtime_pipeline_binding_checked_acceptance_pcs_and_fri\n",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_challenge_query_opening_core_contract",
        &[".right.right.right"],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_verifier_core_contract"
        )
        .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_verifier_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof"),
        "pipeline required external-source projection should expose external evidence and verifier core contract"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_full_soundness_contract"
        )
        .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_full_soundness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_full_soundness_contract"
            )
            .contains("RuntimePipelineBindingEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_full_soundness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_full_soundness_contract"
            )
            .contains("system.traceConsistent publicInput proof trace")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline required external-source projection should expose the accepted full soundness contract"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_proof_system_full_soundness_contract"
        )
        .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("RuntimePipelineBindingEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline required external-source projection should package model soundness with the full soundness contract"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_audited_proof_system_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_contract"
            )
            .contains("RuntimePipelineBindingEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline required external-source projection should package audited crypto assumptions with proof-system soundness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("runtime_pipeline_trace_source_validation validation")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("runtime_pipeline_opening_source_validation validation")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains(
                "validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked"
            )
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.traceConsistent publicInput proof trace")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline required external-source projection should package audited proof-system core obligations"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
        )
        .contains("RuntimePipelineBindingEvidence")
            && !theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations"),
        "compact required external-source proof-system core contract should not force full artifact evidence"
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_binding_required_external_source_contracts_core_contract",
        &[
            "validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests",
            "validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_required_external_source_contracts_core_contract",
        &["runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_required_external_source_concrete_opening_core_contract",
        &[
            "AssumptionBundle system",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimePipelineBindingCheckedAcceptance",
            "requiresExternalSource",
            "ExternalSourceOpeningEvidence",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "ProofSystemSound system",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_required_external_source_concrete_opening_core_contract",
        &[
            "runtime_pipeline_binding_required_external_source_audited_accepts_sound_witness_contract",
            "runtime_pipeline_checked_acceptance_audited_concrete_opening_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_pipeline_required_external_source_concrete_opening_core_contract",
        &["runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_pipeline_required_external_source_hash_concrete_opening_core_contract",
        &[
            "AssumptionBundle system",
            "HashCollisionResistanceAssumption",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "hashAssumptions",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "RuntimePipelineBindingCheckedAcceptance",
            "requiresExternalSource",
            "ExternalSourceOpeningEvidence",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "ProofSystemSound system",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_required_external_source_hash_concrete_opening_core_contract",
        &[
            "runtime_pipeline_binding_required_external_source_audited_accepts_sound_witness_contract",
            "runtime_pipeline_binding_checked_acceptance_hash_concrete_opening_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_pipeline_required_external_source_hash_concrete_opening_core_contract",
        &[
            "runtime_pipeline_checked_acceptance_audited_concrete_opening_core_contract",
            "runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract",
        ],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_audited_accepts_sound_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("runtime_pipeline_trace_source_validation validation")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("runtime_pipeline_opening_source_validation validation")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline required external-source projection should expose compact audited acceptance, both external-source evidences, and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_audited_accepts_sound_witness_contract"
        )
        .contains("RuntimePipelineBindingEvidence"),
        "compact required external-source pipeline contract should not force callers to unpack full pipeline evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_audited_pcs_accepts_sound_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("runtime_pipeline_trace_source_validation validation")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("runtime_pipeline_opening_source_validation validation")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline required external-source projection should expose compact audited acceptance, both external-source evidences, PCS openings, and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_audited_pcs_accepts_sound_witness_contract"
        )
        .contains("RuntimePipelineBindingEvidence"),
        "compact required external-source pipeline PCS contract should not force callers to unpack full pipeline evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_audited_pcs_fri_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("runtime_pipeline_trace_source_validation validation")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("runtime_pipeline_opening_source_validation validation")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline required external-source projection should expose compact audited acceptance, both external-source evidences, PCS openings, FRI queries, and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_audited_pcs_fri_witness_contract"
        )
        .contains("RuntimePipelineBindingEvidence"),
        "compact required external-source pipeline PCS/FRI contract should not force callers to unpack full pipeline evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("runtime_pipeline_trace_source_validation validation")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("runtime_pipeline_opening_source_validation validation")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains(
                "validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked"
            )
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline required external-source projection should expose compact audited seeded PCS/FRI, verifier core obligations, and witness evidence"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract",
        &[
            "runtime_pipeline_binding_checked_acceptance_seed_binds_witness_tree_digests",
            "runtime_pipeline_binding_checked_acceptance_seeded_fri_opening_requirements_checked",
        ],
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract"
        )
        .contains("RuntimePipelineBindingEvidence"),
        "compact required external-source pipeline PCS/FRI core contract should not force callers to unpack full pipeline evidence"
    );
    assert!(
        lean_source.contains("runtime_trace_constraint_required_external_source_pcs_sound")
            && lean_source.contains("runtime_opening_required_external_source_sound")
            && lean_source.contains("RuntimeOpeningSegmentBindingBoundContract")
            && lean_source.contains("RowMajorDigestPrefixEvidence")
            && lean_source.contains("wideLinearDigestsBindRows")
            && lean_source.contains("constraintBackendConformant")
            && lean_source.contains("system.accepts publicInput proof")
            && lean_source.contains("RuntimeArtifactSoundnessObligations")
            && lean_source.contains("RuntimeVerifierCoreContract")
            && lean_source.contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && lean_source.contains("assumption_bundle_carries_required_crypto_evidence")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean pipeline binding should expose query-plan, opening, execution, and full runtime soundness evidence"
    );
    assert!(
        model_source.contains("def RuntimeVerifierCoreContract")
            && model_source.contains("sound_witness_implies_verifier_core_contract")
            && model_source.contains("SoundWitness system publicInput proof")
            && model_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && runtime_soundness_source.contains("RuntimeVerifierCoreContract")
            && runtime_soundness_source.contains("runtime_soundness_checked_acceptance_core_obligations"),
        "Lean runtime soundness should name the verifier core obligations shared by pipeline soundness"
    );
    assert!(
        conformance_source.contains("RuntimeArtifactSoundnessObligations")
            && conformance_source.contains("RuntimeVerifierCoreContract")
            && external_source_source.contains("ExternalSourceOpeningSoundnessObligations")
            && external_source_source.contains("RuntimeVerifierCoreContract")
            && trace_constraint_source.contains("RuntimeTraceConstraintSoundnessObligations")
            && trace_constraint_source.contains("RuntimeVerifierCoreContract"),
        "Lean soundness obligations should share the model-level verifier core contract"
    );
    assert!(
        setup_preflight_source.contains("validate_global_source_lookup_hints")
            && setup_preflight_source.contains("SourceLookupBalance::default")
            && setup_preflight_source.contains("validate_all_units"),
        "setup preflight should keep source lookup balance validation wired into proof checks"
    );
    assert!(
        proof_artifact_source.contains("build_pcs_query_plan_segment")
            && proof_artifact_source.contains("build_witness_opening_segment_batch_from_trace_outputs")
            && proof_artifact_source
                .contains("build_pcs_fri_transcript_values_from_trace_segment_refs")
            && proof_artifact_source
                .contains("build_pcs_fri_opening_segment_from_transcript_values"),
        "proof artifact building should keep query-plan, transcript, and opening builders wired together"
    );
}

fn theorem_prefix(source: &str, name: &str) -> String {
    let theorem_start = source
        .find(&format!("theorem {name}"))
        .unwrap_or_else(|| panic!("Lean source should contain theorem {name}"));
    let proof_start = source[theorem_start..]
        .find(" := by")
        .unwrap_or_else(|| panic!("Lean theorem {name} should have a proof body"));
    source[theorem_start..theorem_start + proof_start].to_owned()
}
