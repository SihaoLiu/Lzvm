/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.BatchOpeningBinding
import Lzvm.MerklePathSoundness

/-!
Runtime retained leaf digest opening obligations.
-/

namespace Lzvm

universe uDigest

structure RuntimeRetainedLeafDigestOpeningValidation (system : VerifierModel) where
  batchRowsValidation : RuntimeBatchWitnessOpeningRowsValidation system
  retainedLeafDigestOpeningAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedLeafDigestLevelAvailable : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedLeafDigestPathBound : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedLeafDigestRootMatchesExpectedRoot : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedLeafDigestRowsFromSource : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedLeafDigestRowsBoundToQueryPlan : RuntimeArtifact -> PublicInput -> Proof -> Prop
  retainedLeafDigestShiftedRowWeightCacheUsed : RuntimeArtifact -> PublicInput -> Proof -> Prop
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
  retainedLeafDigestOpeningAcceptedImpliesShiftedRowWeightCacheUsed :
    forall artifact publicInput proof,
      retainedLeafDigestOpeningAccepted artifact publicInput proof ->
        retainedLeafDigestShiftedRowWeightCacheUsed artifact publicInput proof
  retainedLeafDigestShiftedRowWeightCacheImpliesRowsFromSource :
    forall artifact publicInput proof,
      retainedLeafDigestShiftedRowWeightCacheUsed artifact publicInput proof ->
        retainedLeafDigestRowsFromSource artifact publicInput proof
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

structure RuntimeRetainedLeafDigestConcretePathBinding
    (system : VerifierModel)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    (Digest : Type uDigest)
    (compress : Digest -> Digest -> Digest) where
  root : RuntimeArtifact -> PublicInput -> Proof -> Digest
  leaf : RuntimeArtifact -> PublicInput -> Proof -> Digest
  path :
    RuntimeArtifact ->
      PublicInput ->
        Proof ->
          List (MerklePathLayer Digest)
  concretePathVerifies :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        MerklePathVerifies
          compress
          (root artifact publicInput proof)
          (leaf artifact publicInput proof)
          (path artifact publicInput proof)
  retainedLeafDigestPathRootCommitsToLeafImpliesPathBound :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        MerklePathRootCommitsToLeafAtIndex
          compress
          (root artifact publicInput proof)
          (leaf artifact publicInput proof)
          (path artifact publicInput proof) ->
            validation.retainedLeafDigestPathBound artifact publicInput proof

structure RuntimeRetainedLeafDigestNAryConcretePathBinding
    (system : VerifierModel)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    (Digest : Type uDigest)
    (compress : List Digest -> Digest) where
  root : RuntimeArtifact -> PublicInput -> Proof -> Digest
  leaf : RuntimeArtifact -> PublicInput -> Proof -> Digest
  path :
    RuntimeArtifact ->
      PublicInput ->
        Proof ->
          List (NAryMerklePathLayer Digest)
  concretePathVerifies :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        NAryMerklePathVerifies
          compress
          (root artifact publicInput proof)
          (leaf artifact publicInput proof)
          (path artifact publicInput proof)
  retainedLeafDigestPathRootCommitsToLeafImpliesPathBound :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        NAryMerklePathRootCommitsToLeafAtIndex
          compress
          (root artifact publicInput proof)
          (leaf artifact publicInput proof)
          (path artifact publicInput proof) ->
            validation.retainedLeafDigestPathBound artifact publicInput proof

structure RuntimeRetainedLeafDigestNAryConcreteOpeningBinding
    (system : VerifierModel)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    (Digest : Type uDigest)
    (compress : List Digest -> Digest) where
  root : RuntimeArtifact -> PublicInput -> Proof -> Digest
  opening :
    RuntimeArtifact ->
      PublicInput ->
        Proof ->
          NAryMerklePathOpening Digest
  concreteOpeningVerifies :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        NAryMerklePathOpeningVerifies
          compress
          (root artifact publicInput proof)
          (opening artifact publicInput proof)
  retainedLeafDigestPathRootCommitsToLeafImpliesPathBound :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        NAryMerklePathRootCommitsToLeafAtIndex
          compress
          (root artifact publicInput proof)
          ((opening artifact publicInput proof).leaf)
          ((opening artifact publicInput proof).layers) ->
            validation.retainedLeafDigestPathBound artifact publicInput proof

