/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Assumptions

/-!
Merkle authentication path soundness derived from the centralized hash assumption.
-/

namespace Lzvm

universe uRoot uLeaf uPath

structure MerklePathModel
    (Root : Type uRoot)
    (Leaf : Type uLeaf)
    (Path : Type uPath) where
  verifies : Root -> Leaf -> Path -> Prop

variable {Root : Type uRoot}
variable {Leaf : Type uLeaf}
variable {Path : Type uPath}

def MerklePathCollisionResistance
    (model : MerklePathModel Root Leaf Path) : Prop :=
  forall root leaf path otherLeaf otherPath,
    model.verifies root leaf path ->
      model.verifies root otherLeaf otherPath ->
        otherLeaf = leaf

def MerkleRootCommitsToLeaf
    (model : MerklePathModel Root Leaf Path)
    (root : Root)
    (leaf : Leaf) : Prop :=
  forall otherLeaf otherPath,
    model.verifies root otherLeaf otherPath ->
      otherLeaf = leaf

def CentralizedMerklePathCollisionResistance
    (hashAssumptions : HashCollisionResistanceAssumption)
    (model : MerklePathModel Root Leaf Path) : Prop :=
  hashAssumptions.merkleHashCollisionResistanceStatement =
    MerklePathCollisionResistance model

theorem centralized_merkle_path_collision_resistance
    (hashAssumptions : HashCollisionResistanceAssumption)
    {model : MerklePathModel Root Leaf Path}
    (centralized :
      CentralizedMerklePathCollisionResistance hashAssumptions model) :
    MerklePathCollisionResistance model := by
  exact
    Eq.mp
      centralized
      hashAssumptions.merkleHashCollisionResistance.evidence

theorem verified_merkle_path_implies_root_commits_to_leaf
    {model : MerklePathModel Root Leaf Path}
    (collisionResistance : MerklePathCollisionResistance model) :
    forall root leaf path,
      model.verifies root leaf path ->
        MerkleRootCommitsToLeaf model root leaf := by
  intro root leaf path verified otherLeaf otherPath otherVerified
  exact
    collisionResistance
      root
      leaf
      path
      otherLeaf
      otherPath
      verified
      otherVerified

theorem verified_merkle_path_implies_root_commits_to_leaf_from_assumption
    (hashAssumptions : HashCollisionResistanceAssumption)
    {model : MerklePathModel Root Leaf Path}
    (centralized :
      CentralizedMerklePathCollisionResistance hashAssumptions model) :
    forall root leaf path,
      model.verifies root leaf path ->
        MerkleRootCommitsToLeaf model root leaf := by
  intro root leaf path verified
  exact
    verified_merkle_path_implies_root_commits_to_leaf
      (centralized_merkle_path_collision_resistance hashAssumptions centralized)
      root
      leaf
      path
      verified

theorem verified_merkle_path_implies_root_commits_to_leaf_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {model : MerklePathModel Root Leaf Path}
    (centralized :
      CentralizedMerklePathCollisionResistance
        assumptions.crypto.hashCollisionResistance
        model) :
    forall root leaf path,
      model.verifies root leaf path ->
        MerkleRootCommitsToLeaf model root leaf := by
  intro root leaf path verified
  exact
    verified_merkle_path_implies_root_commits_to_leaf_from_assumption
      assumptions.crypto.hashCollisionResistance
      centralized
      root
      leaf
      path
      verified

end Lzvm
