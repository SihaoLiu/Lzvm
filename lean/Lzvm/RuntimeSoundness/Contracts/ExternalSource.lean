/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.RuntimeSoundness.Contracts.Base

/-!
Runtime soundness contracts for required external source obligations.
-/

namespace Lzvm

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

theorem runtime_soundness_required_external_source_artifact_audited_concrete_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          (let artifactValidation :=
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
              proof)
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  exact
    And.intro
      (runtime_soundness_required_external_source_artifact_audited_segment_ids_contract
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        checked
        required)
      (runtime_soundness_checked_acceptance_concrete_segment_ids_allowed
        validation
        binding
        artifact
        publicInput
        proof
        requiresExternalSource
        checked)

theorem runtime_soundness_required_external_source_artifact_audited_finalized_segment_ids_contract
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
            /\ RuntimeProofArtifactFinalized
              system
              artifactValidation
              artifact
              publicInput
              proof
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
    runtime_soundness_required_external_source_artifact_audited_segment_ids_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have finalizedContract :=
    runtime_soundness_required_external_source_audited_finalized_core_sound_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  rcases segmentContract with
    ⟨artifactEvidence,
      auditedCrypto,
      auditedSemantic,
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
  rcases finalizedContract with
    ⟨_finalizedCrypto,
      _finalizedSemantic,
      artifactFinalized,
      _finalizedExternalSourceEvidence,
      _finalizedCore,
      _finalizedExecutionObligations,
      _finalizedSoundWitness⟩
  exact
    ⟨artifactEvidence,
      auditedCrypto,
      auditedSemantic,
      artifactFinalized,
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

theorem
runtime_soundness_required_external_source_artifact_audited_finalized_concrete_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          (let artifactValidation :=
            validation.transcriptValidation.artifactBindingValidation
           RuntimeArtifactEvidence
            system
            artifactValidation.runtimeValidation
            artifact
            publicInput
            proof
            /\ RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ RuntimeProofArtifactFinalized
              system
              artifactValidation
              artifact
              publicInput
              proof
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
              proof)
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  exact
    And.intro
      (runtime_soundness_required_external_source_artifact_audited_finalized_segment_ids_contract
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        checked
        required)
      (runtime_soundness_checked_acceptance_concrete_segment_ids_allowed
        validation
        binding
        artifact
        publicInput
        proof
        requiresExternalSource
        checked)

set_option linter.style.longLine false in
theorem
runtime_soundness_checked_acceptance_audited_finalized_concrete_segment_ids_core_components_contract
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
        let artifactValidation :=
          validation.transcriptValidation.artifactBindingValidation
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeProofArtifactFinalized
            system
            artifactValidation
            artifact
            publicInput
            proof
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
          /\ SoundWitness system publicInput proof
          /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
          /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked
  obtain ⟨finalizedSegmentContract, concreteSegmentIdsAllowed⟩ :=
    runtime_soundness_checked_acceptance_artifact_audited_finalized_concrete_segment_ids_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  rcases finalizedSegmentContract with
    ⟨_artifactEvidence,
      auditedCrypto,
      auditedSemantic,
      artifactFinalized,
      proofSystemSound,
      verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      executionObligations,
      soundWitness,
      _containerCanonical,
      _segmentsPresent,
      _metadataCanonical,
      _segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      _unitValuesTraceIdentityCoverage⟩
  exact
    ⟨auditedCrypto,
      auditedSemantic,
      artifactFinalized,
      proofSystemSound,
      verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      executionObligations,
      soundWitness,
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩

set_option linter.style.longLine false in
theorem
runtime_soundness_required_external_source_audited_finalized_concrete_segment_ids_core_components_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.transcriptValidation.artifactBindingValidation) :
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
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ RuntimeProofArtifactFinalized
              system
              artifactValidation
              artifact
              publicInput
              proof
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
            /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
            /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  obtain ⟨finalizedSegmentContract, concreteSegmentIdsAllowed⟩ :=
    runtime_soundness_required_external_source_artifact_audited_finalized_concrete_segment_ids_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  rcases finalizedSegmentContract with
    ⟨_artifactEvidence,
      auditedCrypto,
      auditedSemantic,
      artifactFinalized,
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
      _containerCanonical,
      _segmentsPresent,
      _metadataCanonical,
      _segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      _unitValuesTraceIdentityCoverage⟩
  exact
    ⟨auditedCrypto,
      auditedSemantic,
      artifactFinalized,
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
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩

