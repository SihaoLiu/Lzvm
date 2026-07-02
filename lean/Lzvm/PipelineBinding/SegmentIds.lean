/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding.Audited
import Lzvm.PipelineBinding.Contracts

/-!
Compact proof segment identity contracts derived from runtime proof pipeline binding.
-/

namespace Lzvm

theorem runtime_pipeline_binding_checked_acceptance_audited_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let queryPlanValidation := validation.queryPlanBindingValidation
        let artifactValidation :=
          queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
        let ethArtifactValidation :=
          validation.ethBindingValidation.proofArtifactBindingValidation
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests
            artifact
            publicInput
            proof
          /\ validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof
          /\ RuntimeProofArtifactBindingValidationAgreement
            ethArtifactValidation
            artifactValidation
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
  intro artifact publicInput proof _requiresExternalSource accepted
  have compactContract :=
    runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      _requiresExternalSource
      accepted
  have containerCanonical :=
    runtime_pipeline_binding_checked_acceptance_container_canonical
      validation
      artifact
      publicInput
      proof
      accepted
  have segmentsPresent :=
    runtime_pipeline_binding_checked_acceptance_segments_present
      validation
      artifact
      publicInput
      proof
      accepted
  have metadataCanonical :=
    runtime_pipeline_binding_checked_acceptance_metadata_canonical
      validation
      artifact
      publicInput
      proof
      accepted
  have segmentIdsUnique :=
    runtime_pipeline_binding_checked_acceptance_segment_ids_unique
      validation
      artifact
      publicInput
      proof
      accepted
  have unitValuesTraceIdentityCoverage :=
    runtime_pipeline_binding_checked_acceptance_unit_values_trace_identity_coverage
      validation
      artifact
      publicInput
      proof
      accepted
  have segmentPayloadsNonempty :=
    runtime_pipeline_binding_checked_acceptance_segment_payloads_nonempty
      validation
      artifact
      publicInput
      proof
      accepted
  have segmentIdsAllowed :=
    runtime_pipeline_binding_checked_acceptance_segment_ids_allowed
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAgreement :=
    runtime_pipeline_binding_checked_acceptance_artifact_binding_validation_agreement
      validation
      artifact
      publicInput
      proof
      accepted
  rcases compactContract with
    ⟨auditedAssumptions,
      proofSystemSound,
      accepts,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      seedBinds,
      seededFriOpeningChecked,
      coreContract,
      soundWitness⟩
  exact
    ⟨auditedAssumptions,
      proofSystemSound,
      accepts,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      seedBinds,
      seededFriOpeningChecked,
      coreContract,
      soundWitness,
      artifactAgreement,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage⟩

theorem runtime_pipeline_binding_checked_acceptance_concrete_core_sound_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      let queryPlanValidation := validation.queryPlanBindingValidation
      let challengeValidation := queryPlanValidation.challengeValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        challengeValidation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        (RuntimePipelineBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof)
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_pipeline_binding_checked_acceptance_evidence_core_and_sound
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (runtime_pipeline_binding_checked_acceptance_concrete_segment_ids_allowed
        validation
        binding
        artifact
        publicInput
        proof
        accepted)

