/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.GpuRuntime.Trace

/-!
Retained GPU runtime budget and retention contracts.
-/

namespace Lzvm

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

theorem gpu_retained_leaf_digest_limit_checked_acceptance_core_and_sound
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    gpu_retained_leaf_digest_limit_checked_acceptance_projects_decision
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

theorem gpu_retained_device_cache_budget_within_limits_projects_source_limit
    (budget : GpuRetainedDeviceCacheBudget) :
    GpuRetainedDeviceCacheBudgetWithinLimits budget ->
      budget.sourceBytes <= budget.sourceLimit := by
  intro withinLimits
  exact withinLimits.left

theorem gpu_retained_device_cache_budget_within_limits_projects_descriptor_limit
    (budget : GpuRetainedDeviceCacheBudget) :
    GpuRetainedDeviceCacheBudgetWithinLimits budget ->
      budget.descriptorBytes <= budget.descriptorLimit := by
  intro withinLimits
  exact withinLimits.right.left

theorem gpu_retained_device_cache_budget_within_limits_projects_leaf_digest_limit
    (budget : GpuRetainedDeviceCacheBudget) :
    GpuRetainedDeviceCacheBudgetWithinLimits budget ->
      budget.leafDigestBytes <= budget.leafDigestLimit := by
  intro withinLimits
  exact withinLimits.right.right.left

theorem gpu_retained_device_cache_budget_within_limits_projects_parent_checkpoint_limit
    (budget : GpuRetainedDeviceCacheBudget) :
    GpuRetainedDeviceCacheBudgetWithinLimits budget ->
      budget.parentCheckpointBytes <= budget.parentCheckpointLimit := by
  intro withinLimits
  exact withinLimits.right.right.right.left

theorem gpu_retained_device_cache_budget_within_limits_projects_combined_limit
    (budget : GpuRetainedDeviceCacheBudget)
    (limit : Nat) :
    budget.combinedLimit = some limit ->
      GpuRetainedDeviceCacheBudgetWithinLimits budget ->
        budget.sourceBytes
          + budget.descriptorBytes
          + budget.leafDigestBytes
          + budget.parentCheckpointBytes <= limit := by
  intro combinedLimit withinLimits
  have combinedWithin := withinLimits.right.right.right.right
  rw [combinedLimit] at combinedWithin
  exact combinedWithin

theorem gpu_retained_device_cache_budget_parent_checkpoint_headroom_implies_combined_limit
    (budget : GpuRetainedDeviceCacheBudget)
    (limit : Nat) :
    budget.parentCheckpointBytes <= budget.parentCheckpointLimit ->
      budget.sourceBytes
        + budget.descriptorBytes
        + budget.leafDigestBytes
        + budget.parentCheckpointLimit <= limit ->
        budget.sourceBytes
          + budget.descriptorBytes
          + budget.leafDigestBytes
          + budget.parentCheckpointBytes <= limit := by
  intro parentWithinLimit headroomWithin
  omega

theorem gpu_retained_device_cache_budget_checked_acceptance_projects_source_limit
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
        budget.sourceBytes <= budget.sourceLimit := by
  intro publicInput proof checked
  exact
    gpu_retained_device_cache_budget_within_limits_projects_source_limit
      budget
      (gpu_retained_device_cache_budget_checked_acceptance_projects_within_limits
        validation
        budget
        publicInput
        proof
        checked)

theorem gpu_retained_device_cache_budget_checked_acceptance_projects_descriptor_limit
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
        budget.descriptorBytes <= budget.descriptorLimit := by
  intro publicInput proof checked
  exact
    gpu_retained_device_cache_budget_within_limits_projects_descriptor_limit
      budget
      (gpu_retained_device_cache_budget_checked_acceptance_projects_within_limits
        validation
        budget
        publicInput
        proof
        checked)

theorem gpu_retained_device_cache_budget_checked_acceptance_projects_leaf_digest_limit
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
        budget.leafDigestBytes <= budget.leafDigestLimit := by
  intro publicInput proof checked
  exact
    gpu_retained_device_cache_budget_within_limits_projects_leaf_digest_limit
      budget
      (gpu_retained_device_cache_budget_checked_acceptance_projects_within_limits
        validation
        budget
        publicInput
        proof
        checked)