theorem runtime_retained_leaf_digest_concrete_path_bound_from_no_collision
    {system : VerifierModel}
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (binding :
      RuntimeRetainedLeafDigestConcretePathBinding
        system
        validation
        Digest
        compress)
    (noCollision : MerkleCompressionNoCollision compress) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedLeafDigestPathBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have verified :=
    binding.concretePathVerifies artifact publicInput proof accepted
  have rootCommitsToLeaf :=
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision
      noCollision
      (binding.root artifact publicInput proof)
      (binding.leaf artifact publicInput proof)
      (binding.path artifact publicInput proof)
      verified
  exact
    binding.retainedLeafDigestPathRootCommitsToLeafImpliesPathBound
      artifact
      publicInput
      proof
      accepted
      rootCommitsToLeaf

theorem runtime_retained_leaf_digest_concrete_path_bound_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestConcretePathBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedLeafDigestPathBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_retained_leaf_digest_concrete_path_bound_from_no_collision
      validation
      binding
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      artifact
      publicInput
      proof
      accepted

theorem runtime_retained_leaf_digest_concrete_path_position_bound_from_no_collision
    {system : VerifierModel}
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (binding :
      RuntimeRetainedLeafDigestConcretePathBinding
        system
        validation
        Digest
        compress)
    (noCollision : MerkleCompressionNoCollision compress) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedLeafDigestPathBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have verified :=
    binding.concretePathVerifies artifact publicInput proof accepted
  have rootCommitsToLeafAtPosition :=
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision
      noCollision
      (binding.root artifact publicInput proof)
      (binding.leaf artifact publicInput proof)
      (binding.path artifact publicInput proof)
      verified
  have rootCommitsToLeafAtIndex :
      MerklePathRootCommitsToLeafAtIndex
        compress
        (binding.root artifact publicInput proof)
        (binding.leaf artifact publicInput proof)
        (binding.path artifact publicInput proof) := by
    intro otherLeaf otherPath sameIndex otherVerified
    have samePosition :=
      merkle_path_same_index_implies_index_depth_eq
        (binding.path artifact publicInput proof)
        otherPath
        sameIndex
    exact
      rootCommitsToLeafAtPosition
        otherLeaf
        otherPath
        samePosition.left
        samePosition.right
        otherVerified
  exact
    binding.retainedLeafDigestPathRootCommitsToLeafImpliesPathBound
      artifact
      publicInput
      proof
      accepted
      rootCommitsToLeafAtIndex

theorem runtime_retained_leaf_digest_concrete_path_position_bound_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestConcretePathBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedLeafDigestPathBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_retained_leaf_digest_concrete_path_position_bound_from_no_collision
      validation
      binding
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      artifact
      publicInput
      proof
      accepted

theorem runtime_retained_leaf_digest_nary_path_position_bound_from_no_collision
    {system : VerifierModel}
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (binding :
      RuntimeRetainedLeafDigestNAryConcretePathBinding
        system
        validation
        Digest
        compress)
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedLeafDigestPathBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have verified :=
    binding.concretePathVerifies artifact publicInput proof accepted
  have rootCommitsToLeaf :=
    verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision
      noCollision
      (binding.root artifact publicInput proof)
      (binding.leaf artifact publicInput proof)
      (binding.path artifact publicInput proof)
      verified
  exact
    binding.retainedLeafDigestPathRootCommitsToLeafImpliesPathBound
      artifact
      publicInput
      proof
      accepted
      rootCommitsToLeaf

theorem runtime_retained_leaf_digest_nary_path_position_bound_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestNAryConcretePathBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedLeafDigestPathBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_retained_leaf_digest_nary_path_position_bound_from_no_collision
      validation
      binding
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      artifact
      publicInput
      proof
      accepted

theorem runtime_retained_leaf_digest_nary_opening_position_bound_from_no_collision
    {system : VerifierModel}
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (binding :
      RuntimeRetainedLeafDigestNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress)
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedLeafDigestPathBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have verified :=
    binding.concreteOpeningVerifies artifact publicInput proof accepted
  have rootCommitsToLeaf :=
    verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_no_collision
      noCollision
      (binding.root artifact publicInput proof)
      (binding.opening artifact publicInput proof)
      verified
  exact
    binding.retainedLeafDigestPathRootCommitsToLeafImpliesPathBound
      artifact
      publicInput
      proof
      accepted
      rootCommitsToLeaf

