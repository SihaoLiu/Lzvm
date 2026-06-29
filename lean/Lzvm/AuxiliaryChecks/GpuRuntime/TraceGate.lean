/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.GpuRuntime.Core

/-!
GPU trace gate auxiliary runtime contracts.
-/

namespace Lzvm

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

theorem gpu_allocator_no_wait_limit_checked_acceptance_core_and_sound
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    gpu_allocator_no_wait_limit_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro decision coreAndSound

theorem guest_pc_trace_parallel_lower_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : GuestPcTraceParallelLowerValidation)
    (config : GuestPcTraceParallelLowerConfig) :
    forall publicInput proof,
      GuestPcTraceParallelLowerCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceParallelLowerDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.parallelLowerConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem guest_pc_trace_parallel_lower_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceParallelLowerValidation)
    (config : GuestPcTraceParallelLowerConfig) :
    forall publicInput proof,
      GuestPcTraceParallelLowerCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceParallelLowerDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (guest_pc_trace_parallel_lower_checked_acceptance_projects_decision
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

theorem guest_pc_trace_parallel_lower_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceParallelLowerValidation)
    (config : GuestPcTraceParallelLowerConfig) :
    forall publicInput proof,
      GuestPcTraceParallelLowerCheckedAcceptance
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

theorem guest_pc_trace_parallel_lower_checked_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceParallelLowerValidation)
    (config : GuestPcTraceParallelLowerConfig) :
    forall publicInput proof,
      GuestPcTraceParallelLowerCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceParallelLowerDecisionMatches config
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    guest_pc_trace_parallel_lower_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro decision coreAndSound

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

theorem guest_pc_trace_segment_queue_checked_acceptance_core_and_sound
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    guest_pc_trace_segment_queue_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro decision coreAndSound

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

theorem guest_pc_trace_large_gpu_gate_memory_check_passes_of_observed_floor
    (config : GuestPcTraceLargeGpuGateConfig)
    {freeBytes : Nat}
    (observed : config.observedFreeGpuMemoryBytes = some freeBytes)
    (floor : config.defaultMinFreeGpuMemoryBytes <= freeBytes) :
    GuestPcTraceLargeGpuGateMemoryCheckPasses config = true := by
  have decided :
      decide (config.defaultMinFreeGpuMemoryBytes <= freeBytes) = true :=
    decide_eq_true floor
  simpa [GuestPcTraceLargeGpuGateMemoryCheckPasses, observed] using decided

theorem guest_pc_trace_large_gpu_gate_memory_check_passes_iff_observed_floor
    (config : GuestPcTraceLargeGpuGateConfig) :
    GuestPcTraceLargeGpuGateMemoryCheckPasses config = true
      <-> exists freeBytes,
        config.observedFreeGpuMemoryBytes = some freeBytes
          /\ config.defaultMinFreeGpuMemoryBytes <= freeBytes := by
  constructor
  · exact guest_pc_trace_large_gpu_gate_memory_check_passes_projects_observed_floor config
  · intro observedFloor
    rcases observedFloor with ⟨freeBytes, observed, floor⟩
    exact
      guest_pc_trace_large_gpu_gate_memory_check_passes_of_observed_floor
        config
        observed
        floor

theorem guest_pc_trace_large_gpu_gate_memory_check_fails_without_observation
    (config : GuestPcTraceLargeGpuGateConfig)
    (missing : config.observedFreeGpuMemoryBytes = none) :
    GuestPcTraceLargeGpuGateMemoryCheckPasses config = false := by
  simp [GuestPcTraceLargeGpuGateMemoryCheckPasses, missing]

theorem guest_pc_trace_large_gpu_gate_memory_check_fails_below_floor
    (config : GuestPcTraceLargeGpuGateConfig)
    {freeBytes : Nat}
    (observed : config.observedFreeGpuMemoryBytes = some freeBytes)
    (below : freeBytes < config.defaultMinFreeGpuMemoryBytes) :
    GuestPcTraceLargeGpuGateMemoryCheckPasses config = false := by
  have notEnough : ¬ config.defaultMinFreeGpuMemoryBytes <= freeBytes :=
    Nat.not_le_of_gt below
  simp [GuestPcTraceLargeGpuGateMemoryCheckPasses, observed, notEnough]

