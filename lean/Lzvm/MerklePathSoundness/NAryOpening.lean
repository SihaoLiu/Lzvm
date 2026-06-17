/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.MerklePathSoundness.NAry

/-!
N-ary Merkle opening soundness wrappers derived from concrete path folding.
-/

namespace Lzvm

universe uDigest

theorem
  nary_merkle_opening_root_commits_to_leaf_at_same_position_index_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root opening,
      NAryMerklePathOpeningVerifies compress root opening ->
        NAryMerklePathRootCommitsToLeafAtSamePositionIndex
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    nary_merkle_path_root_commits_to_leaf_at_same_position_index_from_no_collision
      noCollision
      root
      opening.leaf
      opening.layers
      verified

theorem
  verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root opening,
      NAryMerklePathOpeningVerifies compress root opening ->
        NAryMerklePathRootCommitsToLeafAtIndex
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    nary_merkle_opening_root_commits_to_leaf_at_same_position_index_from_no_collision
      noCollision
      root
      opening
      verified

theorem verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root opening,
      NAryMerklePathOpeningVerifies compress root opening ->
        NAryMerklePathRootCommitsToLeafAtIndex
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      opening
      verified

theorem
  nary_merkle_opening_root_commits_to_leaf_at_same_position_index_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening,
      NAryMerklePathOpeningVerifies compress root opening ->
        NAryMerklePathRootCommitsToLeafAtSamePositionIndex
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    nary_merkle_opening_root_commits_to_leaf_at_same_position_index_from_no_collision
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      root
      opening
      verified

theorem verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening,
      NAryMerklePathOpeningVerifies compress root opening ->
        NAryMerklePathRootCommitsToLeafAtIndex
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      opening
      verified

theorem
  verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_position_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root opening,
      NAryMerklePathOpeningVerifies compress root opening ->
        NAryMerklePathRootCommitsToLeafAtPosition
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision
      noCollision
      root
      opening.leaf
      opening.layers
      verified

theorem
  verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_position_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root opening,
      NAryMerklePathOpeningVerifies compress root opening ->
        NAryMerklePathRootCommitsToLeafAtPosition
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_position_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      opening
      verified

theorem verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_position_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening,
      NAryMerklePathOpeningVerifies compress root opening ->
        NAryMerklePathRootCommitsToLeafAtPosition
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_position_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      opening
      verified

set_option linter.style.longLine false in
theorem
  verified_concrete_nary_merkle_path_arity_two_implies_root_commits_to_leaf_at_index_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root leaf path,
      NAryMerklePathHasArity 2 path ->
        NAryMerklePathVerifies compress root leaf path ->
          NAryMerklePathRootCommitsToLeafAtArityIndex
            compress
            2
            root
            leaf
            path := by
  intro root leaf path pathArity verified
  exact
    And.intro pathArity
      (by
        intro otherLeaf otherPath otherPathArity sameIndex sameDepth otherVerified
        exact
          nary_merkle_path_arity_two_index_binding_from_no_collision
            noCollision
            root
            leaf
            path
            pathArity
            verified
            otherLeaf
            otherPath
            otherPathArity
            sameIndex
            sameDepth
            otherVerified)

set_option linter.style.longLine false in
theorem
  verified_concrete_nary_merkle_path_arity_four_implies_root_commits_to_leaf_at_index_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root leaf path,
      NAryMerklePathHasArity 4 path ->
        NAryMerklePathVerifies compress root leaf path ->
          NAryMerklePathRootCommitsToLeafAtArityIndex
            compress
            4
            root
            leaf
            path := by
  intro root leaf path pathArity verified
  exact
    And.intro pathArity
      (by
        intro otherLeaf otherPath otherPathArity sameIndex sameDepth otherVerified
        exact
          nary_merkle_path_arity_four_index_binding_from_no_collision
            noCollision
            root
            leaf
            path
            pathArity
            verified
            otherLeaf
            otherPath
            otherPathArity
            sameIndex
            sameDepth
            otherVerified)

