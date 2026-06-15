/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.MerklePathSoundness.Core

/-!
N-ary Merkle authentication path soundness derived from concrete path folding.
-/

namespace Lzvm

universe uDigest

theorem nary_merkle_compression_collision_free_of_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    NAryMerkleCompressionCollisionFree compress := by
  intro children otherChildren sameDigest
  by_contra differentChildren
  apply noCollision
  exact
    { children := children
      otherChildren := otherChildren
      sameDigest := sameDigest
      differentInputs := differentChildren }

theorem centralized_nary_merkle_compression_collision_free
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    NAryMerkleCompressionCollisionFree compress := by
  exact
    nary_merkle_compression_collision_free_of_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)

theorem nary_merkle_path_same_position_implies_index_depth_eq
    {Digest : Type uDigest} :
    forall path otherPath : List (NAryMerklePathLayer Digest),
      NAryMerklePathSamePosition path otherPath ->
        NAryMerklePathIndex path = NAryMerklePathIndex otherPath
          /\ path.length = otherPath.length := by
  intro path
  induction path with
  | nil =>
      intro otherPath samePosition
      cases otherPath with
      | nil =>
          simp [NAryMerklePathIndex]
      | cons _ _ =>
          simp [NAryMerklePathSamePosition] at samePosition
  | cons layer rest ih =>
      intro otherPath samePosition
      cases otherPath with
      | nil =>
          simp [NAryMerklePathSamePosition] at samePosition
      | cons otherLayer otherRest =>
          have sameLeftLength :
              layer.leftSiblings.length =
                otherLayer.leftSiblings.length := by
            exact samePosition.left
          have sameRightLength :
              layer.rightSiblings.length =
                otherLayer.rightSiblings.length := by
            exact samePosition.right.left
          have sameRest :
              NAryMerklePathSamePosition rest otherRest := by
            exact samePosition.right.right
          have restBound := ih otherRest sameRest
          constructor
          · rw [NAryMerklePathIndex, NAryMerklePathIndex]
            simp [
              NAryMerklePathLayer.childSlot,
              NAryMerklePathLayer.arity,
              sameLeftLength,
              sameRightLength,
              restBound.left,
            ]
          · simp [restBound.right]

theorem nary_merkle_path_arity_two_index_implies_same_position
    {Digest : Type uDigest} :
    forall path otherPath : List (NAryMerklePathLayer Digest),
      NAryMerklePathHasArity 2 path ->
        NAryMerklePathHasArity 2 otherPath ->
          NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
            path.length = otherPath.length ->
              NAryMerklePathSamePosition path otherPath := by
  intro path
  induction path with
  | nil =>
      intro otherPath _pathArity _otherArity _sameIndex sameDepth
      cases otherPath with
      | nil =>
          simp [NAryMerklePathSamePosition]
      | cons _ _ =>
          simp at sameDepth
  | cons layer rest ih =>
      intro otherPath pathArity otherArity sameIndex sameDepth
      cases otherPath with
      | nil =>
          simp at sameDepth
      | cons otherLayer otherRest =>
          have layerArity :
              NAryMerklePathLayerHasArity 2 layer := by
            exact pathArity.left
          have restArity :
              NAryMerklePathHasArity 2 rest := by
            exact pathArity.right
          have otherLayerArity :
              NAryMerklePathLayerHasArity 2 otherLayer := by
            exact otherArity.left
          have otherRestArity :
              NAryMerklePathHasArity 2 otherRest := by
            exact otherArity.right
          have indexEq := sameIndex
          rw [NAryMerklePathIndex, NAryMerklePathIndex] at indexEq
          rw [layerArity, otherLayerArity] at indexEq
          simp [NAryMerklePathLayer.childSlot] at indexEq
          have layerShape := layerArity
          have otherLayerShape := otherLayerArity
          simp [
            NAryMerklePathLayerHasArity,
            NAryMerklePathLayer.arity,
          ] at layerShape otherLayerShape
          have sameLeft :
              layer.leftSiblings.length =
                otherLayer.leftSiblings.length := by
            omega
          have sameRight :
              layer.rightSiblings.length =
                otherLayer.rightSiblings.length := by
            omega
          have sameRestIndex :
              NAryMerklePathIndex rest =
                NAryMerklePathIndex otherRest := by
            omega
          constructor
          · exact sameLeft
          · constructor
            · exact sameRight
            · exact
                ih
                  otherRest
                  restArity
                  otherRestArity
                  sameRestIndex
                  (Nat.succ.inj sameDepth)

