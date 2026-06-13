/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Assumptions

/-!
Merkle authentication path soundness derived from concrete path folding and a
centralized compression collision-resistance assumption.
-/

namespace Lzvm

universe uDigest

inductive MerklePathDirection where
  | currentOnLeft
  | currentOnRight
  deriving DecidableEq

structure MerklePathLayer (Digest : Type uDigest) where
  sibling : Digest
  direction : MerklePathDirection

structure MerklePathOpening (Digest : Type uDigest) where
  leaf : Digest
  layers : List (MerklePathLayer Digest)

namespace MerklePathLayer

def parentDigest
    {Digest : Type uDigest}
    (compress : Digest -> Digest -> Digest)
    (current : Digest)
    (layer : MerklePathLayer Digest) : Digest :=
  match layer.direction with
  | MerklePathDirection.currentOnLeft => compress current layer.sibling
  | MerklePathDirection.currentOnRight => compress layer.sibling current

end MerklePathLayer

def MerklePathFold
    {Digest : Type uDigest}
    (compress : Digest -> Digest -> Digest) :
    Digest -> List (MerklePathLayer Digest) -> Digest
  | leaf, [] => leaf
  | leaf, layer :: rest =>
      MerklePathFold
        compress
        (MerklePathLayer.parentDigest compress leaf layer)
        rest

def MerklePathVerifies
    {Digest : Type uDigest}
    (compress : Digest -> Digest -> Digest)
    (root : Digest)
    (leaf : Digest)
    (path : List (MerklePathLayer Digest)) : Prop :=
  MerklePathFold compress leaf path = root

def MerklePathOpeningVerifies
    {Digest : Type uDigest}
    (compress : Digest -> Digest -> Digest)
    (root : Digest)
    (opening : MerklePathOpening Digest) : Prop :=
  MerklePathVerifies compress root opening.leaf opening.layers

def MerklePathSameIndex
    {Digest : Type uDigest} :
    List (MerklePathLayer Digest) -> List (MerklePathLayer Digest) -> Prop
  | [], [] => True
  | leftLayer :: leftRest, rightLayer :: rightRest =>
      leftLayer.direction = rightLayer.direction
        /\ MerklePathSameIndex leftRest rightRest
  | _, _ => False

structure MerkleCompressionCollision
    {Digest : Type uDigest}
    (compress : Digest -> Digest -> Digest) where
  left : Digest
  right : Digest
  otherLeft : Digest
  otherRight : Digest
  sameDigest : compress left right = compress otherLeft otherRight
  differentInputs : left ≠ otherLeft \/ right ≠ otherRight

def MerkleCompressionNoCollision
    {Digest : Type uDigest}
    (compress : Digest -> Digest -> Digest) : Prop :=
  forall _ : MerkleCompressionCollision compress, False

def MerkleCompressionCollisionFree
    {Digest : Type uDigest}
    (compress : Digest -> Digest -> Digest) : Prop :=
  forall left right otherLeft otherRight,
    compress left right = compress otherLeft otherRight ->
      left = otherLeft /\ right = otherRight

def MerklePathRootCommitsToLeafAtIndex
    {Digest : Type uDigest}
    (compress : Digest -> Digest -> Digest)
    (root : Digest)
    (leaf : Digest)
    (path : List (MerklePathLayer Digest)) : Prop :=
  forall otherLeaf otherPath,
    MerklePathSameIndex path otherPath ->
      MerklePathVerifies compress root otherLeaf otherPath ->
      otherLeaf = leaf

def CentralizedMerkleCompressionCollisionResistance
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    (compress : Digest -> Digest -> Digest) : Prop :=
  hashAssumptions.merkleHashCollisionResistanceStatement =
    MerkleCompressionNoCollision compress

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

end Lzvm
