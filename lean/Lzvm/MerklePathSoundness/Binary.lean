/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.MerklePathSoundness.Core

/-!
Binary Merkle authentication path soundness derived from concrete path folding.
-/

namespace Lzvm

universe uDigest

theorem merkle_compression_collision_free_of_no_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (noCollision : MerkleCompressionNoCollision compress) :
    MerkleCompressionCollisionFree compress := by
  intro left right otherLeft otherRight sameDigest
  by_contra differentPair
  apply noCollision
  exact
    { left := left
      right := right
      otherLeft := otherLeft
      otherRight := otherRight
      sameDigest := sameDigest
      differentInputs := by
        by_cases sameLeft : left = otherLeft
        · right
          intro sameRight
          exact differentPair (And.intro sameLeft sameRight)
        · left
          exact sameLeft }

theorem centralized_merkle_compression_collision_free
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    MerkleCompressionCollisionFree compress := by
  exact
    merkle_compression_collision_free_of_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)

theorem merkle_parent_digest_injective
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (collisionFree : MerkleCompressionCollisionFree compress)
    {leaf otherLeaf : Digest}
    {layer otherLayer : MerklePathLayer Digest}
    (sameDirection : layer.direction = otherLayer.direction)
    (sameParent :
      MerklePathLayer.parentDigest compress leaf layer =
        MerklePathLayer.parentDigest compress otherLeaf otherLayer) :
    leaf = otherLeaf := by
  cases layer with
  | mk sibling direction =>
      cases otherLayer with
      | mk otherSibling otherDirection =>
          cases direction with
          | currentOnLeft =>
              cases otherDirection with
              | currentOnLeft =>
                  change
                    compress leaf sibling =
                      compress otherLeaf otherSibling at sameParent
                  exact
                    (collisionFree
                      leaf
                      sibling
                      otherLeaf
                      otherSibling
                      sameParent).left
              | currentOnRight =>
                  cases sameDirection
          | currentOnRight =>
              cases otherDirection with
              | currentOnLeft =>
                  cases sameDirection
              | currentOnRight =>
                  change
                    compress sibling leaf =
                      compress otherSibling otherLeaf at sameParent
                  exact
                    (collisionFree
                      sibling
                      leaf
                      otherSibling
                      otherLeaf
                      sameParent).right

theorem merkle_path_same_index_implies_index_depth_eq
    {Digest : Type uDigest} :
    forall path otherPath : List (MerklePathLayer Digest),
      MerklePathSameIndex path otherPath ->
        MerklePathIndex path = MerklePathIndex otherPath
          /\ path.length = otherPath.length := by
  intro path
  induction path with
  | nil =>
      intro otherPath sameIndex
      cases otherPath with
      | nil =>
          simp [MerklePathIndex]
      | cons _ _ =>
          simp [MerklePathSameIndex] at sameIndex
  | cons layer rest ih =>
      intro otherPath sameIndex
      cases otherPath with
      | nil =>
          simp [MerklePathSameIndex] at sameIndex
      | cons otherLayer otherRest =>
          have sameDirection :
              layer.direction = otherLayer.direction := by
            exact sameIndex.left
          have sameRest :
              MerklePathSameIndex rest otherRest := by
            exact sameIndex.right
          have restBound := ih otherRest sameRest
          constructor
          · rw [MerklePathIndex, MerklePathIndex, sameDirection, restBound.left]
          · simp [restBound.right]

