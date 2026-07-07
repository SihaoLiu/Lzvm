/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.TraceConstraintValidation

/-!
Runtime trace constraint artifact binding obligations.
-/

namespace Lzvm

structure RuntimeTraceConstraintArtifactBindingValidation (system : VerifierModel) where
  traceConstraintValidation : RuntimeTraceConstraintValidation system
  traceArtifactBindingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  traceConstraintSegmentPayloadValid : RuntimeArtifact -> PublicInput -> Proof -> Prop
  witnessCommitmentSegmentsMatchTraceEvidence : RuntimeArtifact -> PublicInput -> Proof -> Prop
  constraintCatalogMatchesTraceEvidence : RuntimeArtifact -> PublicInput -> Proof -> Prop
  traceArtifactBindingAcceptedImpliesTraceConstraintAccepted :
    forall artifact publicInput proof,
      traceArtifactBindingAccepted artifact publicInput proof ->
        traceConstraintValidation.traceConstraintAccepted artifact publicInput proof
  traceArtifactBindingAcceptedImpliesPayloadValid :
    forall artifact publicInput proof,
      traceArtifactBindingAccepted artifact publicInput proof ->
        traceConstraintSegmentPayloadValid artifact publicInput proof
  traceArtifactBindingAcceptedImpliesWitnessCommitmentSegmentsMatch :
    forall artifact publicInput proof,
      traceArtifactBindingAccepted artifact publicInput proof ->
        witnessCommitmentSegmentsMatchTraceEvidence artifact publicInput proof
  traceArtifactBindingAcceptedImpliesConstraintCatalogMatches :
    forall artifact publicInput proof,
      traceArtifactBindingAccepted artifact publicInput proof ->
        constraintCatalogMatchesTraceEvidence artifact publicInput proof
  traceArtifactChecksImplyWitnessCommitmentBinding :
    forall artifact publicInput proof,
      traceConstraintSegmentPayloadValid artifact publicInput proof ->
        witnessCommitmentSegmentsMatchTraceEvidence artifact publicInput proof ->
          traceConstraintValidation.traceEvidenceMatchesWitnessCommitments
            artifact
            publicInput
            proof
  traceArtifactChecksImplyConstraintCatalogBinding :
    forall artifact publicInput proof,
      traceConstraintSegmentPayloadValid artifact publicInput proof ->
        constraintCatalogMatchesTraceEvidence artifact publicInput proof ->
          traceConstraintValidation.traceEvidenceMatchesConstraintCatalog
            artifact
            publicInput
            proof