theorem nary_merkle_path_arity_four_index_implies_same_position
    {Digest : Type uDigest} :
    forall path otherPath : List (NAryMerklePathLayer Digest),
      NAryMerklePathHasArity 4 path ->
        NAryMerklePathHasArity 4 otherPath ->
          NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
            path.length = otherPath.length ->
              NAryMerklePathSamePosition path otherPath := by
  intro path
  induction path with
  | nil =>
      intro otherPath _pathArity _otherArity _sameIndex sameDepth
      cases otherPath with
      | nil =>
          simp [NAryMerklePathSamePosition]
      | cons _ _ =>
          simp at sameDepth
  | cons layer rest ih =>
      intro otherPath pathArity otherArity sameIndex sameDepth
      cases otherPath with
      | nil =>
          simp at sameDepth
      | cons otherLayer otherRest =>
          have layerArity :
              NAryMerklePathLayerHasArity 4 layer := by
            exact pathArity.left
          have restArity :
              NAryMerklePathHasArity 4 rest := by
            exact pathArity.right
          have otherLayerArity :
              NAryMerklePathLayerHasArity 4 otherLayer := by
            exact otherArity.left
          have otherRestArity :
              NAryMerklePathHasArity 4 otherRest := by
            exact otherArity.right
          have indexEq := sameIndex
          rw [NAryMerklePathIndex, NAryMerklePathIndex] at indexEq
          rw [layerArity, otherLayerArity] at indexEq
          simp [NAryMerklePathLayer.childSlot] at indexEq
          have layerShape := layerArity
          have otherLayerShape := otherLayerArity
          simp [
            NAryMerklePathLayerHasArity,
            NAryMerklePathLayer.arity,
          ] at layerShape otherLayerShape
          have sameLeft :
              layer.leftSiblings.length =
                otherLayer.leftSiblings.length := by
            omega
          have sameRight :
              layer.rightSiblings.length =
                otherLayer.rightSiblings.length := by
            omega
          have sameRestIndex :
              NAryMerklePathIndex rest =
                NAryMerklePathIndex otherRest := by
            omega
          constructor
          · exact sameLeft
          · constructor
            · exact sameRight
            · exact
                ih
                  otherRest
                  restArity
                  otherRestArity
                  sameRestIndex
                  (Nat.succ.inj sameDepth)

theorem nary_merkle_children_current_eq_of_eq
    {Digest : Type uDigest} :
    forall leftSiblings otherLeftSiblings : List Digest,
      forall current otherCurrent : Digest,
        forall rightSiblings otherRightSiblings : List Digest,
          leftSiblings.length = otherLeftSiblings.length ->
            leftSiblings ++ current :: rightSiblings =
              otherLeftSiblings ++ otherCurrent :: otherRightSiblings ->
                current = otherCurrent := by
  intro leftSiblings
  induction leftSiblings with
  | nil =>
      intro otherLeftSiblings current otherCurrent rightSiblings
        otherRightSiblings sameLength sameChildren
      cases otherLeftSiblings with
      | nil =>
          exact (List.cons.inj sameChildren).left
      | cons _ _ =>
          simp at sameLength
  | cons _ leftRest ih =>
      intro otherLeftSiblings current otherCurrent rightSiblings
        otherRightSiblings sameLength sameChildren
      cases otherLeftSiblings with
      | nil =>
          simp at sameLength
      | cons _ otherLeftRest =>
          have sameTail := (List.cons.inj sameChildren).right
          exact
            ih
              otherLeftRest
              current
              otherCurrent
              rightSiblings
              otherRightSiblings
              (Nat.succ.inj sameLength)
              sameTail

