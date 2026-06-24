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
  ethBlockInputMatches : RuntimeArtifact -> PublicInput -> Proof -> Prop
  ethPublicValuesMatch : RuntimeArtifact -> PublicInput -> Proof -> Prop
  ethBindingAcceptedImpliesProofArtifactBindingAccepted :
    forall artifact publicInput proof,
      ethBlockBindingAccepted artifact publicInput proof ->
        proofArtifactBindingValidation.artifactBindingAccepted artifact publicInput proof
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
  validation.ethBlockInputMatches artifact publicInput proof
    /\ validation.ethPublicValuesMatch artifact publicInput proof

def RuntimeEthBlockPublicInputBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeEthBlockPublicInputBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.ethBlockBindingAccepted artifact publicInput proof

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
    And.intro
      (validation.ethBindingAcceptedImpliesEthBlockInputMatches
        artifact
        publicInput
        proof
        accepted)
      (validation.ethBindingAcceptedImpliesEthPublicValuesMatch
        artifact
        publicInput
        proof
        accepted)

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
  exact
    ⟨containerCanonical,
      metadataCanonical,
      segmentsPresent,
      segmentPayloadsNonempty,
      segmentIdsAllowed,
      segmentIdsUnique⟩

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

end Lzvm
