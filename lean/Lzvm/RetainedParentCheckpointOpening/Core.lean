/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.BatchOpeningBinding
import Lzvm.MerklePathSoundness

/-!
Runtime retained parent checkpoint opening obligations.
-/

namespace Lzvm

universe uDigest

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
  retainedParentCheckpointPrefixBatchUsed : RuntimeArtifact -> PublicInput -> Proof -> Prop
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
  retainedParentCheckpointOpeningAcceptedImpliesPrefixBatchUsed :
    forall artifact publicInput proof,
      retainedParentCheckpointOpeningAccepted artifact publicInput proof ->
        retainedParentCheckpointPrefixBatchUsed artifact publicInput proof
  retainedParentCheckpointPrefixBatchImpliesLowerPrefixBound :
    forall artifact publicInput proof,
      retainedParentCheckpointPrefixBatchUsed artifact publicInput proof ->
        retainedParentCheckpointLowerPrefixBound artifact publicInput proof
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

structure RuntimeRetainedParentCheckpointConcretePathBinding
    (system : VerifierModel)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    (Digest : Type uDigest)
    (compress : Digest -> Digest -> Digest) where
  root : RuntimeArtifact -> PublicInput -> Proof -> Digest
  leaf : RuntimeArtifact -> PublicInput -> Proof -> Digest
  stitchedPath :
    RuntimeArtifact ->
      PublicInput ->
        Proof ->
          List (MerklePathLayer Digest)
  concreteStitchedPathVerifies :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        MerklePathVerifies
          compress
          (root artifact publicInput proof)
          (leaf artifact publicInput proof)
          (stitchedPath artifact publicInput proof)
  stitchedPathRootCommitsToLeafImpliesStitchedPathBound :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        MerklePathRootCommitsToLeafAtIndex
          compress
          (root artifact publicInput proof)
          (leaf artifact publicInput proof)
          (stitchedPath artifact publicInput proof) ->
            validation.retainedParentCheckpointStitchedPathBound
              artifact
              publicInput
              proof

structure RuntimeRetainedParentCheckpointNAryConcretePathBinding
    (system : VerifierModel)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    (Digest : Type uDigest)
    (compress : List Digest -> Digest) where
  root : RuntimeArtifact -> PublicInput -> Proof -> Digest
  leaf : RuntimeArtifact -> PublicInput -> Proof -> Digest
  stitchedPath :
    RuntimeArtifact ->
      PublicInput ->
        Proof ->
          List (NAryMerklePathLayer Digest)
  concreteStitchedPathVerifies :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        NAryMerklePathVerifies
          compress
          (root artifact publicInput proof)
          (leaf artifact publicInput proof)
          (stitchedPath artifact publicInput proof)
  stitchedPathRootCommitsToLeafImpliesStitchedPathBound :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        NAryMerklePathRootCommitsToLeafAtIndex
          compress
          (root artifact publicInput proof)
          (leaf artifact publicInput proof)
          (stitchedPath artifact publicInput proof) ->
            validation.retainedParentCheckpointStitchedPathBound
              artifact
              publicInput
              proof

structure RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding
    (system : VerifierModel)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
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
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        NAryMerklePathOpeningVerifies
          compress
          (root artifact publicInput proof)
          (opening artifact publicInput proof)
  stitchedPathRootCommitsToLeafImpliesStitchedPathBound :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
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
            validation.retainedParentCheckpointStitchedPathBound
              artifact
              publicInput
              proof

theorem runtime_retained_parent_checkpoint_concrete_path_bound_from_no_collision
    {system : VerifierModel}
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (binding :
      RuntimeRetainedParentCheckpointConcretePathBinding
        system
        validation
        Digest
        compress)
    (noCollision : MerkleCompressionNoCollision compress) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedParentCheckpointStitchedPathBound
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have verified :=
    binding.concreteStitchedPathVerifies artifact publicInput proof accepted
  have rootCommitsToLeaf :=
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision
      noCollision
      (binding.root artifact publicInput proof)
      (binding.leaf artifact publicInput proof)
      (binding.stitchedPath artifact publicInput proof)
      verified
  exact
    binding.stitchedPathRootCommitsToLeafImpliesStitchedPathBound
      artifact
      publicInput
      proof
      accepted
      rootCommitsToLeaf

theorem runtime_retained_parent_checkpoint_concrete_path_bound_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedParentCheckpointConcretePathBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedParentCheckpointStitchedPathBound
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_retained_parent_checkpoint_concrete_path_bound_from_no_collision
      validation
      binding
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      artifact
      publicInput
      proof
      accepted

theorem runtime_retained_parent_checkpoint_concrete_path_position_bound_from_no_collision
    {system : VerifierModel}
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (binding :
      RuntimeRetainedParentCheckpointConcretePathBinding
        system
        validation
        Digest
        compress)
    (noCollision : MerkleCompressionNoCollision compress) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedParentCheckpointStitchedPathBound
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have verified :=
    binding.concreteStitchedPathVerifies artifact publicInput proof accepted
  have rootCommitsToLeafAtPosition :=
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision
      noCollision
      (binding.root artifact publicInput proof)
      (binding.leaf artifact publicInput proof)
      (binding.stitchedPath artifact publicInput proof)
      verified
  have rootCommitsToLeafAtIndex :
      MerklePathRootCommitsToLeafAtIndex
        compress
        (binding.root artifact publicInput proof)
        (binding.leaf artifact publicInput proof)
        (binding.stitchedPath artifact publicInput proof) := by
    intro otherLeaf otherPath sameIndex otherVerified
    have samePosition :=
      merkle_path_same_index_implies_index_depth_eq
        (binding.stitchedPath artifact publicInput proof)
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
    binding.stitchedPathRootCommitsToLeafImpliesStitchedPathBound
      artifact
      publicInput
      proof
      accepted
      rootCommitsToLeafAtIndex