theorem different_leaf_same_position_verified_nary_paths_imply_merkle_compression_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest} :
    forall root leaf path otherLeaf otherPath,
      NAryMerklePathSamePosition path otherPath ->
        NAryMerklePathVerifies compress root leaf path ->
          NAryMerklePathVerifies compress root otherLeaf otherPath ->
            otherLeaf ≠ leaf ->
              Nonempty (NAryMerkleCompressionCollision compress) := by
  intro root leaf path
  induction path generalizing root leaf with
  | nil =>
      intro otherLeaf otherPath samePosition verified otherVerified differentLeaf
      cases otherPath with
      | nil =>
          simp [NAryMerklePathVerifies, NAryMerklePathFold] at verified otherVerified
          exact False.elim (differentLeaf (otherVerified.trans verified.symm))
      | cons _ _ =>
          simp [NAryMerklePathSamePosition] at samePosition
  | cons layer rest ih =>
      intro otherLeaf otherPath samePosition verified otherVerified differentLeaf
      cases otherPath with
      | nil =>
          simp [NAryMerklePathSamePosition] at samePosition
      | cons otherLayer otherRest =>
          have sameLeftLength :
              layer.leftSiblings.length =
                otherLayer.leftSiblings.length := by
            exact samePosition.left
          have sameRest :
              NAryMerklePathSamePosition rest otherRest := by
            exact samePosition.right.right
          let parent :=
            NAryMerklePathLayer.parentDigest compress leaf layer
          let otherParent :=
            NAryMerklePathLayer.parentDigest compress otherLeaf otherLayer
          by_cases sameParent : parent = otherParent
          · exact
              Nonempty.intro
                { children := NAryMerklePathLayer.children leaf layer
                  otherChildren :=
                    NAryMerklePathLayer.children otherLeaf otherLayer
                  sameDigest := by exact sameParent
                  differentInputs := by
                    intro sameChildren
                    exact
                      differentLeaf
                        ((nary_merkle_children_current_eq_of_eq
                          layer.leftSiblings
                          otherLayer.leftSiblings
                          leaf
                          otherLeaf
                          layer.rightSiblings
                          otherLayer.rightSiblings
                          sameLeftLength
                          sameChildren).symm) }
          · exact
              ih
                root
                parent
                otherParent
                otherRest
                sameRest
                verified
                otherVerified
                (by
                  intro reverseParent
                  exact sameParent reverseParent.symm)

theorem different_leaf_same_position_verified_nary_openings_imply_merkle_compression_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest} :
    forall root opening otherOpening,
      NAryMerklePathSamePosition opening.layers otherOpening.layers ->
        NAryMerklePathOpeningVerifies compress root opening ->
          NAryMerklePathOpeningVerifies compress root otherOpening ->
            otherOpening.leaf ≠ opening.leaf ->
              Nonempty (NAryMerkleCompressionCollision compress) := by
  intro root opening otherOpening samePosition verified otherVerified differentLeaf
  exact
    different_leaf_same_position_verified_nary_paths_imply_merkle_compression_collision
      root
      opening.leaf
      opening.layers
      otherOpening.leaf
      otherOpening.layers
      samePosition
      verified
      otherVerified
      differentLeaf

theorem different_leaf_same_position_verified_nary_paths_contradict_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root leaf path otherLeaf otherPath,
      NAryMerklePathSamePosition path otherPath ->
        NAryMerklePathVerifies compress root leaf path ->
          NAryMerklePathVerifies compress root otherLeaf otherPath ->
            otherLeaf ≠ leaf ->
              False := by
  intro root leaf path otherLeaf otherPath samePosition verified otherVerified
    differentLeaf
  have collisionWitness :
      Nonempty (NAryMerkleCompressionCollision compress) :=
    different_leaf_same_position_verified_nary_paths_imply_merkle_compression_collision
      root
      leaf
      path
      otherLeaf
      otherPath
      samePosition
      verified
      otherVerified
      differentLeaf
  cases collisionWitness with
  | intro collision =>
      exact noCollision collision

