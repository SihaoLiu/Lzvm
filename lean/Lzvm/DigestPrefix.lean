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
      ((abstract_verifier_sound_with_semantic_evidence assumptions).right
        publicInput
        proof
        checked.left)

theorem row_major_digest_prefix_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RowMajorDigestPrefixValidation system) :
    forall publicInput proof,
      RowMajorDigestPrefixCheckedAcceptance
          system
          validation
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have accepted := checked.left
  exact
    And.intro
      (assumption_bundle_fiat_shamir_transcript_binding
        assumptions
        publicInput
        proof
        accepted)
      (And.intro
        (assumption_bundle_public_input_binding
          assumptions
          publicInput
          proof
          accepted)
        (And.intro
          (assumption_bundle_pcs_opening_soundness
            assumptions
            publicInput
            proof
            accepted)
          (assumption_bundle_fri_query_soundness
            assumptions
            publicInput
            proof
            accepted)))

theorem row_major_digest_prefix_checked_acceptance_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RowMajorDigestPrefixValidation system) :
    forall publicInput proof,
      RowMajorDigestPrefixCheckedAcceptance
          system
          validation
          publicInput
          proof ->
        RowMajorDigestPrefixEvidence system validation publicInput proof
          /\ validation.leafValidation.wideLinearDigestsBindRows publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    row_major_digest_prefix_checked_acceptance_sound
      assumptions
      validation
      publicInput
      proof
      checked
  have core :=
    row_major_digest_prefix_checked_acceptance_verifier_core_contract
      assumptions
      validation
      publicInput
      proof
      checked
  exact And.intro checked.right
    (And.intro sound.left (And.intro core sound.right))

theorem row_major_digest_prefix_checked_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RowMajorDigestPrefixValidation system) :
    forall publicInput proof,
      RowMajorDigestPrefixCheckedAcceptance
          system
          validation
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RowMajorDigestPrefixEvidence system validation publicInput proof
          /\ validation.leafValidation.wideLinearDigestsBindRows publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have wideLinear :=
    row_major_digest_prefix_evidence_implies_wide_linear_digests
      validation
      publicInput
      proof
      checked.right
  have core :=
    row_major_digest_prefix_checked_acceptance_verifier_core_contract
      assumptions
      validation
      publicInput
      proof
      checked
  have sound :=
    (abstract_verifier_sound_with_semantic_evidence assumptions).right
      publicInput
      proof
      checked.left
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        (And.intro checked.right
          (And.intro wideLinear (And.intro core sound))))

end Lzvm