theorem runtime_retained_leaf_digest_nary_opening_position_bound_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedLeafDigestPathBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_retained_leaf_digest_nary_opening_position_bound_from_no_collision
      validation
      binding
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      artifact
      publicInput
      proof
      accepted

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

def RuntimeRetainedLeafDigestOpeningShiftedRowSourceContract
    (_system : VerifierModel)
    (validation : RuntimeRetainedLeafDigestOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.retainedLeafDigestShiftedRowWeightCacheUsed artifact publicInput proof
    /\ validation.retainedLeafDigestRowsFromSource artifact publicInput proof
    /\ validation.retainedLeafDigestRowsBoundToQueryPlan artifact publicInput proof

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

theorem runtime_retained_leaf_digest_shifted_row_weight_cache_implies_source_rows
    {system : VerifierModel}
    (validation : RuntimeRetainedLeafDigestOpeningValidation system) :
    forall artifact publicInput proof,
      validation.retainedLeafDigestShiftedRowWeightCacheUsed artifact publicInput proof ->
        validation.retainedLeafDigestRowsFromSource artifact publicInput proof := by
  intro artifact publicInput proof shiftedRowWeightCacheUsed
  exact
    validation.retainedLeafDigestShiftedRowWeightCacheImpliesRowsFromSource
      artifact
      publicInput
      proof
      shiftedRowWeightCacheUsed

theorem runtime_retained_leaf_digest_opening_checked_acceptance_shifted_row_source_contract
    {system : VerifierModel}
    (validation : RuntimeRetainedLeafDigestOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedLeafDigestOpeningShiftedRowSourceContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have shiftedRowWeightCacheUsed :=
    validation.retainedLeafDigestOpeningAcceptedImpliesShiftedRowWeightCacheUsed
      artifact
      publicInput
      proof
      accepted
  have rowsFromSource :=
    runtime_retained_leaf_digest_shifted_row_weight_cache_implies_source_rows
      validation
      artifact
      publicInput
      proof
      shiftedRowWeightCacheUsed
  have rowsBoundToQueryPlan :=
    validation.retainedLeafDigestOpeningAcceptedImpliesRowsBoundToQueryPlan
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro shiftedRowWeightCacheUsed
      (And.intro rowsFromSource rowsBoundToQueryPlan)

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

theorem runtime_retained_leaf_digest_opening_evidence_implies_batch_rows_evidence
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
        RuntimeBatchWitnessOpeningRowsEvidence
          system
          validation.batchRowsValidation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.left

theorem runtime_retained_leaf_digest_opening_checked_acceptance_batch_rows_evidence
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
        RuntimeBatchWitnessOpeningRowsEvidence
          system
          validation.batchRowsValidation
          artifact
          publicInput
          proof
          requiresExternalSource := by
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
  exact
    runtime_retained_leaf_digest_opening_evidence_implies_batch_rows_evidence
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      evidence

theorem runtime_retained_leaf_digest_opening_evidence_implies_batch_rows_bound_contract
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
        RuntimeBatchWitnessOpeningRowsBoundContract
          system
          validation.batchRowsValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  have batchEvidence :=
    runtime_retained_leaf_digest_opening_evidence_implies_batch_rows_evidence
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

theorem runtime_retained_leaf_digest_opening_checked_acceptance_batch_rows_bound_contract
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
        RuntimeBatchWitnessOpeningRowsBoundContract
          system
          validation.batchRowsValidation
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
    runtime_retained_leaf_digest_opening_evidence_implies_batch_rows_bound_contract
      validation
      artifact
      publicInput
      proof
      False
      evidence

theorem runtime_retained_leaf_digest_opening_evidence_implies_opening_evidence
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
        RuntimeOpeningEvidence
          system
          validation.batchRowsValidation.openingSegmentValidation.openingValidation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource evidence
  have batchEvidence :=
    runtime_retained_leaf_digest_opening_evidence_implies_batch_rows_evidence
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

theorem runtime_retained_leaf_digest_opening_checked_acceptance_opening_evidence
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
        RuntimeOpeningEvidence
          system
          validation.batchRowsValidation.openingSegmentValidation.openingValidation
          artifact
          publicInput
          proof
          requiresExternalSource := by
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
  exact
    runtime_retained_leaf_digest_opening_evidence_implies_opening_evidence
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      evidence

theorem runtime_retained_leaf_digest_opening_checked_acceptance_batch_path_and_opening_evidence
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
        RuntimeBatchWitnessOpeningRowsEvidence
            system
            validation.batchRowsValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ validation.retainedLeafDigestPathBound artifact publicInput proof
          /\ validation.retainedLeafDigestRootMatchesExpectedRoot artifact publicInput proof
          /\ RuntimeOpeningEvidence
            system
            validation.batchRowsValidation.openingSegmentValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have batchEvidence :=
    runtime_retained_leaf_digest_opening_checked_acceptance_batch_rows_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
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
  have openingEvidence :=
    runtime_retained_leaf_digest_opening_checked_acceptance_opening_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    And.intro batchEvidence
      (And.intro pathBound
        (And.intro rootMatches openingEvidence))

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

theorem runtime_retained_leaf_digest_concrete_path_digest_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestConcretePathBinding
        system
        validation
        Digest
        compress) :
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
    runtime_retained_leaf_digest_concrete_path_position_bound_from_bundle
      assumptions
      validation
      centralized
      binding
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

theorem runtime_retained_leaf_digest_nary_path_digest_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestNAryConcretePathBinding
        system
        validation
        Digest
        compress) :
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
    runtime_retained_leaf_digest_nary_path_position_bound_from_bundle
      assumptions
      validation
      centralized
      binding
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

