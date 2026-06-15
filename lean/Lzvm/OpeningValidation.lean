/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.MerklePathSoundness
import Lzvm.RuntimeSoundness

/-!
Runtime PCS and FRI opening validation obligations.
-/

namespace Lzvm

universe uDigest

structure RuntimeOpeningValidation (system : VerifierModel) where
  runtimeSoundnessValidation : RuntimeSoundnessValidation system
  openingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  constantOpeningsBound : RuntimeArtifact -> PublicInput -> Proof -> Prop
  witnessOpeningsBound : RuntimeArtifact -> PublicInput -> Proof -> Prop
  friOpeningBound : RuntimeArtifact -> PublicInput -> Proof -> Prop
  openingAcceptedImpliesRuntimeSoundnessAccepted :
    forall artifact publicInput proof requiresExternalSource,
      openingAccepted artifact publicInput proof ->
        RuntimeSoundnessCheckedAcceptance
          system
          runtimeSoundnessValidation
          artifact
          publicInput
          proof
          requiresExternalSource
  openingAcceptedImpliesConstantOpeningsBound :
    forall artifact publicInput proof,
      openingAccepted artifact publicInput proof ->
        constantOpeningsBound artifact publicInput proof
  openingAcceptedImpliesWitnessOpeningsBound :
    forall artifact publicInput proof,
      openingAccepted artifact publicInput proof ->
        witnessOpeningsBound artifact publicInput proof
  openingAcceptedImpliesFriOpeningBound :
    forall artifact publicInput proof,
      openingAccepted artifact publicInput proof ->
        friOpeningBound artifact publicInput proof
  openingChecksImplyPcsOpeningsValid :
    forall artifact publicInput proof,
      constantOpeningsBound artifact publicInput proof ->
        witnessOpeningsBound artifact publicInput proof ->
          friOpeningBound artifact publicInput proof ->
            system.pcsOpeningsValid publicInput proof
  friOpeningImpliesFriQueriesValid :
    forall artifact publicInput proof,
      friOpeningBound artifact publicInput proof ->
        system.friQueriesValid publicInput proof

def RuntimeOpeningEvidence
    (system : VerifierModel)
    (validation : RuntimeOpeningValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof)
    (requiresExternalSource : Prop) : Prop :=
  RuntimeSoundnessEvidence
      system
      validation.runtimeSoundnessValidation
      artifact
      publicInput
      proof
      requiresExternalSource
    /\ validation.constantOpeningsBound artifact publicInput proof
    /\ validation.witnessOpeningsBound artifact publicInput proof
    /\ validation.friOpeningBound artifact publicInput proof
    /\ system.pcsOpeningsValid publicInput proof
    /\ system.friQueriesValid publicInput proof

