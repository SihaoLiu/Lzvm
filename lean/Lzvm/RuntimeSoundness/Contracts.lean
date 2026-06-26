/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.RuntimeSoundness.SegmentIds

/-!
Compact runtime soundness contracts.
-/

namespace Lzvm

theorem runtime_soundness_checked_acceptance_artifact_segment_ids_contract
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
        RuntimeArtifactEvidence
          system
          artifactValidation.runtimeValidation
          artifact
          publicInput
          proof
          /\ artifactValidation.proofContainerCanonical artifact publicInput proof
          /\ artifactValidation.proofSegmentsPresent artifact publicInput proof
          /\ artifactValidation.proofMetadataCanonical artifact publicInput proof
          /\ artifactValidation.proofSegmentPayloadsNonempty artifact publicInput proof
          /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
          /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
          /\ artifactValidation.proofUnitValuesTraceIdentityCoverage
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have artifactEvidence :=
    runtime_soundness_checked_acceptance_runtime_artifact_evidence
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have containerCanonical :=
    runtime_soundness_checked_acceptance_container_canonical
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have segmentsPresent :=
    runtime_soundness_checked_acceptance_segments_present
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have metadataCanonical :=
    runtime_soundness_checked_acceptance_metadata_canonical
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have segmentIdsUnique :=
    runtime_soundness_checked_acceptance_segment_ids_unique
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have unitValuesTraceIdentityCoverage :=
    runtime_soundness_checked_acceptance_unit_values_trace_identity_coverage
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have segmentPayloadsNonempty :=
    runtime_soundness_checked_acceptance_segment_payloads_nonempty
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have segmentIdsAllowed :=
    runtime_soundness_checked_acceptance_segment_ids_allowed
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact
    And.intro
      artifactEvidence
      (And.intro
        containerCanonical
        (And.intro
          segmentsPresent
          (And.intro
            metadataCanonical
            (And.intro segmentPayloadsNonempty
              (And.intro segmentIdsAllowed
                (And.intro segmentIdsUnique unitValuesTraceIdentityCoverage))))))

theorem runtime_soundness_checked_acceptance_concrete_artifact_segment_ids_contract
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
        (let artifactValidation := validation.transcriptValidation.artifactBindingValidation
         RuntimeArtifactEvidence
          system
          artifactValidation.runtimeValidation
          artifact
          publicInput
          proof
          /\ artifactValidation.proofContainerCanonical artifact publicInput proof
          /\ artifactValidation.proofSegmentsPresent artifact publicInput proof
          /\ artifactValidation.proofMetadataCanonical artifact publicInput proof
          /\ artifactValidation.proofSegmentPayloadsNonempty artifact publicInput proof
          /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
          /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
          /\ artifactValidation.proofUnitValuesTraceIdentityCoverage
            artifact
            publicInput
            proof)
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked
  exact
    And.intro
      (runtime_soundness_checked_acceptance_artifact_segment_ids_contract
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

theorem runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have proofSystemSound := abstract_verifier_sound assumptions
  have verifierAccepts :=
    runtime_soundness_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have transcriptBound :=
    runtime_soundness_checked_acceptance_transcript_bound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have publicInputBound :=
    runtime_soundness_checked_acceptance_public_input_bound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have pcsAndFri :=
    runtime_soundness_checked_acceptance_pcs_and_fri
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have coreContract :=
    runtime_soundness_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have soundWitness :=
    runtime_soundness_checked_acceptance_verifier_sound_witness
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro transcriptBound
            (And.intro publicInputBound
              (And.intro pcsAndFri.left
                (And.intro pcsAndFri.right
                  (And.intro coreContract soundWitness)))))))

theorem runtime_soundness_checked_acceptance_contracts_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have compactContract :=
    runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have executionObligations :=
    runtime_soundness_checked_acceptance_execution_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  rcases compactContract with
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      soundWitness⟩
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro transcriptBound
            (And.intro publicInputBound
              (And.intro pcsOpenings
                (And.intro friQueries
                  (And.intro verifierCore
                    (And.intro executionObligations soundWitness))))))))

theorem runtime_soundness_checked_acceptance_audited_soundness_contracts_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have compactContract :=
    runtime_soundness_checked_acceptance_contracts_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  rcases compactContract with
    ⟨_auditedCrypto,
      proofSystemSound,
      verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      executionObligations,
      soundWitness⟩
  exact
    And.intro auditedAssumptions.left
      (And.intro auditedAssumptions.right
        (And.intro proofSystemSound
          (And.intro verifierAccepts
            (And.intro transcriptBound
              (And.intro publicInputBound
                (And.intro pcsOpenings
                  (And.intro friQueries
                    (And.intro verifierCore
                      (And.intro executionObligations soundWitness)))))))))

theorem runtime_soundness_checked_acceptance_artifact_contracts_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeArtifactEvidence
          system
          validation.transcriptValidation.artifactBindingValidation.runtimeValidation
          artifact
          publicInput
          proof
          /\ RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have artifactEvidence :=
    runtime_soundness_checked_acceptance_runtime_artifact_evidence
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have compactContract :=
    runtime_soundness_checked_acceptance_contracts_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact And.intro artifactEvidence compactContract

