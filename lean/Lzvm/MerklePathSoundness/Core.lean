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

def NAryMerklePathIndex
    {Digest : Type uDigest} :
    List (NAryMerklePathLayer Digest) -> Nat
  | [] => 0
  | layer :: rest =>
      NAryMerklePathLayer.childSlot layer
        + NAryMerklePathLayer.arity layer * NAryMerklePathIndex rest

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

def NAryMerklePathLayerHasArity
    {Digest : Type uDigest}
    (arity : Nat)
    (layer : NAryMerklePathLayer Digest) : Prop :=
  NAryMerklePathLayer.arity layer = arity

def NAryMerklePathHasArity
    {Digest : Type uDigest}
    (arity : Nat) :
    List (NAryMerklePathLayer Digest) -> Prop
  | [] => True
  | layer :: rest =>
      NAryMerklePathLayerHasArity arity layer
        /\ NAryMerklePathHasArity arity rest

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

def NAryMerklePathRootCommitsToLeafAtSamePositionIndex
    {Digest : Type uDigest}
    (compress : List Digest -> Digest)
    (root : Digest)
    (leaf : Digest)
    (path : List (NAryMerklePathLayer Digest)) : Prop :=
  forall otherLeaf otherPath,
    NAryMerklePathSamePosition path otherPath ->
      NAryMerklePathIndex path = NAryMerklePathIndex otherPath ->
        path.length = otherPath.length ->
          NAryMerklePathVerifies compress root otherLeaf otherPath ->
          otherLeaf = leaf

def NAryMerklePathRootCommitsToLeafAtIndex
    {Digest : Type uDigest}
    (compress : List Digest -> Digest)
    (root : Digest)
    (leaf : Digest)
    (path : List (NAryMerklePathLayer Digest)) : Prop :=
  NAryMerklePathRootCommitsToLeafAtSamePositionIndex
    compress
    root
    leaf
    path

def CentralizedNAryMerkleCompressionCollisionResistance
    {Digest : Type uDigest}
    (hashAssumptions : HashCollisionResistanceAssumption)
    (compress : List Digest -> Digest) : Prop :=
  hashAssumptions.merkleHashCollisionResistanceStatement =
    NAryMerkleCompressionNoCollision compress

end Lzvm
