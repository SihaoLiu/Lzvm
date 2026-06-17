/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.MerklePathSoundness.NAry

/-!
Direct leaf-equality wrappers for fixed-arity n-ary Merkle paths.
-/

namespace Lzvm

universe uDigest

theorem
  verified_concrete_nary_merkle_path_arity_two_same_index_leaf_eq_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root leaf path otherLeaf otherPath,
      NAryMerklePathHasArity 2 path ->
        NAryMerklePathHasArity 2 otherPath ->
          NAryMerklePathVerifies compress root leaf path ->
            NAryMerklePathVerifies compress root otherLeaf otherPath ->
              NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
                path.length = otherPath.length ->
                  otherLeaf = leaf := by
  intro root leaf path otherLeaf otherPath pathArity otherPathArity verified
    otherVerified sameIndex sameDepth
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
      otherVerified

theorem
  verified_concrete_nary_merkle_path_arity_two_same_index_leaf_eq_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root leaf path otherLeaf otherPath,
      NAryMerklePathHasArity 2 path ->
        NAryMerklePathHasArity 2 otherPath ->
          NAryMerklePathVerifies compress root leaf path ->
            NAryMerklePathVerifies compress root otherLeaf otherPath ->
              NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
                path.length = otherPath.length ->
                  otherLeaf = leaf := by
  intro root leaf path otherLeaf otherPath pathArity otherPathArity verified
    otherVerified sameIndex sameDepth
  exact
    verified_concrete_nary_merkle_path_arity_two_same_index_leaf_eq_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      leaf
      path
      otherLeaf
      otherPath
      pathArity
      otherPathArity
      verified
      otherVerified
      sameIndex
      sameDepth

theorem
  verified_concrete_nary_merkle_path_arity_two_same_index_leaf_eq_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root leaf path otherLeaf otherPath,
      NAryMerklePathHasArity 2 path ->
        NAryMerklePathHasArity 2 otherPath ->
          NAryMerklePathVerifies compress root leaf path ->
            NAryMerklePathVerifies compress root otherLeaf otherPath ->
              NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
                path.length = otherPath.length ->
                  otherLeaf = leaf := by
  intro root leaf path otherLeaf otherPath pathArity otherPathArity verified
    otherVerified sameIndex sameDepth
  exact
    verified_concrete_nary_merkle_path_arity_two_same_index_leaf_eq_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      leaf
      path
      otherLeaf
      otherPath
      pathArity
      otherPathArity
      verified
      otherVerified
      sameIndex
      sameDepth

theorem
  verified_concrete_nary_merkle_path_arity_four_same_index_leaf_eq_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root leaf path otherLeaf otherPath,
      NAryMerklePathHasArity 4 path ->
        NAryMerklePathHasArity 4 otherPath ->
          NAryMerklePathVerifies compress root leaf path ->
            NAryMerklePathVerifies compress root otherLeaf otherPath ->
              NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
                path.length = otherPath.length ->
                  otherLeaf = leaf := by
  intro root leaf path otherLeaf otherPath pathArity otherPathArity verified
    otherVerified sameIndex sameDepth
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
      otherVerified

theorem
  verified_concrete_nary_merkle_path_arity_four_same_index_leaf_eq_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root leaf path otherLeaf otherPath,
      NAryMerklePathHasArity 4 path ->
        NAryMerklePathHasArity 4 otherPath ->
          NAryMerklePathVerifies compress root leaf path ->
            NAryMerklePathVerifies compress root otherLeaf otherPath ->
              NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
                path.length = otherPath.length ->
                  otherLeaf = leaf := by
  intro root leaf path otherLeaf otherPath pathArity otherPathArity verified
    otherVerified sameIndex sameDepth
  exact
    verified_concrete_nary_merkle_path_arity_four_same_index_leaf_eq_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      leaf
      path
      otherLeaf
      otherPath
      pathArity
      otherPathArity
      verified
      otherVerified
      sameIndex
      sameDepth

theorem
  verified_concrete_nary_merkle_path_arity_four_same_index_leaf_eq_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root leaf path otherLeaf otherPath,
      NAryMerklePathHasArity 4 path ->
        NAryMerklePathHasArity 4 otherPath ->
          NAryMerklePathVerifies compress root leaf path ->
            NAryMerklePathVerifies compress root otherLeaf otherPath ->
              NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
                path.length = otherPath.length ->
                  otherLeaf = leaf := by
  intro root leaf path otherLeaf otherPath pathArity otherPathArity verified
    otherVerified sameIndex sameDepth
  exact
    verified_concrete_nary_merkle_path_arity_four_same_index_leaf_eq_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      leaf
      path
      otherLeaf
      otherPath
      pathArity
      otherPathArity
      verified
      otherVerified
      sameIndex
      sameDepth

end Lzvm