theorem gpu_retained_device_cache_budget_checked_acceptance_projects_parent_checkpoint_limit
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
        budget.parentCheckpointBytes <= budget.parentCheckpointLimit := by
  intro publicInput proof checked
  exact
    gpu_retained_device_cache_budget_within_limits_projects_parent_checkpoint_limit
      budget
      (gpu_retained_device_cache_budget_checked_acceptance_projects_within_limits
        validation
        budget
        publicInput
        proof
        checked)

theorem gpu_retained_device_cache_budget_checked_acceptance_projects_combined_limit
    {system : VerifierModel}
    (validation : GpuRetainedDeviceCacheBudgetValidation)
    (budget : GpuRetainedDeviceCacheBudget)
    (limit : Nat) :
    budget.combinedLimit = some limit ->
      forall publicInput proof,
        GpuRetainedDeviceCacheBudgetCheckedAcceptance
            system
            validation
            budget
            publicInput
            proof ->
          budget.sourceBytes
            + budget.descriptorBytes
            + budget.leafDigestBytes
            + budget.parentCheckpointBytes <= limit := by
  intro combinedLimit publicInput proof checked
  exact
    gpu_retained_device_cache_budget_within_limits_projects_combined_limit
      budget
      limit
      combinedLimit
      (gpu_retained_device_cache_budget_checked_acceptance_projects_within_limits
        validation
        budget
        publicInput
        proof
        checked)

theorem gpu_retained_device_cache_budget_checked_parent_checkpoint_headroom_implies_combined_limit
    {system : VerifierModel}
    (validation : GpuRetainedDeviceCacheBudgetValidation)
    (budget : GpuRetainedDeviceCacheBudget)
    (limit : Nat) :
    budget.sourceBytes
      + budget.descriptorBytes
      + budget.leafDigestBytes
      + budget.parentCheckpointLimit <= limit ->
      forall publicInput proof,
        GpuRetainedDeviceCacheBudgetCheckedAcceptance
            system
            validation
            budget
            publicInput
            proof ->
          budget.sourceBytes
            + budget.descriptorBytes
            + budget.leafDigestBytes
            + budget.parentCheckpointBytes <= limit := by
  intro headroomWithin publicInput proof checked
  exact
    gpu_retained_device_cache_budget_parent_checkpoint_headroom_implies_combined_limit
      budget
      limit
      (gpu_retained_device_cache_budget_checked_acceptance_projects_parent_checkpoint_limit
        validation
        budget
        publicInput
        proof
        checked)
      headroomWithin

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

theorem gpu_retained_device_cache_budget_checked_acceptance_core_and_sound
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have withinLimits :=
    gpu_retained_device_cache_budget_checked_acceptance_projects_within_limits
      validation
      budget
      publicInput
      proof
      checked
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro withinLimits coreAndSound

theorem gpu_retained_device_cache_budget_checked_limits_core_and_sound
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
        budget.sourceBytes <= budget.sourceLimit
          /\ budget.descriptorBytes <= budget.descriptorLimit
          /\ budget.leafDigestBytes <= budget.leafDigestLimit
          /\ budget.parentCheckpointBytes <= budget.parentCheckpointLimit
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have sourceLimit :=
    gpu_retained_device_cache_budget_checked_acceptance_projects_source_limit
      validation
      budget
      publicInput
      proof
      checked
  have descriptorLimit :=
    gpu_retained_device_cache_budget_checked_acceptance_projects_descriptor_limit
      validation
      budget
      publicInput
      proof
      checked
  have leafDigestLimit :=
    gpu_retained_device_cache_budget_checked_acceptance_projects_leaf_digest_limit
      validation
      budget
      publicInput
      proof
      checked
  have parentCheckpointLimit :=
    gpu_retained_device_cache_budget_checked_acceptance_projects_parent_checkpoint_limit
      validation
      budget
      publicInput
      proof
      checked
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro sourceLimit
      (And.intro descriptorLimit
        (And.intro leafDigestLimit
          (And.intro parentCheckpointLimit coreAndSound)))