set_option linter.style.longLine false in
theorem
  verified_concrete_nary_merkle_opening_arity_two_implies_root_commits_to_leaf_at_index_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root opening,
      NAryMerklePathHasArity 2 opening.layers ->
        NAryMerklePathOpeningVerifies compress root opening ->
          NAryMerklePathRootCommitsToLeafAtArityIndex
            compress
            2
            root
            opening.leaf
            opening.layers := by
  intro root opening openingArity verified
  exact
    verified_concrete_nary_merkle_path_arity_two_implies_root_commits_to_leaf_at_index_from_no_collision
      noCollision
      root
      opening.leaf
      opening.layers
      openingArity
      verified

set_option linter.style.longLine false in
theorem
  verified_concrete_nary_merkle_opening_arity_four_implies_root_commits_to_leaf_at_index_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root opening,
      NAryMerklePathHasArity 4 opening.layers ->
        NAryMerklePathOpeningVerifies compress root opening ->
          NAryMerklePathRootCommitsToLeafAtArityIndex
            compress
            4
            root
            opening.leaf
            opening.layers := by
  intro root opening openingArity verified
  exact
    verified_concrete_nary_merkle_path_arity_four_implies_root_commits_to_leaf_at_index_from_no_collision
      noCollision
      root
      opening.leaf
      opening.layers
      openingArity
      verified

set_option linter.style.longLine false in
theorem
  verified_concrete_nary_merkle_opening_arity_two_implies_root_commits_to_leaf_at_index_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening,
      NAryMerklePathHasArity 2 opening.layers ->
        NAryMerklePathOpeningVerifies compress root opening ->
          NAryMerklePathRootCommitsToLeafAtArityIndex
            compress
            2
            root
            opening.leaf
            opening.layers := by
  intro root opening openingArity verified
  exact
    verified_concrete_nary_merkle_opening_arity_two_implies_root_commits_to_leaf_at_index_from_no_collision
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      root
      opening
      openingArity
      verified

set_option linter.style.longLine false in
theorem
  verified_concrete_nary_merkle_opening_arity_four_implies_root_commits_to_leaf_at_index_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening,
      NAryMerklePathHasArity 4 opening.layers ->
        NAryMerklePathOpeningVerifies compress root opening ->
          NAryMerklePathRootCommitsToLeafAtArityIndex
            compress
            4
            root
            opening.leaf
            opening.layers := by
  intro root opening openingArity verified
  exact
    verified_concrete_nary_merkle_opening_arity_four_implies_root_commits_to_leaf_at_index_from_no_collision
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      root
      opening
      openingArity
      verified

theorem
  verified_concrete_nary_merkle_opening_arity_two_same_index_leaf_eq_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
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
    nary_merkle_path_arity_two_index_binding_from_no_collision
      noCollision
      root
      opening.leaf
      opening.layers
      openingArity
      verified
      otherOpening.leaf
      otherOpening.layers
      otherOpeningArity
      sameIndex
      sameDepth
      otherVerified

theorem
  verified_concrete_nary_merkle_opening_arity_two_same_index_leaf_eq_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
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
    verified_concrete_nary_merkle_opening_arity_two_same_index_leaf_eq_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      opening
      otherOpening
      openingArity
      otherOpeningArity
      verified
      otherVerified
      sameIndex
      sameDepth

theorem
  verified_concrete_nary_merkle_opening_arity_two_same_index_leaf_eq_from_bundle
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
    verified_concrete_nary_merkle_opening_arity_two_same_index_leaf_eq_from_assumption
      assumptions.crypto.hashCollisionResistance
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

theorem
  verified_concrete_nary_merkle_opening_arity_four_same_index_leaf_eq_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
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
    nary_merkle_path_arity_four_index_binding_from_no_collision
      noCollision
      root
      opening.leaf
      opening.layers
      openingArity
      verified
      otherOpening.leaf
      otherOpening.layers
      otherOpeningArity
      sameIndex
      sameDepth
      otherVerified

