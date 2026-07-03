/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding.Contracts.Core

/-!
External-source contracts derived from runtime proof pipeline binding.
-/

namespace Lzvm

universe uDigest

theorem runtime_pipeline_required_external_source_concrete_opening_core_contract
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
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
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have externalCore :=
    runtime_pipeline_binding_required_external_source_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have concreteCore :=
    runtime_pipeline_checked_acceptance_audited_concrete_opening_core_contract
      assumptions
      validation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  rcases externalCore with
    ⟨traceExternalEvidence,
      openingExternalEvidence,
      verifierCore⟩
  rcases concreteCore with
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      seedBinds,
      seededFriOpeningChecked,
      _verifierCore,
      executionObligations,
      soundWitness⟩
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro traceExternalEvidence
            (And.intro openingExternalEvidence
              (And.intro transcriptBound
                (And.intro publicInputBound
                  (And.intro pcsOpenings
                    (And.intro friQueries
                      (And.intro seedBinds
                        (And.intro seededFriOpeningChecked
                          (And.intro verifierCore
                            (And.intro executionObligations soundWitness))))))))))))

theorem runtime_pipeline_required_external_source_concrete_manifest_contract
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ RuntimeQueryPlanMaterialManifestContract
              system
              validation.queryPlanBindingValidation
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanSegmentCanonical
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanMaterialManifestMatchesSchedule
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
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have coreContract :=
    runtime_pipeline_required_external_source_concrete_opening_core_contract
      assumptions
      validation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have materialManifestComponents :=
    runtime_pipeline_binding_checked_acceptance_query_plan_material_manifest_components
      validation
      artifact
      publicInput
      proof
      accepted
  rcases materialManifestComponents with
    ⟨materialManifest, segmentCanonical, materialManifestMatches⟩
  rcases coreContract with
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      seedBinds,
      seededFriOpeningChecked,
      verifierCore,
      executionObligations,
      soundWitness⟩
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro materialManifest
            (And.intro segmentCanonical
              (And.intro materialManifestMatches
                (And.intro traceExternalEvidence
                  (And.intro openingExternalEvidence
                    (And.intro transcriptBound
                      (And.intro publicInputBound
                        (And.intro pcsOpenings
                          (And.intro friQueries
                            (And.intro seedBinds
                              (And.intro seededFriOpeningChecked
                                (And.intro verifierCore
                                  (And.intro executionObligations soundWitness)))))))))))))))

set_option linter.style.longLine false in
theorem runtime_pipeline_required_external_source_hash_concrete_opening_core_contract
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (hashAssumptions : HashCollisionResistanceAssumption)
    (validation : RuntimePipelineBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
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
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have externalCore :=
    runtime_pipeline_binding_required_external_source_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have concreteCore :=
    runtime_pipeline_binding_checked_acceptance_hash_concrete_opening_core_contract
      assumptions
      hashAssumptions
      validation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  rcases externalCore with
    ⟨traceExternalEvidence,
      openingExternalEvidence,
      verifierCore⟩
  rcases concreteCore with
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      seedBinds,
      seededFriOpeningChecked,
      _concreteVerifierCore,
      executionObligations,
      soundWitness⟩
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro traceExternalEvidence
            (And.intro openingExternalEvidence
              (And.intro transcriptBound
                (And.intro publicInputBound
                  (And.intro pcsOpenings
                    (And.intro friQueries
                      (And.intro seedBinds
                        (And.intro seededFriOpeningChecked
                          (And.intro verifierCore
                            (And.intro executionObligations soundWitness))))))))))))

theorem runtime_pipeline_required_external_source_hash_manifest_contract
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (hashAssumptions : HashCollisionResistanceAssumption)
    (validation : RuntimePipelineBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ RuntimeQueryPlanMaterialManifestContract
              system
              validation.queryPlanBindingValidation
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanSegmentCanonical
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanMaterialManifestMatchesSchedule
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
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have coreContract :=
    runtime_pipeline_required_external_source_hash_concrete_opening_core_contract
      assumptions
      hashAssumptions
      validation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have materialManifestComponents :=
    runtime_pipeline_binding_checked_acceptance_query_plan_material_manifest_components
      validation
      artifact
      publicInput
      proof
      accepted
  rcases materialManifestComponents with
    ⟨materialManifest, segmentCanonical, materialManifestMatches⟩
  rcases coreContract with
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      seedBinds,
      seededFriOpeningChecked,
      verifierCore,
      executionObligations,
      soundWitness⟩
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro materialManifest
            (And.intro segmentCanonical
              (And.intro materialManifestMatches
                (And.intro traceExternalEvidence
                  (And.intro openingExternalEvidence
                    (And.intro transcriptBound
                      (And.intro publicInputBound
                        (And.intro pcsOpenings
                          (And.intro friQueries
                            (And.intro seedBinds
                              (And.intro seededFriOpeningChecked
                                (And.intro verifierCore
                                  (And.intro executionObligations soundWitness)))))))))))))))


end Lzvm
