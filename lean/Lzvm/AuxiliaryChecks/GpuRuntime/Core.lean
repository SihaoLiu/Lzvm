/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.GpuRuntime.Common

/-!
GPU auxiliary runtime cache and reuse contracts.
-/

namespace Lzvm

theorem gpu_setup_checked_acceptance_projects_constants_sound
    {system : VerifierModel}
    (validation : GpuSetupCacheValidation)
    (request : GpuSetupRequest) :
    forall publicInput proof,
      GpuSetupCheckedAcceptance system validation request publicInput proof ->
        validation.constantsSoundFor request.device request.requiredBits := by
  intro publicInput proof checked
  exact checked.right

theorem gpu_setup_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuSetupCacheValidation)
    (request : GpuSetupRequest) :
    forall publicInput proof,
      GpuSetupCheckedAcceptance system validation request publicInput proof ->
        validation.constantsSoundFor request.device request.requiredBits
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithSetup
  exact
    And.intro
      (gpu_setup_checked_acceptance_projects_constants_sound
        validation
        request
        publicInput
        proof
        acceptedWithSetup)
      (GpuRuntimeInternal.checked_acceptance_sound_witness
        (auxiliaryAccepted := fun _publicInput _proof =>
          validation.constantsSoundFor request.device request.requiredBits)
        assumptions
        publicInput
        proof
        acceptedWithSetup)

theorem gpu_setup_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuSetupCacheValidation)
    (request : GpuSetupRequest) :
    forall publicInput proof,
      GpuSetupCheckedAcceptance system validation request publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      (auxiliaryAccepted := fun _publicInput _proof =>
        validation.constantsSoundFor request.device request.requiredBits)
      assumptions
      publicInput
      proof
      checked

theorem gpu_setup_checked_acceptance_constants_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuSetupCacheValidation)
    (request : GpuSetupRequest) :
    forall publicInput proof,
      GpuSetupCheckedAcceptance system validation request publicInput proof ->
        validation.constantsSoundFor request.device request.requiredBits
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_setup_checked_acceptance_sound
      assumptions
      validation
      request
      publicInput
      proof
      checked
  have core :=
    gpu_setup_checked_acceptance_verifier_core_contract
      assumptions
      validation
      request
      publicInput
      proof
      checked
  exact And.intro sound.left (And.intro core sound.right)

theorem gpu_allocation_cache_reuse_preserves_written_contents
    (validation : GpuAllocationCacheValidation)
    (cached fresh : GpuAllocationSource) :
    GpuAllocationSameRequest cached fresh ->
      (forall publicInput proof,
        validation.writtenContentsBound fresh publicInput proof ->
          validation.writtenContentsBound cached publicInput proof) := by
  intro sameRequest publicInput proof freshBound
  exact
    validation.cachedReusePreservesWrittenContents
      cached
      fresh
      publicInput
      proof
      sameRequest
      freshBound