theorem different_leaf_same_position_verified_nary_openings_contradict_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root opening otherOpening,
      NAryMerklePathSamePosition opening.layers otherOpening.layers ->
        NAryMerklePathOpeningVerifies compress root opening ->
          NAryMerklePathOpeningVerifies compress root otherOpening ->
            otherOpening.leaf ≠ opening.leaf ->
              False := by
  intro root opening otherOpening samePosition verified otherVerified differentLeaf
  have collisionWitness :
      Nonempty (NAryMerkleCompressionCollision compress) :=
    different_leaf_same_position_verified_nary_openings_imply_merkle_compression_collision
      root
      opening
      otherOpening
      samePosition
      verified
      otherVerified
      differentLeaf
  cases collisionWitness with
  | intro collision =>
      exact noCollision collision

theorem different_leaf_same_position_verified_nary_openings_contradict_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root opening otherOpening,
      NAryMerklePathSamePosition opening.layers otherOpening.layers ->
        NAryMerklePathOpeningVerifies compress root opening ->
          NAryMerklePathOpeningVerifies compress root otherOpening ->
            otherOpening.leaf ≠ opening.leaf ->
              False := by
  intro root opening otherOpening samePosition verified otherVerified differentLeaf
  exact
    different_leaf_same_position_verified_nary_openings_contradict_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      opening
      otherOpening
      samePosition
      verified
      otherVerified
      differentLeaf

theorem different_leaf_same_position_verified_nary_openings_contradict_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening otherOpening,
      NAryMerklePathSamePosition opening.layers otherOpening.layers ->
        NAryMerklePathOpeningVerifies compress root opening ->
          NAryMerklePathOpeningVerifies compress root otherOpening ->
            otherOpening.leaf ≠ opening.leaf ->
              False := by
  intro root opening otherOpening samePosition verified otherVerified differentLeaf
  exact
    different_leaf_same_position_verified_nary_openings_contradict_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      opening
      otherOpening
      samePosition
      verified
      otherVerified
      differentLeaf

theorem
  different_leaf_same_arity_two_index_verified_nary_paths_imply_merkle_compression_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest} :
    forall root leaf path otherLeaf otherPath,
      NAryMerklePathHasArity 2 path ->
        NAryMerklePathHasArity 2 otherPath ->
          NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
            path.length = otherPath.length ->
              NAryMerklePathVerifies compress root leaf path ->
                NAryMerklePathVerifies compress root otherLeaf otherPath ->
                  otherLeaf ≠ leaf ->
                    Nonempty (NAryMerkleCompressionCollision compress) := by
  intro root leaf path otherLeaf otherPath pathArity otherPathArity sameIndex
    sameDepth verified otherVerified differentLeaf
  exact
    different_leaf_same_position_verified_nary_paths_imply_merkle_compression_collision
      root
      leaf
      path
      otherLeaf
      otherPath
      (nary_merkle_path_arity_two_index_implies_same_position
        path
        otherPath
        pathArity
        otherPathArity
        sameIndex
        sameDepth)
      verified
      otherVerified
      differentLeaf

theorem
  different_leaf_same_arity_two_index_verified_nary_openings_imply_merkle_compression_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest} :
    forall root opening otherOpening,
      NAryMerklePathHasArity 2 opening.layers ->
        NAryMerklePathHasArity 2 otherOpening.layers ->
          NAryMerklePathOpeningVerifies compress root opening ->
            NAryMerklePathOpeningVerifies compress root otherOpening ->
              NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherOpening.layers ->
                opening.layers.length = otherOpening.layers.length ->
                  otherOpening.leaf ≠ opening.leaf ->
                    Nonempty (NAryMerkleCompressionCollision compress) := by
  intro root opening otherOpening openingArity otherOpeningArity verified
    otherVerified sameIndex sameDepth differentLeaf
  exact
    different_leaf_same_arity_two_index_verified_nary_paths_imply_merkle_compression_collision
      root
      opening.leaf
      opening.layers
      otherOpening.leaf
      otherOpening.layers
      openingArity
      otherOpeningArity
      sameIndex
      sameDepth
      verified
      otherVerified
      differentLeaf

