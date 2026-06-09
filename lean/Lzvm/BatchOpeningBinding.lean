/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.OpeningSegmentBinding

/-!
Runtime batch witness opening row obligations.
-/

namespace Lzvm

structure RuntimeBatchWitnessOpeningRowsValidation (system : VerifierModel) where
  openingSegmentValidation : RuntimeOpeningSegmentBindingValidation system
  batchWitnessOpeningRowsAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  perRowWitnessOpeningRowsBound : RuntimeArtifact -> PublicInput -> Proof -> Prop
  batchWitnessOpeningRowsAcceptedImpliesOpeningSegmentAccepted :
    forall artifact publicInput proof,
      batchWitnessOpeningRowsAccepted artifact publicInput proof ->
        openingSegmentValidation.openingSegmentBindingAccepted artifact publicInput proof
  batchWitnessOpeningRowsAcceptedImpliesQueryPlanBound :
    forall artifact publicInput proof,
      batchWitnessOpeningRowsAccepted artifact publicInput proof ->
        openingSegmentValidation.queryPlanBound artifact publicInput proof
  batchWitnessOpeningRowsAcceptedImpliesPerRowWitnessOpeningRowsBound :
    forall artifact publicInput proof,
      batchWitnessOpeningRowsAccepted artifact publicInput proof ->
        perRowWitnessOpeningRowsBound artifact publicInput proof
  perRowWitnessOpeningRowsImplyWitnessOpeningSegmentsValid :
    forall artifact publicInput proof,
      openingSegmentValidation.queryPlanBound artifact publicInput proof ->
        perRowWitnessOpeningRowsBound artifact publicInput proof ->
          openingSegmentValidation.witnessOpeningSegmentsValid artifact publicInput proof

def RuntimeBatchWitnessOpeningRowsCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeBatchWitnessOpeningRowsValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.batchWitnessOpeningRowsAccepted artifact publicInput proof