set_option linter.style.longLine false in
theorem
runtime_soundness_checked_acceptance_finalized_concrete_core_requirements_contract
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
        let artifactValidation :=
          validation.transcriptValidation.artifactBindingValidation
        RuntimeProofArtifactFinalized
          system
          artifactValidation
          artifact
          publicInput
          proof
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
          /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked
  obtain
    ⟨_auditedCrypto,
      _auditedSemantic,
      artifactFinalized,
      _proofSystemSound,
      _verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      _executionObligations,
      _soundWitness,
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩ :=
    runtime_soundness_checked_acceptance_audited_finalized_concrete_segment_ids_core_components_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact
    ⟨artifactFinalized,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩

set_option linter.style.longLine false in
theorem
runtime_soundness_checked_acceptance_artifact_finalized_concrete_core_requirements_contract
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
        let artifactValidation :=
          validation.transcriptValidation.artifactBindingValidation
        RuntimeArtifactEvidence
          system
          artifactValidation.runtimeValidation
          artifact
          publicInput
          proof
          /\ RuntimeProofArtifactFinalized
            system
            artifactValidation
            artifact
            publicInput
            proof
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
          /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked
  obtain ⟨finalizedSegmentContract, concreteSegmentIdsAllowed⟩ :=
    runtime_soundness_checked_acceptance_artifact_audited_finalized_concrete_segment_ids_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  rcases finalizedSegmentContract with
    ⟨artifactEvidence,
      _auditedCrypto,
      _auditedSemantic,
      artifactFinalized,
      _proofSystemSound,
      _verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      _executionObligations,
      _soundWitness,
      _containerCanonical,
      _segmentsPresent,
      _metadataCanonical,
      _segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      _unitValuesTraceIdentityCoverage⟩
  exact
    ⟨artifactEvidence,
      artifactFinalized,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩

set_option linter.style.longLine false in
theorem
runtime_soundness_checked_acceptance_artifact_finalized_concrete_segment_requirements_contract
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
        let artifactValidation :=
          validation.transcriptValidation.artifactBindingValidation
        RuntimeArtifactEvidence
          system
          artifactValidation.runtimeValidation
          artifact
          publicInput
          proof
          /\ RuntimeProofArtifactFinalized
            system
            artifactValidation
            artifact
            publicInput
            proof
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ artifactValidation.proofContainerCanonical artifact publicInput proof
          /\ artifactValidation.proofSegmentsPresent artifact publicInput proof
          /\ artifactValidation.proofMetadataCanonical artifact publicInput proof
          /\ artifactValidation.proofSegmentPayloadsNonempty artifact publicInput proof
          /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
          /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
          /\ artifactValidation.proofUnitValuesTraceIdentityCoverage
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked
  obtain ⟨finalizedSegmentContract, concreteSegmentIdsAllowed⟩ :=
    runtime_soundness_checked_acceptance_artifact_audited_finalized_concrete_segment_ids_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  rcases finalizedSegmentContract with
    ⟨artifactEvidence,
      _auditedCrypto,
      _auditedSemantic,
      artifactFinalized,
      _proofSystemSound,
      _verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      _executionObligations,
      _soundWitness,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage⟩
  exact
    ⟨artifactEvidence,
      artifactFinalized,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage,
      concreteSegmentIdsAllowed⟩

set_option linter.style.longLine false in
theorem
runtime_soundness_checked_acceptance_finalized_concrete_segment_requirements_contract
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
        let artifactValidation :=
          validation.transcriptValidation.artifactBindingValidation
        RuntimeProofArtifactFinalized
          system
          artifactValidation
          artifact
          publicInput
          proof
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ artifactValidation.proofContainerCanonical artifact publicInput proof
          /\ artifactValidation.proofSegmentsPresent artifact publicInput proof
          /\ artifactValidation.proofMetadataCanonical artifact publicInput proof
          /\ artifactValidation.proofSegmentPayloadsNonempty artifact publicInput proof
          /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
          /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
          /\ artifactValidation.proofUnitValuesTraceIdentityCoverage
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked
  rcases
    runtime_soundness_checked_acceptance_artifact_finalized_concrete_segment_requirements_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked with
    ⟨_artifactEvidence,
      artifactFinalized,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage,
      concreteSegmentIdsAllowed⟩
  exact
    ⟨artifactFinalized,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage,
      concreteSegmentIdsAllowed⟩

