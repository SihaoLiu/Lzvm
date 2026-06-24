/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding.Accepts

/-!
Compact proof-system contracts derived from runtime proof pipeline binding.
-/

namespace Lzvm

universe uDigest

theorem runtime_pipeline_binding_checked_acceptance_opening_bound_contract
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningBoundContract
          system
          validation.queryPlanBindingValidation.openingValidation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have openingSegmentAccepted :=
    runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_checked_acceptance_opening_bound_contract
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentAccepted

theorem runtime_pipeline_binding_checked_acceptance_constant_opening_bound_from_concrete_nary_merkle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let openingValidation :=
          validation.queryPlanBindingValidation.openingValidation.openingValidation
        openingValidation.constantOpeningsBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have openingSegmentAccepted :=
    runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentAccepted
  exact
    runtime_constant_opening_nary_checked_acceptance_constant_bound_from_bundle
      assumptions
      validation.queryPlanBindingValidation.openingValidation.openingValidation
      centralized
      binding
      artifact
      publicInput
      proof
      openingAccepted

theorem runtime_pipeline_binding_checked_acceptance_witness_opening_bound_from_concrete_nary_merkle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let openingValidation :=
          validation.queryPlanBindingValidation.openingValidation.openingValidation
        openingValidation.witnessOpeningsBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have openingSegmentAccepted :=
    runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentAccepted
  exact
    runtime_witness_opening_nary_checked_acceptance_witness_bound_from_bundle
      assumptions
      validation.queryPlanBindingValidation.openingValidation.openingValidation
      centralized
      binding
      artifact
      publicInput
      proof
      openingAccepted

theorem runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle
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
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  have openingSegmentAccepted :=
    runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentAccepted
  exact
    runtime_opening_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle
      assumptions
      validation.queryPlanBindingValidation.openingValidation.openingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      openingAccepted

set_option linter.style.longLine false in
theorem runtime_pipeline_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle
    {Digest : Type uDigest}
    {system : VerifierModel}
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
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  have openingSegmentAccepted :=
    runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentAccepted
  exact
    runtime_opening_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle
      hashAssumptions
      validation.queryPlanBindingValidation.openingValidation.openingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      openingAccepted

theorem runtime_pipeline_binding_checked_acceptance_runtime_soundness_evidence_from_opening_checks
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
        RuntimeSoundnessEvidence
          system
          (runtime_pipeline_runtime_soundness_validation validation)
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have openingSegmentAccepted :=
    runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentAccepted
  exact
    runtime_opening_checked_acceptance_runtime_soundness_evidence_from_opening_checks
      assumptions
      validation.queryPlanBindingValidation.openingValidation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted

set_option linter.style.longLine false in
theorem runtime_pipeline_binding_checked_acceptance_runtime_soundness_evidence_from_hash_concrete_opening
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
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeSoundnessEvidence
          system
          (runtime_pipeline_runtime_soundness_validation validation)
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have openingSegmentAccepted :=
    runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have segmentSound :=
    runtime_opening_segment_binding_checked_acceptance_sound_from_hash_concrete_opening
      assumptions
      hashAssumptions
      validation.queryPlanBindingValidation.openingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      openingSegmentAccepted
  exact segmentSound.right.left.left

set_option linter.style.longLine false in
theorem runtime_pipeline_binding_checked_acceptance_runtime_soundness_evidence_from_concrete_nary_merkle
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
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeSoundnessEvidence
          system
          (runtime_pipeline_runtime_soundness_validation validation)
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have openingSegmentAccepted :=
    runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have segmentSound :=
    runtime_opening_segment_binding_checked_acceptance_sound_from_concrete_nary_merkle
      assumptions
      validation.queryPlanBindingValidation.openingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      openingSegmentAccepted
  exact segmentSound.right.left.left

set_option linter.style.longLine false in
theorem runtime_pipeline_binding_checked_acceptance_sound_from_concrete_nary_merkle
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
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimePipelineBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  have traceAccepted :=
    runtime_pipeline_binding_checked_acceptance_trace
      validation
      artifact
      publicInput
      proof
      accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  have ethSound :=
    runtime_eth_block_public_input_binding_checked_acceptance_sound
      assumptions
      validation.ethBindingValidation
      artifact
      publicInput
      proof
      ethAccepted
  have traceSound :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_sound
      assumptions
      validation.traceBindingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceAccepted
  have queryPlanSound :=
    runtime_query_plan_binding_checked_acceptance_sound_from_concrete_nary_merkle
      assumptions
      validation.queryPlanBindingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      queryPlanAccepted
  have ethEvidence := ethSound.left
  have artifactEvidence := ethSound.right.left
  have runtimeArtifactEvidence := ethSound.right.right.left
  have tracePreflightEvidence := traceSound.left
  have traceConstraintEvidence := traceSound.right.left
  have queryPlanEvidence := queryPlanSound.left
  have challengeEvidence := queryPlanSound.right.left
  have openingSegmentEvidence := queryPlanSound.right.right.left
  have openingEvidence := queryPlanSound.right.right.right.left
  have transcriptBound := queryPlanSound.right.right.right.right.left
  have pcsOpeningsValid := queryPlanSound.right.right.right.right.right.left
  have friQueriesValid := queryPlanSound.right.right.right.right.right.right.left
  have soundWitness := queryPlanSound.right.right.right.right.right.right.right
  have coreContract :=
    runtime_pipeline_binding_checked_acceptance_core_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  rcases coreContract with
    ⟨_coreTranscriptBound, corePublicInputBound, _corePcsOpeningsValid, _coreFriQueriesValid⟩
  have publicInputBound : system.publicInputBound publicInput proof :=
    corePublicInputBound
  exact
    And.intro
      (And.intro ethEvidence
        (And.intro artifactEvidence
          (And.intro runtimeArtifactEvidence
            (And.intro tracePreflightEvidence
              (And.intro traceConstraintEvidence
                (And.intro queryPlanEvidence
                  (And.intro challengeEvidence
                    (And.intro openingSegmentEvidence
                      (And.intro openingEvidence
                        (And.intro transcriptBound
                          (And.intro publicInputBound
                            (And.intro pcsOpeningsValid friQueriesValid))))))))))))
      soundWitness