theorem
  different_leaf_same_arity_four_index_verified_nary_paths_imply_merkle_compression_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest} :
    forall root leaf path otherLeaf otherPath,
      NAryMerklePathHasArity 4 path ->
        NAryMerklePathHasArity 4 otherPath ->
          NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
            path.length = otherPath.length ->
              NAryMerklePathVerifies compress root leaf path ->
                NAryMerklePathVerifies compress root otherLeaf otherPath ->
                  otherLeaf ≠ leaf ->
                    Nonempty (NAryMerkleCompressionCollision compress) := by
  intro root leaf path otherLeaf otherPath pathArity otherPathArity sameIndex
    sameDepth verified otherVerified differentLeaf
  exact
    different_leaf_same_position_verified_nary_paths_imply_merkle_compression_collision
      root
      leaf
      path
      otherLeaf
      otherPath
      (nary_merkle_path_arity_four_index_implies_same_position
        path
        otherPath
        pathArity
        otherPathArity
        sameIndex
        sameDepth)
      verified
      otherVerified
      differentLeaf

theorem
  different_leaf_same_arity_four_index_verified_nary_openings_imply_merkle_compression_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest} :
    forall root opening otherOpening,
      NAryMerklePathHasArity 4 opening.layers ->
        NAryMerklePathHasArity 4 otherOpening.layers ->
          NAryMerklePathOpeningVerifies compress root opening ->
            NAryMerklePathOpeningVerifies compress root otherOpening ->
              NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherOpening.layers ->
                opening.layers.length = otherOpening.layers.length ->
                  otherOpening.leaf ≠ opening.leaf ->
                    Nonempty (NAryMerkleCompressionCollision compress) := by
  intro root opening otherOpening openingArity otherOpeningArity verified
    otherVerified sameIndex sameDepth differentLeaf
  exact
    different_leaf_same_arity_four_index_verified_nary_paths_imply_merkle_compression_collision
      root
      opening.leaf
      opening.layers
      otherOpening.leaf
      otherOpening.layers
      openingArity
      otherOpeningArity
      sameIndex
      sameDepth
      verified
      otherVerified
      differentLeaf

theorem
  different_leaf_same_arity_two_index_verified_nary_openings_contradict_no_collision
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
                  otherOpening.leaf ≠ opening.leaf ->
                    False := by
  intro root opening otherOpening openingArity otherOpeningArity verified
    otherVerified sameIndex sameDepth differentLeaf
  have collisionWitness :
      Nonempty (NAryMerkleCompressionCollision compress) :=
    different_leaf_same_arity_two_index_verified_nary_openings_imply_merkle_compression_collision
      root
      opening
      otherOpening
      openingArity
      otherOpeningArity
      verified
      otherVerified
      sameIndex
      sameDepth
      differentLeaf
  cases collisionWitness with
  | intro collision =>
      exact noCollision collision

theorem
  different_leaf_same_arity_four_index_verified_nary_openings_contradict_no_collision
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
                  otherOpening.leaf ≠ opening.leaf ->
                    False := by
  intro root opening otherOpening openingArity otherOpeningArity verified
    otherVerified sameIndex sameDepth differentLeaf
  have collisionWitness :
      Nonempty (NAryMerkleCompressionCollision compress) :=
    different_leaf_same_arity_four_index_verified_nary_openings_imply_merkle_compression_collision
      root
      opening
      otherOpening
      openingArity
      otherOpeningArity
      verified
      otherVerified
      sameIndex
      sameDepth
      differentLeaf
  cases collisionWitness with
  | intro collision =>
      exact noCollision collision