theorem gpu_allocation_checked_acceptance_projects_written_contents
    {system : VerifierModel}
    (validation : GpuAllocationCacheValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocationCheckedAcceptance system validation allocation publicInput proof ->
        validation.writtenContentsBound allocation publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem gpu_allocation_checked_acceptance_projects_cached_written_contents
    {system : VerifierModel}
    (validation : GpuAllocationCacheValidation)
    (cached fresh : GpuAllocationSource) :
    GpuAllocationSameRequest cached fresh ->
      forall publicInput proof,
        GpuAllocationCheckedAcceptance system validation fresh publicInput proof ->
          validation.writtenContentsBound cached publicInput proof := by
  intro sameRequest publicInput proof checked
  exact
    gpu_allocation_cache_reuse_preserves_written_contents
      validation
      cached
      fresh
      sameRequest
      publicInput
      proof
      (gpu_allocation_checked_acceptance_projects_written_contents
        validation
        fresh
        publicInput
        proof
        checked)

theorem gpu_allocation_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuAllocationCacheValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocationCheckedAcceptance system validation allocation publicInput proof ->
        validation.writtenContentsBound allocation publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithAllocation
  exact
    And.intro
      (gpu_allocation_checked_acceptance_projects_written_contents
        validation
        allocation
        publicInput
        proof
        acceptedWithAllocation)
      (GpuRuntimeInternal.checked_acceptance_sound_witness
        assumptions
        publicInput
        proof
        acceptedWithAllocation)

theorem gpu_allocation_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuAllocationCacheValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocationCheckedAcceptance system validation allocation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem gpu_allocation_checked_acceptance_written_contents_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuAllocationCacheValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocationCheckedAcceptance system validation allocation publicInput proof ->
        validation.writtenContentsBound allocation publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_allocation_checked_acceptance_sound
      assumptions
      validation
      allocation
      publicInput
      proof
      checked
  have core :=
    gpu_allocation_checked_acceptance_verifier_core_contract
      assumptions
      validation
      allocation
      publicInput
      proof
      checked
  exact And.intro sound.left (And.intro core sound.right)

theorem gpu_host_device_copy_round_trip_implies_written_contents
    (validation : GpuHostDeviceCopyRoundTripValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      validation.uploadedBytesRoundTrip allocation publicInput proof ->
        validation.allocationValidation.writtenContentsBound allocation publicInput proof := by
  intro publicInput proof roundTrip
  exact
    validation.roundTripImpliesWrittenContents
      allocation
      publicInput
      proof
      roundTrip

theorem gpu_host_device_copy_round_trip_checked_acceptance_projects_round_trip
    {system : VerifierModel}
    (validation : GpuHostDeviceCopyRoundTripValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuHostDeviceCopyRoundTripCheckedAcceptance
          system
          validation
          allocation
          publicInput
          proof ->
        validation.uploadedBytesRoundTrip allocation publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem gpu_host_device_copy_round_trip_checked_acceptance_projects_written_contents
    {system : VerifierModel}
    (validation : GpuHostDeviceCopyRoundTripValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuHostDeviceCopyRoundTripCheckedAcceptance
          system
          validation
          allocation
          publicInput
          proof ->
        validation.allocationValidation.writtenContentsBound allocation publicInput proof := by
  intro publicInput proof checked
  exact
    gpu_host_device_copy_round_trip_implies_written_contents
      validation
      allocation
      publicInput
      proof
      (gpu_host_device_copy_round_trip_checked_acceptance_projects_round_trip
        validation
        allocation
        publicInput
        proof
        checked)

theorem gpu_host_device_copy_round_trip_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuHostDeviceCopyRoundTripValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuHostDeviceCopyRoundTripCheckedAcceptance
          system
          validation
          allocation
          publicInput
          proof ->
        validation.allocationValidation.writtenContentsBound allocation publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (gpu_host_device_copy_round_trip_checked_acceptance_projects_written_contents
        validation
        allocation
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness
        assumptions
        publicInput
        proof
        checked)

theorem gpu_host_device_copy_round_trip_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuHostDeviceCopyRoundTripValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuHostDeviceCopyRoundTripCheckedAcceptance
          system
          validation
          allocation
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem gpu_host_device_copy_round_trip_checked_acceptance_written_contents_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuHostDeviceCopyRoundTripValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuHostDeviceCopyRoundTripCheckedAcceptance
          system
          validation
          allocation
          publicInput
          proof ->
        validation.allocationValidation.writtenContentsBound allocation publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_host_device_copy_round_trip_checked_acceptance_sound
      assumptions
      validation
      allocation
      publicInput
      proof
      checked
  have core :=
    gpu_host_device_copy_round_trip_checked_acceptance_verifier_core_contract
      assumptions
      validation
      allocation
      publicInput
      proof
      checked
  exact And.intro sound.left (And.intro core sound.right)

theorem gpu_temporary_buffer_reuse_implies_same_request
    (validation : GpuTemporaryBufferReuseValidation)
    (previous next : GpuAllocationSource) :
    forall publicInput proof,
      validation.temporaryBufferReuseAllowed previous next publicInput proof ->
        GpuAllocationSameRequest previous next := by
  intro publicInput proof reuseAllowed
  exact
    validation.temporaryBufferReuseImpliesSameRequest
      previous
      next
      publicInput
      proof
      reuseAllowed

theorem gpu_temporary_buffer_reuse_implies_pending_reads_complete
    (validation : GpuTemporaryBufferReuseValidation)
    (previous next : GpuAllocationSource) :
    forall publicInput proof,
      validation.temporaryBufferReuseAllowed previous next publicInput proof ->
        validation.pendingDeviceReadsComplete previous publicInput proof := by
  intro publicInput proof reuseAllowed
  exact
    validation.temporaryBufferReuseImpliesPendingReadsComplete
      previous
      next
      publicInput
      proof
      reuseAllowed

theorem gpu_temporary_buffer_reuse_checked_acceptance_projects_same_request
    {system : VerifierModel}
    (validation : GpuTemporaryBufferReuseValidation)
    (previous next : GpuAllocationSource) :
    forall publicInput proof,
      GpuTemporaryBufferReuseCheckedAcceptance
          system
          validation
          previous
          next
          publicInput
          proof ->
        GpuAllocationSameRequest previous next := by
  intro publicInput proof checked
  exact
    gpu_temporary_buffer_reuse_implies_same_request
      validation
      previous
      next
      publicInput
      proof
      checked.right

theorem gpu_temporary_buffer_reuse_checked_acceptance_projects_pending_reads_complete
    {system : VerifierModel}
    (validation : GpuTemporaryBufferReuseValidation)
    (previous next : GpuAllocationSource) :
    forall publicInput proof,
      GpuTemporaryBufferReuseCheckedAcceptance
          system
          validation
          previous
          next
          publicInput
          proof ->
        validation.pendingDeviceReadsComplete previous publicInput proof := by
  intro publicInput proof checked
  exact
    gpu_temporary_buffer_reuse_implies_pending_reads_complete
      validation
      previous
      next
      publicInput
      proof
      checked.right

theorem gpu_temporary_buffer_reuse_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuTemporaryBufferReuseValidation)
    (previous next : GpuAllocationSource) :
    forall publicInput proof,
      GpuTemporaryBufferReuseCheckedAcceptance
          system
          validation
          previous
          next
          publicInput
          proof ->
        GpuAllocationSameRequest previous next
          /\ validation.pendingDeviceReadsComplete previous publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have sameRequest :=
    gpu_temporary_buffer_reuse_checked_acceptance_projects_same_request
      validation
      previous
      next
      publicInput
      proof
      checked
  have pendingComplete :=
    gpu_temporary_buffer_reuse_checked_acceptance_projects_pending_reads_complete
      validation
      previous
      next
      publicInput
      proof
      checked
  exact
    And.intro sameRequest
      (And.intro pendingComplete
        (GpuRuntimeInternal.checked_acceptance_sound_witness
          assumptions
          publicInput
          proof
          checked))

theorem gpu_temporary_buffer_reuse_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuTemporaryBufferReuseValidation)
    (previous next : GpuAllocationSource) :
    forall publicInput proof,
      GpuTemporaryBufferReuseCheckedAcceptance
          system
          validation
          previous
          next
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem gpu_temporary_buffer_reuse_checked_acceptance_reuse_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuTemporaryBufferReuseValidation)
    (previous next : GpuAllocationSource) :
    forall publicInput proof,
      GpuTemporaryBufferReuseCheckedAcceptance
          system
          validation
          previous
          next
          publicInput
          proof ->
        GpuAllocationSameRequest previous next
          /\ validation.pendingDeviceReadsComplete previous publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_temporary_buffer_reuse_checked_acceptance_sound
      assumptions
      validation
      previous
      next
      publicInput
      proof
      checked
  have core :=
    gpu_temporary_buffer_reuse_checked_acceptance_verifier_core_contract
      assumptions
      validation
      previous
      next
      publicInput
      proof
      checked
  exact And.intro sound.left
    (And.intro sound.right.left (And.intro core sound.right.right))

theorem gpu_allocator_no_wait_bypass_implies_same_request
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource) :
    forall publicInput proof,
      validation.noWaitBypassAllowed pending fresh publicInput proof ->
        GpuAllocationSameRequest pending fresh := by
  intro publicInput proof bypassAllowed
  exact
    validation.noWaitBypassImpliesSameRequest
      pending
      fresh
      publicInput
      proof
      bypassAllowed

