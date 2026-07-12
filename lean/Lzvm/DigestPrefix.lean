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

theorem row_major_digest_prefix_checked_acceptance_accepts_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RowMajorDigestPrefixValidation system) :
    forall publicInput proof,
      RowMajorDigestPrefixCheckedAcceptance
          system
          validation
          publicInput
          proof ->
        system.accepts publicInput proof
          /\ RowMajorDigestPrefixEvidence system validation publicInput proof
          /\ validation.leafValidation.wideLinearDigestsBindRows publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro checked.left
      (row_major_digest_prefix_checked_acceptance_evidence_core_and_sound
        assumptions
        validation
        publicInput
        proof
        checked)

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

def RowMajorMatrixWordAt
    {α : Type u}
    (words : List α)
    (columnCount row column : Nat) : List α :=
  (words.drop (row * columnCount + column)).take 1

def ColumnMajorMatrixWordAt
    {α : Type u}
    (words : List α)
    (rowCount row column : Nat) : List α :=
  (words.drop (column * rowCount + row)).take 1

structure ColumnMajorLayoutEvidence (α : Type u) where
  rowMajorWords : List α
  columnMajorWords : List α
  rowCount : Nat
  columnCount : Nat
  wordsMatch :
    forall row column,
      row < rowCount ->
        column < columnCount ->
          ColumnMajorMatrixWordAt columnMajorWords rowCount row column =
            RowMajorMatrixWordAt rowMajorWords columnCount row column

theorem column_major_layout_evidence_preserves_word_observation
    (evidence : ColumnMajorLayoutEvidence α) :
    forall row column,
      row < evidence.rowCount ->
        column < evidence.columnCount ->
          ColumnMajorMatrixWordAt
              evidence.columnMajorWords
              evidence.rowCount
              row
              column =
            RowMajorMatrixWordAt
              evidence.rowMajorWords
              evidence.columnCount
              row
              column := by
  intro row column rowBound columnBound
  exact evidence.wordsMatch row column rowBound columnBound

structure ColumnMajorDigestPrefixValidation (system : VerifierModel) where
  rowMajorValidation : RowMajorDigestPrefixValidation system
  columnMajorLayoutMatchesRows : PublicInput -> Proof -> Prop
  layoutMatchImpliesRowMajorEvidence :
    forall publicInput proof,
      columnMajorLayoutMatchesRows publicInput proof ->
        RowMajorDigestPrefixEvidence
          system
          rowMajorValidation
          publicInput
          proof

def ColumnMajorDigestPrefixEvidence
    (system : VerifierModel)
    (validation : ColumnMajorDigestPrefixValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.columnMajorLayoutMatchesRows publicInput proof

def ColumnMajorDigestPrefixCheckedAcceptance
    (system : VerifierModel)
    (validation : ColumnMajorDigestPrefixValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ ColumnMajorDigestPrefixEvidence system validation publicInput proof

theorem column_major_digest_prefix_evidence_projects_row_major_evidence
    {system : VerifierModel}
    (validation : ColumnMajorDigestPrefixValidation system) :
    forall publicInput proof,
      ColumnMajorDigestPrefixEvidence system validation publicInput proof ->
        RowMajorDigestPrefixEvidence
          system
          validation.rowMajorValidation
          publicInput
          proof := by
  intro publicInput proof evidence
  exact
    validation.layoutMatchImpliesRowMajorEvidence
      publicInput
      proof
      evidence

theorem column_major_digest_prefix_evidence_implies_wide_linear_digests
    {system : VerifierModel}
    (validation : ColumnMajorDigestPrefixValidation system) :
    forall publicInput proof,
      ColumnMajorDigestPrefixEvidence system validation publicInput proof ->
        validation.rowMajorValidation.leafValidation.wideLinearDigestsBindRows
          publicInput
          proof := by
  intro publicInput proof evidence
  exact
    row_major_digest_prefix_evidence_implies_wide_linear_digests
      validation.rowMajorValidation
      publicInput
      proof
      (column_major_digest_prefix_evidence_projects_row_major_evidence
        validation
        publicInput
        proof
        evidence)

theorem column_major_digest_prefix_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : ColumnMajorDigestPrefixValidation system) :
    forall publicInput proof,
      ColumnMajorDigestPrefixCheckedAcceptance
          system
          validation
          publicInput
          proof ->
        validation.rowMajorValidation.leafValidation.wideLinearDigestsBindRows
            publicInput
            proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (column_major_digest_prefix_evidence_implies_wide_linear_digests
        validation
        publicInput
        proof
        checked.right)
      ((abstract_verifier_sound_with_semantic_evidence assumptions).right
        publicInput
        proof
        checked.left)

theorem column_major_digest_prefix_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : ColumnMajorDigestPrefixValidation system) :
    forall publicInput proof,
      ColumnMajorDigestPrefixCheckedAcceptance
          system
          validation
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (assumption_bundle_fiat_shamir_transcript_binding
        assumptions
        publicInput
        proof
        checked.left)
      (And.intro
        (assumption_bundle_public_input_binding
          assumptions
          publicInput
          proof
          checked.left)
        (And.intro
          (assumption_bundle_pcs_opening_soundness
            assumptions
            publicInput
            proof
            checked.left)
          (assumption_bundle_fri_query_soundness
            assumptions
            publicInput
            proof
            checked.left)))

