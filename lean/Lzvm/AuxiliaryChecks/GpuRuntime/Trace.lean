/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.GpuRuntime.Core

/-!
GPU trace, retained-source, and retained-cache auxiliary runtime contracts.
-/

namespace Lzvm

theorem guest_pc_trace_device_trace_source_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : GuestPcTraceDeviceTraceSourceValidation)
    (config : GuestPcTraceDeviceTraceSourceConfig) :
    forall publicInput proof,
      GuestPcTraceDeviceTraceSourceCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceDeviceTraceSourceDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.deviceTraceSourceConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem guest_pc_trace_device_trace_source_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceDeviceTraceSourceValidation)
    (config : GuestPcTraceDeviceTraceSourceConfig) :
    forall publicInput proof,
      GuestPcTraceDeviceTraceSourceCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceDeviceTraceSourceDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (guest_pc_trace_device_trace_source_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness assumptions publicInput proof checked)

theorem guest_pc_trace_device_trace_source_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceDeviceTraceSourceValidation)
    (config : GuestPcTraceDeviceTraceSourceConfig) :
    forall publicInput proof,
      GuestPcTraceDeviceTraceSourceCheckedAcceptance
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

theorem guest_pc_trace_sparse_source_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : GuestPcTraceSparseSourceValidation)
    (config : GuestPcTraceSparseSourceConfig) :
    forall publicInput proof,
      GuestPcTraceSparseSourceCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceSparseSourceDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.sparseSourceConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem guest_pc_trace_sparse_source_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceSparseSourceValidation)
    (config : GuestPcTraceSparseSourceConfig) :
    forall publicInput proof,
      GuestPcTraceSparseSourceCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceSparseSourceDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (guest_pc_trace_sparse_source_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness assumptions publicInput proof checked)

theorem guest_pc_trace_sparse_source_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceSparseSourceValidation)
    (config : GuestPcTraceSparseSourceConfig) :
    forall publicInput proof,
      GuestPcTraceSparseSourceCheckedAcceptance
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

theorem guest_pc_trace_terminal_sparse_source_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : GuestPcTraceTerminalSparseSourceValidation)
    (config : GuestPcTraceTerminalSparseSourceConfig) :
    forall publicInput proof,
      GuestPcTraceTerminalSparseSourceCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceTerminalSparseSourceDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.terminalSparseSourceConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem guest_pc_trace_terminal_sparse_source_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceTerminalSparseSourceValidation)
    (config : GuestPcTraceTerminalSparseSourceConfig) :
    forall publicInput proof,
      GuestPcTraceTerminalSparseSourceCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceTerminalSparseSourceDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (guest_pc_trace_terminal_sparse_source_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness assumptions publicInput proof checked)

theorem guest_pc_trace_terminal_sparse_source_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceTerminalSparseSourceValidation)
    (config : GuestPcTraceTerminalSparseSourceConfig) :
    forall publicInput proof,
      GuestPcTraceTerminalSparseSourceCheckedAcceptance
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

theorem fri_retained_stage_source_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : FriRetainedStageSourceValidation)
    (config : FriRetainedStageSourceConfig) :
    forall publicInput proof,
      FriRetainedStageSourceCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        FriRetainedStageSourceDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.retainedStageSourceConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem fri_retained_stage_source_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : FriRetainedStageSourceValidation)
    (config : FriRetainedStageSourceConfig) :
    forall publicInput proof,
      FriRetainedStageSourceCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        FriRetainedStageSourceDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (fri_retained_stage_source_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness assumptions publicInput proof checked)

theorem fri_retained_stage_source_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : FriRetainedStageSourceValidation)
    (config : FriRetainedStageSourceConfig) :
    forall publicInput proof,
      FriRetainedStageSourceCheckedAcceptance
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

theorem guest_pc_trace_cuda_run_checked_acceptance_projects_decision
    {system : VerifierModel}
    (validation : GuestPcTraceCudaRunValidation)
    (config : GuestPcTraceCudaRunConfig) :
    forall publicInput proof,
      GuestPcTraceCudaRunCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceCudaRunDecisionMatches config := by
  intro publicInput proof checked
  exact
    validation.traceCudaRunConfigImpliesDecisionMatches
      config
      publicInput
      proof
      checked.right

theorem guest_pc_trace_cuda_run_sparse_source_matches
    (config : GuestPcTraceCudaRunConfig) :
    GuestPcTraceCudaRunDecisionMatches config ->
      config.selectedSparseSource =
        config.sparseSourceConfig.effectiveSparseSourceSelected := by
  intro decision
  exact decision.sparseSourceSelected

theorem guest_pc_trace_cuda_run_sparse_source_debug_matches
    (config : GuestPcTraceCudaRunConfig) :
    GuestPcTraceCudaRunDecisionMatches config ->
      config.selectedSparseSourceDebug =
        config.sparseSourceDebugConfig.effectiveSparseSourceDebug := by
  intro decision
  exact decision.sparseSourceDebugSelected