def RuntimeBatchWitnessOpeningRowsBoundContract
    (_system : VerifierModel)
    (validation : RuntimeBatchWitnessOpeningRowsValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.openingSegmentValidation.queryPlanBound artifact publicInput proof
    /\ validation.perRowWitnessOpeningRowsBound artifact publicInput proof
    /\ validation.openingSegmentValidation.witnessOpeningSegmentsValid artifact publicInput proof
    /\ validation.openingSegmentValidation.openingValidation.witnessOpeningsBound
      artifact
      publicInput
      proof

def RuntimeBatchWitnessOpeningRowsEvidence
    (system : VerifierModel)
    (validation : RuntimeBatchWitnessOpeningRowsValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof)
    (requiresExternalSource : Prop) : Prop :=
  RuntimeOpeningSegmentBindingEvidence
      system
      validation.openingSegmentValidation
      artifact
      publicInput
      proof
    /\ RuntimeOpeningEvidence
      system
      validation.openingSegmentValidation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
    /\ validation.openingSegmentValidation.queryPlanBound artifact publicInput proof
    /\ validation.perRowWitnessOpeningRowsBound artifact publicInput proof
    /\ validation.openingSegmentValidation.witnessOpeningSegmentsValid artifact publicInput proof
    /\ validation.openingSegmentValidation.openingValidation.witnessOpeningsBound
      artifact
      publicInput
      proof

theorem runtime_batch_witness_opening_rows_checked_acceptance_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeBatchWitnessOpeningRowsValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeBatchWitnessOpeningRowsCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeBatchWitnessOpeningRowsEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have segmentAccepted :=
    validation.batchWitnessOpeningRowsAcceptedImpliesOpeningSegmentAccepted
      artifact
      publicInput
      proof
      accepted
  have segmentSound :=
    runtime_opening_segment_binding_checked_acceptance_sound
      assumptions
      validation.openingSegmentValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      segmentAccepted
  have queryPlanBound :=
    validation.batchWitnessOpeningRowsAcceptedImpliesQueryPlanBound
      artifact
      publicInput
      proof
      accepted
  have perRowWitnessOpeningRows :=
    validation.batchWitnessOpeningRowsAcceptedImpliesPerRowWitnessOpeningRowsBound
      artifact
      publicInput
      proof
      accepted
  have witnessOpeningSegments :=
    validation.perRowWitnessOpeningRowsImplyWitnessOpeningSegmentsValid
      artifact
      publicInput
      proof
      queryPlanBound
      perRowWitnessOpeningRows
  have witnessOpeningsBound :=
    validation.openingSegmentValidation.openingSegmentChecksImplyWitnessOpeningsBound
      artifact
      publicInput
      proof
      queryPlanBound
      witnessOpeningSegments
  exact
    And.intro segmentSound.left
      (And.intro segmentSound.right.left
        (And.intro queryPlanBound
          (And.intro perRowWitnessOpeningRows
            (And.intro witnessOpeningSegments witnessOpeningsBound))))

theorem runtime_batch_witness_opening_rows_evidence_implies_opening_segment_evidence
    {system : VerifierModel}
    (validation : RuntimeBatchWitnessOpeningRowsValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeBatchWitnessOpeningRowsEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeOpeningSegmentBindingEvidence
          system
          validation.openingSegmentValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.left

theorem runtime_batch_witness_opening_rows_evidence_implies_opening_evidence
    {system : VerifierModel}
    (validation : RuntimeBatchWitnessOpeningRowsValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeBatchWitnessOpeningRowsEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeOpeningEvidence
          system
          validation.openingSegmentValidation.openingValidation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.right.left

theorem runtime_batch_witness_opening_rows_evidence_implies_bound_contract
    {system : VerifierModel}
    (validation : RuntimeBatchWitnessOpeningRowsValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeBatchWitnessOpeningRowsEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeBatchWitnessOpeningRowsBoundContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact
    And.intro evidence.right.right.left
      (And.intro evidence.right.right.right.left
        (And.intro evidence.right.right.right.right.left
          evidence.right.right.right.right.right))

theorem runtime_batch_witness_opening_rows_checked_acceptance_opening_segment_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeBatchWitnessOpeningRowsValidation system) :
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimeBatchWitnessOpeningRowsCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingEvidence
          system
          validation.openingSegmentValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have evidence :=
    runtime_batch_witness_opening_rows_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    runtime_batch_witness_opening_rows_evidence_implies_opening_segment_evidence
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      evidence

theorem runtime_batch_witness_opening_rows_checked_acceptance_opening_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeBatchWitnessOpeningRowsValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeBatchWitnessOpeningRowsCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningEvidence
          system
          validation.openingSegmentValidation.openingValidation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have evidence :=
    runtime_batch_witness_opening_rows_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    runtime_batch_witness_opening_rows_evidence_implies_opening_evidence
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      evidence

theorem runtime_batch_witness_opening_rows_checked_acceptance_bound_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeBatchWitnessOpeningRowsValidation system) :
    forall artifact publicInput proof,
      RuntimeBatchWitnessOpeningRowsCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeBatchWitnessOpeningRowsBoundContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_batch_witness_opening_rows_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  exact
    runtime_batch_witness_opening_rows_evidence_implies_bound_contract
      validation
      artifact
      publicInput
      proof
      False
      evidence

theorem runtime_batch_witness_opening_rows_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeBatchWitnessOpeningRowsValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeBatchWitnessOpeningRowsCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeBatchWitnessOpeningRowsEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have evidence :=
    runtime_batch_witness_opening_rows_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have segmentAccepted :=
    validation.batchWitnessOpeningRowsAcceptedImpliesOpeningSegmentAccepted
      artifact
      publicInput
      proof
      accepted
  have segmentSound :=
    runtime_opening_segment_binding_checked_acceptance_sound
      assumptions
      validation.openingSegmentValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      segmentAccepted
  exact And.intro evidence segmentSound.right.right

theorem runtime_batch_witness_opening_rows_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeBatchWitnessOpeningRowsValidation system) :
    forall artifact publicInput proof,
      RuntimeBatchWitnessOpeningRowsCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_batch_witness_opening_rows_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  exact sound_witness_implies_verifier_core_contract sound.right

theorem runtime_batch_witness_opening_rows_checked_acceptance_bound_and_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeBatchWitnessOpeningRowsValidation system) :
    forall artifact publicInput proof,
      RuntimeBatchWitnessOpeningRowsCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeBatchWitnessOpeningRowsBoundContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_batch_witness_opening_rows_checked_acceptance_bound_contract
        assumptions
        validation
        artifact
        publicInput
        proof
        accepted)
      (runtime_batch_witness_opening_rows_checked_acceptance_verifier_core_contract
        assumptions
        validation
        artifact
        publicInput
        proof
        accepted)

end Lzvm