theorem gpu_allocator_no_wait_bypass_implies_pending_not_reused
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource) :
    forall publicInput proof,
      validation.noWaitBypassAllowed pending fresh publicInput proof ->
        validation.pendingAllocationNotReused pending publicInput proof := by
  intro publicInput proof bypassAllowed
  exact
    validation.noWaitBypassImpliesPendingNotReused
      pending
      fresh
      publicInput
      proof
      bypassAllowed

theorem gpu_allocator_no_wait_bypass_implies_fresh_allocation
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource) :
    forall publicInput proof,
      validation.noWaitBypassAllowed pending fresh publicInput proof ->
        validation.freshAllocationIssued fresh publicInput proof := by
  intro publicInput proof bypassAllowed
  exact
    validation.noWaitBypassImpliesFreshAllocation
      pending
      fresh
      publicInput
      proof
      bypassAllowed

theorem gpu_allocator_no_wait_bypass_checked_acceptance_projects_same_request
    {system : VerifierModel}
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocatorNoWaitBypassCheckedAcceptance
          system
          validation
          pending
          fresh
          publicInput
          proof ->
        GpuAllocationSameRequest pending fresh := by
  intro publicInput proof checked
  exact
    gpu_allocator_no_wait_bypass_implies_same_request
      validation
      pending
      fresh
      publicInput
      proof
      checked.right

theorem gpu_allocator_no_wait_bypass_checked_acceptance_projects_pending_not_reused
    {system : VerifierModel}
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocatorNoWaitBypassCheckedAcceptance
          system
          validation
          pending
          fresh
          publicInput
          proof ->
        validation.pendingAllocationNotReused pending publicInput proof := by
  intro publicInput proof checked
  exact
    gpu_allocator_no_wait_bypass_implies_pending_not_reused
      validation
      pending
      fresh
      publicInput
      proof
      checked.right

