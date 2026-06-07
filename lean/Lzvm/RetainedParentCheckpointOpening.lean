/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.BatchOpeningBinding

/-!
Runtime retained parent checkpoint opening obligations.
-/

namespace Lzvm

structure RuntimeRetainedParentCheckpointOpeningValidation (system : VerifierModel) where
  batchRowsValidation : RuntimeBatchWitnessOpeningRowsValidation system
  retainedParentCheckpointOpeningAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedParentCheckpointLevelAvailable : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedParentCheckpointLowerPrefixBound : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedParentCheckpointUpperSuffixBound : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedParentCheckpointStitchedPathBound : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedParentCheckpointRootMatchesExpectedRoot : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedParentCheckpointRowsFromSource : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedParentCheckpointRowsBoundToQueryPlan : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedParentCheckpointOpeningAcceptedImpliesBatchRowsAccepted :
    forall artifact publicInput proof,
      retainedParentCheckpointOpeningAccepted artifact publicInput proof ->
        batchRowsValidation.batchWitnessOpeningRowsAccepted artifact publicInput proof
  retainedParentCheckpointOpeningAcceptedImpliesLevelAvailable :
    forall artifact publicInput proof,
      retainedParentCheckpointOpeningAccepted artifact publicInput proof ->
        retainedParentCheckpointLevelAvailable artifact publicInput proof
  retainedParentCheckpointOpeningAcceptedImpliesLowerPrefixBound :
    forall artifact publicInput proof,
      retainedParentCheckpointOpeningAccepted artifact publicInput proof ->
        retainedParentCheckpointLowerPrefixBound artifact publicInput proof
  retainedParentCheckpointOpeningAcceptedImpliesUpperSuffixBound :
    forall artifact publicInput proof,
      retainedParentCheckpointOpeningAccepted artifact publicInput proof ->
        retainedParentCheckpointUpperSuffixBound artifact publicInput proof
  retainedParentCheckpointOpeningAcceptedImpliesStitchedPathBound :
    forall artifact publicInput proof,
      retainedParentCheckpointOpeningAccepted artifact publicInput proof ->
        retainedParentCheckpointStitchedPathBound artifact publicInput proof
  retainedParentCheckpointOpeningAcceptedImpliesRootMatchesExpectedRoot :
    forall artifact publicInput proof,
      retainedParentCheckpointOpeningAccepted artifact publicInput proof ->
        retainedParentCheckpointRootMatchesExpectedRoot artifact publicInput proof
  retainedParentCheckpointOpeningAcceptedImpliesRowsFromSource :
    forall artifact publicInput proof,
      retainedParentCheckpointOpeningAccepted artifact publicInput proof ->
        retainedParentCheckpointRowsFromSource artifact publicInput proof
  retainedParentCheckpointOpeningAcceptedImpliesRowsBoundToQueryPlan :
    forall artifact publicInput proof,
      retainedParentCheckpointOpeningAccepted artifact publicInput proof ->
        retainedParentCheckpointRowsBoundToQueryPlan artifact publicInput proof
  retainedParentCheckpointChecksImplyPerRowWitnessOpeningRowsBound :
    forall artifact publicInput proof,
      batchRowsValidation.openingSegmentValidation.queryPlanBound artifact publicInput proof ->
        retainedParentCheckpointRowsBoundToQueryPlan artifact publicInput proof ->
          retainedParentCheckpointRowsFromSource artifact publicInput proof ->
            retainedParentCheckpointLowerPrefixBound artifact publicInput proof ->
              retainedParentCheckpointUpperSuffixBound artifact publicInput proof ->
                retainedParentCheckpointStitchedPathBound artifact publicInput proof ->
                  retainedParentCheckpointRootMatchesExpectedRoot artifact publicInput proof ->
                    batchRowsValidation.perRowWitnessOpeningRowsBound
                      artifact
                      publicInput
                      proof

def RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.retainedParentCheckpointOpeningAccepted artifact publicInput proof

