/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.RetainedParentCheckpointOpening.Core

/-!
Fixed-arity retained parent checkpoint opening wrappers.
-/

namespace Lzvm

universe uDigest

set_option linter.style.longLine false in
theorem runtime_retained_parent_checkpoint_nary_path_arity_four_position_bound_from_no_collision
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
    (pathArity :
      forall artifact publicInput proof,
        RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
            system
            validation
            artifact
            publicInput
            proof ->
          NAryMerklePathHasArity 4 (binding.stitchedPath artifact publicInput proof))
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
  have arityProof := pathArity artifact publicInput proof accepted
  have rootCommitsAtArity :=
    verified_concrete_nary_merkle_path_arity_four_implies_root_commits_to_leaf_at_index_from_no_collision
      noCollision
      (binding.root artifact publicInput proof)
      (binding.leaf artifact publicInput proof)
      (binding.stitchedPath artifact publicInput proof)
      arityProof
      verified
  have rootCommitsToLeaf :
      NAryMerklePathRootCommitsToLeafAtIndex
        compress
        (binding.root artifact publicInput proof)
        (binding.leaf artifact publicInput proof)
        (binding.stitchedPath artifact publicInput proof) := by
    intro otherLeaf otherPath samePosition sameIndex sameDepth otherVerified
    have otherArity :
        NAryMerklePathHasArity 4 otherPath :=
      nary_merkle_path_same_position_preserves_arity
        4
        (binding.stitchedPath artifact publicInput proof)
        otherPath
        arityProof
        samePosition
    exact
      rootCommitsAtArity.right
        otherLeaf
        otherPath
        otherArity
        sameIndex
        sameDepth
        otherVerified
  exact
    binding.stitchedPathRootCommitsToLeafImpliesStitchedPathBound
      artifact
      publicInput
      proof
      accepted
      rootCommitsToLeaf

theorem runtime_retained_parent_checkpoint_nary_path_arity_four_position_bound_from_bundle
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
        compress)
    (pathArity :
      forall artifact publicInput proof,
        RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
            system
            validation
            artifact
            publicInput
            proof ->
          NAryMerklePathHasArity 4 (binding.stitchedPath artifact publicInput proof)) :
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
    runtime_retained_parent_checkpoint_nary_path_arity_four_position_bound_from_no_collision
      validation
      binding
      pathArity
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      artifact
      publicInput
      proof
      accepted

set_option linter.style.longLine false in
theorem runtime_retained_parent_checkpoint_nary_opening_arity_four_position_bound_from_no_collision
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
    (openingArity :
      forall artifact publicInput proof,
        RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
            system
            validation
            artifact
            publicInput
            proof ->
          NAryMerklePathHasArity 4
            ((binding.opening artifact publicInput proof).layers))
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
  have arityProof := openingArity artifact publicInput proof accepted
  have rootCommitsAtArity :=
    verified_concrete_nary_merkle_opening_arity_four_implies_root_commits_to_leaf_at_index_from_no_collision
      noCollision
      (binding.root artifact publicInput proof)
      (binding.opening artifact publicInput proof)
      arityProof
      verified
  have rootCommitsToLeaf :
      NAryMerklePathRootCommitsToLeafAtIndex
        compress
        (binding.root artifact publicInput proof)
        ((binding.opening artifact publicInput proof).leaf)
        ((binding.opening artifact publicInput proof).layers) := by
    intro otherLeaf otherPath samePosition sameIndex sameDepth otherVerified
    have otherArity :
        NAryMerklePathHasArity 4 otherPath :=
      nary_merkle_path_same_position_preserves_arity
        4
        ((binding.opening artifact publicInput proof).layers)
        otherPath
        arityProof
        samePosition
    exact
      rootCommitsAtArity.right
        otherLeaf
        otherPath
        otherArity
        sameIndex
        sameDepth
        otherVerified
  exact
    binding.stitchedPathRootCommitsToLeafImpliesStitchedPathBound
      artifact
      publicInput
      proof
      accepted
      rootCommitsToLeaf

theorem runtime_retained_parent_checkpoint_nary_opening_arity_four_position_bound_from_bundle
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
        compress)
    (openingArity :
      forall artifact publicInput proof,
        RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
            system
            validation
            artifact
            publicInput
            proof ->
          NAryMerklePathHasArity 4
            ((binding.opening artifact publicInput proof).layers)) :
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
    runtime_retained_parent_checkpoint_nary_opening_arity_four_position_bound_from_no_collision
      validation
      binding
      openingArity
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      artifact
      publicInput
      proof
      accepted

end Lzvm