theorem merkle_path_same_position_implies_same_index
    {Digest : Type uDigest} :
    forall path otherPath : List (MerklePathLayer Digest),
      MerklePathIndex path = MerklePathIndex otherPath ->
        path.length = otherPath.length ->
          MerklePathSameIndex path otherPath := by
  intro path
  induction path with
  | nil =>
      intro otherPath _samePosition sameDepth
      cases otherPath with
      | nil =>
          simp [MerklePathSameIndex]
      | cons _ _ =>
          simp at sameDepth
  | cons layer rest ih =>
      intro otherPath samePosition sameDepth
      cases otherPath with
      | nil =>
          simp at sameDepth
      | cons otherLayer otherRest =>
          cases layer with
          | mk _sibling direction =>
              cases otherLayer with
              | mk _otherSibling otherDirection =>
                  cases direction <;> cases otherDirection
                  · constructor
                    · rfl
                    · apply ih
                      · simp [MerklePathIndex, MerklePathDirection.indexBit]
                          at samePosition ⊢
                        omega
                      · exact Nat.succ.inj sameDepth
                  · simp [MerklePathIndex, MerklePathDirection.indexBit]
                      at samePosition
                    omega
                  · simp [MerklePathIndex, MerklePathDirection.indexBit]
                        at samePosition
                    omega
                  · constructor
                    · rfl
                    · apply ih
                      · simp [MerklePathIndex, MerklePathDirection.indexBit]
                          at samePosition ⊢
                        omega
                      · exact Nat.succ.inj sameDepth

theorem merkle_path_layer_to_nary_has_arity_two
    {Digest : Type uDigest}
    (layer : MerklePathLayer Digest) :
    NAryMerklePathLayerHasArity 2 (MerklePathLayerToNAry layer) := by
  cases layer with
  | mk _sibling direction =>
      cases direction <;>
        simp [
          MerklePathLayerToNAry,
          NAryMerklePathLayerHasArity,
          NAryMerklePathLayer.arity,
        ]

theorem merkle_path_to_nary_has_arity_two
    {Digest : Type uDigest} :
    forall path : List (MerklePathLayer Digest),
      NAryMerklePathHasArity 2 (MerklePathLayersToNAry path) := by
  intro path
  induction path with
  | nil =>
      simp [MerklePathLayersToNAry, NAryMerklePathHasArity]
  | cons layer rest ih =>
      simp [
        MerklePathLayersToNAry,
        NAryMerklePathHasArity,
        merkle_path_layer_to_nary_has_arity_two,
        ih,
      ]

theorem merkle_path_to_nary_index_eq
    {Digest : Type uDigest} :
    forall path : List (MerklePathLayer Digest),
      NAryMerklePathIndex (MerklePathLayersToNAry path) =
        MerklePathIndex path := by
  intro path
  induction path with
  | nil =>
      simp [MerklePathLayersToNAry, NAryMerklePathIndex, MerklePathIndex]
  | cons layer rest ih =>
      cases layer with
      | mk _sibling direction =>
          cases direction <;>
            simp [
              MerklePathLayersToNAry,
              MerklePathLayerToNAry,
              NAryMerklePathIndex,
              MerklePathIndex,
              NAryMerklePathLayer.childSlot,
              NAryMerklePathLayer.arity,
              MerklePathDirection.indexBit,
              ih,
            ]

theorem merkle_path_to_nary_fold_eq
    {Digest : Type uDigest}
    (binaryCompress : Digest -> Digest -> Digest)
    (naryCompress : List Digest -> Digest)
    (compatible :
      forall left right, naryCompress [left, right] = binaryCompress left right) :
    forall leaf path,
      NAryMerklePathFold naryCompress leaf (MerklePathLayersToNAry path) =
      MerklePathFold binaryCompress leaf path := by
  intro leaf path
  induction path generalizing leaf with
  | nil =>
      simp [MerklePathLayersToNAry, NAryMerklePathFold, MerklePathFold]
  | cons layer rest ih =>
      cases layer with
      | mk _sibling direction =>
          cases direction <;>
            simp [
              MerklePathLayersToNAry,
              MerklePathLayerToNAry,
              NAryMerklePathFold,
              MerklePathFold,
              NAryMerklePathLayer.parentDigest,
              NAryMerklePathLayer.children,
              MerklePathLayer.parentDigest,
              compatible,
              ih,
            ]