theorem gpu_allocator_no_wait_bypass_checked_acceptance_projects_fresh_allocation
    {system : VerifierModel}
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocatorNoWaitBypassCheckedAcceptance
          system
          validation
          pending
          fresh
          publicInput
          proof ->
        validation.freshAllocationIssued fresh publicInput proof := by
  intro publicInput proof checked
  exact
    gpu_allocator_no_wait_bypass_implies_fresh_allocation
      validation
      pending
      fresh
      publicInput
      proof
      checked.right

theorem gpu_allocator_no_wait_bypass_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocatorNoWaitBypassCheckedAcceptance
          system
          validation
          pending
          fresh
          publicInput
          proof ->
        GpuAllocationSameRequest pending fresh
          /\ validation.pendingAllocationNotReused pending publicInput proof
          /\ validation.freshAllocationIssued fresh publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have sameRequest :=
    gpu_allocator_no_wait_bypass_checked_acceptance_projects_same_request
      validation
      pending
      fresh
      publicInput
      proof
      checked
  have pendingNotReused :=
    gpu_allocator_no_wait_bypass_checked_acceptance_projects_pending_not_reused
      validation
      pending
      fresh
      publicInput
      proof
      checked
  have freshIssued :=
    gpu_allocator_no_wait_bypass_checked_acceptance_projects_fresh_allocation
      validation
      pending
      fresh
      publicInput
      proof
      checked
  exact
    And.intro sameRequest
      (And.intro pendingNotReused
        (And.intro freshIssued
          (GpuRuntimeInternal.checked_acceptance_sound_witness
            assumptions
            publicInput
            proof
            checked)))

theorem gpu_allocator_no_wait_bypass_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocatorNoWaitBypassCheckedAcceptance
          system
          validation
          pending
          fresh
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem gpu_allocator_no_wait_limit_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : GpuAllocatorNoWaitLimitValidation)
    (config : GpuAllocatorNoWaitLimitConfig) :
    forall publicInput proof,
      GpuAllocatorNoWaitLimitCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GpuAllocatorNoWaitLimitDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.noWaitLimitConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem gpu_allocator_no_wait_limit_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuAllocatorNoWaitLimitValidation)
    (config : GpuAllocatorNoWaitLimitConfig) :
    forall publicInput proof,
      GpuAllocatorNoWaitLimitCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GpuAllocatorNoWaitLimitDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (gpu_allocator_no_wait_limit_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness
        assumptions
        publicInput
        proof
        checked)

theorem gpu_allocator_no_wait_limit_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuAllocatorNoWaitLimitValidation)
    (config : GpuAllocatorNoWaitLimitConfig) :
    forall publicInput proof,
      GpuAllocatorNoWaitLimitCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem guest_pc_trace_segment_queue_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : GuestPcTraceSegmentQueueValidation)
    (config : GuestPcTraceSegmentQueueConfig) :
    forall publicInput proof,
      GuestPcTraceSegmentQueueCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceSegmentQueueDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.segmentQueueConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem guest_pc_trace_segment_queue_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceSegmentQueueValidation)
    (config : GuestPcTraceSegmentQueueConfig) :
    forall publicInput proof,
      GuestPcTraceSegmentQueueCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceSegmentQueueDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (guest_pc_trace_segment_queue_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness
        assumptions
        publicInput
        proof
        checked)

theorem guest_pc_trace_segment_queue_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceSegmentQueueValidation)
    (config : GuestPcTraceSegmentQueueConfig) :
    forall publicInput proof,
      GuestPcTraceSegmentQueueCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem guest_pc_trace_large_gpu_gate_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : GuestPcTraceLargeGpuGateValidation)
    (config : GuestPcTraceLargeGpuGateConfig) :
    forall publicInput proof,
      GuestPcTraceLargeGpuGateCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceLargeGpuGateDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.largeGpuGateConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem guest_pc_trace_large_gpu_gate_memory_check_passes_projects_observed_floor
    (config : GuestPcTraceLargeGpuGateConfig) :
    GuestPcTraceLargeGpuGateMemoryCheckPasses config = true ->
      exists freeBytes,
        config.observedFreeGpuMemoryBytes = some freeBytes
          /\ config.defaultMinFreeGpuMemoryBytes <= freeBytes := by
  intro memoryCheck
  cases hObserved : config.observedFreeGpuMemoryBytes with
  | none =>
      simp [GuestPcTraceLargeGpuGateMemoryCheckPasses, hObserved] at memoryCheck
  | some freeBytes =>
      exists freeBytes
      constructor
      · rfl
      · have decided :
            decide (config.defaultMinFreeGpuMemoryBytes <= freeBytes) = true := by
          simpa [GuestPcTraceLargeGpuGateMemoryCheckPasses, hObserved] using memoryCheck
        exact of_decide_eq_true decided

