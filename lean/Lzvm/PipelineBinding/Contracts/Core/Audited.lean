/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding.Contracts.Core.Base

/-!
Audited and aggregate proof-system contracts derived from runtime proof pipeline binding.
-/

namespace Lzvm

universe uDigest

theorem runtime_pipeline_binding_evidence_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RequiredCryptographicAssumptionStatements assumptions.crypto
        /\ system.transcriptBound publicInput proof
        /\ system.publicInputBound publicInput proof
        /\ system.pcsOpeningsValid publicInput proof
        /\ system.friQueriesValid publicInput proof
        /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro evidence
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have transcriptBound :=
    runtime_pipeline_binding_evidence_implies_transcript_bound evidence
  have publicInputBound :=
    runtime_pipeline_binding_evidence_implies_public_input_bound evidence
  have pcsAndFri :=
    runtime_pipeline_binding_evidence_implies_pcs_and_fri evidence
  have coreContract :=
    runtime_pipeline_binding_evidence_implies_core_obligations evidence
  exact
    And.intro auditedAssumptions
      (And.intro transcriptBound
        (And.intro publicInputBound
          (And.intro pcsAndFri.left
            (And.intro pcsAndFri.right coreContract))))

theorem runtime_pipeline_binding_evidence_audited_soundness_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RequiredCryptographicAssumptionStatements assumptions.crypto
        /\ RequiredSemanticAssumptionStatements assumptions.semantic
        /\ system.transcriptBound publicInput proof
        /\ system.publicInputBound publicInput proof
        /\ system.pcsOpeningsValid publicInput proof
        /\ system.friQueriesValid publicInput proof
        /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro evidence
  have auditedCore :=
    runtime_pipeline_binding_evidence_audited_core_contract
      assumptions
      evidence
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  rcases auditedCore with
    ⟨_cryptoEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      coreContract⟩
  exact
    ⟨auditedAssumptions.left,
      auditedAssumptions.right,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      coreContract⟩

theorem runtime_pipeline_binding_checked_acceptance_audited_query_opening_core_sound_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ RuntimeQueryPlanBindingEvidence
            system
            validation.queryPlanBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeChallengeSegmentBindingEvidence
            system
            validation.queryPlanBindingValidation.challengeValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingEvidence
            system
            validation.queryPlanBindingValidation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.queryPlanBindingValidation.openingValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have proofSystemSound := abstract_verifier_sound assumptions
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have queryOpeningCore :=
    runtime_pipeline_binding_checked_acceptance_query_opening_evidence_core_and_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      queryOpeningCore.left,
      queryOpeningCore.right.left,
      queryOpeningCore.right.right.left,
      queryOpeningCore.right.right.right.left,
      queryOpeningCore.right.right.right.right.left,
      queryOpeningCore.right.right.right.right.right⟩

theorem runtime_pipeline_binding_checked_acceptance_audited_manifest_query_opening_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ RuntimeQueryPlanBindingEvidence
            system
            validation.queryPlanBindingValidation
            artifact
            publicInput
            proof
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
          /\ RuntimeChallengeSegmentBindingEvidence
            system
            validation.queryPlanBindingValidation.challengeValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingEvidence
            system
            validation.queryPlanBindingValidation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.queryPlanBindingValidation.openingValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have compactContract :=
    runtime_pipeline_binding_checked_acceptance_audited_query_opening_core_sound_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  rcases compactContract with
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      queryPlanEvidence,
      challengeEvidence,
      openingSegmentEvidence,
      openingEvidence,
      verifierCore,
      soundWitness⟩
  have materialManifestComponents :=
    runtime_pipeline_binding_checked_acceptance_query_plan_material_manifest_components
      validation
      artifact
      publicInput
      proof
      accepted
  rcases materialManifestComponents with
    ⟨materialManifest, segmentCanonical, materialManifestMatches⟩
  exact
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      queryPlanEvidence,
      materialManifest,
      segmentCanonical,
      materialManifestMatches,
      challengeEvidence,
      openingSegmentEvidence,
      openingEvidence,
      verifierCore,
      soundWitness⟩

theorem runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract
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
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof _requiresExternalSource accepted
  have compactContract :=
    runtime_pipeline_binding_checked_acceptance_audited_accepts_sound_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      _requiresExternalSource
      accepted
  have transcriptBound :=
    runtime_pipeline_binding_checked_acceptance_transcript_bound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have publicInputBound :=
    runtime_pipeline_binding_checked_acceptance_public_input_bound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have pcsAndFri :=
    runtime_pipeline_binding_checked_acceptance_pcs_and_fri
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have coreContract :=
    runtime_pipeline_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have seedBinds :=
    runtime_pipeline_binding_checked_acceptance_seed_binds_witness_tree_digests
      validation
      artifact
      publicInput
      proof
      accepted
  have seededFriOpeningChecked :=
    runtime_pipeline_binding_checked_acceptance_seeded_fri_opening_requirements_checked
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro compactContract.left
      (And.intro compactContract.right.left
        (And.intro compactContract.right.right.left
          (And.intro transcriptBound
            (And.intro publicInputBound
              (And.intro pcsAndFri.left
                (And.intro pcsAndFri.right
                  (And.intro seedBinds
                    (And.intro seededFriOpeningChecked
                      (And.intro coreContract.right
                        compactContract.right.right.right)))))))))