theorem merkle_path_to_nary_verifies
    {Digest : Type uDigest}
    (binaryCompress : Digest -> Digest -> Digest)
    (naryCompress : List Digest -> Digest)
    (compatible :
      forall left right, naryCompress [left, right] = binaryCompress left right)
    {root leaf : Digest}
    {path : List (MerklePathLayer Digest)}
    (verified : MerklePathVerifies binaryCompress root leaf path) :
    NAryMerklePathVerifies naryCompress root leaf (MerklePathLayersToNAry path) := by
  unfold MerklePathVerifies NAryMerklePathVerifies at *
  rw [merkle_path_to_nary_fold_eq binaryCompress naryCompress compatible leaf path]
  exact verified

theorem merkle_opening_to_nary_verifies
    {Digest : Type uDigest}
    (binaryCompress : Digest -> Digest -> Digest)
    (naryCompress : List Digest -> Digest)
    (compatible :
      forall left right, naryCompress [left, right] = binaryCompress left right)
    {root : Digest}
    {opening : MerklePathOpening Digest}
    (verified : MerklePathOpeningVerifies binaryCompress root opening) :
    NAryMerklePathOpeningVerifies
      naryCompress
      root
      (MerklePathOpeningToNAry opening) := by
  cases opening with
  | mk openingLeaf openingLayers =>
      simpa [
        MerklePathOpeningToNAry,
        MerklePathOpeningVerifies,
        NAryMerklePathOpeningVerifies,
      ] using
        merkle_path_to_nary_verifies
          binaryCompress
          naryCompress
          compatible
          (root := root)
          (leaf := openingLeaf)
          (path := openingLayers)
          verified

theorem different_leaf_same_index_verified_paths_imply_merkle_compression_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest} :
    forall root leaf path otherLeaf otherPath,
      MerklePathSameIndex path otherPath ->
        MerklePathVerifies compress root leaf path ->
          MerklePathVerifies compress root otherLeaf otherPath ->
            otherLeaf ≠ leaf ->
              Nonempty (MerkleCompressionCollision compress) := by
  intro root leaf path
  induction path generalizing root leaf with
  | nil =>
      intro otherLeaf otherPath sameIndex verified otherVerified differentLeaf
      cases otherPath with
      | nil =>
          simp [MerklePathVerifies, MerklePathFold] at verified otherVerified
          exact False.elim (differentLeaf (otherVerified.trans verified.symm))
      | cons _ _ =>
          simp [MerklePathSameIndex] at sameIndex
  | cons layer rest ih =>
      intro otherLeaf otherPath sameIndex verified otherVerified differentLeaf
      cases otherPath with
      | nil =>
          simp [MerklePathSameIndex] at sameIndex
      | cons otherLayer otherRest =>
          have sameDirection :
              layer.direction = otherLayer.direction := by
            exact sameIndex.left
          have sameRest :
              MerklePathSameIndex rest otherRest := by
            exact sameIndex.right
          let parent :=
            MerklePathLayer.parentDigest compress leaf layer
          let otherParent :=
            MerklePathLayer.parentDigest compress otherLeaf otherLayer
          by_cases sameParent : parent = otherParent
          · cases layer with
            | mk sibling direction =>
                cases otherLayer with
                | mk otherSibling otherDirection =>
                    cases direction with
                    | currentOnLeft =>
                        cases otherDirection with
                        | currentOnLeft =>
                            exact
                              Nonempty.intro
                                { left := leaf
                                  right := sibling
                                  otherLeft := otherLeaf
                                  otherRight := otherSibling
                                  sameDigest := by exact sameParent
                                  differentInputs := by
                                    left
                                    intro sameLeaf
                                    exact differentLeaf sameLeaf.symm }
                        | currentOnRight =>
                            cases sameDirection
                    | currentOnRight =>
                        cases otherDirection with
                        | currentOnLeft =>
                            cases sameDirection
                        | currentOnRight =>
                            exact
                              Nonempty.intro
                                { left := sibling
                                  right := leaf
                                  otherLeft := otherSibling
                                  otherRight := otherLeaf
                                  sameDigest := by exact sameParent
                                  differentInputs := by
                                    right
                                    intro sameLeaf
                                    exact differentLeaf sameLeaf.symm }
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