theorem
  verified_concrete_nary_merkle_opening_arity_four_same_index_leaf_eq_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
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
    verified_concrete_nary_merkle_opening_arity_four_same_index_leaf_eq_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      opening
      otherOpening
      openingArity
      otherOpeningArity
      verified
      otherVerified
      sameIndex
      sameDepth

theorem
  verified_concrete_nary_merkle_opening_arity_four_same_index_leaf_eq_from_bundle
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
    verified_concrete_nary_merkle_opening_arity_four_same_index_leaf_eq_from_assumption
      assumptions.crypto.hashCollisionResistance
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

theorem
  nary_merkle_opening_arity_two_index_binding_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root opening,
      NAryMerklePathHasArity 2 opening.layers ->
        NAryMerklePathOpeningVerifies compress root opening ->
          forall otherLeaf otherPath,
            NAryMerklePathHasArity 2 otherPath ->
              NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherPath ->
                opening.layers.length = otherPath.length ->
                  NAryMerklePathVerifies compress root otherLeaf otherPath ->
                    otherLeaf = opening.leaf := by
  intro root opening openingArity verified otherLeaf otherPath otherPathArity
    sameIndex sameDepth otherVerified
  exact
    nary_merkle_path_arity_two_index_binding_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      opening.leaf
      opening.layers
      openingArity
      verified
      otherLeaf
      otherPath
      otherPathArity
      sameIndex
      sameDepth
      otherVerified

theorem
  nary_merkle_opening_arity_two_index_binding_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening,
      NAryMerklePathHasArity 2 opening.layers ->
        NAryMerklePathOpeningVerifies compress root opening ->
          forall otherLeaf otherPath,
            NAryMerklePathHasArity 2 otherPath ->
              NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherPath ->
                opening.layers.length = otherPath.length ->
                  NAryMerklePathVerifies compress root otherLeaf otherPath ->
                    otherLeaf = opening.leaf := by
  intro root opening openingArity verified otherLeaf otherPath otherPathArity
    sameIndex sameDepth otherVerified
  exact
    nary_merkle_opening_arity_two_index_binding_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      opening
      openingArity
      verified
      otherLeaf
      otherPath
      otherPathArity
      sameIndex
      sameDepth
      otherVerified

theorem
  nary_merkle_opening_arity_four_index_binding_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root opening,
      NAryMerklePathHasArity 4 opening.layers ->
        NAryMerklePathOpeningVerifies compress root opening ->
          forall otherLeaf otherPath,
            NAryMerklePathHasArity 4 otherPath ->
              NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherPath ->
                opening.layers.length = otherPath.length ->
                  NAryMerklePathVerifies compress root otherLeaf otherPath ->
                    otherLeaf = opening.leaf := by
  intro root opening openingArity verified otherLeaf otherPath otherPathArity
    sameIndex sameDepth otherVerified
  exact
    nary_merkle_path_arity_four_index_binding_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      opening.leaf
      opening.layers
      openingArity
      verified
      otherLeaf
      otherPath
      otherPathArity
      sameIndex
      sameDepth
      otherVerified

theorem
  nary_merkle_opening_arity_four_index_binding_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening,
      NAryMerklePathHasArity 4 opening.layers ->
        NAryMerklePathOpeningVerifies compress root opening ->
          forall otherLeaf otherPath,
            NAryMerklePathHasArity 4 otherPath ->
              NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherPath ->
                opening.layers.length = otherPath.length ->
                  NAryMerklePathVerifies compress root otherLeaf otherPath ->
                    otherLeaf = opening.leaf := by
  intro root opening openingArity verified otherLeaf otherPath otherPathArity
    sameIndex sameDepth otherVerified
  exact
    nary_merkle_opening_arity_four_index_binding_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      opening
      openingArity
      verified
      otherLeaf
      otherPath
      otherPathArity
      sameIndex
      sameDepth
      otherVerified

end Lzvm