theorem runtime_pipeline_binding_checked_acceptance_audited_concrete_opening_contract
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
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
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
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof _requiresExternalSource accepted
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have proofSystemSound := abstract_verifier_sound assumptions
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have pipelineSound :=
    runtime_pipeline_binding_checked_acceptance_sound_from_concrete_nary_merkle
      assumptions
      validation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      _requiresExternalSource
      accepted
  have transcriptBound :=
    runtime_pipeline_binding_checked_acceptance_transcript_bound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have publicInputBound :=
    runtime_pipeline_binding_checked_acceptance_public_input_bound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have pcsAndFri :=
    runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle
      assumptions
      validation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      accepted
  have coreContract :=
    runtime_pipeline_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have seedBinds :=
    runtime_pipeline_binding_checked_acceptance_seed_binds_witness_tree_digests
      validation
      artifact
      publicInput
      proof
      accepted
  have seededFriOpeningChecked :=
    runtime_pipeline_binding_checked_acceptance_seeded_fri_opening_requirements_checked
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro transcriptBound
            (And.intro publicInputBound
              (And.intro pcsAndFri.left
                (And.intro pcsAndFri.right
                  (And.intro seedBinds
                    (And.intro seededFriOpeningChecked
                      (And.intro coreContract.right
                        pipelineSound.right)))))))))
theorem runtime_pipeline_checked_acceptance_audited_concrete_opening_core_contract
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
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
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
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof _requiresExternalSource accepted
  have concreteContract :=
    runtime_pipeline_binding_checked_acceptance_audited_concrete_opening_contract
      assumptions
      validation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      _requiresExternalSource
      accepted
  have executionObligations :=
    runtime_pipeline_binding_checked_acceptance_execution_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have seedBinds :=
    concreteContract.right.right.right.right.right.right.right.left
  have seededFriOpeningChecked :=
    concreteContract.right.right.right.right.right.right.right.right.left
  have verifierCore :=
    concreteContract.right.right.right.right.right.right.right.right.right.left
  have soundWitness :=
    concreteContract.right.right.right.right.right.right.right.right.right.right
  exact
    And.intro concreteContract.left
      (And.intro concreteContract.right.left
        (And.intro concreteContract.right.right.left
          (And.intro concreteContract.right.right.right.left
            (And.intro concreteContract.right.right.right.right.left
              (And.intro concreteContract.right.right.right.right.right.left
                (And.intro concreteContract.right.right.right.right.right.right.left
                  (And.intro seedBinds
                    (And.intro seededFriOpeningChecked
                      (And.intro verifierCore
                        (And.intro executionObligations soundWitness))))))))))
theorem
runtime_pipeline_checked_acceptance_concrete_opening_audited_soundness_core_contract
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
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
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
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have concreteContract :=
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
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  rcases concreteContract with
    ⟨_auditedCrypto,
      proofSystemSound,
      verifierAccepts,
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
    ⟨auditedAssumptions.left,
      auditedAssumptions.right,
      proofSystemSound,
      verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      seedBinds,
      seededFriOpeningChecked,
      verifierCore,
      executionObligations,
      soundWitness⟩
set_option linter.style.longLine false in
theorem runtime_pipeline_binding_checked_acceptance_hash_concrete_opening_core_contract
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
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
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
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof _requiresExternalSource accepted
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have proofSystemSound := abstract_verifier_sound assumptions
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have pipelineSound :=
    runtime_pipeline_binding_checked_acceptance_sound_from_hash_concrete_opening
      assumptions
      hashAssumptions
      validation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      _requiresExternalSource
      accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  have queryPlanContract :=
    runtime_query_plan_binding_checked_acceptance_seeded_hash_concrete_opening_and_core_contract
      assumptions
      hashAssumptions
      validation.queryPlanBindingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      _requiresExternalSource
      queryPlanAccepted
  have executionObligations :=
    runtime_pipeline_binding_checked_acceptance_execution_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  rcases queryPlanContract with
    ⟨seededContract,
      _queryPlanBound,
      _openingSegmentBound,
      _openingEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore⟩
  have seedBinds :=
    runtime_query_plan_binding_seeded_contract_implies_seed_binds_witness_tree_digests
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      seededContract
  have seededFriOpeningChecked :=
    runtime_query_plan_binding_seeded_contract_implies_seeded_fri_opening_requirements_checked
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      seededContract
  exact
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      seedBinds,
      seededFriOpeningChecked,
      verifierCore,
      executionObligations,
      pipelineSound.right⟩