theorem guest_pc_trace_large_gpu_gate_decision_requires_runtime_memory_for_large_allowed
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (decision : GuestPcTraceLargeGpuGateDecisionMatches config)
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit)
    (allowed : config.largeTraceAllowed = true) :
    config.gpuBackendAvailable = true
      /\ GuestPcTraceLargeGpuGateMemoryCheckPasses config = true := by
  rcases decision with ⟨_thresholdMatches, _minMemoryMatches, allowedDecision⟩
  rw [requested] at allowedDecision
  have decided :
      decide (limit < config.defaultLargeTraceInstructionThreshold
        \/ (config.gpuBackendAvailable = true
          /\ GuestPcTraceLargeGpuGateMemoryCheckPasses config = true)) = true := by
    rw [← allowedDecision]
    exact allowed
  have allowedCases := of_decide_eq_true decided
  cases allowedCases with
  | inl small =>
      exact False.elim ((Nat.not_lt.mpr large) small)
  | inr runtimeMemory =>
      exact runtimeMemory

theorem guest_pc_trace_large_gpu_gate_decision_projects_observed_memory_floor_for_large_allowed
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (decision : GuestPcTraceLargeGpuGateDecisionMatches config)
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit)
    (allowed : config.largeTraceAllowed = true) :
    exists freeBytes,
      config.observedFreeGpuMemoryBytes = some freeBytes
        /\ config.defaultMinFreeGpuMemoryBytes <= freeBytes := by
  have runtimeMemory :=
    guest_pc_trace_large_gpu_gate_decision_requires_runtime_memory_for_large_allowed
      config
      decision
      requested
      large
      allowed
  exact
    guest_pc_trace_large_gpu_gate_memory_check_passes_projects_observed_floor
      config
      runtimeMemory.right

theorem guest_pc_trace_large_gpu_gate_checked_acceptance_requires_runtime_memory_for_large_allowed
    {system : VerifierModel}
    (validation : GuestPcTraceLargeGpuGateValidation)
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit)
    (allowed : config.largeTraceAllowed = true) :
    forall publicInput proof,
      GuestPcTraceLargeGpuGateCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.gpuBackendAvailable = true
          /\ exists freeBytes,
            config.observedFreeGpuMemoryBytes = some freeBytes
              /\ config.defaultMinFreeGpuMemoryBytes <= freeBytes := by
  intro publicInput proof checked
  have decision :=
    guest_pc_trace_large_gpu_gate_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have runtimeMemory :=
    guest_pc_trace_large_gpu_gate_decision_requires_runtime_memory_for_large_allowed
      config
      decision
      requested
      large
      allowed
  exact
    And.intro
      runtimeMemory.left
      (guest_pc_trace_large_gpu_gate_memory_check_passes_projects_observed_floor
        config
        runtimeMemory.right)

theorem guest_pc_trace_large_gpu_gate_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceLargeGpuGateValidation)
    (config : GuestPcTraceLargeGpuGateConfig) :
    forall publicInput proof,
      GuestPcTraceLargeGpuGateCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceLargeGpuGateDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (guest_pc_trace_large_gpu_gate_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness assumptions publicInput proof checked)

theorem guest_pc_trace_large_gpu_gate_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceLargeGpuGateValidation)
    (config : GuestPcTraceLargeGpuGateConfig) :
    forall publicInput proof,
      GuestPcTraceLargeGpuGateCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem guest_pc_trace_traceless_commitment_input_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : GuestPcTraceTracelessCommitmentInputValidation)
    (config : GuestPcTraceTracelessCommitmentInputConfig) :
    forall publicInput proof,
      GuestPcTraceTracelessCommitmentInputCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceTracelessCommitmentInputDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.tracelessCommitmentInputConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem guest_pc_trace_traceless_commitment_input_decision_default_enabled
    (config : GuestPcTraceTracelessCommitmentInputConfig) :
    config.configuredTracelessCommitmentInput = none ->
      GuestPcTraceTracelessCommitmentInputDecisionMatches config ->
        config.effectiveTracelessCommitmentInput = true := by
  intro noConfig decision
  simpa [GuestPcTraceTracelessCommitmentInputDecisionMatches, noConfig] using decision

