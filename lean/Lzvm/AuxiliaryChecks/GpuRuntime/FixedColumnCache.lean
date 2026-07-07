/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.GpuRuntime.Trace

/-!
GPU fixed-column cache auxiliary runtime contracts.
-/

namespace Lzvm

theorem fri_fixed_column_cache_same_request_implies_cached_contents_bound
    (validation : FriFixedColumnCacheValidation)
    (cached fresh : GpuAllocationSource) :
    validation.fixedColumnCacheRequestBound cached fresh ->
      (forall publicInput proof,
        validation.allocationValidation.writtenContentsBound fresh publicInput proof ->
          validation.allocationValidation.writtenContentsBound cached publicInput proof) := by
  intro requestBound publicInput proof freshBound
  have sameRequest :=
    validation.fixedColumnCacheRequestImpliesSameAllocationRequest
      cached
      fresh
      requestBound
  exact
    validation.allocationValidation.cachedReusePreservesWrittenContents
      cached
      fresh
      publicInput
      proof
      sameRequest
      freshBound

theorem fri_fixed_column_cache_checked_acceptance_projects_request_bound
    {system : VerifierModel}
    (validation : FriFixedColumnCacheValidation)
    (cached fresh : GpuAllocationSource) :
    forall publicInput proof,
      FriFixedColumnCacheCheckedAcceptance
          system
          validation
          cached
          fresh
          publicInput
          proof ->
        validation.fixedColumnCacheRequestBound cached fresh := by
  intro publicInput proof checked
  exact checked.right.left

theorem fri_fixed_column_cache_checked_acceptance_projects_fresh_contents_bound
    {system : VerifierModel}
    (validation : FriFixedColumnCacheValidation)
    (cached fresh : GpuAllocationSource) :
    forall publicInput proof,
      FriFixedColumnCacheCheckedAcceptance
          system
          validation
          cached
          fresh
          publicInput
          proof ->
        validation.allocationValidation.writtenContentsBound fresh publicInput proof := by
  intro publicInput proof checked
  exact checked.right.right

theorem fri_fixed_column_cache_checked_acceptance_projects_cached_contents_bound
    {system : VerifierModel}
    (validation : FriFixedColumnCacheValidation)
    (cached fresh : GpuAllocationSource) :
    forall publicInput proof,
      FriFixedColumnCacheCheckedAcceptance
          system
          validation
          cached
          fresh
          publicInput
          proof ->
        validation.allocationValidation.writtenContentsBound cached publicInput proof := by
  intro publicInput proof checked
  have cachedBound :=
    fri_fixed_column_cache_same_request_implies_cached_contents_bound
      validation
      cached
      fresh
      (fri_fixed_column_cache_checked_acceptance_projects_request_bound
        validation
        cached
        fresh
        publicInput
        proof
        checked)
  exact
    cachedBound
      publicInput
      proof
      (fri_fixed_column_cache_checked_acceptance_projects_fresh_contents_bound
        validation
        cached
        fresh
        publicInput
        proof
        checked)

theorem fri_fixed_column_cache_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : FriFixedColumnCacheValidation)
    (cached fresh : GpuAllocationSource) :
    forall publicInput proof,
      FriFixedColumnCacheCheckedAcceptance
          system
          validation
          cached
          fresh
          publicInput
          proof ->
        validation.allocationValidation.writtenContentsBound cached publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (fri_fixed_column_cache_checked_acceptance_projects_cached_contents_bound
        validation
        cached
        fresh
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness
        (auxiliaryAccepted := fun publicInput proof =>
          validation.fixedColumnCacheRequestBound cached fresh
            /\ validation.allocationValidation.writtenContentsBound fresh publicInput proof)
        assumptions
        publicInput
        proof
        checked)

theorem fri_fixed_column_cache_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : FriFixedColumnCacheValidation)
    (cached fresh : GpuAllocationSource) :
    forall publicInput proof,
      FriFixedColumnCacheCheckedAcceptance
          system
          validation
          cached
          fresh
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.fixedColumnCacheRequestBound cached fresh
          /\ validation.allocationValidation.writtenContentsBound fresh publicInput proof)
      assumptions
      publicInput
      proof
      checked

theorem fri_fixed_column_cache_checked_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : FriFixedColumnCacheValidation)
    (cached fresh : GpuAllocationSource) :
    forall publicInput proof,
      FriFixedColumnCacheCheckedAcceptance
          system
          validation
          cached
          fresh
          publicInput
          proof ->
        validation.fixedColumnCacheRequestBound cached fresh
          /\ validation.allocationValidation.writtenContentsBound fresh publicInput proof
          /\ validation.allocationValidation.writtenContentsBound cached publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have requestBound :=
    fri_fixed_column_cache_checked_acceptance_projects_request_bound
      validation
      cached
      fresh
      publicInput
      proof
      checked
  have freshBound :=
    fri_fixed_column_cache_checked_acceptance_projects_fresh_contents_bound
      validation
      cached
      fresh
      publicInput
      proof
      checked
  have cachedBound :=
    fri_fixed_column_cache_checked_acceptance_projects_cached_contents_bound
      validation
      cached
      fresh
      publicInput
      proof
      checked
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      (auxiliaryAccepted := fun publicInput proof =>
        validation.fixedColumnCacheRequestBound cached fresh
          /\ validation.allocationValidation.writtenContentsBound fresh publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact And.intro requestBound
    (And.intro freshBound
      (And.intro cachedBound coreAndSound))

theorem fri_fixed_column_cache_checked_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : FriFixedColumnCacheValidation)
    (cached fresh : GpuAllocationSource) :
    forall publicInput proof,
      FriFixedColumnCacheCheckedAcceptance
          system
          validation
          cached
          fresh
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ validation.fixedColumnCacheRequestBound cached fresh
          /\ validation.allocationValidation.writtenContentsBound fresh publicInput proof
          /\ validation.allocationValidation.writtenContentsBound cached publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  have contracts :=
    fri_fixed_column_cache_checked_acceptance_core_and_sound
      assumptions
      validation
      cached
      fresh
      publicInput
      proof
      checked
  exact
    And.intro auditedAssumptions.left
      (And.intro auditedAssumptions.right contracts)

end Lzvm
