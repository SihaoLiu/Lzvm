/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.BatchOpeningBinding

/-!
Runtime retained leaf digest opening obligations.
-/

namespace Lzvm

structure RuntimeRetainedLeafDigestOpeningValidation (system : VerifierModel) where
  batchRowsValidation : RuntimeBatchWitnessOpeningRowsValidation system
  retainedLeafDigestOpeningAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedLeafDigestLevelAvailable : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedLeafDigestPathBound : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedLeafDigestRootMatchesExpectedRoot : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedLeafDigestRowsFromSource : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedLeafDigestRowsBoundToQueryPlan : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedLeafDigestOpeningAcceptedImpliesBatchRowsAccepted :
    forall artifact publicInput proof,
      retainedLeafDigestOpeningAccepted artifact publicInput proof ->
        batchRowsValidation.batchWitnessOpeningRowsAccepted artifact publicInput proof
  retainedLeafDigestOpeningAcceptedImpliesLevelAvailable :
    forall artifact publicInput proof,
      retainedLeafDigestOpeningAccepted artifact publicInput proof ->
        retainedLeafDigestLevelAvailable artifact publicInput proof
  retainedLeafDigestOpeningAcceptedImpliesPathBound :
    forall artifact publicInput proof,
      retainedLeafDigestOpeningAccepted artifact publicInput proof ->
        retainedLeafDigestPathBound artifact publicInput proof
  retainedLeafDigestOpeningAcceptedImpliesRootMatchesExpectedRoot :
    forall artifact publicInput proof,
      retainedLeafDigestOpeningAccepted artifact publicInput proof ->
        retainedLeafDigestRootMatchesExpectedRoot artifact publicInput proof
  retainedLeafDigestOpeningAcceptedImpliesRowsFromSource :
    forall artifact publicInput proof,
      retainedLeafDigestOpeningAccepted artifact publicInput proof ->
        retainedLeafDigestRowsFromSource artifact publicInput proof
  retainedLeafDigestOpeningAcceptedImpliesRowsBoundToQueryPlan :
    forall artifact publicInput proof,
      retainedLeafDigestOpeningAccepted artifact publicInput proof ->
        retainedLeafDigestRowsBoundToQueryPlan artifact publicInput proof
  retainedLeafDigestChecksImplyPerRowWitnessOpeningRowsBound :
    forall artifact publicInput proof,
      batchRowsValidation.openingSegmentValidation.queryPlanBound artifact publicInput proof ->
        retainedLeafDigestRowsBoundToQueryPlan artifact publicInput proof ->
          retainedLeafDigestRowsFromSource artifact publicInput proof ->
            retainedLeafDigestPathBound artifact publicInput proof ->
              retainedLeafDigestRootMatchesExpectedRoot artifact publicInput proof ->
                batchRowsValidation.perRowWitnessOpeningRowsBound artifact publicInput proof

def RuntimeRetainedLeafDigestOpeningCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeRetainedLeafDigestOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.retainedLeafDigestOpeningAccepted artifact publicInput proof