theorem guest_pc_trace_traceless_commitment_input_checked_acceptance_projects_default_enabled
    {system : VerifierModel}
    (validation : GuestPcTraceTracelessCommitmentInputValidation)
    (config : GuestPcTraceTracelessCommitmentInputConfig) :
    config.configuredTracelessCommitmentInput = none ->
      forall publicInput proof,
        GuestPcTraceTracelessCommitmentInputCheckedAcceptance
            system
            validation
            config
            publicInput
            proof ->
          config.effectiveTracelessCommitmentInput = true := by
  intro noConfig publicInput proof checked
  exact
    guest_pc_trace_traceless_commitment_input_decision_default_enabled
      config
      noConfig
      (guest_pc_trace_traceless_commitment_input_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_traceless_commitment_input_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceTracelessCommitmentInputValidation)
    (config : GuestPcTraceTracelessCommitmentInputConfig) :
    forall publicInput proof,
      GuestPcTraceTracelessCommitmentInputCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceTracelessCommitmentInputDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (guest_pc_trace_traceless_commitment_input_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness assumptions publicInput proof checked)

theorem guest_pc_trace_traceless_commitment_input_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceTracelessCommitmentInputValidation)
    (config : GuestPcTraceTracelessCommitmentInputConfig) :
    forall publicInput proof,
      GuestPcTraceTracelessCommitmentInputCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem guest_pc_trace_traceless_segment_output_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : GuestPcTraceTracelessSegmentOutputValidation)
    (config : GuestPcTraceTracelessSegmentOutputConfig) :
    forall publicInput proof,
      GuestPcTraceTracelessSegmentOutputCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceTracelessSegmentOutputDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.tracelessSegmentOutputConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem guest_pc_trace_traceless_segment_output_decision_default_enabled
    (config : GuestPcTraceTracelessSegmentOutputConfig) :
    config.configuredTracelessSegmentOutput = none ->
      GuestPcTraceTracelessSegmentOutputDecisionMatches config ->
        config.effectiveTracelessSegmentOutput = true := by
  intro noConfig decision
  simpa [GuestPcTraceTracelessSegmentOutputDecisionMatches, noConfig] using decision

theorem guest_pc_trace_traceless_segment_output_checked_acceptance_projects_default_enabled
    {system : VerifierModel}
    (validation : GuestPcTraceTracelessSegmentOutputValidation)
    (config : GuestPcTraceTracelessSegmentOutputConfig) :
    config.configuredTracelessSegmentOutput = none ->
      forall publicInput proof,
        GuestPcTraceTracelessSegmentOutputCheckedAcceptance
            system
            validation
            config
            publicInput
            proof ->
          config.effectiveTracelessSegmentOutput = true := by
  intro noConfig publicInput proof checked
  exact
    guest_pc_trace_traceless_segment_output_decision_default_enabled
      config
      noConfig
      (guest_pc_trace_traceless_segment_output_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_traceless_segment_output_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceTracelessSegmentOutputValidation)
    (config : GuestPcTraceTracelessSegmentOutputConfig) :
    forall publicInput proof,
      GuestPcTraceTracelessSegmentOutputCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceTracelessSegmentOutputDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (guest_pc_trace_traceless_segment_output_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness assumptions publicInput proof checked)

theorem guest_pc_trace_traceless_segment_output_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceTracelessSegmentOutputValidation)
    (config : GuestPcTraceTracelessSegmentOutputConfig) :
    forall publicInput proof,
      GuestPcTraceTracelessSegmentOutputCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem guest_pc_trace_cross_root_materialization_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : GuestPcTraceCrossSegmentRootMaterializationValidation)
    (config : GuestPcTraceCrossSegmentRootMaterializationConfig) :
    forall publicInput proof,
      GuestPcTraceCrossSegmentRootMaterializationCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceCrossSegmentRootMaterializationDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.crossSegmentRootMaterializationConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem guest_pc_trace_cross_root_materialization_decision_default_enabled_when_supported
    (config : GuestPcTraceCrossSegmentRootMaterializationConfig) :
    config.configuredCrossSegmentRootMaterialization = none ->
      config.inputByteCount < config.supportedInputByteLimit ->
        GuestPcTraceCrossSegmentRootMaterializationDecisionMatches config ->
          config.effectiveCrossSegmentRootMaterialization = true := by
  intro noConfig supported decision
  rcases decision with ⟨_limitMatches, decisionMatches⟩
  simpa [GuestPcTraceCrossSegmentRootMaterializationDecisionMatches, noConfig, supported]
    using decisionMatches

