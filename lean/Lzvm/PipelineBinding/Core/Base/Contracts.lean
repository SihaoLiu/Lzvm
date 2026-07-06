/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding.Core.Base.Prelude

/-!
Runtime proof pipeline binding derived contracts.
-/

namespace Lzvm

theorem runtime_pipeline_binding_checked_acceptance_proof_artifact_full_contract
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactBindingEvidence
            system
            validation.ethBindingValidation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingStructuralObligations
            system
            validation.ethBindingValidation.proofArtifactBindingValidation
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_pipeline_binding_checked_acceptance_proof_artifact_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have structural :=
    runtime_pipeline_binding_checked_acceptance_eth_artifact_wellformed_contract
      validation
      artifact
      publicInput
      proof
      accepted
  exact And.intro evidence structural

theorem runtime_pipeline_binding_checked_acceptance_eth_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.ethBindingValidation.proofArtifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_eth_block_public_input_binding_checked_acceptance_concrete_segment_ids_allowed
      validation.ethBindingValidation
      binding
      artifact
      publicInput
      proof
      (runtime_pipeline_binding_checked_acceptance_eth
        validation
        artifact
        publicInput
        proof
        accepted)

theorem runtime_pipeline_binding_eth_concrete_segment_id_binding_of_query_binding
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      let queryPlanValidation := validation.queryPlanBindingValidation
      let challengeValidation := queryPlanValidation.challengeValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        challengeValidation.transcriptValidation.artifactBindingValidation) :
    RuntimeProofArtifactConcreteSegmentIdBinding
      validation.ethBindingValidation.proofArtifactBindingValidation := by
  exact
    runtime_proof_artifact_concrete_segment_id_binding_of_agreement_left
      validation.artifactBindingValidationAgreement
      binding

theorem
    runtime_pipeline_binding_checked_acceptance_eth_concrete_segment_ids_allowed_of_query_binding
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      let queryPlanValidation := validation.queryPlanBindingValidation
      let challengeValidation := queryPlanValidation.challengeValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        challengeValidation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_pipeline_binding_checked_acceptance_eth_concrete_segment_ids_allowed
      validation
      (runtime_pipeline_binding_eth_concrete_segment_id_binding_of_query_binding
        validation
        binding)
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_binding_checked_acceptance_eth_full_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingEvidence
            system
            validation.ethBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingEvidence
            system
            validation.ethBindingValidation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeEthBlockPublicInputBindingStructuralObligations
            system
            validation.ethBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_eth_block_public_input_binding_checked_acceptance_full_contract
      assumptions
      validation.ethBindingValidation
      artifact
      publicInput
      proof
      ethAccepted

theorem runtime_pipeline_binding_checked_acceptance_eth_concrete_core_sound_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.ethBindingValidation.proofArtifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        (RuntimeEthBlockPublicInputBindingEvidence
            system
            validation.ethBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingEvidence
            system
            validation.ethBindingValidation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeEthBlockPublicInputBindingStructuralObligations
            system
            validation.ethBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof)
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_eth_block_public_input_binding_checked_acceptance_concrete_core_sound_contract
      assumptions
      validation.ethBindingValidation
      binding
      artifact
      publicInput
      proof
      ethAccepted

theorem runtime_pipeline_binding_checked_acceptance_eth_public_input_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingEvidence
            system
            validation.ethBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingEvidence
            system
            validation.ethBindingValidation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ system.publicInputBound publicInput proof := by
  intro artifact publicInput proof accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_eth_block_public_input_binding_checked_acceptance_public_input_contract
      assumptions
      validation.ethBindingValidation
      artifact
      publicInput
      proof
      ethAccepted

theorem runtime_pipeline_binding_checked_acceptance_eth_soundness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingSoundnessContract
          system
          validation.ethBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_eth_block_public_input_binding_checked_acceptance_soundness_contract
      assumptions
      validation.ethBindingValidation
      artifact
      publicInput
      proof
      ethAccepted

theorem runtime_pipeline_binding_checked_acceptance_eth_audited_finalized_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeEthBlockPublicInputBindingEvidence
            system
            validation.ethBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactFinalized
            system
            validation.ethBindingValidation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeEthBlockPublicInputBindingStructuralObligations
            system
            validation.ethBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_eth_block_public_input_binding_audited_finalized_segment_ids_contract
      assumptions
      validation.ethBindingValidation
      artifact
      publicInput
      proof
      ethAccepted