theorem runtime_soundness_checked_acceptance_artifact_audited_soundness_contracts_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeArtifactEvidence
          system
          validation.transcriptValidation.artifactBindingValidation.runtimeValidation
          artifact
          publicInput
          proof
          /\ RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have artifactContract :=
    runtime_soundness_checked_acceptance_artifact_contracts_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  rcases artifactContract with
    ⟨artifactEvidence,
      _auditedCrypto,
      proofSystemSound,
      verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      executionObligations,
      soundWitness⟩
  exact
    And.intro artifactEvidence
      (And.intro auditedAssumptions.left
        (And.intro auditedAssumptions.right
          (And.intro proofSystemSound
            (And.intro verifierAccepts
              (And.intro transcriptBound
                (And.intro publicInputBound
                  (And.intro pcsOpenings
                    (And.intro friQueries
                      (And.intro verifierCore
                        (And.intro executionObligations soundWitness))))))))))

theorem runtime_soundness_required_external_source_contracts_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ system.transcriptBound publicInput proof
            /\ system.publicInputBound publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  exact
    runtime_soundness_required_external_source_audited_proof_system_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required

theorem runtime_soundness_required_external_source_contracts_audited_soundness_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ system.transcriptBound publicInput proof
            /\ system.publicInputBound publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have compactContract :=
    runtime_soundness_required_external_source_contracts_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  rcases compactContract with
    ⟨_auditedCrypto,
      proofSystemSound,
      verifierAccepts,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      executionObligations,
      soundWitness⟩
  exact
    And.intro auditedAssumptions.left
      (And.intro auditedAssumptions.right
        (And.intro proofSystemSound
          (And.intro verifierAccepts
            (And.intro externalSourceEvidence
              (And.intro transcriptBound
                (And.intro publicInputBound
                  (And.intro pcsOpenings
                    (And.intro friQueries
                      (And.intro verifierCore
                        (And.intro executionObligations soundWitness))))))))))

theorem runtime_soundness_required_external_source_artifact_contracts_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          RuntimeArtifactEvidence
            system
            validation.transcriptValidation.artifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
            /\ RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ system.transcriptBound publicInput proof
            /\ system.publicInputBound publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have artifactEvidence :=
    runtime_soundness_checked_acceptance_runtime_artifact_evidence
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have compactContract :=
    runtime_soundness_required_external_source_contracts_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact And.intro artifactEvidence compactContract

theorem runtime_soundness_required_external_source_artifact_audited_soundness_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          RuntimeArtifactEvidence
            system
            validation.transcriptValidation.artifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
            /\ RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ system.transcriptBound publicInput proof
            /\ system.publicInputBound publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have artifactContract :=
    runtime_soundness_required_external_source_artifact_contracts_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  rcases artifactContract with
    ⟨artifactEvidence,
      _auditedCrypto,
      proofSystemSound,
      verifierAccepts,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      executionObligations,
      soundWitness⟩
  exact
    And.intro artifactEvidence
      (And.intro auditedAssumptions.left
        (And.intro auditedAssumptions.right
          (And.intro proofSystemSound
            (And.intro verifierAccepts
              (And.intro externalSourceEvidence
                (And.intro transcriptBound
                  (And.intro publicInputBound
                    (And.intro pcsOpenings
                      (And.intro friQueries
                        (And.intro verifierCore
                          (And.intro executionObligations soundWitness)))))))))))

theorem runtime_soundness_required_external_source_artifact_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          let artifactValidation :=
            validation.transcriptValidation.artifactBindingValidation
          RuntimeArtifactEvidence
            system
            artifactValidation.runtimeValidation
            artifact
            publicInput
            proof
            /\ RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ system.transcriptBound publicInput proof
            /\ system.publicInputBound publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof
            /\ artifactValidation.proofContainerCanonical artifact publicInput proof
            /\ artifactValidation.proofSegmentsPresent artifact publicInput proof
            /\ artifactValidation.proofMetadataCanonical artifact publicInput proof
            /\ artifactValidation.proofSegmentPayloadsNonempty artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
            /\ artifactValidation.proofUnitValuesTraceIdentityCoverage
              artifact
              publicInput
              proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have artifactCore :=
    runtime_soundness_required_external_source_artifact_contracts_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have segmentContract :=
    runtime_soundness_checked_acceptance_artifact_segment_ids_contract
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  rcases artifactCore with
    ⟨artifactEvidence,
      auditedCrypto,
      proofSystemSound,
      verifierAccepts,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      executionObligations,
      soundWitness⟩
  rcases segmentContract with
    ⟨_segmentArtifactEvidence,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage⟩
  exact
    ⟨artifactEvidence,
      auditedCrypto,
      proofSystemSound,
      verifierAccepts,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      executionObligations,
      soundWitness,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage⟩

theorem runtime_soundness_required_external_source_artifact_audited_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          let artifactValidation :=
            validation.transcriptValidation.artifactBindingValidation
          RuntimeArtifactEvidence
            system
            artifactValidation.runtimeValidation
            artifact
            publicInput
            proof
            /\ RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ system.transcriptBound publicInput proof
            /\ system.publicInputBound publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof
            /\ artifactValidation.proofContainerCanonical artifact publicInput proof
            /\ artifactValidation.proofSegmentsPresent artifact publicInput proof
            /\ artifactValidation.proofMetadataCanonical artifact publicInput proof
            /\ artifactValidation.proofSegmentPayloadsNonempty artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
            /\ artifactValidation.proofUnitValuesTraceIdentityCoverage
              artifact
              publicInput
              proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have segmentContract :=
    runtime_soundness_required_external_source_artifact_segment_ids_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  rcases segmentContract with
    ⟨artifactEvidence,
      _auditedCrypto,
      proofSystemSound,
      verifierAccepts,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      executionObligations,
      soundWitness,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage⟩
  exact
    ⟨artifactEvidence,
      auditedAssumptions.left,
      auditedAssumptions.right,
      proofSystemSound,
      verifierAccepts,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      executionObligations,
      soundWitness,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage⟩

end Lzvm