theorem guest_pc_trace_cross_root_materialization_decision_disabled_when_unsupported
    (config : GuestPcTraceCrossSegmentRootMaterializationConfig) :
    config.supportedInputByteLimit <= config.inputByteCount ->
      GuestPcTraceCrossSegmentRootMaterializationDecisionMatches config ->
        config.effectiveCrossSegmentRootMaterialization = false := by
  intro unsupported decision
  rcases decision with ⟨_limitMatches, decisionMatches⟩
  have notSupported : ¬ config.inputByteCount < config.supportedInputByteLimit :=
    Nat.not_lt.mpr unsupported
  cases hConfigured : config.configuredCrossSegmentRootMaterialization with
  | none =>
      simpa [
        GuestPcTraceCrossSegmentRootMaterializationDecisionMatches,
        hConfigured,
        notSupported,
      ] using decisionMatches
  | some configured =>
      cases configured <;>
        simpa [
          GuestPcTraceCrossSegmentRootMaterializationDecisionMatches,
          hConfigured,
          notSupported,
        ] using decisionMatches

theorem guest_pc_trace_cross_root_materialization_checked_acceptance_projects_default_enabled
    {system : VerifierModel}
    (validation : GuestPcTraceCrossSegmentRootMaterializationValidation)
    (config : GuestPcTraceCrossSegmentRootMaterializationConfig) :
    config.configuredCrossSegmentRootMaterialization = none ->
      config.inputByteCount < config.supportedInputByteLimit ->
        forall publicInput proof,
          GuestPcTraceCrossSegmentRootMaterializationCheckedAcceptance
              system
              validation
              config
              publicInput
              proof ->
            config.effectiveCrossSegmentRootMaterialization = true := by
  intro noConfig supported publicInput proof checked
  exact
    guest_pc_trace_cross_root_materialization_decision_default_enabled_when_supported
      config
      noConfig
      supported
      (guest_pc_trace_cross_root_materialization_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_cross_root_materialization_checked_acceptance_projects_disabled
    {system : VerifierModel}
    (validation : GuestPcTraceCrossSegmentRootMaterializationValidation)
    (config : GuestPcTraceCrossSegmentRootMaterializationConfig) :
    config.supportedInputByteLimit <= config.inputByteCount ->
      forall publicInput proof,
        GuestPcTraceCrossSegmentRootMaterializationCheckedAcceptance
            system
            validation
            config
            publicInput
            proof ->
          config.effectiveCrossSegmentRootMaterialization = false := by
  intro unsupported publicInput proof checked
  exact
    guest_pc_trace_cross_root_materialization_decision_disabled_when_unsupported
      config
      unsupported
      (guest_pc_trace_cross_root_materialization_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_cross_root_materialization_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceCrossSegmentRootMaterializationValidation)
    (config : GuestPcTraceCrossSegmentRootMaterializationConfig) :
    forall publicInput proof,
      GuestPcTraceCrossSegmentRootMaterializationCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceCrossSegmentRootMaterializationDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (guest_pc_trace_cross_root_materialization_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness assumptions publicInput proof checked)

theorem guest_pc_trace_cross_root_materialization_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceCrossSegmentRootMaterializationValidation)
    (config : GuestPcTraceCrossSegmentRootMaterializationConfig) :
    forall publicInput proof,
      GuestPcTraceCrossSegmentRootMaterializationCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem guest_pc_trace_commit_mode_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : GuestPcTraceSegmentCommitModeValidation)
    (config : GuestPcTraceSegmentCommitModeConfig) :
    forall publicInput proof,
      GuestPcTraceSegmentCommitModeCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceSegmentCommitModeDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.segmentCommitModeConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem guest_pc_trace_commit_mode_effective_worker_positive
    (config : GuestPcTraceSegmentCommitModeConfig) :
    GuestPcTraceSegmentCommitModeDecisionMatches config ->
      0 < config.effectiveWorkerCount := by
  intro decision
  rcases decision with
    ⟨_workerMatch, workerPositive, _asyncMatch, _traceDecision,
      _traceSelected, _rootDecision, _rootSelected, _descriptorDecision,
      _descriptorSelected, _windowPositive, _windowMatch⟩
  exact workerPositive

theorem guest_pc_trace_commit_mode_async_requires_single_worker
    (config : GuestPcTraceSegmentCommitModeConfig) :
    GuestPcTraceSegmentCommitModeDecisionMatches config ->
      config.effectiveAsyncSingleWorker = true ->
        config.effectiveWorkerCount = 1 := by
  intro decision asyncSelected
  rcases decision with
    ⟨_workerMatch, _workerPositive, asyncMatches, _traceDecision,
      _traceSelected, _rootDecision, _rootSelected, _descriptorDecision,
      _descriptorSelected, _windowPositive, _windowMatch⟩
  by_cases singleWorker : config.effectiveWorkerCount = 1
  · exact singleWorker
  · have notSelected :
        ¬ (config.effectiveWorkerCount = 1
          /\ config.configuredAsyncSingleWorker = true) := by
      intro selected
      exact singleWorker selected.left
    have asyncFalse :
        config.effectiveAsyncSingleWorker = false := by
      simpa [notSelected] using asyncMatches
    have impossible : False := by
      rw [asyncFalse] at asyncSelected
      contradiction
    exact False.elim impossible

