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

namespace MerklePathDirection

def indexBit : MerklePathDirection -> Nat
  | currentOnLeft => 0
  | currentOnRight => 1

end MerklePathDirection

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

def MerklePathIndex
    {Digest : Type uDigest} :
    List (MerklePathLayer Digest) -> Nat
  | [] => 0
  | layer :: rest =>
      MerklePathDirection.indexBit layer.direction + 2 * MerklePathIndex rest

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

def MerklePathRootCommitsToLeafAtPosition
    {Digest : Type uDigest}
    (compress : Digest -> Digest -> Digest)
    (root : Digest)
    (leaf : Digest)
    (path : List (MerklePathLayer Digest)) : Prop :=
  forall otherLeaf otherPath,
    MerklePathIndex path = MerklePathIndex otherPath ->
      path.length = otherPath.length ->
        MerklePathVerifies compress root otherLeaf otherPath ->
          otherLeaf = leaf

def CentralizedMerkleCompressionCollisionResistance
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    (compress : Digest -> Digest -> Digest) : Prop :=
  hashAssumptions.merkleHashCollisionResistanceStatement =
    MerkleCompressionNoCollision compress

structure NAryMerklePathLayer (Digest : Type uDigest) where
  leftSiblings : List Digest
  rightSiblings : List Digest

structure NAryMerklePathOpening (Digest : Type uDigest) where
  leaf : Digest
  layers : List (NAryMerklePathLayer Digest)

namespace NAryMerklePathLayer

def children
    {Digest : Type uDigest}
    (current : Digest)
    (layer : NAryMerklePathLayer Digest) : List Digest :=
  layer.leftSiblings ++ current :: layer.rightSiblings

def childSlot
    {Digest : Type uDigest}
    (layer : NAryMerklePathLayer Digest) : Nat :=
  layer.leftSiblings.length

def arity
    {Digest : Type uDigest}
    (layer : NAryMerklePathLayer Digest) : Nat :=
  layer.leftSiblings.length + 1 + layer.rightSiblings.length

def parentDigest
    {Digest : Type uDigest}
    (compress : List Digest -> Digest)
    (current : Digest)
    (layer : NAryMerklePathLayer Digest) : Digest :=
  compress (children current layer)

end NAryMerklePathLayer

def NAryMerklePathFold
    {Digest : Type uDigest}
    (compress : List Digest -> Digest) :
    Digest -> List (NAryMerklePathLayer Digest) -> Digest
  | leaf, [] => leaf
  | leaf, layer :: rest =>
      NAryMerklePathFold
        compress
        (NAryMerklePathLayer.parentDigest compress leaf layer)
        rest

def NAryMerklePathVerifies
    {Digest : Type uDigest}
    (compress : List Digest -> Digest)
    (root : Digest)
    (leaf : Digest)
    (path : List (NAryMerklePathLayer Digest)) : Prop :=
  NAryMerklePathFold compress leaf path = root

def NAryMerklePathOpeningVerifies
    {Digest : Type uDigest}
    (compress : List Digest -> Digest)
    (root : Digest)
    (opening : NAryMerklePathOpening Digest) : Prop :=
  NAryMerklePathVerifies compress root opening.leaf opening.layers

def NAryMerklePathSamePosition
    {Digest : Type uDigest} :
    List (NAryMerklePathLayer Digest) ->
      List (NAryMerklePathLayer Digest) ->
        Prop
  | [], [] => True
  | leftLayer :: leftRest, rightLayer :: rightRest =>
      leftLayer.leftSiblings.length = rightLayer.leftSiblings.length
        /\ leftLayer.rightSiblings.length = rightLayer.rightSiblings.length
        /\ NAryMerklePathSamePosition leftRest rightRest
  | _, _ => False

structure NAryMerkleCompressionCollision
    {Digest : Type uDigest}
    (compress : List Digest -> Digest) where
  children : List Digest
  otherChildren : List Digest
  sameDigest : compress children = compress otherChildren
  differentInputs : children ≠ otherChildren

def NAryMerkleCompressionNoCollision
    {Digest : Type uDigest}
    (compress : List Digest -> Digest) : Prop :=
  forall _ : NAryMerkleCompressionCollision compress, False

def NAryMerkleCompressionCollisionFree
    {Digest : Type uDigest}
    (compress : List Digest -> Digest) : Prop :=
  forall children otherChildren,
    compress children = compress otherChildren ->
      children = otherChildren

def NAryMerklePathRootCommitsToLeafAtPosition
    {Digest : Type uDigest}
    (compress : List Digest -> Digest)
    (root : Digest)
    (leaf : Digest)
    (path : List (NAryMerklePathLayer Digest)) : Prop :=
  forall otherLeaf otherPath,
    NAryMerklePathSamePosition path otherPath ->
      NAryMerklePathVerifies compress root otherLeaf otherPath ->
      otherLeaf = leaf

def CentralizedNAryMerkleCompressionCollisionResistance
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    (compress : List Digest -> Digest) : Prop :=
  hashAssumptions.merkleHashCollisionResistanceStatement =
    NAryMerkleCompressionNoCollision compress

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