theorem
  different_leaf_same_arity_two_index_verified_nary_openings_contradict_bundle
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
                  otherOpening.leaf ≠ opening.leaf ->
                    False := by
  intro root opening otherOpening openingArity otherOpeningArity verified
    otherVerified sameIndex sameDepth differentLeaf
  exact
    different_leaf_same_arity_two_index_verified_nary_openings_contradict_no_collision
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      root
      opening
      otherOpening
      openingArity
      otherOpeningArity
      verified
      otherVerified
      sameIndex
      sameDepth
      differentLeaf

theorem
  different_leaf_same_arity_four_index_verified_nary_openings_contradict_bundle
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
                  otherOpening.leaf ≠ opening.leaf ->
                    False := by
  intro root opening otherOpening openingArity otherOpeningArity verified
    otherVerified sameIndex sameDepth differentLeaf
  exact
    different_leaf_same_arity_four_index_verified_nary_openings_contradict_no_collision
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      root
      opening
      otherOpening
      openingArity
      otherOpeningArity
      verified
      otherVerified
      sameIndex
      sameDepth
      differentLeaf

theorem concrete_nary_merkle_path_same_position_binding_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root leaf path otherLeaf otherPath,
      NAryMerklePathSamePosition path otherPath ->
        NAryMerklePathVerifies compress root leaf path ->
          NAryMerklePathVerifies compress root otherLeaf otherPath ->
            otherLeaf = leaf := by
  intro root leaf path otherLeaf otherPath samePosition verified otherVerified
  by_contra differentLeaf
  have collisionWitness :
      Nonempty (NAryMerkleCompressionCollision compress) :=
    different_leaf_same_position_verified_nary_paths_imply_merkle_compression_collision
      root
      leaf
      path
      otherLeaf
      otherPath
      samePosition
      verified
      otherVerified
      differentLeaf
  cases collisionWitness with
  | intro collision =>
      exact noCollision collision

theorem
  verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root leaf path,
      NAryMerklePathVerifies compress root leaf path ->
        NAryMerklePathRootCommitsToLeafAtPosition compress root leaf path := by
  intro root leaf path verified otherLeaf otherPath samePosition otherVerified
  exact
    concrete_nary_merkle_path_same_position_binding_from_no_collision
      noCollision
      root
      leaf
      path
      otherLeaf
      otherPath
      samePosition
      verified
      otherVerified

theorem
  nary_merkle_path_root_commits_to_leaf_at_same_position_index_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root leaf path,
      NAryMerklePathVerifies compress root leaf path ->
        NAryMerklePathRootCommitsToLeafAtSamePositionIndex
          compress
          root
          leaf
          path := by
  intro root leaf path verified otherLeaf otherPath samePosition _sameIndex
    _sameDepth otherVerified
  exact
    concrete_nary_merkle_path_same_position_binding_from_no_collision
      noCollision
      root
      leaf
      path
      otherLeaf
      otherPath
      samePosition
      verified
      otherVerified

theorem
  verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root leaf path,
      NAryMerklePathVerifies compress root leaf path ->
        NAryMerklePathRootCommitsToLeafAtIndex compress root leaf path := by
  intro root leaf path verified
  exact
    nary_merkle_path_root_commits_to_leaf_at_same_position_index_from_no_collision
      noCollision
      root
      leaf
      path
      verified

theorem
  nary_merkle_path_arity_two_index_binding_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root leaf path,
      NAryMerklePathHasArity 2 path ->
        NAryMerklePathVerifies compress root leaf path ->
          forall otherLeaf otherPath,
            NAryMerklePathHasArity 2 otherPath ->
              NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
                path.length = otherPath.length ->
                  NAryMerklePathVerifies compress root otherLeaf otherPath ->
                    otherLeaf = leaf := by
  intro root leaf path pathArity verified otherLeaf otherPath otherPathArity
    sameIndex sameDepth otherVerified
  exact
    concrete_nary_merkle_path_same_position_binding_from_no_collision
      noCollision
      root
      leaf
      path
      otherLeaf
      otherPath
      (nary_merkle_path_arity_two_index_implies_same_position
        path
        otherPath
        pathArity
        otherPathArity
        sameIndex
        sameDepth)
      verified
      otherVerified