theorem different_leaf_same_index_verified_openings_imply_merkle_compression_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest} :
    forall root opening otherOpening,
      MerklePathSameIndex opening.layers otherOpening.layers ->
        MerklePathOpeningVerifies compress root opening ->
          MerklePathOpeningVerifies compress root otherOpening ->
            otherOpening.leaf ≠ opening.leaf ->
              Nonempty (MerkleCompressionCollision compress) := by
  intro root opening otherOpening sameIndex verified otherVerified differentLeaf
  exact
    different_leaf_same_index_verified_paths_imply_merkle_compression_collision
      root
      opening.leaf
      opening.layers
      otherOpening.leaf
      otherOpening.layers
      sameIndex
      verified
      otherVerified
      differentLeaf

theorem different_leaf_same_position_verified_paths_imply_merkle_compression_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest} :
    forall root leaf path otherLeaf otherPath,
      MerklePathIndex path = MerklePathIndex otherPath ->
        path.length = otherPath.length ->
          MerklePathVerifies compress root leaf path ->
            MerklePathVerifies compress root otherLeaf otherPath ->
              otherLeaf ≠ leaf ->
                Nonempty (MerkleCompressionCollision compress) := by
  intro root leaf path otherLeaf otherPath samePosition sameDepth verified otherVerified
    differentLeaf
  exact
    different_leaf_same_index_verified_paths_imply_merkle_compression_collision
      root
      leaf
      path
      otherLeaf
      otherPath
      (merkle_path_same_position_implies_same_index
        path
        otherPath
        samePosition
        sameDepth)
      verified
      otherVerified
      differentLeaf

theorem different_leaf_same_position_verified_openings_imply_merkle_compression_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest} :
    forall root opening otherOpening,
      MerklePathIndex opening.layers = MerklePathIndex otherOpening.layers ->
        opening.layers.length = otherOpening.layers.length ->
          MerklePathOpeningVerifies compress root opening ->
            MerklePathOpeningVerifies compress root otherOpening ->
              otherOpening.leaf ≠ opening.leaf ->
                Nonempty (MerkleCompressionCollision compress) := by
  intro root opening otherOpening samePosition sameDepth verified otherVerified differentLeaf
  exact
    different_leaf_same_position_verified_paths_imply_merkle_compression_collision
      root
      opening.leaf
      opening.layers
      otherOpening.leaf
      otherOpening.layers
      samePosition
      sameDepth
      verified
      otherVerified
      differentLeaf

theorem different_leaf_same_position_verified_paths_contradict_no_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (noCollision : MerkleCompressionNoCollision compress) :
    forall root leaf path otherLeaf otherPath,
      MerklePathIndex path = MerklePathIndex otherPath ->
        path.length = otherPath.length ->
          MerklePathVerifies compress root leaf path ->
            MerklePathVerifies compress root otherLeaf otherPath ->
              otherLeaf ≠ leaf ->
                False := by
  intro root leaf path otherLeaf otherPath samePosition sameDepth verified otherVerified
    differentLeaf
  have collisionWitness :
      Nonempty (MerkleCompressionCollision compress) :=
    different_leaf_same_position_verified_paths_imply_merkle_compression_collision
      root
      leaf
      path
      otherLeaf
      otherPath
      samePosition
      sameDepth
      verified
      otherVerified
      differentLeaf
  cases collisionWitness with
  | intro collision =>
      exact noCollision collision

theorem different_leaf_same_position_verified_openings_contradict_no_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (noCollision : MerkleCompressionNoCollision compress) :
    forall root opening otherOpening,
      MerklePathIndex opening.layers = MerklePathIndex otherOpening.layers ->
        opening.layers.length = otherOpening.layers.length ->
          MerklePathOpeningVerifies compress root opening ->
            MerklePathOpeningVerifies compress root otherOpening ->
              otherOpening.leaf ≠ opening.leaf ->
                False := by
  intro root opening otherOpening samePosition sameDepth verified otherVerified differentLeaf
  have collisionWitness :
      Nonempty (MerkleCompressionCollision compress) :=
    different_leaf_same_position_verified_openings_imply_merkle_compression_collision
      root
      opening
      otherOpening
      samePosition
      sameDepth
      verified
      otherVerified
      differentLeaf
  cases collisionWitness with
  | intro collision =>
      exact noCollision collision

