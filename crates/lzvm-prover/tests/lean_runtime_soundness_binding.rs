use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_runtime_soundness_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness.lean");
    let runtime_soundness_source =
        std::fs::read_to_string(&lean_path).expect("Lean runtime soundness source should read");
    let contracts_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness/Contracts.lean");
    let contracts_source = std::fs::read_to_string(&contracts_path)
        .expect("Lean runtime soundness contracts source should read");
    let lean_source = format!("{runtime_soundness_source}\n{contracts_source}");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.RuntimeSoundness"),
        "top-level Lean module should import runtime soundness"
    );
    assert!(
        top_level_source.contains("import Lzvm.RuntimeSoundness.Contracts"),
        "top-level Lean module should import runtime soundness contracts"
    );
    assert!(
        lean_source.contains("RuntimeSoundnessValidation")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("system.publicInputBound publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean runtime soundness should expose checked soundness and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_soundness_checked_acceptance_sound",
            "runtime_soundness_evidence_implies_runtime_artifact_evidence",
            "runtime_soundness_evidence_implies_transcript_bound",
            "runtime_soundness_evidence_implies_public_input_bound",
            "runtime_soundness_evidence_implies_core_obligations",
            "runtime_soundness_evidence_implies_external_source_requirement",
            "runtime_soundness_evidence_implies_binding_pcs_fri_contract",
            "runtime_soundness_evidence_implies_runtime_artifact_core_contract",
            "runtime_soundness_checked_acceptance_runtime_artifact_evidence",
            "runtime_soundness_checked_acceptance_transcript_bound",
            "runtime_soundness_checked_acceptance_public_input_bound",
            "runtime_soundness_checked_acceptance_pcs_and_fri",
            "runtime_soundness_checked_acceptance_external_source_requirement",
            "runtime_soundness_checked_acceptance_core_obligations",
            "runtime_soundness_checked_acceptance_runtime_artifact_core_contract",
            "runtime_soundness_checked_acceptance_verifier_core_contract",
            "runtime_soundness_checked_acceptance_verifier_sound_witness",
            "runtime_soundness_checked_acceptance_execution_obligations",
            "runtime_soundness_checked_acceptance_audited_core_contract",
            "runtime_soundness_checked_acceptance_verifier_accepts",
            "runtime_soundness_checked_acceptance_accepts_core_sound_witness",
            "runtime_soundness_checked_acceptance_proof_system_sound",
            "runtime_soundness_checked_acceptance_full_soundness_contract",
            "runtime_soundness_checked_acceptance_accepts_full_soundness_contract",
            "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract",
            "runtime_soundness_checked_acceptance_audited_proof_system_contract",
            "runtime_soundness_checked_acceptance_audited_accepts_sound_witness_contract",
            "runtime_soundness_required_external_source_pcs_sound",
            "runtime_soundness_required_external_source_verifier_core_contract",
            "runtime_soundness_required_external_source_accepts_core_sound_witness",
            "runtime_soundness_required_external_source_full_soundness_contract",
            "runtime_soundness_required_external_source_proof_system_full_soundness_contract",
            "runtime_soundness_required_external_source_audited_proof_system_contract",
            "runtime_soundness_required_external_source_audited_proof_system_core_contract",
            "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract",
            "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract",
            "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract",
            "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract",
            "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract",
            "runtime_soundness_checked_acceptance_contracts_core_contract",
            "runtime_soundness_checked_acceptance_artifact_contracts_core_contract",
            "runtime_soundness_required_external_source_contracts_core_contract",
        ],
    );
    assert!(theorem_prefix(
        &lean_source,
        "runtime_soundness_checked_acceptance_runtime_artifact_evidence"
    )
    .contains("(validation : RuntimeSoundnessValidation system)"));
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_runtime_artifact_evidence"
        )
        .contains("AssumptionBundle"),
        "runtime artifact evidence projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_external_source_requirement"
        )
        .contains("(validation : RuntimeSoundnessValidation system)"),
        "external-source requirement projection should be tied to runtime soundness validation"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_external_source_requirement"
        )
        .contains("AssumptionBundle"),
        "external-source requirement projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_external_source_requirement"
        )
        .contains("(validation : RuntimeSoundnessValidation system)"),
        "runtime soundness evidence projection should be tied to runtime soundness validation"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_external_source_requirement"
        )
        .contains("ExternalSourceOpeningRequirement")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_evidence_implies_external_source_requirement"
            )
            .contains("validation.sourceValidation"),
        "runtime soundness evidence should expose the external-source requirement field"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_external_source_requirement"
        )
        .contains("AssumptionBundle"),
        "runtime soundness evidence external-source projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_binding_pcs_fri_contract"
        )
        .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_evidence_implies_binding_pcs_fri_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_evidence_implies_binding_pcs_fri_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_evidence_implies_binding_pcs_fri_contract"
            )
            .contains("system.friQueriesValid publicInput proof"),
        "runtime soundness evidence should expose the compact transcript/public-input/PCS/FRI contract"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_binding_pcs_fri_contract"
        )
        .contains("AssumptionBundle"),
        "runtime soundness evidence binding PCS/FRI projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_runtime_artifact_core_contract"
        )
        .contains("RuntimeArtifactEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_evidence_implies_runtime_artifact_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof"),
        "runtime soundness evidence should package runtime artifact evidence with verifier core obligations"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_runtime_artifact_core_contract"
        )
        .contains("AssumptionBundle"),
        "runtime soundness evidence artifact-core projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_runtime_artifact_core_contract"
        )
        .contains("(assumptions : AssumptionBundle system)")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_runtime_artifact_core_contract"
            )
            .contains("RuntimeArtifactEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_runtime_artifact_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof"),
        "checked runtime soundness should expose artifact evidence plus verifier core obligations under audited assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_core_contract"
        )
        .contains("(assumptions : AssumptionBundle system)"),
        "audited runtime core contract should require the audited assumption bundle"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_core_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_core_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "audited runtime core contract should package crypto evidence, verifier core obligations, and sound witness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_verifier_accepts"
        )
        .contains("(validation : RuntimeSoundnessValidation system)")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_verifier_accepts"
            )
            .contains("system.accepts publicInput proof"),
        "checked runtime soundness should expose verifier acceptance through validation"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_verifier_accepts"
        )
        .contains("AssumptionBundle"),
        "verifier acceptance projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_accepts_core_sound_witness"
        )
        .contains("(assumptions : AssumptionBundle system)")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_accepts_core_sound_witness"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_accepts_core_sound_witness"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_accepts_core_sound_witness"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should package acceptance, core obligations, and sound witness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_proof_system_sound"
        )
        .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_sound"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_sound"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should expose model-wide proof-system soundness and the accepted proof witness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_accepts_sound_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_accepts_sound_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_accepts_sound_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_accepts_sound_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should expose compact audited acceptance and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_accepts_sound_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact audited runtime acceptance contract should not force callers to unpack full runtime evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_full_soundness_contract"
        )
        .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_full_soundness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should package evidence, artifact obligations, core obligations, and sound witness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_accepts_full_soundness_contract"
        )
        .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_accepts_full_soundness_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_accepts_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_accepts_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should package verifier acceptance with the full soundness contract"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract"
        )
        .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should package model soundness with the accepted full soundness contract"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_proof_system_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_proof_system_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_proof_system_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_proof_system_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_proof_system_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_proof_system_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should package audited crypto assumptions with proof-system soundness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_accepts_core_sound_witness"
        )
        .contains("(assumptions : AssumptionBundle system)")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_accepts_core_sound_witness"
            )
            .contains("requiresExternalSource ->")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_accepts_core_sound_witness"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_accepts_core_sound_witness"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_accepts_core_sound_witness"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_accepts_core_sound_witness"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should package acceptance, external-source evidence, core obligations, and sound witness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_full_soundness_contract"
        )
        .contains("(assumptions : AssumptionBundle system)")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_full_soundness_contract"
            )
            .contains("requiresExternalSource ->")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_full_soundness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_full_soundness_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_full_soundness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should expose the full soundness contract"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
        )
        .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should package model soundness with the full soundness contract"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_proof_system_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should package audited crypto assumptions with proof-system soundness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_proof_system_core_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("validation.sourceValidation")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.traceConsistent publicInput proof trace")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should package audited proof-system core obligations"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_proof_system_core_contract"
        )
        .contains("RuntimeSoundnessEvidence")
            && !theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations"),
        "compact required external-source proof-system core contract should not force full runtime artifact evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should expose compact audited acceptance, external-source evidence, and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact required external-source runtime contract should not force callers to unpack full runtime evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should expose compact audited acceptance, external-source evidence, PCS openings, and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact required external-source runtime PCS contract should not force callers to unpack full runtime evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should expose compact audited acceptance, external-source evidence, PCS openings, FRI queries, and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact required external-source runtime PCS/FRI contract should not force callers to unpack full runtime evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should expose compact audited PCS/FRI, core obligations, and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact required external-source runtime PCS/FRI core contract should not force callers to unpack full runtime evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_contracts_core_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("validation.sourceValidation")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("system.traceConsistent publicInput proof trace")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "runtime contracts should expose required external-source proof-system core obligations"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_contracts_core_contract"
        )
        .contains("RuntimeSoundnessEvidence")
            && !theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations"),
        "runtime required external-source contracts wrapper should keep the compact core surface"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should expose compact audited transcript/public-input/PCS/FRI core witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact checked runtime binding PCS/FRI core contract should not force callers to unpack full runtime evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_contracts_core_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("system.traceConsistent publicInput proof trace")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime contracts should expose compact proof-system core obligations"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_contracts_core_contract"
        )
        .contains("RuntimeSoundnessEvidence")
            && !theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations"),
        "checked runtime contracts wrapper should keep the compact core surface"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
        )
        .contains("RuntimeArtifactEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("system.traceConsistent publicInput proof trace")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime artifact contracts should expose artifact evidence plus compact proof-system core obligations"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
        )
        .contains("RuntimeSoundnessEvidence")
            && !theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations"),
        "checked runtime artifact contracts wrapper should keep the compact artifact-core surface"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_core_obligations",
        &[
            "runtime_soundness_checked_acceptance_evidence",
            "runtime_soundness_evidence_implies_core_obligations",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_core_obligations",
        &["runtime_soundness_checked_acceptance_sound", "sound.right"],
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