theorem
    runtime_pipeline_binding_checked_acceptance_eth_audited_finalized_concrete_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.ethBindingValidation.proofArtifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        (RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeEthBlockPublicInputBindingEvidence
            system
            validation.ethBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactFinalized
            system
            validation.ethBindingValidation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeEthBlockPublicInputBindingStructuralObligations
            system
            validation.ethBindingValidation
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
  intro artifact publicInput proof accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_eth_block_public_input_binding_audited_finalized_concrete_segment_ids_contract
      assumptions
      validation.ethBindingValidation
      binding
      artifact
      publicInput
      proof
      ethAccepted

theorem
    runtime_pipeline_binding_checked_acceptance_eth_audited_concrete_contract_of_query_binding
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      let queryPlanValidation := validation.queryPlanBindingValidation
      let challengeValidation := queryPlanValidation.challengeValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        challengeValidation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        (RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeEthBlockPublicInputBindingEvidence
            system
            validation.ethBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactFinalized
            system
            validation.ethBindingValidation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeEthBlockPublicInputBindingStructuralObligations
            system
            validation.ethBindingValidation
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
  intro artifact publicInput proof accepted
  exact
    runtime_pipeline_binding_checked_acceptance_eth_audited_finalized_concrete_segment_ids_contract
      assumptions
      validation
      (runtime_pipeline_binding_eth_concrete_segment_id_binding_of_query_binding
        validation
        binding)
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_binding_checked_acceptance_framed_guest_input_soundness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (bridge : RuntimePipelineFramedGuestInputBindingBridge system validation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFramedGuestInputBindingSoundnessContract
          system
          bridge.framedGuestInputBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have framedAccepted :=
    runtime_pipeline_binding_checked_acceptance_framed_guest_input
      validation
      bridge
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_framed_guest_input_binding_checked_acceptance_soundness_contract
      assumptions
      bridge.framedGuestInputBindingValidation
      artifact
      publicInput
      proof
      framedAccepted

theorem runtime_pipeline_binding_checked_acceptance_framed_guest_input_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (bridge : RuntimePipelineFramedGuestInputBindingBridge system validation)
    (binding :
      let framedValidation := bridge.framedGuestInputBindingValidation
      let inputValidation := framedValidation.ethBlockValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        inputValidation.proofArtifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_framed_guest_input_binding_checked_acceptance_concrete_segment_ids_allowed
      bridge.framedGuestInputBindingValidation
      binding
      artifact
      publicInput
      proof
      (runtime_pipeline_binding_checked_acceptance_framed_guest_input
        validation
        bridge
        artifact
        publicInput
        proof
        accepted)

theorem runtime_pipeline_binding_checked_acceptance_framed_guest_input_full_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (bridge : RuntimePipelineFramedGuestInputBindingBridge system validation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFramedGuestInputBindingEvidence
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have framedAccepted :=
    runtime_pipeline_binding_checked_acceptance_framed_guest_input
      validation
      bridge
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_framed_guest_input_binding_checked_acceptance_full_contract
      assumptions
      bridge.framedGuestInputBindingValidation
      artifact
      publicInput
      proof
      framedAccepted

theorem runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation.queryPlanBindingValidation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_checked_acceptance_opening
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted

theorem runtime_pipeline_binding_checked_acceptance_opening_segment_evidence
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingEvidence
          system
          validation.queryPlanBindingValidation.openingValidation
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
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentAccepted

theorem runtime_pipeline_binding_checked_acceptance_fri_parser_contract
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (boundary :
      RuntimeFriOpeningSegmentParserBoundary
        system
        validation.queryPlanBindingValidation.openingValidation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFriOpeningSegmentParserContract
          boundary
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
    runtime_opening_segment_binding_checked_acceptance_fri_parser_contract
      validation.queryPlanBindingValidation.openingValidation
      boundary
      artifact
      publicInput
      proof
      openingSegmentAccepted

theorem runtime_pipeline_binding_checked_acceptance_fri_fold_trace_identity_contract
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFriFoldTraceIdentityContract
          system
          validation.queryPlanBindingValidation.openingValidation
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
    runtime_opening_segment_binding_checked_acceptance_fri_fold_trace_identity_contract
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentAccepted

theorem runtime_pipeline_binding_checked_acceptance_fri_fold_query_plan_order_contract
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFriFoldQueryPlanOrderContract
          system
          validation.queryPlanBindingValidation.openingValidation
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
    runtime_opening_segment_binding_checked_acceptance_fri_fold_query_plan_order_contract
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentAccepted

theorem runtime_pipeline_binding_checked_acceptance_verifier_accepts
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.accepts publicInput proof := by
  intro artifact publicInput proof accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding
      validation.ethBindingValidation
      artifact
      publicInput
      proof
      ethAccepted
  let proofArtifactValidation :=
    validation.ethBindingValidation.proofArtifactBindingValidation
  have runtimeAccepted :=
    proofArtifactValidation.bindingAcceptedImpliesRuntimeAccepted
      artifact
      publicInput
      proof
      artifactAccepted
  exact
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      proofArtifactValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted

theorem runtime_pipeline_binding_checked_acceptance_sound
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
        RuntimePipelineBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
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
  have ethFull :=
    runtime_pipeline_binding_checked_acceptance_eth_full_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have traceSound :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_sound
      assumptions
      validation.traceBindingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceAccepted
  have queryPlanFull :=
    runtime_query_plan_binding_checked_acceptance_full_soundness_contract
      assumptions
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      queryPlanAccepted
  rcases queryPlanFull with
    ⟨queryPlanEvidence,
      _queryPlanBound,
      challengeEvidence,
      openingSegmentEvidence,
      _openingSegmentBound,
      openingEvidence,
      _openingBound,
      transcriptBound,
      pcsOpeningsValid,
      friQueriesValid,
      _queryPlanCoreContract,
      soundWitness⟩
  have ethEvidence := ethFull.left
  have artifactEvidence := ethFull.right.left
  have runtimeArtifactEvidence := ethFull.right.right.right.left
  have tracePreflightEvidence := traceSound.left
  have traceConstraintEvidence := traceSound.right.left
  have verifierAccepted :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have publicInputBound : system.publicInputBound publicInput proof :=
    assumption_bundle_public_input_binding
      assumptions
      publicInput
      proof
      verifierAccepted
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



end Lzvm