theorem gpu_retained_device_cache_budget_checked_combined_limit_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuRetainedDeviceCacheBudgetValidation)
    (budget : GpuRetainedDeviceCacheBudget)
    (limit : Nat) :
    budget.combinedLimit = some limit ->
      forall publicInput proof,
        GpuRetainedDeviceCacheBudgetCheckedAcceptance
            system
            validation
            budget
            publicInput
            proof ->
          budget.sourceBytes
            + budget.descriptorBytes
            + budget.leafDigestBytes
            + budget.parentCheckpointBytes <= limit
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro combinedLimit publicInput proof checked
  have combinedWithin :=
    gpu_retained_device_cache_budget_checked_acceptance_projects_combined_limit
      validation
      budget
      limit
      combinedLimit
      publicInput
      proof
      checked
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro combinedWithin coreAndSound

theorem gpu_retained_device_cache_budget_checked_parent_checkpoint_headroom_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuRetainedDeviceCacheBudgetValidation)
    (budget : GpuRetainedDeviceCacheBudget)
    (limit : Nat) :
    budget.sourceBytes
      + budget.descriptorBytes
      + budget.leafDigestBytes
      + budget.parentCheckpointLimit <= limit ->
      forall publicInput proof,
        GpuRetainedDeviceCacheBudgetCheckedAcceptance
            system
            validation
            budget
            publicInput
            proof ->
          budget.sourceBytes
            + budget.descriptorBytes
            + budget.leafDigestBytes
            + budget.parentCheckpointBytes <= limit
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro headroomWithin publicInput proof checked
  have combinedWithin :=
    gpu_retained_device_cache_budget_checked_parent_checkpoint_headroom_implies_combined_limit
      validation
      budget
      limit
      headroomWithin
      publicInput
      proof
      checked
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro combinedWithin coreAndSound

theorem guest_pc_trace_commit_mode_checked_acceptance_audited_core_contract
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ GuestPcTraceSegmentCommitModeDecisionMatches config
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    guest_pc_trace_commit_mode_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have audited :=
    GpuRuntimeInternal.checked_acceptance_audited_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.segmentCommitModeConfigAccepted config publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro decision audited.right.right))

theorem guest_pc_trace_device_trace_source_checked_acceptance_audited_core_contract
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ GuestPcTraceDeviceTraceSourceDecisionMatches config
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    guest_pc_trace_device_trace_source_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have audited :=
    GpuRuntimeInternal.checked_acceptance_audited_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.deviceTraceSourceConfigAccepted config publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro decision audited.right.right))

theorem guest_pc_trace_sparse_source_checked_acceptance_audited_core_contract
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ GuestPcTraceSparseSourceDecisionMatches config
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    guest_pc_trace_sparse_source_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have audited :=
    GpuRuntimeInternal.checked_acceptance_audited_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.sparseSourceConfigAccepted config publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro decision audited.right.right))

theorem guest_pc_trace_terminal_sparse_source_checked_acceptance_audited_core_contract
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ GuestPcTraceTerminalSparseSourceDecisionMatches config
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    guest_pc_trace_terminal_sparse_source_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have audited :=
    GpuRuntimeInternal.checked_acceptance_audited_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.terminalSparseSourceConfigAccepted config publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro decision audited.right.right))

theorem fri_retained_stage_source_checked_acceptance_audited_core_contract
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ FriRetainedStageSourceDecisionMatches config
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    fri_retained_stage_source_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have audited :=
    GpuRuntimeInternal.checked_acceptance_audited_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.retainedStageSourceConfigAccepted config publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro decision audited.right.right))