theorem
  nary_merkle_path_arity_four_index_binding_from_no_collision
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (noCollision : NAryMerkleCompressionNoCollision compress) :
    forall root leaf path,
      NAryMerklePathHasArity 4 path ->
        NAryMerklePathVerifies compress root leaf path ->
          forall otherLeaf otherPath,
            NAryMerklePathHasArity 4 otherPath ->
              NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
                path.length = otherPath.length ->
                  NAryMerklePathVerifies compress root otherLeaf otherPath ->
                    otherLeaf = leaf := by
  intro root leaf path pathArity verified otherLeaf otherPath otherPathArity
    sameIndex sameDepth otherVerified
  exact
    concrete_nary_merkle_path_same_position_binding_from_no_collision
      noCollision
      root
      leaf
      path
      otherLeaf
      otherPath
      (nary_merkle_path_arity_four_index_implies_same_position
        path
        otherPath
        pathArity
        otherPathArity
        sameIndex
        sameDepth)
      verified
      otherVerified

theorem
  nary_merkle_path_arity_two_index_binding_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root leaf path,
      NAryMerklePathHasArity 2 path ->
        NAryMerklePathVerifies compress root leaf path ->
          forall otherLeaf otherPath,
            NAryMerklePathHasArity 2 otherPath ->
              NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
                path.length = otherPath.length ->
                  NAryMerklePathVerifies compress root otherLeaf otherPath ->
                    otherLeaf = leaf := by
  intro root leaf path pathArity verified otherLeaf otherPath otherPathArity
    sameIndex sameDepth otherVerified
  exact
    nary_merkle_path_arity_two_index_binding_from_no_collision
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
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
  nary_merkle_path_arity_four_index_binding_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root leaf path,
      NAryMerklePathHasArity 4 path ->
        NAryMerklePathVerifies compress root leaf path ->
          forall otherLeaf otherPath,
            NAryMerklePathHasArity 4 otherPath ->
              NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
                path.length = otherPath.length ->
                  NAryMerklePathVerifies compress root otherLeaf otherPath ->
                    otherLeaf = leaf := by
  intro root leaf path pathArity verified otherLeaf otherPath otherPathArity
    sameIndex sameDepth otherVerified
  exact
    nary_merkle_path_arity_four_index_binding_from_no_collision
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
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

theorem verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root leaf path,
      NAryMerklePathVerifies compress root leaf path ->
        NAryMerklePathRootCommitsToLeafAtIndex compress root leaf path := by
  intro root leaf path verified
  exact
    verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      leaf
      path
      verified

theorem
  nary_merkle_path_root_commits_to_leaf_at_same_position_index_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root leaf path,
      NAryMerklePathVerifies compress root leaf path ->
        NAryMerklePathRootCommitsToLeafAtSamePositionIndex
          compress
          root
          leaf
          path := by
  intro root leaf path verified
  exact
    nary_merkle_path_root_commits_to_leaf_at_same_position_index_from_no_collision
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
      root
      leaf
      path
      verified

theorem verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root leaf path,
      NAryMerklePathVerifies compress root leaf path ->
        NAryMerklePathRootCommitsToLeafAtIndex compress root leaf path := by
  intro root leaf path verified
  exact
    verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      leaf
      path
      verified

theorem verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_position_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root leaf path,
      NAryMerklePathVerifies compress root leaf path ->
        NAryMerklePathRootCommitsToLeafAtPosition compress root leaf path := by
  intro root leaf path verified
  exact
    verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      leaf
      path
      verified

theorem verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_position_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root leaf path,
      NAryMerklePathVerifies compress root leaf path ->
        NAryMerklePathRootCommitsToLeafAtPosition compress root leaf path := by
  intro root leaf path verified
  exact
    verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_position_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      leaf
      path
      verified

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
    nary_merkle_path_arity_two_index_binding_from_no_collision
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
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
    nary_merkle_path_arity_four_index_binding_from_no_collision
      (Eq.mp
        centralized
        assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistance.evidence)
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

end Lzvm