theorem guest_pc_trace_descriptor_buffer_retention_default_disabled_for_parallel_lower
    (config : GuestPcTraceDescriptorBufferRetentionConfig) :
    config.configuredDescriptorBufferRetention = none ->
      config.parallelLowerEnabledForDescriptorRetention = true ->
        GuestPcTraceDescriptorBufferRetentionDecisionMatches config ->
          config.effectiveDescriptorBufferRetention = false := by
  intro configuredNone parallelEnabled decision
  rcases decision with ⟨_limitPositive, retentionMatches⟩
  simpa [configuredNone, parallelEnabled] using retentionMatches

theorem guest_pc_trace_commit_mode_descriptor_retention_matches
    (config : GuestPcTraceSegmentCommitModeConfig) :
    GuestPcTraceSegmentCommitModeDecisionMatches config ->
      config.selectedDescriptorBufferRetention =
        config.descriptorBufferRetentionConfig.effectiveDescriptorBufferRetention := by
  intro decision
  rcases decision with
    ⟨_workerMatch, _workerPositive, _asyncMatches, _traceDecision,
      _traceSelected, _rootDecision, _rootSelected, _descriptorDecision,
      descriptorSelected, _windowPositive, _windowMatch⟩
  exact descriptorSelected

theorem guest_pc_trace_commit_mode_disabled_root_window_is_one
    (config : GuestPcTraceSegmentCommitModeConfig) :
    config.selectedCrossSegmentRootMaterialization = false ->
      GuestPcTraceSegmentCommitModeDecisionMatches config ->
        config.effectivePendingRootMaterializationWindow = 1 := by
  intro disabled decision
  rcases decision with
    ⟨_workerMatch, _workerPositive, _asyncMatches, _traceDecision,
      _traceSelected, _rootDecision, _rootSelected, _descriptorDecision,
      _descriptorSelected, _windowPositive, windowMatches⟩
  cases hConfigured : config.configuredPendingRootMaterializationWindow with
  | none =>
      have reduced :
          config.effectivePendingRootMaterializationWindow =
            if config.selectedCrossSegmentRootMaterialization then
              config.defaultPendingRootMaterializationWindow
            else
              1 := by
        simpa [hConfigured] using windowMatches
      simpa [disabled] using reduced
  | some configured =>
      have reduced :
          0 < configured
            /\ config.effectivePendingRootMaterializationWindow =
              if config.selectedCrossSegmentRootMaterialization then
                configured
              else
                1 := by
        simpa [hConfigured] using windowMatches
      rcases reduced with ⟨_configuredPositive, reduced⟩
      simpa [disabled] using reduced

theorem guest_pc_trace_commit_mode_checked_acceptance_projects_disabled_root_window
    {system : VerifierModel}
    (validation : GuestPcTraceSegmentCommitModeValidation)
    (config : GuestPcTraceSegmentCommitModeConfig) :
    config.selectedCrossSegmentRootMaterialization = false ->
      forall publicInput proof,
        GuestPcTraceSegmentCommitModeCheckedAcceptance
            system
            validation
            config
            publicInput
            proof ->
          config.effectivePendingRootMaterializationWindow = 1 := by
  intro disabled publicInput proof checked
  exact
    guest_pc_trace_commit_mode_disabled_root_window_is_one
      config
      disabled
      (guest_pc_trace_commit_mode_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_commit_mode_checked_acceptance_projects_descriptor_retention
    {system : VerifierModel}
    (validation : GuestPcTraceSegmentCommitModeValidation)
    (config : GuestPcTraceSegmentCommitModeConfig) :
    forall publicInput proof,
      GuestPcTraceSegmentCommitModeCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.selectedDescriptorBufferRetention =
          config.descriptorBufferRetentionConfig.effectiveDescriptorBufferRetention := by
  intro publicInput proof checked
  exact
    guest_pc_trace_commit_mode_descriptor_retention_matches
      config
      (guest_pc_trace_commit_mode_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_commit_mode_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceSegmentCommitModeValidation)
    (config : GuestPcTraceSegmentCommitModeConfig) :
    forall publicInput proof,
      GuestPcTraceSegmentCommitModeCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceSegmentCommitModeDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (guest_pc_trace_commit_mode_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness assumptions publicInput proof checked)

theorem guest_pc_trace_commit_mode_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceSegmentCommitModeValidation)
    (config : GuestPcTraceSegmentCommitModeConfig) :
    forall publicInput proof,
      GuestPcTraceSegmentCommitModeCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked


end Lzvm