theorem different_leaf_same_position_verified_openings_contradict_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root opening otherOpening,
      MerklePathIndex opening.layers = MerklePathIndex otherOpening.layers ->
        opening.layers.length = otherOpening.layers.length ->
          MerklePathOpeningVerifies compress root opening ->
            MerklePathOpeningVerifies compress root otherOpening ->
              otherOpening.leaf ≠ opening.leaf ->
                False := by
  intro root opening otherOpening samePosition sameDepth verified otherVerified
    differentLeaf
  exact
    different_leaf_same_position_verified_openings_contradict_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      opening
      otherOpening
      samePosition
      sameDepth
      verified
      otherVerified
      differentLeaf

theorem different_leaf_same_position_verified_openings_contradict_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening otherOpening,
      MerklePathIndex opening.layers = MerklePathIndex otherOpening.layers ->
        opening.layers.length = otherOpening.layers.length ->
          MerklePathOpeningVerifies compress root opening ->
            MerklePathOpeningVerifies compress root otherOpening ->
              otherOpening.leaf ≠ opening.leaf ->
                False := by
  intro root opening otherOpening samePosition sameDepth verified otherVerified
    differentLeaf
  exact
    different_leaf_same_position_verified_openings_contradict_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      opening
      otherOpening
      samePosition
      sameDepth
      verified
      otherVerified
      differentLeaf

theorem verified_concrete_merkle_path_same_position_leaf_eq_from_no_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (noCollision : MerkleCompressionNoCollision compress) :
    forall root leaf path otherLeaf otherPath,
      MerklePathIndex path = MerklePathIndex otherPath ->
        path.length = otherPath.length ->
          MerklePathVerifies compress root leaf path ->
            MerklePathVerifies compress root otherLeaf otherPath ->
              otherLeaf = leaf := by
  intro root leaf path otherLeaf otherPath samePosition sameDepth verified otherVerified
  by_contra differentLeaf
  exact
    False.elim
      (different_leaf_same_position_verified_paths_contradict_no_collision
        noCollision
        root
        leaf
        path
        otherLeaf
        otherPath
        samePosition
        sameDepth
        verified
        otherVerified
        differentLeaf)

theorem verified_concrete_merkle_opening_same_position_leaf_eq_from_no_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (noCollision : MerkleCompressionNoCollision compress) :
    forall root opening otherOpening,
      MerklePathIndex opening.layers = MerklePathIndex otherOpening.layers ->
        opening.layers.length = otherOpening.layers.length ->
          MerklePathOpeningVerifies compress root opening ->
            MerklePathOpeningVerifies compress root otherOpening ->
              otherOpening.leaf = opening.leaf := by
  intro root opening otherOpening samePosition sameDepth verified otherVerified
  by_contra differentLeaf
  exact
    False.elim
      (different_leaf_same_position_verified_openings_contradict_no_collision
        noCollision
        root
        opening
        otherOpening
        samePosition
        sameDepth
        verified
        otherVerified
        differentLeaf)

theorem verified_concrete_merkle_opening_same_position_leaf_eq_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root opening otherOpening,
      MerklePathIndex opening.layers = MerklePathIndex otherOpening.layers ->
        opening.layers.length = otherOpening.layers.length ->
          MerklePathOpeningVerifies compress root opening ->
            MerklePathOpeningVerifies compress root otherOpening ->
              otherOpening.leaf = opening.leaf := by
  intro root opening otherOpening samePosition sameDepth verified otherVerified
  exact
    verified_concrete_merkle_opening_same_position_leaf_eq_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      opening
      otherOpening
      samePosition
      sameDepth
      verified
      otherVerified