def RuntimeRetainedParentCheckpointOpeningDigestContract
    (_system : VerifierModel)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.retainedParentCheckpointLevelAvailable artifact publicInput proof
    /\ validation.retainedParentCheckpointLowerPrefixBound artifact publicInput proof
    /\ validation.retainedParentCheckpointUpperSuffixBound artifact publicInput proof
    /\ validation.retainedParentCheckpointStitchedPathBound artifact publicInput proof
    /\ validation.retainedParentCheckpointRootMatchesExpectedRoot artifact publicInput proof
    /\ validation.retainedParentCheckpointRowsFromSource artifact publicInput proof
    /\ validation.retainedParentCheckpointRowsBoundToQueryPlan artifact publicInput proof

def RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
    (_system : VerifierModel)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  let openingValidation :=
    validation.batchRowsValidation.openingSegmentValidation.openingValidation
  validation.retainedParentCheckpointRowsBoundToQueryPlan artifact publicInput proof
    /\ validation.retainedParentCheckpointRowsFromSource artifact publicInput proof
    /\ validation.batchRowsValidation.perRowWitnessOpeningRowsBound
      artifact
      publicInput
      proof
    /\ validation.batchRowsValidation.openingSegmentValidation.witnessOpeningSegmentsValid
      artifact
      publicInput
      proof
    /\ openingValidation.witnessOpeningsBound
      artifact
      publicInput
      proof

def RuntimeRetainedParentCheckpointOpeningEvidence
    (system : VerifierModel)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof)
    (requiresExternalSource : Prop) : Prop :=
  RuntimeBatchWitnessOpeningRowsEvidence
      system
      validation.batchRowsValidation
      artifact
      publicInput
      proof
      requiresExternalSource
    /\ RuntimeRetainedParentCheckpointOpeningDigestContract
      system
      validation
      artifact
      publicInput
      proof
    /\ RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
      system
      validation
      artifact
      publicInput
      proof

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have batchAccepted :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesBatchRowsAccepted
      artifact
      publicInput
      proof
      accepted
  have batchEvidence :=
    runtime_batch_witness_opening_rows_checked_acceptance_evidence
      assumptions
      validation.batchRowsValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      batchAccepted
  have queryPlanBound :=
    validation.batchRowsValidation.batchWitnessOpeningRowsAcceptedImpliesQueryPlanBound
      artifact
      publicInput
      proof
      batchAccepted
  have levelAvailable :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesLevelAvailable
      artifact
      publicInput
      proof
      accepted
  have lowerPrefix :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesLowerPrefixBound
      artifact
      publicInput
      proof
      accepted
  have upperSuffix :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesUpperSuffixBound
      artifact
      publicInput
      proof
      accepted
  have stitchedPath :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesStitchedPathBound
      artifact
      publicInput
      proof
      accepted
  have rootMatches :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesRootMatchesExpectedRoot
      artifact
      publicInput
      proof
      accepted
  have rowsFromSource :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesRowsFromSource
      artifact
      publicInput
      proof
      accepted
  have rowsBoundToQueryPlan :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesRowsBoundToQueryPlan
      artifact
      publicInput
      proof
      accepted
  have retainedPerRow :=
    validation.retainedParentCheckpointChecksImplyPerRowWitnessOpeningRowsBound
      artifact
      publicInput
      proof
      queryPlanBound
      rowsBoundToQueryPlan
      rowsFromSource
      lowerPrefix
      upperSuffix
      stitchedPath
      rootMatches
  let segmentValidation := validation.batchRowsValidation.openingSegmentValidation
  let openingValidation := segmentValidation.openingValidation
  have witnessSegments :=
    validation.batchRowsValidation.perRowWitnessOpeningRowsImplyWitnessOpeningSegmentsValid
      artifact
      publicInput
      proof
      queryPlanBound
      retainedPerRow
  have witnessOpeningsBound : openingValidation.witnessOpeningsBound
      artifact
      publicInput
      proof :=
    segmentValidation.openingSegmentChecksImplyWitnessOpeningsBound
      artifact
      publicInput
      proof
      queryPlanBound
      witnessSegments
  exact
    And.intro batchEvidence
      (And.intro
        (And.intro levelAvailable
          (And.intro lowerPrefix
            (And.intro upperSuffix
              (And.intro stitchedPath
                (And.intro rootMatches
                  (And.intro rowsFromSource rowsBoundToQueryPlan))))))
        (And.intro rowsBoundToQueryPlan
          (And.intro rowsFromSource
            (And.intro retainedPerRow
              (And.intro witnessSegments witnessOpeningsBound)))))

