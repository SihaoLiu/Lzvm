/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.ProofArtifactBinding

/-!
Runtime ETH block public-input binding obligations.
-/

namespace Lzvm

structure RuntimeEthBlockPublicInputBindingValidation (system : VerifierModel) where
  proofArtifactBindingValidation : RuntimeProofArtifactBindingValidation system
  ethBlockBindingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  ethBlockInputSegmentPresent : RuntimeArtifact -> PublicInput -> Proof -> Prop
  ethBlockInputSectionsUnique : RuntimeArtifact -> PublicInput -> Proof -> Prop
  ethBlockInputMatches : RuntimeArtifact -> PublicInput -> Proof -> Prop
  ethPublicValuesMatch : RuntimeArtifact -> PublicInput -> Proof -> Prop
  ethBindingAcceptedImpliesProofArtifactBindingAccepted :
    forall artifact publicInput proof,
      ethBlockBindingAccepted artifact publicInput proof ->
        proofArtifactBindingValidation.artifactBindingAccepted artifact publicInput proof
  ethBindingAcceptedImpliesEthBlockInputSegmentPresent :
    forall artifact publicInput proof,
      ethBlockBindingAccepted artifact publicInput proof ->
        ethBlockInputSegmentPresent artifact publicInput proof
  ethBindingAcceptedImpliesEthBlockInputSectionsUnique :
    forall artifact publicInput proof,
      ethBlockBindingAccepted artifact publicInput proof ->
        ethBlockInputSectionsUnique artifact publicInput proof
  ethBindingAcceptedImpliesEthBlockInputMatches :
    forall artifact publicInput proof,
      ethBlockBindingAccepted artifact publicInput proof ->
        ethBlockInputMatches artifact publicInput proof
  ethBindingAcceptedImpliesEthPublicValuesMatch :
    forall artifact publicInput proof,
      ethBlockBindingAccepted artifact publicInput proof ->
        ethPublicValuesMatch artifact publicInput proof

