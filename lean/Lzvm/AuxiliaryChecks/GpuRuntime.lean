/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks

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
      (abstract_verifier_sound assumptions publicInput proof acceptedWithSetup.left)

theorem gpu_setup_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuSetupCacheValidation)
    (request : GpuSetupRequest) :
    forall publicInput proof,
      GpuSetupCheckedAcceptance system validation request publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_setup_checked_acceptance_sound
      assumptions
      validation
      request
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

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
      (abstract_verifier_sound assumptions publicInput proof acceptedWithAllocation.left)

theorem gpu_allocation_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuAllocationCacheValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocationCheckedAcceptance system validation allocation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_allocation_checked_acceptance_sound
      assumptions
      validation
      allocation
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

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
      (abstract_verifier_sound assumptions publicInput proof checked.left)

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
  have sound :=
    gpu_host_device_copy_round_trip_checked_acceptance_sound
      assumptions
      validation
      allocation
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

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
        (abstract_verifier_sound assumptions publicInput proof checked.left))

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
  have sound :=
    gpu_temporary_buffer_reuse_checked_acceptance_sound
      assumptions
      validation
      previous
      next
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right.right

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
          (abstract_verifier_sound assumptions publicInput proof checked.left)))

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
  have sound :=
    gpu_allocator_no_wait_bypass_checked_acceptance_sound
      assumptions
      validation
      pending
      fresh
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right.right.right

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
      (abstract_verifier_sound assumptions publicInput proof checked.left)

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
  have sound :=
    gpu_allocator_no_wait_limit_checked_acceptance_sound
      assumptions
      validation
      config
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

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
      (abstract_verifier_sound assumptions publicInput proof checked.left)

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
  have sound :=
    guest_pc_trace_segment_queue_checked_acceptance_sound
      assumptions
      validation
      config
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

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
      (abstract_verifier_sound assumptions publicInput proof checked.left)

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
  have sound :=
    guest_pc_trace_large_gpu_gate_checked_acceptance_sound
      assumptions
      validation
      config
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

theorem gpu_retained_leaf_digest_limit_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : GpuRetainedLeafDigestLimitValidation)
    (config : GpuRetainedLeafDigestLimitConfig) :
    forall publicInput proof,
      GpuRetainedLeafDigestLimitCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GpuRetainedLeafDigestLimitDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.retainedLeafDigestLimitConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem gpu_retained_leaf_digest_limit_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuRetainedLeafDigestLimitValidation)
    (config : GpuRetainedLeafDigestLimitConfig) :
    forall publicInput proof,
      GpuRetainedLeafDigestLimitCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GpuRetainedLeafDigestLimitDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (gpu_retained_leaf_digest_limit_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      (abstract_verifier_sound assumptions publicInput proof checked.left)

theorem gpu_retained_leaf_digest_limit_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuRetainedLeafDigestLimitValidation)
    (config : GpuRetainedLeafDigestLimitConfig) :
    forall publicInput proof,
      GpuRetainedLeafDigestLimitCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_retained_leaf_digest_limit_checked_acceptance_sound
      assumptions
      validation
      config
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

theorem gpu_retained_device_cache_budget_checked_acceptance_projects_within_limits
    {system : VerifierModel}
    (validation : GpuRetainedDeviceCacheBudgetValidation)
    (budget : GpuRetainedDeviceCacheBudget) :
    forall publicInput proof,
      GpuRetainedDeviceCacheBudgetCheckedAcceptance
          system
          validation
          budget
          publicInput
          proof ->
        GpuRetainedDeviceCacheBudgetWithinLimits budget := by
  intro publicInput proof checked
  exact
    validation.retainedDeviceCacheBudgetImpliesWithinLimits
      budget
      publicInput
      proof
      checked.right

theorem gpu_retained_device_cache_budget_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuRetainedDeviceCacheBudgetValidation)
    (budget : GpuRetainedDeviceCacheBudget) :
    forall publicInput proof,
      GpuRetainedDeviceCacheBudgetCheckedAcceptance
          system
          validation
          budget
          publicInput
          proof ->
        GpuRetainedDeviceCacheBudgetWithinLimits budget
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (gpu_retained_device_cache_budget_checked_acceptance_projects_within_limits
        validation
        budget
        publicInput
        proof
        checked)
      (abstract_verifier_sound assumptions publicInput proof checked.left)

theorem gpu_retained_device_cache_budget_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuRetainedDeviceCacheBudgetValidation)
    (budget : GpuRetainedDeviceCacheBudget) :
    forall publicInput proof,
      GpuRetainedDeviceCacheBudgetCheckedAcceptance
          system
          validation
          budget
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_retained_device_cache_budget_checked_acceptance_sound
      assumptions
      validation
      budget
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

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
      (abstract_verifier_sound assumptions publicInput proof checked.left)

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
  have sound :=
    fri_fixed_column_cache_checked_acceptance_sound
      assumptions
      validation
      cached
      fresh
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right


end Lzvm