theorem runtime_retained_parent_checkpoint_concrete_path_position_bound_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedParentCheckpointConcretePathBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedParentCheckpointStitchedPathBound
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_retained_parent_checkpoint_concrete_path_position_bound_from_no_collision
      validation
      binding
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      artifact
      publicInput
      proof
      accepted

theorem runtime_retained_parent_checkpoint_nary_path_position_bound_from_no_collision
    {system : VerifierModel}
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (binding :
      RuntimeRetainedParentCheckpointNAryConcretePathBinding
        system
        validation
        Digest
        compress)
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedParentCheckpointStitchedPathBound
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have verified :=
    binding.concreteStitchedPathVerifies artifact publicInput proof accepted
  have rootCommitsToLeaf :=
    verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision
      noCollision
      (binding.root artifact publicInput proof)
      (binding.leaf artifact publicInput proof)
      (binding.stitchedPath artifact publicInput proof)
      verified
  exact
    binding.stitchedPathRootCommitsToLeafImpliesStitchedPathBound
      artifact
      publicInput
      proof
      accepted
      rootCommitsToLeaf

theorem runtime_retained_parent_checkpoint_nary_path_position_bound_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedParentCheckpointNAryConcretePathBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedParentCheckpointStitchedPathBound
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_retained_parent_checkpoint_nary_path_position_bound_from_no_collision
      validation
      binding
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      artifact
      publicInput
      proof
      accepted

theorem
  runtime_retained_parent_checkpoint_nary_opening_position_bound_from_no_collision
    {system : VerifierModel}
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (binding :
      RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress)
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedParentCheckpointStitchedPathBound
          artifact
          publicInput
          proof := by
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
    binding.stitchedPathRootCommitsToLeafImpliesStitchedPathBound
      artifact
      publicInput
      proof
      accepted
      rootCommitsToLeaf

theorem runtime_retained_parent_checkpoint_nary_opening_position_bound_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedParentCheckpointStitchedPathBound
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_retained_parent_checkpoint_nary_opening_position_bound_from_no_collision
      validation
      binding
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      artifact
      publicInput
      proof
      accepted

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

def RuntimeRetainedParentCheckpointOpeningPrefixBatchContract
    (_system : VerifierModel)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.retainedParentCheckpointPrefixBatchUsed artifact publicInput proof
    /\ validation.retainedParentCheckpointLowerPrefixBound artifact publicInput proof
    /\ validation.retainedParentCheckpointRowsBoundToQueryPlan artifact publicInput proof

def RuntimeRetainedParentCheckpointOpeningSourceContract
    (_system : VerifierModel)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.retainedParentCheckpointPrefixBatchUsed artifact publicInput proof
    /\ validation.retainedParentCheckpointLowerPrefixBound artifact publicInput proof
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

theorem runtime_retained_parent_checkpoint_prefix_batch_implies_lower_prefix_bound
    {system : VerifierModel}
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof,
      validation.retainedParentCheckpointPrefixBatchUsed artifact publicInput proof ->
        validation.retainedParentCheckpointLowerPrefixBound artifact publicInput proof := by
  intro artifact publicInput proof prefixBatchUsed
  exact
    validation.retainedParentCheckpointPrefixBatchImpliesLowerPrefixBound
      artifact
      publicInput
      proof
      prefixBatchUsed

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_prefix_batch_contract
    {system : VerifierModel}
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningPrefixBatchContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have prefixBatchUsed :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesPrefixBatchUsed
      artifact
      publicInput
      proof
      accepted
  have lowerPrefix :=
    runtime_retained_parent_checkpoint_prefix_batch_implies_lower_prefix_bound
      validation
      artifact
      publicInput
      proof
      prefixBatchUsed
  have rowsBoundToQueryPlan :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesRowsBoundToQueryPlan
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro prefixBatchUsed
      (And.intro lowerPrefix rowsBoundToQueryPlan)

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_source_contract
    {system : VerifierModel}
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningSourceContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have prefixBatchUsed :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesPrefixBatchUsed
      artifact
      publicInput
      proof
      accepted
  have lowerPrefix :=
    runtime_retained_parent_checkpoint_prefix_batch_implies_lower_prefix_bound
      validation
      artifact
      publicInput
      proof
      prefixBatchUsed
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
    And.intro prefixBatchUsed
      (And.intro lowerPrefix
        (And.intro rowsFromSource rowsBoundToQueryPlan))

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

theorem runtime_retained_parent_checkpoint_concrete_path_digest_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedParentCheckpointConcretePathBinding
        system
        validation
        Digest
        compress) :
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
    runtime_retained_parent_checkpoint_concrete_path_position_bound_from_bundle
      assumptions
      validation
      centralized
      binding
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

theorem runtime_retained_parent_checkpoint_nary_path_digest_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedParentCheckpointNAryConcretePathBinding
        system
        validation
        Digest
        compress) :
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
    runtime_retained_parent_checkpoint_nary_path_position_bound_from_bundle
      assumptions
      validation
      centralized
      binding
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

theorem runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
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
    runtime_retained_parent_checkpoint_nary_opening_position_bound_from_bundle
      assumptions
      validation
      centralized
      binding
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

end Lzvm