def RuntimeTraceConstraintPreflightBindingEvidence
    (_system : VerifierModel)
    (validation : RuntimeTraceConstraintArtifactBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.traceConstraintSegmentPayloadValid artifact publicInput proof
    /\ validation.witnessCommitmentSegmentsMatchTraceEvidence artifact publicInput proof
    /\ validation.constraintCatalogMatchesTraceEvidence artifact publicInput proof
    /\ validation.traceConstraintValidation.traceEvidenceMatchesWitnessCommitments
      artifact
      publicInput
      proof
    /\ validation.traceConstraintValidation.traceEvidenceMatchesConstraintCatalog
      artifact
      publicInput
      proof

def RuntimeTraceConstraintArtifactBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeTraceConstraintArtifactBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.traceArtifactBindingAccepted artifact publicInput proof

theorem runtime_trace_constraint_artifact_binding_checked_acceptance_evidence
    {system : VerifierModel}
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTraceConstraintArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintPreflightBindingEvidence
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have payloadValid :=
    validation.traceArtifactBindingAcceptedImpliesPayloadValid
      artifact
      publicInput
      proof
      accepted
  have witnessSegmentsMatch :=
    validation.traceArtifactBindingAcceptedImpliesWitnessCommitmentSegmentsMatch
      artifact
      publicInput
      proof
      accepted
  have constraintCatalogMatches :=
    validation.traceArtifactBindingAcceptedImpliesConstraintCatalogMatches
      artifact
      publicInput
      proof
      accepted
  have witnessCommitmentBinding :=
    validation.traceArtifactChecksImplyWitnessCommitmentBinding
      artifact
      publicInput
      proof
      payloadValid
      witnessSegmentsMatch
  have constraintCatalogBinding :=
    validation.traceArtifactChecksImplyConstraintCatalogBinding
      artifact
      publicInput
      proof
      payloadValid
      constraintCatalogMatches
  exact
    And.intro payloadValid
      (And.intro witnessSegmentsMatch
        (And.intro constraintCatalogMatches
          (And.intro witnessCommitmentBinding constraintCatalogBinding)))

theorem runtime_trace_constraint_preflight_binding_evidence_implies_payload_valid
    {system : VerifierModel}
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTraceConstraintPreflightBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        validation.traceConstraintSegmentPayloadValid
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof evidence
  exact evidence.left

theorem runtime_trace_constraint_preflight_binding_evidence_implies_witness_segments_match
    {system : VerifierModel}
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTraceConstraintPreflightBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        validation.witnessCommitmentSegmentsMatchTraceEvidence
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof evidence
  exact evidence.right.left

theorem runtime_trace_constraint_preflight_binding_evidence_implies_constraint_catalog_matches
    {system : VerifierModel}
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTraceConstraintPreflightBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        validation.constraintCatalogMatchesTraceEvidence
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof evidence
  exact evidence.right.right.left

theorem runtime_trace_constraint_preflight_binding_evidence_implies_artifact_binding_evidence
    {system : VerifierModel}
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTraceConstraintPreflightBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintArtifactBindingEvidence
          system
          validation.traceConstraintValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof evidence
  exact And.intro evidence.right.right.right.left evidence.right.right.right.right

theorem runtime_trace_constraint_artifact_binding_checked_acceptance_artifact_binding_evidence
    {system : VerifierModel}
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTraceConstraintArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintArtifactBindingEvidence
          system
          validation.traceConstraintValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have preflightEvidence :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_trace_constraint_preflight_binding_evidence_implies_artifact_binding_evidence
      validation
      artifact
      publicInput
      proof
      preflightEvidence

theorem runtime_trace_constraint_artifact_binding_checked_acceptance_trace_constraint
    {system : VerifierModel}
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTraceConstraintArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintCheckedAcceptance
          system
          validation.traceConstraintValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.traceArtifactBindingAcceptedImpliesTraceConstraintAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_trace_constraint_artifact_binding_checked_acceptance_semantic_evidence_complete
    {system : VerifierModel}
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTraceConstraintArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintSemanticEvidenceComplete
          system
          validation.traceConstraintValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have traceConstraintAccepted :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_trace_constraint
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_trace_constraint_checked_acceptance_semantic_evidence_complete
      validation.traceConstraintValidation
      artifact
      publicInput
      proof
      traceConstraintAccepted

theorem runtime_trace_constraint_artifact_binding_checked_acceptance_pcs_fri_backend_contract
    {system : VerifierModel}
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTraceConstraintArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintPreflightBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeTraceConstraintArtifactBindingEvidence
            system
            validation.traceConstraintValidation
            artifact
            publicInput
            proof
          /\ RuntimeTraceConstraintSemanticEvidenceComplete
            system
            validation.traceConstraintValidation
            artifact
            publicInput
            proof
          /\ RuntimeTraceConstraintBackendContract
            system
            validation.traceConstraintValidation
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted
  have artifactEvidence :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have traceConstraintAccepted :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_trace_constraint
      validation
      artifact
      publicInput
      proof
      accepted
  have traceContract :=
    runtime_trace_constraint_checked_acceptance_pcs_fri_backend_contract
      validation.traceConstraintValidation
      artifact
      publicInput
      proof
      traceConstraintAccepted
  rcases traceContract with
    ⟨pcsOpeningsValid,
      friQueriesValid,
      traceArtifactBindingEvidence,
      semanticEvidenceComplete,
      backendContract⟩
  exact
    And.intro artifactEvidence
      (And.intro pcsOpeningsValid
        (And.intro friQueriesValid
          (And.intro traceArtifactBindingEvidence
            (And.intro semanticEvidenceComplete backendContract))))

theorem runtime_trace_constraint_artifact_binding_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeTraceConstraintArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintPreflightBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeTraceConstraintEvidence
            system
            validation.traceConstraintValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have artifactEvidence :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have traceConstraintAccepted :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_trace_constraint
      validation
      artifact
      publicInput
      proof
      accepted
  have traceSound :=
    runtime_trace_constraint_checked_acceptance_sound
      assumptions
      validation.traceConstraintValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceConstraintAccepted
  exact And.intro artifactEvidence traceSound

theorem runtime_trace_constraint_artifact_binding_checked_acceptance_soundness_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeTraceConstraintArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintPreflightBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeTraceConstraintSoundnessObligations
            system
            validation.traceConstraintValidation
            artifact
            publicInput
            proof
            requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have artifactEvidence :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have traceConstraintAccepted :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_trace_constraint
      validation
      artifact
      publicInput
      proof
      accepted
  have traceObligations :=
    runtime_trace_constraint_checked_acceptance_obligations
      assumptions
      validation.traceConstraintValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceConstraintAccepted
  exact And.intro artifactEvidence traceObligations

theorem runtime_trace_constraint_artifact_binding_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTraceConstraintArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have traceConstraintAccepted :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_trace_constraint
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_trace_constraint_checked_acceptance_verifier_core_contract
      assumptions
      validation.traceConstraintValidation
      artifact
      publicInput
      proof
      False
      traceConstraintAccepted

theorem runtime_trace_constraint_artifact_binding_checked_acceptance_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeTraceConstraintArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintPreflightBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeTraceConstraintEvidence
            system
            validation.traceConstraintValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have checkedSound :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have coreContract :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro checkedSound.left
      (And.intro checkedSound.right.left
        (And.intro coreContract checkedSound.right.right))

theorem runtime_trace_constraint_artifact_binding_checked_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeTraceConstraintArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeTraceConstraintPreflightBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeTraceConstraintEvidence
            system
            validation.traceConstraintValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have checkedSound :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have coreContract :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        (And.intro checkedSound.left
          (And.intro checkedSound.right.left
            (And.intro coreContract checkedSound.right.right))))

set_option linter.style.longLine false in
theorem runtime_trace_constraint_artifact_binding_required_external_source_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTraceConstraintArtifactBindingValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimeTraceConstraintArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          RuntimeTraceConstraintPreflightBindingEvidence
              system
              validation
              artifact
              publicInput
              proof
            /\ RuntimeTraceConstraintEvidence
              system
              validation.traceConstraintValidation
              artifact
              publicInput
              proof
              requiresExternalSource
            /\ ExternalSourceOpeningEvidence
              system
              validation.traceConstraintValidation.openingValidation.runtimeSoundnessValidation.sourceValidation
              publicInput
              proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have artifactEvidence :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have traceConstraintAccepted :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_trace_constraint
      validation
      artifact
      publicInput
      proof
      accepted
  have requiredContract :=
    runtime_trace_constraint_required_external_source_evidence_core_and_sound
      assumptions
      validation.traceConstraintValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceConstraintAccepted
      required
  exact
    And.intro artifactEvidence
      (And.intro requiredContract.left
        (And.intro requiredContract.right.left
          (And.intro requiredContract.right.right.left
            requiredContract.right.right.right)))

end Lzvm
