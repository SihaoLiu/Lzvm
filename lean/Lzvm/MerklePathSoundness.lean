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
    MerkleCompressionCollisionFree compress

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
    Eq.mp
      centralized
      hashAssumptions.merkleHashCollisionResistance.evidence

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
    verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index
      (centralized_merkle_compression_collision_free
        hashAssumptions
        centralized)
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
