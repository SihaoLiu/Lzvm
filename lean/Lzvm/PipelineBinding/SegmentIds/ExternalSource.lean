/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding.SegmentIds.Base

/-!
Pipeline binding segment-id contracts for required external source obligations.
-/

namespace Lzvm

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

theorem runtime_pipeline_binding_required_external_source_audited_soundness_segment_ids_contract
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
  have auditedContract :=
    runtime_pipeline_binding_required_external_source_audited_segment_ids_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  exact
    And.intro auditedAssumptions.left
      (And.intro auditedAssumptions.right auditedContract.right)

theorem
runtime_pipeline_required_external_source_audited_segment_ids_core_components_contract
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
  intro artifact publicInput proof requiresExternalSource accepted required
  have segmentContract :=
    runtime_pipeline_binding_required_external_source_audited_soundness_segment_ids_contract
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
      auditedSemantic,
      proofSystemSound,
      verifierAccepts,
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
  have transcriptBound := coreContract.left
  have publicInputBound := coreContract.right.left
  exact
    ⟨auditedCrypto,
      auditedSemantic,
      proofSystemSound,
      verifierAccepts,
      traceSourceEvidence,
      openingSourceEvidence,
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

theorem
runtime_pipeline_binding_required_external_source_audited_soundness_concrete_segment_ids_contract
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
      (runtime_pipeline_binding_required_external_source_audited_soundness_segment_ids_contract
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

set_option linter.style.longLine false in
theorem
runtime_pipeline_required_external_source_audited_finalized_concrete_segment_ids_core_components_contract
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
          let queryPlanValidation := validation.queryPlanBindingValidation
          let artifactValidation :=
            queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
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
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof
            /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
            /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have finalizedConcreteContract :=
    runtime_pipeline_binding_required_external_source_audited_finalized_concrete_segment_ids_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  rcases finalizedConcreteContract with
    ⟨finalizedSegmentContract, concreteSegmentIdsAllowed⟩
  rcases finalizedSegmentContract with
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
      _artifactAgreement,
      _containerCanonical,
      _segmentsPresent,
      _metadataCanonical,
      _segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      _unitValuesTraceIdentityCoverage⟩
  have transcriptBound := coreContract.left
  have publicInputBound := coreContract.right.left
  exact
    ⟨auditedCrypto,
      auditedSemantic,
      artifactFinalized,
      proofSystemSound,
      verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      seedBinds,
      seededFriOpeningChecked,
      coreContract,
      executionObligations,
      soundWitness,
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩

set_option linter.style.longLine false in
theorem
runtime_pipeline_required_external_source_finalized_concrete_opening_evidence_contract
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
          let queryPlanValidation := validation.queryPlanBindingValidation
          let artifactValidation :=
            queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
          RuntimeProofArtifactFinalized
            system
            artifactValidation
            artifact
            publicInput
            proof
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
            /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have coreComponents :=
    runtime_pipeline_required_external_source_audited_finalized_concrete_segment_ids_core_components_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  rcases coreComponents with
    ⟨_auditedCrypto,
      _auditedSemantic,
      artifactFinalized,
      _proofSystemSound,
      _verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      _transcriptBound,
      _publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      _seedBinds,
      _seededFriOpeningChecked,
      _coreContract,
      _executionObligations,
      _soundWitness,
      _segmentIdsAllowed,
      _segmentIdsUnique,
      concreteSegmentIdsAllowed⟩
  exact
    ⟨artifactFinalized,
      traceExternalEvidence,
      openingExternalEvidence,
      pcsOpeningsValid,
      friQueriesValid,
      concreteSegmentIdsAllowed⟩

set_option linter.style.longLine false in
theorem
runtime_pipeline_required_external_source_finalized_concrete_segment_seeded_requirements_contract
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
          let queryPlanValidation := validation.queryPlanBindingValidation
          let artifactValidation :=
            queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
          RuntimeProofArtifactFinalized
            system
            artifactValidation
            artifact
            publicInput
            proof
            /\ validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked
              artifact
              publicInput
              proof
            /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
            /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have coreComponents :=
    runtime_pipeline_required_external_source_audited_finalized_concrete_segment_ids_core_components_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  rcases coreComponents with
    ⟨_auditedCrypto,
      _auditedSemantic,
      artifactFinalized,
      _proofSystemSound,
      _verifierAccepts,
      _traceExternalEvidence,
      _openingExternalEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid,
      seedBinds,
      seededFriOpeningChecked,
      _coreContract,
      _executionObligations,
      _soundWitness,
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩
  exact
    ⟨artifactFinalized,
      seedBinds,
      seededFriOpeningChecked,
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩

set_option linter.style.longLine false in
theorem
runtime_pipeline_required_external_source_finalized_concrete_opening_seeded_requirements_contract
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
          let queryPlanValidation := validation.queryPlanBindingValidation
          let artifactValidation :=
            queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
          RuntimeProofArtifactFinalized
            system
            artifactValidation
            artifact
            publicInput
            proof
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
            /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
            /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have openingEvidence :=
    runtime_pipeline_required_external_source_finalized_concrete_opening_evidence_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have seededRequirements :=
    runtime_pipeline_required_external_source_finalized_concrete_segment_seeded_requirements_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  rcases openingEvidence with
    ⟨artifactFinalized,
      traceExternalEvidence,
      openingExternalEvidence,
      pcsOpeningsValid,
      friQueriesValid,
      concreteSegmentIdsAllowed⟩
  rcases seededRequirements with
    ⟨_artifactFinalizedAgain,
      seedBinds,
      seededFriOpeningChecked,
      segmentIdsAllowed,
      segmentIdsUnique,
      _concreteSegmentIdsAllowedAgain⟩
  exact
    ⟨artifactFinalized,
      traceExternalEvidence,
      openingExternalEvidence,
      pcsOpeningsValid,
      friQueriesValid,
      seedBinds,
      seededFriOpeningChecked,
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩

set_option linter.style.longLine false in
theorem
runtime_pipeline_required_external_source_finalized_concrete_core_opening_seeded_requirements_contract
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
          let queryPlanValidation := validation.queryPlanBindingValidation
          let artifactValidation :=
            queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
          RuntimeProofArtifactFinalized
            system
            artifactValidation
            artifact
            publicInput
            proof
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
            /\ artifactValidation.proofSegmentIdsAllowed artifact publicInput proof
            /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
            /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have coreComponents :=
    runtime_pipeline_required_external_source_audited_finalized_concrete_segment_ids_core_components_contract
      assumptions
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  rcases coreComponents with
    ⟨_auditedCrypto,
      _auditedSemantic,
      artifactFinalized,
      _proofSystemSound,
      _verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      seedBinds,
      seededFriOpeningChecked,
      coreContract,
      _executionObligations,
      _soundWitness,
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩
  exact
    ⟨artifactFinalized,
      traceExternalEvidence,
      openingExternalEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      seedBinds,
      seededFriOpeningChecked,
      coreContract,
      segmentIdsAllowed,
      segmentIdsUnique,
      concreteSegmentIdsAllowed⟩


end Lzvm