set_option linter.style.longLine false in
theorem runtime_pipeline_binding_checked_acceptance_hash_concrete_opening_audited_soundness_core_contract
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
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
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
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have concreteContract :=
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
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  rcases concreteContract with
    ⟨_auditedCrypto,
      proofSystemSound,
      verifierAccepts,
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
    ⟨auditedAssumptions.left,
      auditedAssumptions.right,
      proofSystemSound,
      verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      seedBinds,
      seededFriOpeningChecked,
      verifierCore,
      executionObligations,
      soundWitness⟩

theorem runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract
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
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
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
  have executionObligations :=
    runtime_pipeline_binding_checked_acceptance_execution_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have seedBinds :=
    compactContract.right.right.right.right.right.right.right.left
  have seededFriOpeningChecked :=
    compactContract.right.right.right.right.right.right.right.right.left
  have verifierCore :=
    compactContract.right.right.right.right.right.right.right.right.right.left
  have soundWitness :=
    compactContract.right.right.right.right.right.right.right.right.right.right
  exact
    And.intro compactContract.left
      (And.intro compactContract.right.left
        (And.intro compactContract.right.right.left
          (And.intro compactContract.right.right.right.left
            (And.intro compactContract.right.right.right.right.left
              (And.intro compactContract.right.right.right.right.right.left
                (And.intro compactContract.right.right.right.right.right.right.left
                  (And.intro seedBinds
                    (And.intro seededFriOpeningChecked
                      (And.intro verifierCore
                        (And.intro executionObligations soundWitness))))))))))

theorem runtime_pipeline_binding_checked_acceptance_contracts_core_contract
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
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted

theorem runtime_pipeline_binding_checked_acceptance_contracts_audited_soundness_core_contract
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
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
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have compactContract :=
    runtime_pipeline_binding_checked_acceptance_contracts_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
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
      seedBinds,
      seededFriOpeningChecked,
      verifierCore,
      executionObligations,
      soundWitness⟩
  exact
    ⟨auditedAssumptions.left,
      auditedAssumptions.right,
      proofSystemSound,
      verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      seedBinds,
      seededFriOpeningChecked,
      verifierCore,
      executionObligations,
      soundWitness⟩

theorem runtime_pipeline_binding_checked_acceptance_artifact_contracts_core_contract
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
        RuntimeArtifactEvidence
          system
          validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
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
  intro artifact publicInput proof requiresExternalSource accepted
  have artifactEvidence :=
    runtime_pipeline_binding_checked_acceptance_runtime_artifact_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have compactContract :=
    runtime_pipeline_binding_checked_acceptance_contracts_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact ⟨artifactEvidence, compactContract⟩

set_option linter.style.longLine false in
theorem
runtime_pipeline_binding_checked_acceptance_concrete_opening_artifact_audited_soundness_core_contract
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
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeArtifactEvidence
          system
          validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
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
  intro artifact publicInput proof requiresExternalSource accepted
  have artifactEvidence :=
    runtime_pipeline_binding_checked_acceptance_runtime_artifact_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have auditedConcrete :=
    runtime_pipeline_checked_acceptance_concrete_opening_audited_soundness_core_contract
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
  exact ⟨artifactEvidence, auditedConcrete⟩

set_option linter.style.longLine false in
theorem
runtime_pipeline_binding_checked_acceptance_hash_concrete_opening_artifact_audited_soundness_core_contract
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
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeArtifactEvidence
          system
          validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
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
  intro artifact publicInput proof requiresExternalSource accepted
  have artifactEvidence :=
    runtime_pipeline_binding_checked_acceptance_runtime_artifact_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have auditedConcrete :=
    runtime_pipeline_binding_checked_acceptance_hash_concrete_opening_audited_soundness_core_contract
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
  exact ⟨artifactEvidence, auditedConcrete⟩

theorem runtime_pipeline_binding_checked_acceptance_artifact_audited_core_contract
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
        RuntimeArtifactEvidence
          system
          validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
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
  intro artifact publicInput proof requiresExternalSource accepted
  have artifactEvidence :=
    runtime_pipeline_binding_checked_acceptance_runtime_artifact_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have auditedCompactContract :=
    runtime_pipeline_binding_checked_acceptance_contracts_audited_soundness_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact ⟨artifactEvidence, auditedCompactContract⟩

end Lzvm