def RuntimeRetainedLeafDigestOpeningDigestContract
    (_system : VerifierModel)
    (validation : RuntimeRetainedLeafDigestOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.retainedLeafDigestLevelAvailable artifact publicInput proof
    /\ validation.retainedLeafDigestPathBound artifact publicInput proof
    /\ validation.retainedLeafDigestRootMatchesExpectedRoot artifact publicInput proof
    /\ validation.retainedLeafDigestRowsFromSource artifact publicInput proof
    /\ validation.retainedLeafDigestRowsBoundToQueryPlan artifact publicInput proof

def RuntimeRetainedLeafDigestOpeningRetainedRowsContract
    (_system : VerifierModel)
    (validation : RuntimeRetainedLeafDigestOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  let openingValidation :=
    validation.batchRowsValidation.openingSegmentValidation.openingValidation
  validation.retainedLeafDigestRowsBoundToQueryPlan artifact publicInput proof
    /\ validation.retainedLeafDigestRowsFromSource artifact publicInput proof
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

def RuntimeRetainedLeafDigestOpeningEvidence
    (system : VerifierModel)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
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
    /\ RuntimeRetainedLeafDigestOpeningDigestContract
      system
      validation
      artifact
      publicInput
      proof
    /\ RuntimeRetainedLeafDigestOpeningRetainedRowsContract
      system
      validation
      artifact
      publicInput
      proof

theorem runtime_retained_leaf_digest_opening_checked_acceptance_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedLeafDigestOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have batchAccepted :=
    validation.retainedLeafDigestOpeningAcceptedImpliesBatchRowsAccepted
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
    validation.retainedLeafDigestOpeningAcceptedImpliesLevelAvailable
      artifact
      publicInput
      proof
      accepted
  have pathBound :=
    validation.retainedLeafDigestOpeningAcceptedImpliesPathBound
      artifact
      publicInput
      proof
      accepted
  have rootMatches :=
    validation.retainedLeafDigestOpeningAcceptedImpliesRootMatchesExpectedRoot
      artifact
      publicInput
      proof
      accepted
  have rowsFromSource :=
    validation.retainedLeafDigestOpeningAcceptedImpliesRowsFromSource
      artifact
      publicInput
      proof
      accepted
  have rowsBoundToQueryPlan :=
    validation.retainedLeafDigestOpeningAcceptedImpliesRowsBoundToQueryPlan
      artifact
      publicInput
      proof
      accepted
  have retainedPerRow :=
    validation.retainedLeafDigestChecksImplyPerRowWitnessOpeningRowsBound
      artifact
      publicInput
      proof
      queryPlanBound
      rowsBoundToQueryPlan
      rowsFromSource
      pathBound
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
          (And.intro pathBound
            (And.intro rootMatches
              (And.intro rowsFromSource rowsBoundToQueryPlan))))
        (And.intro rowsBoundToQueryPlan
          (And.intro rowsFromSource
            (And.intro retainedPerRow
              (And.intro witnessSegments witnessOpeningsBound)))))

theorem runtime_retained_leaf_digest_opening_evidence_implies_digest_contract
    {system : VerifierModel}
    (validation : RuntimeRetainedLeafDigestOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedLeafDigestOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeRetainedLeafDigestOpeningDigestContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.right.left

theorem runtime_retained_leaf_digest_opening_evidence_implies_retained_rows_contract
    {system : VerifierModel}
    (validation : RuntimeRetainedLeafDigestOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedLeafDigestOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeRetainedLeafDigestOpeningRetainedRowsContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.right.right

theorem runtime_retained_leaf_digest_opening_checked_acceptance_retained_rows_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedLeafDigestOpeningRetainedRowsContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_retained_leaf_digest_opening_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  exact
    runtime_retained_leaf_digest_opening_evidence_implies_retained_rows_contract
      validation
      artifact
      publicInput
      proof
      False
      evidence

theorem runtime_retained_leaf_digest_opening_checked_acceptance_digest_contract
    {system : VerifierModel}
    (validation : RuntimeRetainedLeafDigestOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedLeafDigestOpeningDigestContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have levelAvailable :=
    validation.retainedLeafDigestOpeningAcceptedImpliesLevelAvailable
      artifact
      publicInput
      proof
      accepted
  have pathBound :=
    validation.retainedLeafDigestOpeningAcceptedImpliesPathBound
      artifact
      publicInput
      proof
      accepted
  have rootMatches :=
    validation.retainedLeafDigestOpeningAcceptedImpliesRootMatchesExpectedRoot
      artifact
      publicInput
      proof
      accepted
  have rowsFromSource :=
    validation.retainedLeafDigestOpeningAcceptedImpliesRowsFromSource
      artifact
      publicInput
      proof
      accepted
  have rowsBoundToQueryPlan :=
    validation.retainedLeafDigestOpeningAcceptedImpliesRowsBoundToQueryPlan
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro levelAvailable
      (And.intro pathBound
        (And.intro rootMatches
          (And.intro rowsFromSource rowsBoundToQueryPlan)))

theorem runtime_retained_leaf_digest_opening_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedLeafDigestOpeningEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have evidence :=
    runtime_retained_leaf_digest_opening_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have batchAccepted :=
    validation.retainedLeafDigestOpeningAcceptedImpliesBatchRowsAccepted
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

end Lzvm