theorem guest_pc_trace_cuda_run_checked_acceptance_audited_core_contract
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ GuestPcTraceCudaRunDecisionMatches config
          /\ config.selectedSparseSource =
            config.sparseSourceConfig.effectiveSparseSourceSelected
          /\ config.selectedSparseSourceDebug =
            config.sparseSourceDebugConfig.effectiveSparseSourceDebug
          /\ config.selectedTerminalSparseSource =
            config.terminalSparseSourceConfig.effectiveTerminalSparseSourceSelected
          /\ config.selectedRetainedStageSource =
            config.retainedStageSourceConfig.effectiveRetainedStageSourceEnabled
          /\ config.selectedRetainedStageSourceDebug =
            config.retainedStageSourceDebugConfig.effectiveRetainedStageSourceDebug
          /\ (config.selectedRetainedStageSourceDebug = true ->
            config.selectedRetainedStageSource = true)
          /\ config.selectedDescriptorBufferRetention =
            config.descriptorBufferRetentionConfig.effectiveDescriptorBufferRetention
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    guest_pc_trace_cuda_run_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have sparse :=
    guest_pc_trace_cuda_run_checked_acceptance_projects_sparse_source
      validation
      config
      publicInput
      proof
      checked
  have sparseDebug :=
    guest_pc_trace_cuda_run_checked_acceptance_projects_sparse_source_debug
      validation
      config
      publicInput
      proof
      checked
  have terminalSparse :=
    guest_pc_trace_cuda_run_checked_acceptance_projects_terminal_sparse_source
      validation
      config
      publicInput
      proof
      checked
  have retainedStage :=
    guest_pc_trace_cuda_run_checked_acceptance_projects_retained_stage_source
      validation
      config
      publicInput
      proof
      checked
  have retainedDebug :=
    guest_pc_trace_cuda_run_checked_acceptance_projects_retained_source_debug
      validation
      config
      publicInput
      proof
      checked
  have retainedDebugRequiresRetention :=
    guest_pc_trace_cuda_run_checked_acceptance_projects_retained_debug_requires_retention
      validation
      config
      publicInput
      proof
      checked
  have descriptorRetention :=
    guest_pc_trace_cuda_run_checked_acceptance_projects_descriptor_retention
      validation
      config
      publicInput
      proof
      checked
  have audited :=
    GpuRuntimeInternal.checked_acceptance_audited_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.traceCudaRunConfigAccepted config publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro decision
          (And.intro sparse
            (And.intro sparseDebug
              (And.intro terminalSparse
                (And.intro retainedStage
                  (And.intro retainedDebug
                    (And.intro retainedDebugRequiresRetention
                      (And.intro descriptorRetention audited.right.right)))))))))

theorem guest_pc_trace_cuda_run_checked_parallel_lower_retention_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceCudaRunValidation)
    (config : GuestPcTraceCudaRunConfig) :
    config.descriptorBufferRetentionConfig.configuredDescriptorBufferRetention = none ->
      config.descriptorBufferRetentionConfig.parallelLowerEnabledForDescriptorRetention = true ->
        forall publicInput proof,
          GuestPcTraceCudaRunCheckedAcceptance
              system
              validation
              config
              publicInput
              proof ->
            RequiredCryptographicAssumptionStatements assumptions.crypto
              /\ RequiredSemanticAssumptionStatements assumptions.semantic
              /\ config.selectedDescriptorBufferRetention = false
              /\ RuntimeVerifierCoreContract system publicInput proof
              /\ SoundWitness system publicInput proof := by
  intro configuredNone parallelEnabled publicInput proof checked
  have audited :=
    GpuRuntimeInternal.checked_acceptance_audited_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.traceCudaRunConfigAccepted config publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro
          (guest_pc_trace_cuda_run_checked_acceptance_parallel_lower_disables_descriptor_retention
            validation
            config
            configuredNone
            parallelEnabled
            publicInput
            proof
            checked)
          audited.right.right))

theorem guest_pc_trace_cuda_run_checked_explicit_retention_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GuestPcTraceCudaRunValidation)
    (config : GuestPcTraceCudaRunConfig)
    (configured : Bool) :
    config.descriptorBufferRetentionConfig.configuredDescriptorBufferRetention = some configured ->
      forall publicInput proof,
        GuestPcTraceCudaRunCheckedAcceptance
            system
            validation
            config
            publicInput
            proof ->
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ config.selectedDescriptorBufferRetention = configured
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro configuredSome publicInput proof checked
  have audited :=
    GpuRuntimeInternal.checked_acceptance_audited_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.traceCudaRunConfigAccepted config publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro
          (guest_pc_trace_cuda_run_checked_acceptance_explicit_retention_override_matches
            validation
            config
            configured
            configuredSome
            publicInput
            proof
            checked)
          audited.right.right))

theorem gpu_retained_leaf_digest_limit_checked_acceptance_audited_core_contract
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ GpuRetainedLeafDigestLimitDecisionMatches config
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have decision :=
    gpu_retained_leaf_digest_limit_checked_acceptance_projects_decision
      validation
      config
      publicInput
      proof
      checked
  have audited :=
    GpuRuntimeInternal.checked_acceptance_audited_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.retainedLeafDigestLimitConfigAccepted config publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro decision audited.right.right))