theorem verified_concrete_merkle_opening_same_position_leaf_eq_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening otherOpening,
      MerklePathIndex opening.layers = MerklePathIndex otherOpening.layers ->
        opening.layers.length = otherOpening.layers.length ->
          MerklePathOpeningVerifies compress root opening ->
            MerklePathOpeningVerifies compress root otherOpening ->
              otherOpening.leaf = opening.leaf := by
  intro root opening otherOpening samePosition sameDepth verified otherVerified
  exact
    verified_concrete_merkle_opening_same_position_leaf_eq_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      opening
      otherOpening
      samePosition
      sameDepth
      verified
      otherVerified

theorem concrete_merkle_path_same_index_binding
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (collisionFree : MerkleCompressionCollisionFree compress) :
    forall root leaf path otherLeaf otherPath,
      MerklePathSameIndex path otherPath ->
        MerklePathVerifies compress root leaf path ->
          MerklePathVerifies compress root otherLeaf otherPath ->
            otherLeaf = leaf := by
  intro root leaf path
  induction path generalizing root leaf with
  | nil =>
      intro otherLeaf otherPath sameIndex verified otherVerified
      cases otherPath with
      | nil =>
          simp [MerklePathVerifies, MerklePathFold] at verified otherVerified
          exact otherVerified.trans verified.symm
      | cons _ _ =>
          simp [MerklePathSameIndex] at sameIndex
  | cons layer rest ih =>
      intro otherLeaf otherPath sameIndex verified otherVerified
      cases otherPath with
      | nil =>
          simp [MerklePathSameIndex] at sameIndex
      | cons otherLayer otherRest =>
          have sameDirection :
              layer.direction = otherLayer.direction := by
            exact sameIndex.left
          have sameRest :
              MerklePathSameIndex rest otherRest := by
            exact sameIndex.right
          have sameParent :
              MerklePathLayer.parentDigest compress leaf layer =
                MerklePathLayer.parentDigest
                  compress
                  otherLeaf
                  otherLayer := by
            exact
              (ih
                root
                (MerklePathLayer.parentDigest compress leaf layer)
                (MerklePathLayer.parentDigest
                  compress
                  otherLeaf
                  otherLayer)
                otherRest
                sameRest
                verified
                otherVerified).symm
          exact
            (merkle_parent_digest_injective
              collisionFree
              sameDirection
              sameParent).symm

theorem concrete_merkle_path_same_index_binding_from_no_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (noCollision : MerkleCompressionNoCollision compress) :
    forall root leaf path otherLeaf otherPath,
      MerklePathSameIndex path otherPath ->
        MerklePathVerifies compress root leaf path ->
          MerklePathVerifies compress root otherLeaf otherPath ->
            otherLeaf = leaf := by
  intro root leaf path otherLeaf otherPath sameIndex verified otherVerified
  by_contra differentLeaf
  have collisionWitness :
      Nonempty (MerkleCompressionCollision compress) :=
    different_leaf_same_index_verified_paths_imply_merkle_compression_collision
      root
      leaf
      path
      otherLeaf
      otherPath
      sameIndex
      verified
      otherVerified
      differentLeaf
  cases collisionWitness with
  | intro collision =>
      exact noCollision collision

theorem verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (collisionFree : MerkleCompressionCollisionFree compress) :
    forall root leaf path,
      MerklePathVerifies compress root leaf path ->
        MerklePathRootCommitsToLeafAtIndex compress root leaf path := by
  intro root leaf path verified otherLeaf otherPath sameIndex otherVerified
  exact
    concrete_merkle_path_same_index_binding
      collisionFree
      root
      leaf
      path
      otherLeaf
      otherPath
      sameIndex
      verified
      otherVerified

theorem verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (noCollision : MerkleCompressionNoCollision compress) :
    forall root leaf path,
      MerklePathVerifies compress root leaf path ->
        MerklePathRootCommitsToLeafAtIndex compress root leaf path := by
  intro root leaf path verified otherLeaf otherPath sameIndex otherVerified
  exact
    concrete_merkle_path_same_index_binding_from_no_collision
      noCollision
      root
      leaf
      path
      otherLeaf
      otherPath
      sameIndex
      verified
      otherVerified

