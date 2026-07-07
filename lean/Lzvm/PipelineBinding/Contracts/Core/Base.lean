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
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  have ethFull :=
    runtime_pipeline_binding_checked_acceptance_eth_full_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have traceArtifactContract :=
    runtime_pipeline_binding_checked_acceptance_trace_artifact_evidence_core_and_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
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
  have ethEvidence := ethFull.left
  have artifactEvidence := ethFull.right.left
  have runtimeArtifactEvidence := ethFull.right.right.right.left
  have tracePreflightEvidence := traceArtifactContract.left
  have traceConstraintEvidence := traceArtifactContract.right.left
  have traceCoreContract := traceArtifactContract.right.right.left
  have queryPlanEvidence := queryPlanSound.left
  have challengeEvidence := queryPlanSound.right.left
  have openingSegmentEvidence := queryPlanSound.right.right.left
  have openingEvidence := queryPlanSound.right.right.right.left
  have transcriptBound := queryPlanSound.right.right.right.right.left
  have pcsOpeningsValid := queryPlanSound.right.right.right.right.right.left
  have friQueriesValid := queryPlanSound.right.right.right.right.right.right.left
  have soundWitness := traceArtifactContract.right.right.right
  rcases traceCoreContract with
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
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  have ethFull :=
    runtime_pipeline_binding_checked_acceptance_eth_full_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have traceArtifactContract :=
    runtime_pipeline_binding_checked_acceptance_trace_artifact_evidence_core_and_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
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
  have ethEvidence := ethFull.left
  have artifactEvidence := ethFull.right.left
  have runtimeArtifactEvidence := ethFull.right.right.right.left
  have tracePreflightEvidence := traceArtifactContract.left
  have traceConstraintEvidence := traceArtifactContract.right.left
  have traceCoreContract := traceArtifactContract.right.right.left
  have queryPlanEvidence := queryPlanSound.left
  have challengeEvidence := queryPlanSound.right.left
  have openingSegmentEvidence := queryPlanSound.right.right.left
  have openingEvidence := queryPlanSound.right.right.right.left
  have transcriptBound := queryPlanSound.right.right.right.right.left
  have pcsOpeningsValid := queryPlanSound.right.right.right.right.right.left
  have friQueriesValid := queryPlanSound.right.right.right.right.right.right.left
  have soundWitness := traceArtifactContract.right.right.right
  rcases traceCoreContract with
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
              /\ SoundWitness system publicInput proof
              /\ RuntimeFriFoldTraceIdentityContract system
                validation.queryPlanBindingValidation.openingValidation
                artifact publicInput proof
              /\ RuntimeFriFoldQueryPlanOrderContract system
                validation.queryPlanBindingValidation.openingValidation
                artifact publicInput proof := by
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
              /\ SoundWitness system publicInput proof
              /\ RuntimeFriFoldTraceIdentityContract system
                validation.queryPlanBindingValidation.openingValidation
                artifact publicInput proof
              /\ RuntimeFriFoldQueryPlanOrderContract system
                validation.queryPlanBindingValidation.openingValidation
                artifact publicInput proof := by
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
              /\ SoundWitness system publicInput proof
              /\ RuntimeFriFoldTraceIdentityContract system
                validation.queryPlanBindingValidation.openingValidation
                artifact publicInput proof
              /\ RuntimeFriFoldQueryPlanOrderContract system
                validation.queryPlanBindingValidation.openingValidation
                artifact publicInput proof := by
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
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        auditedContract.right)
theorem runtime_pipeline_binding_checked_acceptance_audited_framed_guest_input_full_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (bridge : RuntimePipelineFramedGuestInputBindingBridge system validation) :
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
          /\ RuntimeFramedGuestInputBindingEvidence
            system
            bridge.framedGuestInputBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeFramedGuestInputBindingStructuralObligations
            system
            bridge.framedGuestInputBindingValidation
            artifact
            publicInput
            proof
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
              /\ SoundWitness system publicInput proof
              /\ RuntimeFriFoldTraceIdentityContract system
                validation.queryPlanBindingValidation.openingValidation
                artifact publicInput proof
              /\ RuntimeFriFoldQueryPlanOrderContract system
                validation.queryPlanBindingValidation.openingValidation
                artifact publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have auditedContract :=
    runtime_pipeline_binding_checked_acceptance_audited_assumption_full_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have framedContract :=
    runtime_pipeline_binding_checked_acceptance_framed_guest_input_full_contract
      assumptions
      validation
      bridge
      artifact
      publicInput
      proof
      accepted
  rcases auditedContract with
    ⟨cryptoAssumptions,
      semanticAssumptions,
        proofSystemSound,
        verifierAccepts,
        pipelineEvidence,
        artifactSoundness,
        coreContract,
        executionObligations,
        soundWitness,
        foldTraceIdentityContract,
        foldQueryPlanOrderContract⟩
  rcases framedContract with
    ⟨framedEvidence,
      framedStructural,
      _framedCoreContract,
      _framedSoundWitness⟩
  exact
    ⟨cryptoAssumptions,
      semanticAssumptions,
      proofSystemSound,
      verifierAccepts,
        pipelineEvidence,
        framedEvidence,
        framedStructural,
        artifactSoundness,
        coreContract,
        executionObligations,
        soundWitness,
        foldTraceIdentityContract,
        foldQueryPlanOrderContract⟩

end Lzvm