theorem runtime_pipeline_binding_checked_acceptance_audited_concrete_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      let queryPlanValidation := validation.queryPlanBindingValidation
      let challengeValidation := queryPlanValidation.challengeValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        challengeValidation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        (let queryPlanValidation := validation.queryPlanBindingValidation
         let artifactValidation :=
          queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
         let ethArtifactValidation :=
          validation.ethBindingValidation.proofArtifactBindingValidation
         RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests
            artifact
            publicInput
            proof
          /\ validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof
          /\ RuntimeProofArtifactBindingValidationAgreement
            ethArtifactValidation
            artifactValidation
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
  intro artifact publicInput proof _requiresExternalSource accepted
  exact
    And.intro
      (runtime_pipeline_binding_checked_acceptance_audited_segment_ids_contract
        assumptions
        validation
        artifact
        publicInput
        proof
        _requiresExternalSource
        accepted)
      (runtime_pipeline_binding_checked_acceptance_concrete_segment_ids_allowed
        validation
        binding
        artifact
        publicInput
        proof
        accepted)

theorem runtime_pipeline_binding_required_external_source_audited_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          let queryPlanValidation := validation.queryPlanBindingValidation
          let artifactValidation :=
            queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
          let ethArtifactValidation :=
            validation.ethBindingValidation.proofArtifactBindingValidation
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              (runtime_pipeline_trace_source_validation validation)
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence
              system
              (runtime_pipeline_opening_source_validation validation)
              publicInput
              proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked
              artifact
              publicInput
              proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof
            /\ RuntimeProofArtifactBindingValidationAgreement
              ethArtifactValidation
              artifactValidation
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
  intro artifact publicInput proof requiresExternalSource accepted required
  have compactContract :=
    runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have containerCanonical :=
    runtime_pipeline_binding_checked_acceptance_container_canonical
      validation
      artifact
      publicInput
      proof
      accepted
  have segmentsPresent :=
    runtime_pipeline_binding_checked_acceptance_segments_present
      validation
      artifact
      publicInput
      proof
      accepted
  have metadataCanonical :=
    runtime_pipeline_binding_checked_acceptance_metadata_canonical
      validation
      artifact
      publicInput
      proof
      accepted
  have segmentIdsUnique :=
    runtime_pipeline_binding_checked_acceptance_segment_ids_unique
      validation
      artifact
      publicInput
      proof
      accepted
  have unitValuesTraceIdentityCoverage :=
    runtime_pipeline_binding_checked_acceptance_unit_values_trace_identity_coverage
      validation
      artifact
      publicInput
      proof
      accepted
  have segmentPayloadsNonempty :=
    runtime_pipeline_binding_checked_acceptance_segment_payloads_nonempty
      validation
      artifact
      publicInput
      proof
      accepted
  have segmentIdsAllowed :=
    runtime_pipeline_binding_checked_acceptance_segment_ids_allowed
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAgreement :=
    runtime_pipeline_binding_checked_acceptance_artifact_binding_validation_agreement
      validation
      artifact
      publicInput
      proof
      accepted
  rcases compactContract with
    ⟨auditedAssumptions,
      proofSystemSound,
      accepts,
      traceSourceEvidence,
      openingSourceEvidence,
      pcsOpeningsValid,
      friQueriesValid,
      seedBinds,
      seededFriOpeningChecked,
      coreContract,
      soundWitness⟩
  exact
    ⟨auditedAssumptions,
      proofSystemSound,
      accepts,
      traceSourceEvidence,
      openingSourceEvidence,
      pcsOpeningsValid,
      friQueriesValid,
      seedBinds,
      seededFriOpeningChecked,
      coreContract,
      soundWitness,
      artifactAgreement,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage⟩

theorem runtime_pipeline_binding_required_external_source_audited_concrete_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      let queryPlanValidation := validation.queryPlanBindingValidation
      let challengeValidation := queryPlanValidation.challengeValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        challengeValidation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          (let queryPlanValidation := validation.queryPlanBindingValidation
           let artifactValidation :=
            queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
           let ethArtifactValidation :=
            validation.ethBindingValidation.proofArtifactBindingValidation
           RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              (runtime_pipeline_trace_source_validation validation)
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence
              system
              (runtime_pipeline_opening_source_validation validation)
              publicInput
              proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked
              artifact
              publicInput
              proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof
            /\ RuntimeProofArtifactBindingValidationAgreement
              ethArtifactValidation
              artifactValidation
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
  intro artifact publicInput proof requiresExternalSource accepted required
  exact
    And.intro
      (runtime_pipeline_binding_required_external_source_audited_segment_ids_contract
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted
        required)
      (runtime_pipeline_binding_checked_acceptance_concrete_segment_ids_allowed
        validation
        binding
        artifact
        publicInput
        proof
        accepted)

theorem runtime_pipeline_binding_required_external_source_audited_finalized_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          let queryPlanValidation := validation.queryPlanBindingValidation
          let artifactValidation :=
            queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
          let ethArtifactValidation :=
            validation.ethBindingValidation.proofArtifactBindingValidation
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
              (runtime_pipeline_trace_source_validation validation)
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence
              system
              (runtime_pipeline_opening_source_validation validation)
              publicInput
              proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked
              artifact
              publicInput
              proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof
            /\ RuntimeProofArtifactBindingValidationAgreement
              ethArtifactValidation
              artifactValidation
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
  intro artifact publicInput proof requiresExternalSource accepted required
  have segmentContract :=
    runtime_pipeline_binding_required_external_source_audited_segment_ids_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have finalizedContract :=
    runtime_pipeline_required_external_source_audited_finalized_core_sound_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  rcases segmentContract with
    ⟨auditedCrypto,
      proofSystemSound,
      verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      pcsOpeningsValid,
      friQueriesValid,
      seedBinds,
      seededFriOpeningChecked,
      coreContract,
      soundWitness,
      artifactAgreement,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage⟩
  rcases finalizedContract with
    ⟨_finalizedCrypto,
      auditedSemantic,
      artifactFinalized,
      _finalizedTraceExternalEvidence,
      _finalizedOpeningExternalEvidence,
      _finalizedSeedBinds,
      _finalizedSeededFriOpeningChecked,
      _finalizedCore,
      executionObligations,
      _finalizedSoundWitness⟩
  exact
    ⟨auditedCrypto,
      auditedSemantic,
      artifactFinalized,
      proofSystemSound,
      verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      pcsOpeningsValid,
      friQueriesValid,
      seedBinds,
      seededFriOpeningChecked,
      coreContract,
      executionObligations,
      soundWitness,
      artifactAgreement,
      containerCanonical,
      segmentsPresent,
      metadataCanonical,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage⟩

theorem
runtime_pipeline_binding_required_external_source_audited_finalized_concrete_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      let queryPlanValidation := validation.queryPlanBindingValidation
      let challengeValidation := queryPlanValidation.challengeValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        challengeValidation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          (let queryPlanValidation := validation.queryPlanBindingValidation
           let artifactValidation :=
            queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
           let ethArtifactValidation :=
            validation.ethBindingValidation.proofArtifactBindingValidation
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
              (runtime_pipeline_trace_source_validation validation)
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence
              system
              (runtime_pipeline_opening_source_validation validation)
              publicInput
              proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked
              artifact
              publicInput
              proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof
            /\ RuntimeProofArtifactBindingValidationAgreement
              ethArtifactValidation
              artifactValidation
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
  intro artifact publicInput proof requiresExternalSource accepted required
  exact
    And.intro
      (runtime_pipeline_binding_required_external_source_audited_finalized_segment_ids_contract
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted
        required)
      (runtime_pipeline_binding_checked_acceptance_concrete_segment_ids_allowed
        validation
        binding
        artifact
        publicInput
        proof
        accepted)

end Lzvm