theorem verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root leaf path,
      MerklePathVerifies compress root leaf path ->
        MerklePathRootCommitsToLeafAtIndex compress root leaf path := by
  intro root leaf path verified
  exact
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      leaf
      path
      verified

theorem verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root leaf path,
      MerklePathVerifies compress root leaf path ->
        MerklePathRootCommitsToLeafAtIndex compress root leaf path := by
  intro root leaf path verified
  exact
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      leaf
      path
      verified

theorem verified_concrete_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (noCollision : MerkleCompressionNoCollision compress) :
    forall root leaf path,
      MerklePathVerifies compress root leaf path ->
        MerklePathRootCommitsToLeafAtPosition compress root leaf path := by
  intro root leaf path verified otherLeaf otherPath samePosition sameDepth otherVerified
  exact
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision
      noCollision
      root
      leaf
      path
      verified
      otherLeaf
      otherPath
      (merkle_path_same_position_implies_same_index
        path
        otherPath
        samePosition
        sameDepth)
      otherVerified

theorem verified_concrete_merkle_path_implies_root_commits_to_leaf_at_position_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root leaf path,
      MerklePathVerifies compress root leaf path ->
        MerklePathRootCommitsToLeafAtPosition compress root leaf path := by
  intro root leaf path verified
  exact
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      leaf
      path
      verified

theorem verified_concrete_merkle_path_implies_root_commits_to_leaf_at_position_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root leaf path,
      MerklePathVerifies compress root leaf path ->
        MerklePathRootCommitsToLeafAtPosition compress root leaf path := by
  intro root leaf path verified
  exact
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_position_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      leaf
      path
      verified

theorem verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_index_from_no_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (noCollision : MerkleCompressionNoCollision compress) :
    forall root opening,
      MerklePathOpeningVerifies compress root opening ->
        MerklePathRootCommitsToLeafAtIndex
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision
      noCollision
      root
      opening.leaf
      opening.layers
      verified

theorem verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_index
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (collisionFree : MerkleCompressionCollisionFree compress) :
    forall root opening,
      MerklePathOpeningVerifies compress root opening ->
        MerklePathRootCommitsToLeafAtIndex
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index
      collisionFree
      root
      opening.leaf
      opening.layers
      verified

theorem verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_index_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root opening,
      MerklePathOpeningVerifies compress root opening ->
        MerklePathRootCommitsToLeafAtIndex
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_index_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      opening
      verified

theorem verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_index_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening,
      MerklePathOpeningVerifies compress root opening ->
        MerklePathRootCommitsToLeafAtIndex
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_index_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      opening
      verified

theorem verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_position_from_no_collision
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (noCollision : MerkleCompressionNoCollision compress) :
    forall root opening,
      MerklePathOpeningVerifies compress root opening ->
        MerklePathRootCommitsToLeafAtPosition
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision
      noCollision
      root
      opening.leaf
      opening.layers
      verified

theorem verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_position
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (collisionFree : MerkleCompressionCollisionFree compress) :
    forall root opening,
      MerklePathOpeningVerifies compress root opening ->
        MerklePathRootCommitsToLeafAtPosition
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified otherLeaf otherPath samePosition sameDepth otherVerified
  exact
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index
      collisionFree
      root
      opening.leaf
      opening.layers
      verified
      otherLeaf
      otherPath
      (merkle_path_same_position_implies_same_index
        opening.layers
        otherPath
        samePosition
        sameDepth)
      otherVerified

theorem verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_position_from_assumption
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        hashAssumptions
        compress) :
    forall root opening,
      MerklePathOpeningVerifies compress root opening ->
        MerklePathRootCommitsToLeafAtPosition
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_position_from_no_collision
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      root
      opening
      verified

theorem verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_position_from_bundle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    forall root opening,
      MerklePathOpeningVerifies compress root opening ->
        MerklePathRootCommitsToLeafAtPosition
          compress
          root
          opening.leaf
          opening.layers := by
  intro root opening verified
  exact
    verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_position_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      opening
      verified

end Lzvm