theorem guest_pc_trace_large_gpu_gate_decision_allows_unrequested
    (config : GuestPcTraceLargeGpuGateConfig)
    (decision : GuestPcTraceLargeGpuGateDecisionMatches config)
    (unrequested : config.requestedInstructionLimit = none) :
    config.largeTraceAllowed = true := by
  rcases decision with ⟨_thresholdMatches, _minMemoryMatches, allowedDecision⟩
  simpa [unrequested] using allowedDecision

theorem guest_pc_trace_large_gpu_gate_decision_allows_below_threshold
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (decision : GuestPcTraceLargeGpuGateDecisionMatches config)
    (requested : config.requestedInstructionLimit = some limit)
    (small : limit < config.defaultLargeTraceInstructionThreshold) :
    config.largeTraceAllowed = true := by
  rcases decision with ⟨_thresholdMatches, _minMemoryMatches, allowedDecision⟩
  rw [requested] at allowedDecision
  rw [allowedDecision]
  simp [small]

theorem guest_pc_trace_large_gpu_gate_decision_rejects_large_without_gpu
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (decision : GuestPcTraceLargeGpuGateDecisionMatches config)
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit)
    (gpuUnavailable : config.gpuBackendAvailable = false) :
    config.largeTraceAllowed = false := by
  rcases decision with ⟨_thresholdMatches, _minMemoryMatches, allowedDecision⟩
  rw [requested] at allowedDecision
  rw [allowedDecision]
  have notSmall : ¬ limit < config.defaultLargeTraceInstructionThreshold :=
    Nat.not_lt.mpr large
  simp [notSmall, gpuUnavailable]

theorem guest_pc_trace_large_gpu_gate_decision_rejects_large_without_memory
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (decision : GuestPcTraceLargeGpuGateDecisionMatches config)
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit)
    (memoryCheckFailed : GuestPcTraceLargeGpuGateMemoryCheckPasses config = false) :
    config.largeTraceAllowed = false := by
  rcases decision with ⟨_thresholdMatches, _minMemoryMatches, allowedDecision⟩
  rw [requested] at allowedDecision
  rw [allowedDecision]
  have notSmall : ¬ limit < config.defaultLargeTraceInstructionThreshold :=
    Nat.not_lt.mpr large
  simp [notSmall, memoryCheckFailed]

theorem guest_pc_trace_large_gpu_gate_decision_rejects_large_without_observed_memory
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (decision : GuestPcTraceLargeGpuGateDecisionMatches config)
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit)
    (missing : config.observedFreeGpuMemoryBytes = none) :
    config.largeTraceAllowed = false := by
  exact
    guest_pc_trace_large_gpu_gate_decision_rejects_large_without_memory
      config
      decision
      requested
      large
      (guest_pc_trace_large_gpu_gate_memory_check_fails_without_observation
        config
        missing)

theorem guest_pc_trace_large_gpu_gate_decision_rejects_large_below_memory_floor
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit freeBytes : Nat}
    (decision : GuestPcTraceLargeGpuGateDecisionMatches config)
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit)
    (observed : config.observedFreeGpuMemoryBytes = some freeBytes)
    (below : freeBytes < config.defaultMinFreeGpuMemoryBytes) :
    config.largeTraceAllowed = false := by
  exact
    guest_pc_trace_large_gpu_gate_decision_rejects_large_without_memory
      config
      decision
      requested
      large
      (guest_pc_trace_large_gpu_gate_memory_check_fails_below_floor
        config
        observed
        below)

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