def RuntimeOpeningBoundContract
    (_system : VerifierModel)
    (validation : RuntimeOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.constantOpeningsBound artifact publicInput proof
    /\ validation.witnessOpeningsBound artifact publicInput proof
    /\ validation.friOpeningBound artifact publicInput proof

def RuntimeOpeningCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.openingAccepted artifact publicInput proof

theorem runtime_opening_checked_acceptance_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have runtimeAccepted :=
    validation.openingAcceptedImpliesRuntimeSoundnessAccepted
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have runtimeEvidence :=
    runtime_soundness_checked_acceptance_evidence
      assumptions
      validation.runtimeSoundnessValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      runtimeAccepted
  have constantOpenings :=
    validation.openingAcceptedImpliesConstantOpeningsBound
      artifact
      publicInput
      proof
      accepted
  have witnessOpenings :=
    validation.openingAcceptedImpliesWitnessOpeningsBound
      artifact
      publicInput
      proof
      accepted
  have friOpening :=
    validation.openingAcceptedImpliesFriOpeningBound
      artifact
      publicInput
      proof
      accepted
  have pcsOpenings :=
    validation.openingChecksImplyPcsOpeningsValid
      artifact
      publicInput
      proof
      constantOpenings
      witnessOpenings
      friOpening
  have friQueries :=
    validation.friOpeningImpliesFriQueriesValid
      artifact
      publicInput
      proof
      friOpening
  exact
    And.intro runtimeEvidence
      (And.intro constantOpenings
        (And.intro witnessOpenings
          (And.intro friOpening
            (And.intro pcsOpenings friQueries))))

theorem runtime_opening_evidence_implies_bound_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeOpeningBoundContract system validation artifact publicInput proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact
    And.intro evidence.right.left
      (And.intro evidence.right.right.left evidence.right.right.right.left)

theorem runtime_opening_checked_acceptance_bound_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeOpeningBoundContract system validation artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (validation.openingAcceptedImpliesConstantOpeningsBound
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (validation.openingAcceptedImpliesWitnessOpeningsBound
          artifact
          publicInput
          proof
          accepted)
        (validation.openingAcceptedImpliesFriOpeningBound
          artifact
          publicInput
          proof
          accepted))

theorem runtime_opening_evidence_implies_pcs_and_fri
    {system : VerifierModel}
    (validation : RuntimeOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.right.right.right.right

theorem runtime_opening_checked_acceptance_pcs_and_fri_without_assumptions
    {system : VerifierModel}
    (validation : RuntimeOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  have constantOpenings :=
    validation.openingAcceptedImpliesConstantOpeningsBound
      artifact
      publicInput
      proof
      accepted
  have witnessOpenings :=
    validation.openingAcceptedImpliesWitnessOpeningsBound
      artifact
      publicInput
      proof
      accepted
  have friOpening :=
    validation.openingAcceptedImpliesFriOpeningBound
      artifact
      publicInput
      proof
      accepted
  have pcsOpenings :=
    validation.openingChecksImplyPcsOpeningsValid
      artifact
      publicInput
      proof
      constantOpenings
      witnessOpenings
      friOpening
  have friQueries :=
    validation.friOpeningImpliesFriQueriesValid
      artifact
      publicInput
      proof
      friOpening
  exact And.intro pcsOpenings friQueries

theorem runtime_opening_checked_acceptance_pcs_and_fri
    {system : VerifierModel}
    (_assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_opening_checked_acceptance_pcs_and_fri_without_assumptions
      validation
      artifact
      publicInput
      proof
      accepted

theorem runtime_witness_opening_arity_two_same_index_leaf_binding_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening otherOpening,
      NAryMerklePathHasArity 2 opening.layers ->
        NAryMerklePathHasArity 2 otherOpening.layers ->
          NAryMerklePathOpeningVerifies compress root opening ->
            NAryMerklePathOpeningVerifies compress root otherOpening ->
              NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherOpening.layers ->
                opening.layers.length = otherOpening.layers.length ->
                  otherOpening.leaf = opening.leaf := by
  intro root opening otherOpening openingArity otherOpeningArity verified
    otherVerified sameIndex sameDepth
  exact
    verified_concrete_nary_merkle_opening_arity_two_same_index_leaf_eq_from_bundle
      assumptions
      centralized
      root
      opening
      otherOpening
      openingArity
      otherOpeningArity
      verified
      otherVerified
      sameIndex
      sameDepth

theorem runtime_witness_opening_arity_four_same_index_leaf_binding_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening otherOpening,
      NAryMerklePathHasArity 4 opening.layers ->
        NAryMerklePathHasArity 4 otherOpening.layers ->
          NAryMerklePathOpeningVerifies compress root opening ->
            NAryMerklePathOpeningVerifies compress root otherOpening ->
              NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherOpening.layers ->
                opening.layers.length = otherOpening.layers.length ->
                  otherOpening.leaf = opening.leaf := by
  intro root opening otherOpening openingArity otherOpeningArity verified
    otherVerified sameIndex sameDepth
  exact
    verified_concrete_nary_merkle_opening_arity_four_same_index_leaf_eq_from_bundle
      assumptions
      centralized
      root
      opening
      otherOpening
      openingArity
      otherOpeningArity
      verified
      otherVerified
      sameIndex
      sameDepth

theorem runtime_opening_evidence_implies_external_source_requirement
    {system : VerifierModel}
    (validation : RuntimeOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        ExternalSourceOpeningRequirement
          system
          validation.runtimeSoundnessValidation.sourceValidation
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact
    runtime_soundness_evidence_implies_external_source_requirement
      validation.runtimeSoundnessValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      evidence.left

theorem runtime_opening_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeOpeningEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have runtimeAccepted :=
    validation.openingAcceptedImpliesRuntimeSoundnessAccepted
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have evidence :=
    runtime_opening_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have runtimeSound :=
    runtime_soundness_checked_acceptance_sound
      assumptions
      validation.runtimeSoundnessValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      runtimeAccepted
  exact And.intro evidence runtimeSound.right

theorem runtime_opening_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningValidation system) :
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof _requiresExternalSource accepted
  have sound :=
    runtime_opening_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      _requiresExternalSource
      accepted
  exact sound_witness_implies_verifier_core_contract sound.right

theorem runtime_opening_required_external_source_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof ->
        requiresExternalSource ->
          RuntimeOpeningEvidence
              system
              validation
              artifact
              publicInput
              proof
              requiresExternalSource
            /\ ExternalSourceOpeningEvidence
              system
              validation.runtimeSoundnessValidation.sourceValidation
              publicInput
              proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have runtimeAccepted :=
    validation.openingAcceptedImpliesRuntimeSoundnessAccepted
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have openingSound :=
    runtime_opening_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have externalSound :=
    runtime_soundness_required_external_source_sound
      assumptions
      validation.runtimeSoundnessValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      runtimeAccepted
      required
  exact
    And.intro openingSound.left
      (And.intro externalSound.right.left openingSound.right)

theorem runtime_opening_required_external_source_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof ->
        requiresExternalSource ->
          RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have sound :=
    runtime_opening_required_external_source_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  exact sound_witness_implies_verifier_core_contract sound.right.right

end Lzvm