theorem guest_pc_trace_cuda_run_terminal_sparse_source_matches
    (config : GuestPcTraceCudaRunConfig) :
    GuestPcTraceCudaRunDecisionMatches config ->
      config.selectedTerminalSparseSource =
        config.terminalSparseSourceConfig.effectiveTerminalSparseSourceSelected := by
  intro decision
  exact decision.terminalSparseSourceSelected

theorem guest_pc_trace_cuda_run_retained_stage_source_matches
    (config : GuestPcTraceCudaRunConfig) :
    GuestPcTraceCudaRunDecisionMatches config ->
      config.selectedRetainedStageSource =
        config.retainedStageSourceConfig.effectiveRetainedStageSourceEnabled := by
  intro decision
  exact decision.retainedStageSourceSelected

theorem guest_pc_trace_cuda_run_retained_stage_source_debug_uses_selected_source
    (config : GuestPcTraceCudaRunConfig) :
    GuestPcTraceCudaRunDecisionMatches config ->
      config.retainedStageSourceDebugConfig.selectedRetainedStageSource =
        config.selectedRetainedStageSource := by
  intro decision
  exact decision.retainedStageSourceDebugUsesSelectedSource

theorem guest_pc_trace_cuda_run_retained_stage_source_debug_decision_matches
    (config : GuestPcTraceCudaRunConfig) :
    GuestPcTraceCudaRunDecisionMatches config ->
      FriRetainedStageSourceDebugDecisionMatches
        config.retainedStageSourceDebugConfig := by
  intro decision
  exact decision.retainedStageSourceDebugDecision

theorem guest_pc_trace_cuda_run_retained_stage_source_debug_matches
    (config : GuestPcTraceCudaRunConfig) :
    GuestPcTraceCudaRunDecisionMatches config ->
      config.selectedRetainedStageSourceDebug =
        config.retainedStageSourceDebugConfig.effectiveRetainedStageSourceDebug := by
  intro decision
  exact decision.retainedStageSourceDebugSelected

theorem guest_pc_trace_cuda_run_descriptor_retention_matches
    (config : GuestPcTraceCudaRunConfig) :
    GuestPcTraceCudaRunDecisionMatches config ->
      config.selectedDescriptorBufferRetention =
        config.descriptorBufferRetentionConfig.effectiveDescriptorBufferRetention := by
  intro decision
  exact decision.descriptorBufferRetentionSelected

theorem fri_retained_stage_source_debug_requires_retention
    (config : FriRetainedStageSourceDebugConfig) :
    FriRetainedStageSourceDebugDecisionMatches config ->
      config.effectiveRetainedStageSourceDebug = true ->
        config.selectedRetainedStageSource = true := by
  intro decision debugEnabled
  cases hConfigured : config.configuredRetainedStageSourceDebug with
  | none =>
      have debugDisabled :
          config.effectiveRetainedStageSourceDebug = false := by
        simpa [FriRetainedStageSourceDebugDecisionMatches, hConfigured] using
          decision
      rw [debugDisabled] at debugEnabled
      contradiction
  | some configured =>
      have debugMatches :
          config.effectiveRetainedStageSourceDebug =
            (config.selectedRetainedStageSource && configured) := by
        simpa [FriRetainedStageSourceDebugDecisionMatches, hConfigured] using
          decision
      rw [debugMatches] at debugEnabled
      cases hSelected : config.selectedRetainedStageSource with
      | false =>
          simp [hSelected] at debugEnabled
      | true =>
          rfl

theorem guest_pc_trace_cuda_run_retained_stage_source_debug_requires_retention
    (config : GuestPcTraceCudaRunConfig) :
    GuestPcTraceCudaRunDecisionMatches config ->
      config.selectedRetainedStageSourceDebug = true ->
        config.selectedRetainedStageSource = true := by
  intro decision debugEnabled
  have debugDecision :=
    guest_pc_trace_cuda_run_retained_stage_source_debug_decision_matches
      config
      decision
  have debugSelected :=
    guest_pc_trace_cuda_run_retained_stage_source_debug_matches
      config
      decision
  have debugSource :=
    guest_pc_trace_cuda_run_retained_stage_source_debug_uses_selected_source
      config
      decision
  have effectiveDebugEnabled :
      config.retainedStageSourceDebugConfig.effectiveRetainedStageSourceDebug =
        true := by
    rw [← debugSelected]
    exact debugEnabled
  have retainedSelected :=
    fri_retained_stage_source_debug_requires_retention
      config.retainedStageSourceDebugConfig
      debugDecision
      effectiveDebugEnabled
  rw [debugSource] at retainedSelected
  exact retainedSelected