def RuntimeEthBlockPublicInputBindingEvidence
    (_system : VerifierModel)
    (validation : RuntimeEthBlockPublicInputBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.ethBlockInputSegmentPresent artifact publicInput proof
    /\ validation.ethBlockInputSectionsUnique artifact publicInput proof
    /\ validation.ethBlockInputMatches artifact publicInput proof
    /\ validation.ethPublicValuesMatch artifact publicInput proof

def RuntimeEthBlockPublicInputBindingStructuralObligations
    (system : VerifierModel)
    (validation : RuntimeEthBlockPublicInputBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeEthBlockPublicInputBindingEvidence
      system
      validation
      artifact
      publicInput
      proof
    /\ RuntimeProofArtifactBindingStructuralObligations
      system
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof

def RuntimeEthBlockPublicInputBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeEthBlockPublicInputBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.ethBlockBindingAccepted artifact publicInput proof

def RuntimeEthBlockPublicInputBindingSoundnessContract
    (system : VerifierModel)
    (validation : RuntimeEthBlockPublicInputBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeEthBlockPublicInputBindingEvidence
      system
      validation
      artifact
      publicInput
      proof
    /\ RuntimeProofArtifactBindingEvidence
      system
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
    /\ RuntimeArtifactEvidence
      system
      validation.proofArtifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
    /\ system.accepts publicInput proof
    /\ validation.proofArtifactBindingValidation.proofContainerCanonical
      artifact
      publicInput
      proof
    /\ validation.proofArtifactBindingValidation.proofMetadataCanonical
      artifact
      publicInput
      proof
    /\ validation.proofArtifactBindingValidation.proofSegmentsPresent
      artifact
      publicInput
      proof
    /\ validation.proofArtifactBindingValidation.proofSegmentPayloadsNonempty
      artifact
      publicInput
      proof
    /\ validation.proofArtifactBindingValidation.proofSegmentIdsAllowed
      artifact
      publicInput
      proof
    /\ validation.proofArtifactBindingValidation.proofSegmentIdsUnique
      artifact
      publicInput
      proof
    /\ validation.proofArtifactBindingValidation.proofUnitValuesTraceIdentityCoverage
      artifact
      publicInput
      proof
    /\ RuntimeVerifierCoreContract system publicInput proof
    /\ SoundWitness system publicInput proof

theorem runtime_eth_block_public_input_binding_checked_acceptance_evidence
    {system : VerifierModel}
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingEvidence
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    ⟨validation.ethBindingAcceptedImpliesEthBlockInputSegmentPresent
        artifact
        publicInput
        proof
        accepted,
      (validation.ethBindingAcceptedImpliesEthBlockInputSectionsUnique
        artifact
        publicInput
        proof
        accepted),
      (validation.ethBindingAcceptedImpliesEthBlockInputMatches
        artifact
        publicInput
        proof
        accepted),
      (validation.ethBindingAcceptedImpliesEthPublicValuesMatch
        artifact
        publicInput
        proof
        accepted)⟩

theorem runtime_eth_block_public_input_binding_checked_acceptance_input_segment_present
    {system : VerifierModel}
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.ethBlockInputSegmentPresent artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.ethBindingAcceptedImpliesEthBlockInputSegmentPresent
      artifact
      publicInput
      proof
      accepted

theorem runtime_eth_block_public_input_binding_checked_acceptance_input_sections_unique
    {system : VerifierModel}
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.ethBlockInputSectionsUnique artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.ethBindingAcceptedImpliesEthBlockInputSectionsUnique
      artifact
      publicInput
      proof
      accepted

theorem runtime_eth_block_public_input_binding_checked_acceptance_input_matches
    {system : VerifierModel}
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.ethBlockInputMatches artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.ethBindingAcceptedImpliesEthBlockInputMatches
      artifact
      publicInput
      proof
      accepted

theorem runtime_eth_block_public_input_binding_checked_acceptance_public_values_match
    {system : VerifierModel}
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.ethPublicValuesMatch artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.ethBindingAcceptedImpliesEthPublicValuesMatch
      artifact
      publicInput
      proof
      accepted

theorem runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding
    {system : VerifierModel}
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation.proofArtifactBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.ethBindingAcceptedImpliesProofArtifactBindingAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_eth_block_public_input_binding_checked_acceptance_artifact_evidence_contract
    {system : VerifierModel}
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingEvidence
            system
            validation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted
  have ethEvidence :=
    runtime_eth_block_public_input_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactEvidence :=
    runtime_proof_artifact_binding_checked_acceptance_evidence
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have runtimeEvidence :=
    runtime_proof_artifact_binding_evidence_implies_runtime_evidence
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactEvidence
  exact
    And.intro ethEvidence
      (And.intro artifactEvidence runtimeEvidence)

theorem runtime_eth_block_public_input_binding_checked_acceptance_artifact_wellformed_contract
    {system : VerifierModel}
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.proofArtifactBindingValidation.proofContainerCanonical artifact publicInput proof
          /\ validation.proofArtifactBindingValidation.proofMetadataCanonical
            artifact
            publicInput
            proof
          /\ validation.proofArtifactBindingValidation.proofSegmentsPresent
            artifact
            publicInput
            proof
          /\ validation.proofArtifactBindingValidation.proofSegmentPayloadsNonempty
            artifact
            publicInput
            proof
          /\ validation.proofArtifactBindingValidation.proofSegmentIdsAllowed
            artifact
            publicInput
            proof
          /\ validation.proofArtifactBindingValidation.proofSegmentIdsUnique
            artifact
            publicInput
            proof
          /\ validation.proofArtifactBindingValidation.proofUnitValuesTraceIdentityCoverage
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted
  have artifactAccepted :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding
      validation
      artifact
      publicInput
      proof
      accepted
  have containerCanonical :=
    runtime_proof_artifact_binding_checked_acceptance_container_canonical
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have metadataCanonical :=
    runtime_proof_artifact_binding_checked_acceptance_metadata_canonical
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have segmentsPresent :=
    runtime_proof_artifact_binding_checked_acceptance_segments_present
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have segmentPayloadsNonempty :=
    runtime_proof_artifact_binding_checked_acceptance_segment_payloads_nonempty
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have segmentIdsAllowed :=
    runtime_proof_artifact_binding_checked_acceptance_segment_ids_allowed
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have segmentIdsUnique :=
    runtime_proof_artifact_binding_checked_acceptance_segment_ids_unique
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have unitValuesTraceIdentityCoverage :=
    runtime_proof_artifact_binding_checked_acceptance_unit_values_trace_identity_coverage
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  exact
    ⟨containerCanonical,
      metadataCanonical,
      segmentsPresent,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique,
      unitValuesTraceIdentityCoverage⟩

theorem runtime_eth_block_public_input_binding_checked_acceptance_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeEthBlockPublicInputBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.proofArtifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_proof_artifact_binding_checked_acceptance_concrete_segment_ids_allowed
      validation.proofArtifactBindingValidation
      binding
      artifact
      publicInput
      proof
      (runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding
        validation
        artifact
        publicInput
        proof
        accepted)

theorem runtime_eth_block_public_input_binding_checked_acceptance_structural_obligations
    {system : VerifierModel}
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingStructuralObligations
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have inputEvidence :=
    runtime_eth_block_public_input_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactStructural :=
    runtime_proof_artifact_binding_checked_acceptance_structural_obligations
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  exact
    And.intro inputEvidence artifactStructural

theorem runtime_eth_block_public_input_binding_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingEvidence
            system
            validation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have evidenceContract :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_evidence_contract
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactSound :=
    runtime_proof_artifact_binding_checked_acceptance_sound
      assumptions
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  exact
    And.intro evidenceContract.left
      (And.intro evidenceContract.right.left
        (And.intro evidenceContract.right.right artifactSound.right.right))

theorem runtime_eth_block_public_input_binding_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have artifactAccepted :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted

theorem runtime_eth_block_public_input_binding_checked_acceptance_public_input_bound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.publicInputBound publicInput proof := by
  intro artifact publicInput proof accepted
  have coreContract :=
    runtime_eth_block_public_input_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact coreContract.right.left

theorem runtime_eth_block_public_input_binding_checked_acceptance_public_input_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingEvidence
            system
            validation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ system.publicInputBound publicInput proof := by
  intro artifact publicInput proof accepted
  have evidenceContract :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_evidence_contract
      validation
      artifact
      publicInput
      proof
      accepted
  have publicInputBound :=
    runtime_eth_block_public_input_binding_checked_acceptance_public_input_bound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro evidenceContract.left
      (And.intro evidenceContract.right.left
        (And.intro evidenceContract.right.right publicInputBound))

theorem runtime_eth_block_public_input_binding_checked_acceptance_soundness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingSoundnessContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_eth_block_public_input_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have wellformed :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_wellformed_contract
      validation
      artifact
      publicInput
      proof
      accepted
  have core :=
    runtime_eth_block_public_input_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding
      validation
      artifact
      publicInput
      proof
      accepted
  have runtimeAccepted :=
    runtime_proof_artifact_binding_checked_acceptance_runtime_accepted
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      validation.proofArtifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted
  exact
    ⟨sound.left,
      sound.right.left,
      sound.right.right.left,
      verifierAccepts,
      wellformed.left,
      wellformed.right.left,
      wellformed.right.right.left,
      wellformed.right.right.right.left,
      wellformed.right.right.right.right.left,
      wellformed.right.right.right.right.right.left,
      wellformed.right.right.right.right.right.right,
      core,
      sound.right.right.right⟩

theorem runtime_eth_block_public_input_binding_checked_acceptance_full_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingEvidence
            system
            validation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeEthBlockPublicInputBindingStructuralObligations
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_eth_block_public_input_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have structural :=
    runtime_eth_block_public_input_binding_checked_acceptance_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro sound.left
      (And.intro sound.right.left
        (And.intro structural sound.right.right))

theorem runtime_eth_block_public_input_binding_checked_acceptance_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingEvidence
            system
            validation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeEthBlockPublicInputBindingStructuralObligations
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have fullContract :=
    runtime_eth_block_public_input_binding_checked_acceptance_full_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have coreContract :=
    runtime_eth_block_public_input_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro fullContract.left
      (And.intro fullContract.right.left
        (And.intro fullContract.right.right.left
          (And.intro fullContract.right.right.right.left
            (And.intro coreContract fullContract.right.right.right.right))))

theorem
  runtime_eth_block_public_input_binding_checked_acceptance_concrete_core_sound_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeEthBlockPublicInputBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.proofArtifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        (RuntimeEthBlockPublicInputBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingEvidence
            system
            validation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeEthBlockPublicInputBindingStructuralObligations
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof)
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_eth_block_public_input_binding_checked_acceptance_evidence_core_and_sound
        assumptions
        validation
        artifact
        publicInput
        proof
        accepted)
      (runtime_eth_block_public_input_binding_checked_acceptance_concrete_segment_ids_allowed
        validation
        binding
        artifact
        publicInput
        proof
        accepted)

theorem
  runtime_eth_block_public_input_binding_audited_finalized_core_sound_witness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeEthBlockPublicInputBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactFinalized
            system
            validation.proofArtifactBindingValidation
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
  have ethEvidence :=
    runtime_eth_block_public_input_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactFinalized :=
    runtime_proof_artifact_finalized_from_checked_acceptance
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have runtimeAccepted :=
    runtime_proof_artifact_binding_checked_acceptance_runtime_accepted
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      validation.proofArtifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted
  have auditedCoreExecutionSound :=
    accepted_proof_audited_core_execution_and_sound_witness
      assumptions
      publicInput
      proof
      verifierAccepts
  rcases auditedCoreExecutionSound with
    ⟨cryptoEvidence, semanticEvidence, coreContract, executionObligations, soundWitness⟩
  exact
    And.intro cryptoEvidence
      (And.intro semanticEvidence
        (And.intro ethEvidence
          (And.intro artifactFinalized
            (And.intro coreContract
              (And.intro executionObligations soundWitness)))))

theorem
  runtime_eth_block_public_input_binding_direct_finalized_core_sound_witness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeEthBlockPublicInputBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactFinalized
            system
            validation.proofArtifactBindingValidation
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
  have ethEvidence :=
    runtime_eth_block_public_input_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactFinalized :=
    runtime_proof_artifact_finalized_from_checked_acceptance
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have runtimeAccepted :=
    runtime_proof_artifact_binding_checked_acceptance_runtime_accepted
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      validation.proofArtifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted
  have cryptoCore :=
    accepted_proof_crypto_core_contract
      assumptions
      publicInput
      proof
      verifierAccepts
  have semanticExecution :=
    accepted_proof_semantic_execution_obligations
      assumptions
      publicInput
      proof
      verifierAccepts
  have soundWitness :=
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      verifierAccepts
  rcases cryptoCore with
    ⟨cryptoEvidence, coreContract⟩
  rcases semanticExecution with
    ⟨semanticEvidence, executionObligations⟩
  exact
    And.intro cryptoEvidence
      (And.intro semanticEvidence
        (And.intro ethEvidence
          (And.intro artifactFinalized
            (And.intro coreContract
              (And.intro executionObligations soundWitness)))))

theorem
  runtime_eth_block_public_input_binding_audited_core_sound_witness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have finalizedCore :=
    runtime_eth_block_public_input_binding_audited_finalized_core_sound_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  rcases finalizedCore with
    ⟨cryptoEvidence,
      semanticEvidence,
      _ethEvidence,
      _artifactFinalized,
      coreContract,
      executionObligations,
      soundWitness⟩
  exact
    ⟨cryptoEvidence,
      semanticEvidence,
      coreContract,
      executionObligations,
      soundWitness⟩

theorem
  runtime_eth_block_public_input_binding_audited_finalized_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeEthBlockPublicInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeEthBlockPublicInputBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactFinalized
            system
            validation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeEthBlockPublicInputBindingStructuralObligations
            system
            validation
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
  have finalizedCore :=
    runtime_eth_block_public_input_binding_audited_finalized_core_sound_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have structural :=
    runtime_eth_block_public_input_binding_checked_acceptance_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  rcases finalizedCore with
    ⟨cryptoEvidence,
      semanticEvidence,
      ethEvidence,
      artifactFinalized,
      coreContract,
      executionObligations,
      soundWitness⟩
  exact
    ⟨cryptoEvidence,
      semanticEvidence,
      ethEvidence,
      artifactFinalized,
      structural,
      coreContract,
      executionObligations,
      soundWitness⟩

theorem
  runtime_eth_block_public_input_binding_audited_finalized_concrete_segment_ids_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeEthBlockPublicInputBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.proofArtifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        (RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeEthBlockPublicInputBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactFinalized
            system
            validation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeEthBlockPublicInputBindingStructuralObligations
            system
            validation
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
    And.intro
      (runtime_eth_block_public_input_binding_audited_finalized_segment_ids_contract
        assumptions
        validation
        artifact
        publicInput
        proof
        accepted)
      (runtime_eth_block_public_input_binding_checked_acceptance_concrete_segment_ids_allowed
        validation
        binding
        artifact
        publicInput
        proof
        accepted)

end Lzvm
