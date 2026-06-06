/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks

/-!
Digest-prefix conformance obligations for row-major Poseidon helpers.
-/

namespace Lzvm

def Poseidon2DigestWordCount : Nat := 4

def Poseidon2DigestPrefix {α : Type u} (fullStateWords : List α) : List α :=
  fullStateWords.take Poseidon2DigestWordCount

def FullStateMerkleObservation {α : Type u} (fullStateWords : List α) : List α :=
  Poseidon2DigestPrefix fullStateWords

def DigestPrefixMerkleObservation {α : Type u} (digestWords : List α) : List α :=
  digestWords

structure DigestPrefixRoundEvidence (α : Type u) where
  fullStateWords : List α
  digestWords : List α
  digestWordsMatchFullStatePrefix :
    digestWords = Poseidon2DigestPrefix fullStateWords

def DigestPrefixRoundVisibleWords
    (evidence : DigestPrefixRoundEvidence α) : List α :=
  evidence.digestWords

theorem digest_prefix_round_visible_words_eq_full_state_prefix
    (evidence : DigestPrefixRoundEvidence α) :
    DigestPrefixRoundVisibleWords evidence =
      Poseidon2DigestPrefix evidence.fullStateWords := by
  exact evidence.digestWordsMatchFullStatePrefix

theorem digest_prefix_round_merkle_observation_eq_full_state
    (evidence : DigestPrefixRoundEvidence α) :
    DigestPrefixMerkleObservation (DigestPrefixRoundVisibleWords evidence) =
      FullStateMerkleObservation evidence.fullStateWords := by
  exact evidence.digestWordsMatchFullStatePrefix

structure RowMajorDigestPrefixValidation (system : VerifierModel) where
  leafValidation : WitnessLeafDigestValidation system
  digestPrefixRoundsMatchFullState : PublicInput -> Proof -> Prop
  fullStateWideLinearDigestsBindRows : PublicInput -> Proof -> Prop
  matchedPrefixImpliesWideLinearDigestsBindRows :
    forall publicInput proof,
      digestPrefixRoundsMatchFullState publicInput proof ->
        fullStateWideLinearDigestsBindRows publicInput proof ->
          leafValidation.wideLinearDigestsBindRows publicInput proof

def RowMajorDigestPrefixEvidence
    (_system : VerifierModel)
    (validation : RowMajorDigestPrefixValidation _system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.digestPrefixRoundsMatchFullState publicInput proof
    /\ validation.fullStateWideLinearDigestsBindRows publicInput proof

def RowMajorDigestPrefixCheckedAcceptance
    (system : VerifierModel)
    (validation : RowMajorDigestPrefixValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ RowMajorDigestPrefixEvidence system validation publicInput proof

theorem row_major_digest_prefix_evidence_implies_wide_linear_digests
    {system : VerifierModel}
    (validation : RowMajorDigestPrefixValidation system) :
    forall publicInput proof,
      RowMajorDigestPrefixEvidence system validation publicInput proof ->
        validation.leafValidation.wideLinearDigestsBindRows
          publicInput
          proof := by
  intro publicInput proof evidence
  exact
    validation.matchedPrefixImpliesWideLinearDigestsBindRows
      publicInput
      proof
      evidence.left
      evidence.right

theorem row_major_digest_prefix_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RowMajorDigestPrefixValidation system) :
    forall publicInput proof,
      RowMajorDigestPrefixCheckedAcceptance
          system
          validation
          publicInput
          proof ->
        validation.leafValidation.wideLinearDigestsBindRows publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (row_major_digest_prefix_evidence_implies_wide_linear_digests
        validation
        publicInput
        proof
        checked.right)
      (abstract_verifier_sound assumptions publicInput proof checked.left)

end Lzvm