set_option linter.style.longLine false in
theorem
runtime_soundness_required_external_source_finalized_concrete_core_source_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.transcriptValidation.artifactBindingValidation) :
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
          RuntimeProofArtifactFinalized
            system
            artifactValidation
            artifact
            publicInput
            proof
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
            /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
            /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  obtain
    ⟨_auditedCrypto,
      _auditedSemantic,
      artifactFinalized,
      _proofSystemSound,
      _verifierAccepts,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      _executionObligations,
      _soundWitness,
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩ :=
    runtime_soundness_required_external_source_audited_finalized_concrete_segment_ids_core_components_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact
    ⟨artifactFinalized,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩

set_option linter.style.longLine false in
theorem
runtime_soundness_required_external_source_artifact_finalized_concrete_core_source_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.transcriptValidation.artifactBindingValidation) :
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
            /\ RuntimeProofArtifactFinalized
              system
              artifactValidation
              artifact
              publicInput
              proof
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
            /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
            /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  obtain ⟨finalizedSegmentContract, concreteSegmentIdsAllowed⟩ :=
    runtime_soundness_required_external_source_artifact_audited_finalized_concrete_segment_ids_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  rcases finalizedSegmentContract with
    ⟨artifactEvidence,
      _auditedCrypto,
      _auditedSemantic,
      artifactFinalized,
      _proofSystemSound,
      _verifierAccepts,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      _executionObligations,
      _soundWitness,
      _containerCanonical,
      _segmentsPresent,
      _metadataCanonical,
      _segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      _unitValuesTraceIdentityCoverage⟩
  exact
    ⟨artifactEvidence,
      artifactFinalized,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩

set_option linter.style.longLine false in
theorem
runtime_soundness_required_external_source_artifact_finalized_concrete_segment_requirements_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.transcriptValidation.artifactBindingValidation) :
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
            /\ RuntimeProofArtifactFinalized
              system
              artifactValidation
              artifact
              publicInput
              proof
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
            /\ artifactValidation.proofContainerCanonical artifact publicInput proof
            /\ artifactValidation.proofSegmentsPresent artifact publicInput proof
            /\ artifactValidation.proofMetadataCanonical artifact publicInput proof
            /\ artifactValidation.proofSegmentPayloadsNonempty artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
            /\ artifactValidation.proofUnitValuesTraceIdentityCoverage
              artifact
              publicInput
              proof
            /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  obtain ⟨finalizedSegmentContract, concreteSegmentIdsAllowed⟩ :=
    runtime_soundness_required_external_source_artifact_audited_finalized_concrete_segment_ids_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  rcases finalizedSegmentContract with
    ⟨artifactEvidence,
      _auditedCrypto,
      _auditedSemantic,
      artifactFinalized,
      _proofSystemSound,
      _verifierAccepts,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      _executionObligations,
      _soundWitness,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage⟩
  exact
    ⟨artifactEvidence,
      artifactFinalized,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage,
      concreteSegmentIdsAllowed⟩

set_option linter.style.longLine false in
theorem
runtime_soundness_required_external_source_finalized_concrete_segment_requirements_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.transcriptValidation.artifactBindingValidation) :
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
          RuntimeProofArtifactFinalized
            system
            artifactValidation
            artifact
            publicInput
            proof
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
            /\ artifactValidation.proofContainerCanonical artifact publicInput proof
            /\ artifactValidation.proofSegmentsPresent artifact publicInput proof
            /\ artifactValidation.proofMetadataCanonical artifact publicInput proof
            /\ artifactValidation.proofSegmentPayloadsNonempty artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
            /\ artifactValidation.proofUnitValuesTraceIdentityCoverage
              artifact
              publicInput
              proof
            /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  rcases
    runtime_soundness_required_external_source_artifact_finalized_concrete_segment_requirements_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required with
    ⟨_artifactEvidence,
      artifactFinalized,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage,
      concreteSegmentIdsAllowed⟩
  exact
    ⟨artifactFinalized,
      externalSourceEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage,
      concreteSegmentIdsAllowed⟩


end Lzvm