theorem gpu_retained_device_cache_budget_checked_acceptance_audited_core_contract
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ GpuRetainedDeviceCacheBudgetWithinLimits budget
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have withinLimits :=
    gpu_retained_device_cache_budget_checked_acceptance_projects_within_limits
      validation
      budget
      publicInput
      proof
      checked
  have audited :=
    GpuRuntimeInternal.checked_acceptance_audited_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.retainedDeviceCacheBudgetAccepted budget publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro withinLimits audited.right.right))

theorem gpu_retained_device_cache_budget_checked_limits_audited_core_contract
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ budget.sourceBytes <= budget.sourceLimit
          /\ budget.descriptorBytes <= budget.descriptorLimit
          /\ budget.leafDigestBytes <= budget.leafDigestLimit
          /\ budget.parentCheckpointBytes <= budget.parentCheckpointLimit
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have sourceLimit :=
    gpu_retained_device_cache_budget_checked_acceptance_projects_source_limit
      validation
      budget
      publicInput
      proof
      checked
  have descriptorLimit :=
    gpu_retained_device_cache_budget_checked_acceptance_projects_descriptor_limit
      validation
      budget
      publicInput
      proof
      checked
  have leafDigestLimit :=
    gpu_retained_device_cache_budget_checked_acceptance_projects_leaf_digest_limit
      validation
      budget
      publicInput
      proof
      checked
  have parentCheckpointLimit :=
    gpu_retained_device_cache_budget_checked_acceptance_projects_parent_checkpoint_limit
      validation
      budget
      publicInput
      proof
      checked
  have audited :=
    GpuRuntimeInternal.checked_acceptance_audited_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.retainedDeviceCacheBudgetAccepted budget publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro sourceLimit
          (And.intro descriptorLimit
            (And.intro leafDigestLimit
              (And.intro parentCheckpointLimit audited.right.right)))))

theorem gpu_retained_device_cache_budget_checked_combined_limit_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuRetainedDeviceCacheBudgetValidation)
    (budget : GpuRetainedDeviceCacheBudget)
    (limit : Nat) :
    budget.combinedLimit = some limit ->
      forall publicInput proof,
        GpuRetainedDeviceCacheBudgetCheckedAcceptance
            system
            validation
            budget
            publicInput
            proof ->
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ budget.sourceBytes
              + budget.descriptorBytes
              + budget.leafDigestBytes
              + budget.parentCheckpointBytes <= limit
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro combinedLimit publicInput proof checked
  have combinedWithin :=
    gpu_retained_device_cache_budget_checked_acceptance_projects_combined_limit
      validation
      budget
      limit
      combinedLimit
      publicInput
      proof
      checked
  have audited :=
    GpuRuntimeInternal.checked_acceptance_audited_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.retainedDeviceCacheBudgetAccepted budget publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro combinedWithin audited.right.right))

theorem gpu_retained_device_cache_budget_checked_parent_checkpoint_headroom_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuRetainedDeviceCacheBudgetValidation)
    (budget : GpuRetainedDeviceCacheBudget)
    (limit : Nat) :
    budget.sourceBytes
      + budget.descriptorBytes
      + budget.leafDigestBytes
      + budget.parentCheckpointLimit <= limit ->
      forall publicInput proof,
        GpuRetainedDeviceCacheBudgetCheckedAcceptance
            system
            validation
            budget
            publicInput
            proof ->
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ budget.sourceBytes
              + budget.descriptorBytes
              + budget.leafDigestBytes
              + budget.parentCheckpointBytes <= limit
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro headroomWithin publicInput proof checked
  have combinedWithin :=
    gpu_retained_device_cache_budget_checked_parent_checkpoint_headroom_implies_combined_limit
      validation
      budget
      limit
      headroomWithin
      publicInput
      proof
      checked
  have audited :=
    GpuRuntimeInternal.checked_acceptance_audited_core_contract
      (auxiliaryAccepted := fun publicInput proof =>
        validation.retainedDeviceCacheBudgetAccepted budget publicInput proof)
      assumptions
      publicInput
      proof
      checked
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro combinedWithin audited.right.right))

end Lzvm
