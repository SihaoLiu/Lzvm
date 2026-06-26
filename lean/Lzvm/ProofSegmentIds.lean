/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Mathlib

/-!
Concrete proof segment identity allowlist used by runtime artifact validation.
-/

namespace Lzvm

def witnessCommitmentSegmentBaseId : Nat := 100
def pcsMaterialManifestSegmentId : Nat := 10000
def pcsQueryPlanSegmentId : Nat := 10001
def witnessOpeningSegmentId : Nat := 10002
def constantOpeningSegmentId : Nat := 10003
def pcsFriOpeningSegmentId : Nat := 10004
def pcsQueryNonceSegmentId : Nat := 10005
def pcsEvaluationSegmentId : Nat := 10006
def pcsProofValuesSegmentId : Nat := 10007
def groupValuesSegmentId : Nat := 10008
def unitValuesSegmentId : Nat := 10009
def programImageCacheSegmentId : Nat := 10010
def contributionSegmentId : Nat := 10011
def challengeValuesSegmentId : Nat := 10012
def ethBlockInputSegmentId : Nat := 10013
def traceConstraintSegmentId : Nat := 10014
def framedGuestInputSegmentId : Nat := 10015

def fixedProofSegmentIds : List Nat :=
  [ pcsMaterialManifestSegmentId,
    pcsQueryPlanSegmentId,
    witnessOpeningSegmentId,
    constantOpeningSegmentId,
    pcsFriOpeningSegmentId,
    pcsQueryNonceSegmentId,
    pcsEvaluationSegmentId,
    pcsProofValuesSegmentId,
    groupValuesSegmentId,
    unitValuesSegmentId,
    programImageCacheSegmentId,
    contributionSegmentId,
    challengeValuesSegmentId,
    ethBlockInputSegmentId,
    traceConstraintSegmentId,
    framedGuestInputSegmentId
  ]

def IsWitnessCommitmentSegmentId (id : Nat) : Prop :=
  witnessCommitmentSegmentBaseId <= id /\ id < pcsMaterialManifestSegmentId

def isFixedProofSegmentIdBool (id : Nat) : Bool :=
  fixedProofSegmentIds.contains id

def IsFixedProofSegmentId (id : Nat) : Prop :=
  isFixedProofSegmentIdBool id = true

def IsAllowedProofSegmentId (id : Nat) : Prop :=
  IsWitnessCommitmentSegmentId id \/ IsFixedProofSegmentId id

instance isWitnessCommitmentSegmentIdDecidable (id : Nat) :
    Decidable (IsWitnessCommitmentSegmentId id) := by
  unfold IsWitnessCommitmentSegmentId
  infer_instance

instance isFixedProofSegmentIdDecidable (id : Nat) :
    Decidable (IsFixedProofSegmentId id) := by
  unfold IsFixedProofSegmentId
  infer_instance

instance isAllowedProofSegmentIdDecidable (id : Nat) :
    Decidable (IsAllowedProofSegmentId id) := by
  unfold IsAllowedProofSegmentId
  infer_instance

theorem witness_commitment_base_id_allowed :
    IsAllowedProofSegmentId witnessCommitmentSegmentBaseId := by
  left
  exact And.intro (by decide) (by decide)

theorem last_witness_commitment_id_allowed :
    IsAllowedProofSegmentId 9999 := by
  left
  exact And.intro (by decide) (by decide)

theorem pcs_material_manifest_segment_id_allowed :
    IsAllowedProofSegmentId pcsMaterialManifestSegmentId := by
  right
  decide

theorem pcs_query_plan_segment_id_allowed :
    IsAllowedProofSegmentId pcsQueryPlanSegmentId := by
  right
  decide

theorem witness_opening_segment_id_allowed :
    IsAllowedProofSegmentId witnessOpeningSegmentId := by
  right
  decide

theorem constant_opening_segment_id_allowed :
    IsAllowedProofSegmentId constantOpeningSegmentId := by
  right
  decide

theorem pcs_fri_opening_segment_id_allowed :
    IsAllowedProofSegmentId pcsFriOpeningSegmentId := by
  right
  decide

theorem pcs_query_nonce_segment_id_allowed :
    IsAllowedProofSegmentId pcsQueryNonceSegmentId := by
  right
  decide

theorem pcs_evaluation_segment_id_allowed :
    IsAllowedProofSegmentId pcsEvaluationSegmentId := by
  right
  decide

theorem pcs_proof_values_segment_id_allowed :
    IsAllowedProofSegmentId pcsProofValuesSegmentId := by
  right
  decide

theorem group_values_segment_id_allowed :
    IsAllowedProofSegmentId groupValuesSegmentId := by
  right
  decide

theorem unit_values_segment_id_allowed :
    IsAllowedProofSegmentId unitValuesSegmentId := by
  right
  decide

theorem program_image_cache_segment_id_allowed :
    IsAllowedProofSegmentId programImageCacheSegmentId := by
  right
  decide

theorem contribution_segment_id_allowed :
    IsAllowedProofSegmentId contributionSegmentId := by
  right
  decide

theorem challenge_values_segment_id_allowed :
    IsAllowedProofSegmentId challengeValuesSegmentId := by
  right
  decide

theorem eth_block_input_segment_id_allowed :
    IsAllowedProofSegmentId ethBlockInputSegmentId := by
  right
  decide

theorem trace_constraint_segment_id_allowed :
    IsAllowedProofSegmentId traceConstraintSegmentId := by
  right
  decide

theorem framed_guest_input_segment_id_allowed :
    IsAllowedProofSegmentId framedGuestInputSegmentId := by
  right
  decide

theorem reserved_proof_segment_id_not_allowed :
    Not (IsAllowedProofSegmentId 99) := by
  decide

theorem unknown_fixed_proof_segment_id_not_allowed :
    Not (IsAllowedProofSegmentId 20000) := by
  decide

end Lzvm