theorem runtime_retained_leaf_digest_nary_opening_digest_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
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
    runtime_retained_leaf_digest_nary_opening_position_bound_from_bundle
      assumptions
      validation
      centralized
      binding
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

theorem runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract
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
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_retained_leaf_digest_opening_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  exact sound_witness_implies_verifier_core_contract sound.right

theorem runtime_retained_leaf_digest_opening_checked_acceptance_opening_and_core_contract
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
        RuntimeOpeningEvidence
            system
            validation.batchRowsValidation.openingSegmentValidation.openingValidation
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
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_opening_checked_acceptance_opening_evidence
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (And.intro
        (runtime_retained_leaf_digest_opening_checked_acceptance_digest_contract
          validation
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (runtime_retained_leaf_digest_opening_checked_acceptance_retained_rows_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)
          (runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)))

theorem runtime_retained_leaf_digest_concrete_path_opening_and_core_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestConcretePathBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
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
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_opening_checked_acceptance_opening_evidence
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (And.intro
        (runtime_retained_leaf_digest_concrete_path_digest_contract_from_bundle
          assumptions
          validation
          centralized
          binding
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (runtime_retained_leaf_digest_opening_checked_acceptance_retained_rows_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)
          (runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)))

theorem runtime_retained_leaf_digest_nary_path_opening_and_core_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestNAryConcretePathBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
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
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_opening_checked_acceptance_opening_evidence
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (And.intro
        (runtime_retained_leaf_digest_nary_path_digest_contract_from_bundle
          assumptions
          validation
          centralized
          binding
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (runtime_retained_leaf_digest_opening_checked_acceptance_retained_rows_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)
          (runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)))

theorem runtime_retained_leaf_digest_nary_opening_opening_and_core_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
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
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_opening_checked_acceptance_opening_evidence
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (And.intro
        (runtime_retained_leaf_digest_nary_opening_digest_contract_from_bundle
          assumptions
          validation
          centralized
          binding
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (runtime_retained_leaf_digest_opening_checked_acceptance_retained_rows_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)
          (runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)))

theorem runtime_retained_leaf_digest_nary_opening_source_and_core_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
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
            proof
          /\ RuntimeRetainedLeafDigestOpeningShiftedRowSourceContract
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
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_nary_opening_digest_contract_from_bundle
        assumptions
        validation
        centralized
        binding
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (runtime_retained_leaf_digest_opening_checked_acceptance_shifted_row_source_contract
          validation
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (runtime_retained_leaf_digest_opening_checked_acceptance_retained_rows_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)
          (runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)))

theorem runtime_retained_leaf_digest_opening_checked_acceptance_source_and_core_contract
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
        RuntimeRetainedLeafDigestOpeningShiftedRowSourceContract
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
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_opening_checked_acceptance_shifted_row_source_contract
        validation
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (runtime_retained_leaf_digest_opening_checked_acceptance_retained_rows_contract
          assumptions
          validation
          artifact
          publicInput
          proof
          accepted)
        (runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract
          assumptions
          validation
          artifact
          publicInput
          proof
          accepted))

end Lzvm