theorem guest_pc_trace_large_gpu_gate_decision_allows_large_iff_runtime_memory
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (decision : GuestPcTraceLargeGpuGateDecisionMatches config)
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit) :
    config.largeTraceAllowed = true
      <-> config.gpuBackendAvailable = true
        /\ GuestPcTraceLargeGpuGateMemoryCheckPasses config = true := by
  constructor
  · intro allowed
    exact
      guest_pc_trace_large_gpu_gate_decision_requires_runtime_memory_for_large_allowed
        config
        decision
        requested
        large
        allowed
  · intro runtimeMemory
    rcases decision with ⟨_thresholdMatches, _minMemoryMatches, allowedDecision⟩
    rw [requested] at allowedDecision
    rw [allowedDecision]
    have notSmall : ¬ limit < config.defaultLargeTraceInstructionThreshold :=
      Nat.not_lt.mpr large
    simp [notSmall, runtimeMemory.left, runtimeMemory.right]

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

theorem guest_pc_trace_large_gpu_gate_decision_allows_large_iff_backend_and_observed_floor
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (decision : GuestPcTraceLargeGpuGateDecisionMatches config)
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit) :
    config.largeTraceAllowed = true
      <-> config.gpuBackendAvailable = true
        /\ exists freeBytes,
          config.observedFreeGpuMemoryBytes = some freeBytes
            /\ config.defaultMinFreeGpuMemoryBytes <= freeBytes := by
  constructor
  · intro allowed
    have runtimeMemory :=
      (guest_pc_trace_large_gpu_gate_decision_allows_large_iff_runtime_memory
        config
        decision
        requested
        large).mp allowed
    exact
      And.intro
        runtimeMemory.left
        (guest_pc_trace_large_gpu_gate_memory_check_passes_projects_observed_floor
          config
          runtimeMemory.right)
  · intro observedRuntime
    have memoryCheck :=
      (guest_pc_trace_large_gpu_gate_memory_check_passes_iff_observed_floor
        config).mpr observedRuntime.right
    exact
      (guest_pc_trace_large_gpu_gate_decision_allows_large_iff_runtime_memory
        config
        decision
        requested
        large).mpr
        (And.intro observedRuntime.left memoryCheck)