theorem guest_pc_trace_cuda_run_checked_acceptance_projects_sparse_source
    {system : VerifierModel}
    (validation : GuestPcTraceCudaRunValidation)
    (config : GuestPcTraceCudaRunConfig) :
    forall publicInput proof,
      GuestPcTraceCudaRunCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.selectedSparseSource =
          config.sparseSourceConfig.effectiveSparseSourceSelected := by
  intro publicInput proof checked
  exact
    guest_pc_trace_cuda_run_sparse_source_matches
      config
      (guest_pc_trace_cuda_run_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_cuda_run_checked_acceptance_projects_sparse_source_debug
    {system : VerifierModel}
    (validation : GuestPcTraceCudaRunValidation)
    (config : GuestPcTraceCudaRunConfig) :
    forall publicInput proof,
      GuestPcTraceCudaRunCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.selectedSparseSourceDebug =
          config.sparseSourceDebugConfig.effectiveSparseSourceDebug := by
  intro publicInput proof checked
  exact
    guest_pc_trace_cuda_run_sparse_source_debug_matches
      config
      (guest_pc_trace_cuda_run_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_cuda_run_checked_acceptance_projects_terminal_sparse_source
    {system : VerifierModel}
    (validation : GuestPcTraceCudaRunValidation)
    (config : GuestPcTraceCudaRunConfig) :
    forall publicInput proof,
      GuestPcTraceCudaRunCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.selectedTerminalSparseSource =
          config.terminalSparseSourceConfig.effectiveTerminalSparseSourceSelected := by
  intro publicInput proof checked
  exact
    guest_pc_trace_cuda_run_terminal_sparse_source_matches
      config
      (guest_pc_trace_cuda_run_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_cuda_run_checked_acceptance_projects_retained_stage_source
    {system : VerifierModel}
    (validation : GuestPcTraceCudaRunValidation)
    (config : GuestPcTraceCudaRunConfig) :
    forall publicInput proof,
      GuestPcTraceCudaRunCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.selectedRetainedStageSource =
          config.retainedStageSourceConfig.effectiveRetainedStageSourceEnabled := by
  intro publicInput proof checked
  exact
    guest_pc_trace_cuda_run_retained_stage_source_matches
      config
      (guest_pc_trace_cuda_run_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_cuda_run_checked_acceptance_projects_retained_source_debug
    {system : VerifierModel}
    (validation : GuestPcTraceCudaRunValidation)
    (config : GuestPcTraceCudaRunConfig) :
    forall publicInput proof,
      GuestPcTraceCudaRunCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.selectedRetainedStageSourceDebug =
          config.retainedStageSourceDebugConfig.effectiveRetainedStageSourceDebug := by
  intro publicInput proof checked
  exact
    guest_pc_trace_cuda_run_retained_stage_source_debug_matches
      config
      (guest_pc_trace_cuda_run_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_cuda_run_checked_acceptance_projects_retained_debug_requires_retention
    {system : VerifierModel}
    (validation : GuestPcTraceCudaRunValidation)
    (config : GuestPcTraceCudaRunConfig) :
    forall publicInput proof,
      GuestPcTraceCudaRunCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.selectedRetainedStageSourceDebug = true ->
          config.selectedRetainedStageSource = true := by
  intro publicInput proof checked debugEnabled
  exact
    guest_pc_trace_cuda_run_retained_stage_source_debug_requires_retention
      config
      (guest_pc_trace_cuda_run_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      debugEnabled

theorem guest_pc_trace_cuda_run_checked_acceptance_projects_descriptor_retention
    {system : VerifierModel}
    (validation : GuestPcTraceCudaRunValidation)
    (config : GuestPcTraceCudaRunConfig) :
    forall publicInput proof,
      GuestPcTraceCudaRunCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        config.selectedDescriptorBufferRetention =
          config.descriptorBufferRetentionConfig.effectiveDescriptorBufferRetention := by
  intro publicInput proof checked
  exact
    guest_pc_trace_cuda_run_descriptor_retention_matches
      config
      (guest_pc_trace_cuda_run_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_cuda_run_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceCudaRunValidation)
    (config : GuestPcTraceCudaRunConfig) :
    forall publicInput proof,
      GuestPcTraceCudaRunCheckedAcceptance
          system
          validation
          config
          publicInput
          proof ->
        GuestPcTraceCudaRunDecisionMatches config
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (guest_pc_trace_cuda_run_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)
      (GpuRuntimeInternal.checked_acceptance_sound_witness assumptions publicInput proof checked)

theorem guest_pc_trace_cuda_run_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceCudaRunValidation)
    (config : GuestPcTraceCudaRunConfig) :
    forall publicInput proof,
      GuestPcTraceCudaRunCheckedAcceptance
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
      (GpuRuntimeInternal.checked_acceptance_sound_witness
        assumptions
        publicInput
        proof
        checked)

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
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

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
      (GpuRuntimeInternal.checked_acceptance_sound_witness
        assumptions
        publicInput
        proof
        checked)

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
  exact
    GpuRuntimeInternal.checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

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



end Lzvm
