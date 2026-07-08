/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.RuntimeSoundness.Core
import Lzvm.ProofSegmentIds

/-!
Runtime soundness projections for proof segment identifiers.
-/

namespace Lzvm

theorem runtime_soundness_checked_acceptance_segments_present
    {system : VerifierModel}
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        validation.transcriptValidation.artifactBindingValidation.proofSegmentsPresent
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource checked
  exact
    runtime_transcript_binding_checked_acceptance_segments_present
      validation.transcriptValidation
      artifact
      publicInput
      proof
      checked.left

theorem runtime_soundness_checked_acceptance_container_canonical
    {system : VerifierModel}
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        validation.transcriptValidation.artifactBindingValidation.proofContainerCanonical
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource checked
  exact
    runtime_transcript_binding_checked_acceptance_container_canonical
      validation.transcriptValidation
      artifact
      publicInput
      proof
      checked.left

theorem runtime_soundness_checked_acceptance_metadata_canonical
    {system : VerifierModel}
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        validation.transcriptValidation.artifactBindingValidation.proofMetadataCanonical
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource checked
  exact
    runtime_transcript_binding_checked_acceptance_metadata_canonical
      validation.transcriptValidation
      artifact
      publicInput
      proof
      checked.left

theorem runtime_soundness_checked_acceptance_segment_ids_unique
    {system : VerifierModel}
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        validation.transcriptValidation.artifactBindingValidation.proofSegmentIdsUnique
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource checked
  exact
    runtime_transcript_binding_checked_acceptance_segment_ids_unique
      validation.transcriptValidation
      artifact
      publicInput
      proof
      checked.left

theorem runtime_soundness_checked_acceptance_unit_values_trace_identity_coverage
    {system : VerifierModel}
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        let artifactValidation := validation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofUnitValuesTraceIdentityCoverage artifact publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  exact
    runtime_transcript_binding_checked_acceptance_unit_values_trace_identity_coverage
      validation.transcriptValidation
      artifact
      publicInput
      proof
      checked.left

theorem runtime_soundness_checked_acceptance_segment_payloads_nonempty
    {system : VerifierModel}
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        validation.transcriptValidation.artifactBindingValidation.proofSegmentPayloadsNonempty
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource checked
  exact
    runtime_transcript_binding_checked_acceptance_segment_payloads_nonempty
      validation.transcriptValidation
      artifact
      publicInput
      proof
      checked.left

theorem runtime_soundness_checked_acceptance_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        validation.transcriptValidation.artifactBindingValidation.proofSegmentIdsAllowed
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource checked
  exact
    runtime_transcript_binding_checked_acceptance_segment_ids_allowed
      validation.transcriptValidation
      artifact
      publicInput
      proof
      checked.left

theorem runtime_soundness_checked_acceptance_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeSoundnessValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked
  exact
    runtime_transcript_binding_checked_acceptance_concrete_segment_ids_allowed
      validation.transcriptValidation
      binding
      artifact
      publicInput
      proof
      checked.left

theorem runtime_soundness_checked_acceptance_proof_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeSoundnessValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        ProofSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked
  exact
    runtime_soundness_checked_acceptance_concrete_segment_ids_allowed
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked

theorem runtime_soundness_checked_acceptance_concrete_core_sound_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        (RuntimeSoundnessEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof)
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked
  exact
    And.intro
      (runtime_soundness_checked_acceptance_evidence_core_and_sound
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        checked)
      (runtime_soundness_checked_acceptance_concrete_segment_ids_allowed
        validation
        binding
        artifact
        publicInput
        proof
        requiresExternalSource
        checked)

theorem runtime_soundness_checked_acceptance_audited_finalized_concrete_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        (RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeProofArtifactFinalized
            system
            validation.transcriptValidation.artifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof)
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked
  exact
    And.intro
      (runtime_soundness_checked_acceptance_audited_finalized_core_sound_witness_contract
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        checked)
      (runtime_soundness_checked_acceptance_concrete_segment_ids_allowed
        validation
        binding
        artifact
        publicInput
        proof
        requiresExternalSource
        checked)

end Lzvm