theorem guest_pc_trace_large_gpu_gate_checked_acceptance_allows_unrequested
    {system : VerifierModel}
    (validation : GuestPcTraceLargeGpuGateValidation)
    (config : GuestPcTraceLargeGpuGateConfig)
    (unrequested : config.requestedInstructionLimit = none) :
    forall publicInput proof,
      GuestPcTraceLargeGpuGateCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.largeTraceAllowed = true := by
  intro publicInput proof checked
  exact
    guest_pc_trace_large_gpu_gate_decision_allows_unrequested
      config
      (guest_pc_trace_large_gpu_gate_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      unrequested

theorem guest_pc_trace_large_gpu_gate_checked_acceptance_allows_below_threshold
    {system : VerifierModel}
    (validation : GuestPcTraceLargeGpuGateValidation)
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (requested : config.requestedInstructionLimit = some limit)
    (small : limit < config.defaultLargeTraceInstructionThreshold) :
    forall publicInput proof,
      GuestPcTraceLargeGpuGateCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.largeTraceAllowed = true := by
  intro publicInput proof checked
  exact
    guest_pc_trace_large_gpu_gate_decision_allows_below_threshold
      config
      (guest_pc_trace_large_gpu_gate_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      requested
      small

theorem guest_pc_trace_large_gpu_gate_checked_acceptance_rejects_large_without_gpu
    {system : VerifierModel}
    (validation : GuestPcTraceLargeGpuGateValidation)
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit)
    (gpuUnavailable : config.gpuBackendAvailable = false) :
    forall publicInput proof,
      GuestPcTraceLargeGpuGateCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.largeTraceAllowed = false := by
  intro publicInput proof checked
  exact
    guest_pc_trace_large_gpu_gate_decision_rejects_large_without_gpu
      config
      (guest_pc_trace_large_gpu_gate_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      requested
      large
      gpuUnavailable

theorem guest_pc_trace_large_gpu_gate_checked_acceptance_rejects_large_without_memory
    {system : VerifierModel}
    (validation : GuestPcTraceLargeGpuGateValidation)
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit)
    (memoryCheckFailed : GuestPcTraceLargeGpuGateMemoryCheckPasses config = false) :
    forall publicInput proof,
      GuestPcTraceLargeGpuGateCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.largeTraceAllowed = false := by
  intro publicInput proof checked
  exact
    guest_pc_trace_large_gpu_gate_decision_rejects_large_without_memory
      config
      (guest_pc_trace_large_gpu_gate_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      requested
      large
      memoryCheckFailed

theorem guest_pc_trace_large_gpu_gate_checked_acceptance_rejects_large_without_observed_memory
    {system : VerifierModel}
    (validation : GuestPcTraceLargeGpuGateValidation)
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit)
    (missing : config.observedFreeGpuMemoryBytes = none) :
    forall publicInput proof,
      GuestPcTraceLargeGpuGateCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.largeTraceAllowed = false := by
  intro publicInput proof checked
  exact
    guest_pc_trace_large_gpu_gate_decision_rejects_large_without_observed_memory
      config
      (guest_pc_trace_large_gpu_gate_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      requested
      large
      missing

theorem guest_pc_trace_large_gpu_gate_checked_acceptance_rejects_large_below_memory_floor
    {system : VerifierModel}
    (validation : GuestPcTraceLargeGpuGateValidation)
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit freeBytes : Nat}
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit)
    (observed : config.observedFreeGpuMemoryBytes = some freeBytes)
    (below : freeBytes < config.defaultMinFreeGpuMemoryBytes) :
    forall publicInput proof,
      GuestPcTraceLargeGpuGateCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.largeTraceAllowed = false := by
  intro publicInput proof checked
  exact
    guest_pc_trace_large_gpu_gate_decision_rejects_large_below_memory_floor
      config
      (guest_pc_trace_large_gpu_gate_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      requested
      large
      observed
      below

theorem guest_pc_trace_large_gpu_gate_checked_acceptance_allows_large_iff_runtime_memory
    {system : VerifierModel}
    (validation : GuestPcTraceLargeGpuGateValidation)
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit) :
    forall publicInput proof,
      GuestPcTraceLargeGpuGateCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        (config.largeTraceAllowed = true
          <-> config.gpuBackendAvailable = true
            /\ GuestPcTraceLargeGpuGateMemoryCheckPasses config = true) := by
  intro publicInput proof checked
  exact
    guest_pc_trace_large_gpu_gate_decision_allows_large_iff_runtime_memory
      config
      (guest_pc_trace_large_gpu_gate_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      requested
      large

theorem guest_pc_trace_large_gpu_gate_checked_acceptance_allows_large_iff_backend_and_observed_floor
    {system : VerifierModel}
    (validation : GuestPcTraceLargeGpuGateValidation)
    (config : GuestPcTraceLargeGpuGateConfig)
    {limit : Nat}
    (requested : config.requestedInstructionLimit = some limit)
    (large : config.defaultLargeTraceInstructionThreshold <= limit) :
    forall publicInput proof,
      GuestPcTraceLargeGpuGateCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        (config.largeTraceAllowed = true
          <-> config.gpuBackendAvailable = true
            /\ exists freeBytes,
              config.observedFreeGpuMemoryBytes = some freeBytes
                /\ config.defaultMinFreeGpuMemoryBytes <= freeBytes) := by
  intro publicInput proof checked
  exact
    guest_pc_trace_large_gpu_gate_decision_allows_large_iff_backend_and_observed_floor
      config
      (guest_pc_trace_large_gpu_gate_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      requested
      large

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

theorem guest_pc_trace_large_gpu_gate_checked_acceptance_core_and_sound
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    guest_pc_trace_large_gpu_gate_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro decision coreAndSound

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

theorem guest_pc_trace_traceless_commitment_input_checked_acceptance_core_and_sound
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    guest_pc_trace_traceless_commitment_input_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro decision coreAndSound

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

theorem guest_pc_trace_traceless_segment_output_checked_acceptance_core_and_sound
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    guest_pc_trace_traceless_segment_output_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro decision coreAndSound

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

theorem guest_pc_trace_cross_root_materialization_checked_acceptance_core_and_sound
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    guest_pc_trace_cross_root_materialization_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro decision coreAndSound

end Lzvm