structure FusedColumnMajorDigestPrefixValidation (system : VerifierModel) where
  columnMajorValidation : ColumnMajorDigestPrefixValidation system
  fusedCanonicalHashRoundsBindRows : PublicInput -> Proof -> Prop
  fusedRoundsImplyColumnMajorEvidence :
    forall publicInput proof,
      fusedCanonicalHashRoundsBindRows publicInput proof ->
        ColumnMajorDigestPrefixEvidence
          system
          columnMajorValidation
          publicInput
          proof

def FusedColumnMajorDigestPrefixEvidence
    (system : VerifierModel)
    (validation : FusedColumnMajorDigestPrefixValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.fusedCanonicalHashRoundsBindRows publicInput proof

def FusedColumnMajorDigestPrefixCheckedAcceptance
    (system : VerifierModel)
    (validation : FusedColumnMajorDigestPrefixValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ FusedColumnMajorDigestPrefixEvidence system validation publicInput proof

theorem fused_column_major_digest_prefix_evidence_projects_column_major_evidence
    {system : VerifierModel}
    (validation : FusedColumnMajorDigestPrefixValidation system) :
    forall publicInput proof,
      FusedColumnMajorDigestPrefixEvidence system validation publicInput proof ->
        ColumnMajorDigestPrefixEvidence
          system
          validation.columnMajorValidation
          publicInput
          proof := by
  intro publicInput proof evidence
  exact
    validation.fusedRoundsImplyColumnMajorEvidence
      publicInput
      proof
      evidence

theorem fused_column_major_digest_prefix_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : FusedColumnMajorDigestPrefixValidation system) :
    forall publicInput proof,
      FusedColumnMajorDigestPrefixCheckedAcceptance
          system
          validation
          publicInput
          proof ->
        validation.columnMajorValidation.rowMajorValidation.leafValidation.wideLinearDigestsBindRows
            publicInput
            proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    column_major_digest_prefix_checked_acceptance_sound
      assumptions
      validation.columnMajorValidation
      publicInput
      proof
      (And.intro
        checked.left
        (fused_column_major_digest_prefix_evidence_projects_column_major_evidence
          validation
          publicInput
          proof
          checked.right))

theorem fused_column_major_digest_prefix_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : FusedColumnMajorDigestPrefixValidation system) :
    forall publicInput proof,
      FusedColumnMajorDigestPrefixCheckedAcceptance
          system
          validation
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    column_major_digest_prefix_checked_acceptance_verifier_core_contract
      assumptions
      validation.columnMajorValidation
      publicInput
      proof
      (And.intro
        checked.left
        (fused_column_major_digest_prefix_evidence_projects_column_major_evidence
          validation
          publicInput
          proof
          checked.right))

end Lzvm