set_option linter.style.longLine false in
theorem runtime_pipeline_binding_checked_acceptance_sound_from_hash_concrete_opening
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
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimePipelineBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  have traceAccepted :=
    runtime_pipeline_binding_checked_acceptance_trace
      validation
      artifact
      publicInput
      proof
      accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  have ethSound :=
    runtime_eth_block_public_input_binding_checked_acceptance_sound
      assumptions
      validation.ethBindingValidation
      artifact
      publicInput
      proof
      ethAccepted
  have traceSound :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_sound
      assumptions
      validation.traceBindingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceAccepted
  have queryPlanSound :=
    runtime_query_plan_binding_checked_acceptance_sound_from_hash_concrete_opening
      assumptions
      hashAssumptions
      validation.queryPlanBindingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      queryPlanAccepted
  have ethEvidence := ethSound.left
  have artifactEvidence := ethSound.right.left
  have runtimeArtifactEvidence := ethSound.right.right.left
  have tracePreflightEvidence := traceSound.left
  have traceConstraintEvidence := traceSound.right.left
  have queryPlanEvidence := queryPlanSound.left
  have challengeEvidence := queryPlanSound.right.left
  have openingSegmentEvidence := queryPlanSound.right.right.left
  have openingEvidence := queryPlanSound.right.right.right.left
  have transcriptBound := queryPlanSound.right.right.right.right.left
  have pcsOpeningsValid := queryPlanSound.right.right.right.right.right.left
  have friQueriesValid := queryPlanSound.right.right.right.right.right.right.left
  have soundWitness := queryPlanSound.right.right.right.right.right.right.right
  have coreContract :=
    runtime_pipeline_binding_checked_acceptance_core_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  rcases coreContract with
    ⟨_coreTranscriptBound, corePublicInputBound, _corePcsOpeningsValid, _coreFriQueriesValid⟩
  have publicInputBound : system.publicInputBound publicInput proof :=
    corePublicInputBound
  exact
    And.intro
      (And.intro ethEvidence
        (And.intro artifactEvidence
          (And.intro runtimeArtifactEvidence
            (And.intro tracePreflightEvidence
              (And.intro traceConstraintEvidence
                (And.intro queryPlanEvidence
                  (And.intro challengeEvidence
                    (And.intro openingSegmentEvidence
                      (And.intro openingEvidence
                        (And.intro transcriptBound
                          (And.intro publicInputBound
                            (And.intro pcsOpeningsValid friQueriesValid))))))))))))
      soundWitness

set_option linter.style.longLine false in
theorem runtime_pipeline_binding_checked_acceptance_accepts_concrete_opening_sound_witness_contract
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
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.accepts publicInput proof
          /\ RuntimeSoundnessEvidence
            system
            (runtime_pipeline_runtime_soundness_validation validation)
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have runtimeEvidence :=
    runtime_pipeline_binding_checked_acceptance_runtime_soundness_evidence_from_concrete_nary_merkle
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
      requiresExternalSource
      accepted
  exact And.intro verifierAccepts
    (And.intro runtimeEvidence pipelineSound.right)

theorem runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract
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
        ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ RuntimePipelineBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeArtifactSoundnessObligations
            system
            validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
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
  have proofSystemSound := abstract_verifier_sound assumptions
  have acceptsFullContract :=
    runtime_pipeline_binding_checked_acceptance_accepts_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact And.intro proofSystemSound acceptsFullContract

theorem runtime_pipeline_binding_checked_acceptance_audited_proof_system_contract
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
          /\ RuntimePipelineBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeArtifactSoundnessObligations
            system
            validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
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
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have fullContract :=
    runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact And.intro auditedAssumptions fullContract

theorem runtime_pipeline_binding_checked_acceptance_audited_assumption_full_contract
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
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ RuntimePipelineBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeArtifactSoundnessObligations
            system
            validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
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
  have auditedContract :=
    runtime_pipeline_binding_checked_acceptance_audited_proof_system_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    And.intro auditedContract.left
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        auditedContract.right)

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

theorem runtime_pipeline_binding_required_external_source_contracts_core_contract
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
  have auditedCore :=
    runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
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
  rcases externalCore with
    ⟨traceExternalEvidence,
      openingExternalEvidence,
      verifierCore⟩
  rcases auditedCore with
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      _traceExternalEvidence,
      _openingExternalEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      seedBinds,
      seededFriOpeningChecked,
      _auditedVerifierCore,
      executionObligations,
      soundWitness⟩
  exact
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

end Lzvm