theorem runtime_retained_parent_checkpoint_opening_evidence_implies_digest_contract
    {system : VerifierModel}
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeRetainedParentCheckpointOpeningDigestContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.right.left

theorem runtime_retained_parent_checkpoint_opening_evidence_implies_batch_rows_evidence
    {system : VerifierModel}
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeBatchWitnessOpeningRowsEvidence
          system
          validation.batchRowsValidation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.left

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_batch_rows_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeBatchWitnessOpeningRowsEvidence
          system
          validation.batchRowsValidation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have evidence :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    runtime_retained_parent_checkpoint_opening_evidence_implies_batch_rows_evidence
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      evidence

theorem runtime_retained_parent_checkpoint_opening_evidence_implies_batch_rows_bound_contract
    {system : VerifierModel}
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeBatchWitnessOpeningRowsBoundContract
          system
          validation.batchRowsValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  have batchEvidence :=
    runtime_retained_parent_checkpoint_opening_evidence_implies_batch_rows_evidence
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      evidence
  exact
    runtime_batch_witness_opening_rows_evidence_implies_bound_contract
      validation.batchRowsValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      batchEvidence

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_batch_rows_bound_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeBatchWitnessOpeningRowsBoundContract
          system
          validation.batchRowsValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  exact
    runtime_retained_parent_checkpoint_opening_evidence_implies_batch_rows_bound_contract
      validation
      artifact
      publicInput
      proof
      False
      evidence

theorem runtime_retained_parent_checkpoint_opening_evidence_implies_opening_evidence
    {system : VerifierModel}
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeOpeningEvidence
          system
          validation.batchRowsValidation.openingSegmentValidation.openingValidation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource evidence
  have batchEvidence :=
    runtime_retained_parent_checkpoint_opening_evidence_implies_batch_rows_evidence
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      evidence
  exact
    runtime_batch_witness_opening_rows_evidence_implies_opening_evidence
      validation.batchRowsValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      batchEvidence

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningEvidence
          system
          validation.batchRowsValidation.openingSegmentValidation.openingValidation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have evidence :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    runtime_retained_parent_checkpoint_opening_evidence_implies_opening_evidence
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      evidence

theorem runtime_retained_parent_checkpoint_opening_evidence_implies_retained_rows_contract
    {system : VerifierModel}
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.right.right

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_retained_rows_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  exact
    runtime_retained_parent_checkpoint_opening_evidence_implies_retained_rows_contract
      validation
      artifact
      publicInput
      proof
      False
      evidence

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_digest_contract
    {system : VerifierModel}
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningDigestContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have levelAvailable :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesLevelAvailable
      artifact
      publicInput
      proof
      accepted
  have lowerPrefix :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesLowerPrefixBound
      artifact
      publicInput
      proof
      accepted
  have upperSuffix :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesUpperSuffixBound
      artifact
      publicInput
      proof
      accepted
  have stitchedPath :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesStitchedPathBound
      artifact
      publicInput
      proof
      accepted
  have rootMatches :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesRootMatchesExpectedRoot
      artifact
      publicInput
      proof
      accepted
  have rowsFromSource :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesRowsFromSource
      artifact
      publicInput
      proof
      accepted
  have rowsBoundToQueryPlan :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesRowsBoundToQueryPlan
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro levelAvailable
      (And.intro lowerPrefix
        (And.intro upperSuffix
          (And.intro stitchedPath
            (And.intro rootMatches
              (And.intro rowsFromSource rowsBoundToQueryPlan)))))

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have evidence :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have batchAccepted :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesBatchRowsAccepted
      artifact
      publicInput
      proof
      accepted
  have batchSound :=
    runtime_batch_witness_opening_rows_checked_acceptance_sound
      assumptions
      validation.batchRowsValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      batchAccepted
  exact And.intro evidence batchSound.right

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  exact sound_witness_implies_verifier_core_contract sound.right

end Lzvm
