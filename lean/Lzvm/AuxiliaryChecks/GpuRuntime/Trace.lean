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

theorem guest_pc_trace_commit_worker_default_positive
    (config : GuestPcTraceSegmentCommitWorkerDefaultConfig) :
    GuestPcTraceSegmentCommitWorkerDefaultDecisionMatches config ->
      0 < config.defaultWorkerCount := by
  intro decision
  rcases decision with
    ⟨_thresholdPositive, pipelinePositive, defaultMatches⟩
  cases selected : GuestPcTraceSegmentCommitWorkerDefaultPipelineSelected config <;>
    simp [defaultMatches, selected, pipelinePositive]

theorem guest_pc_trace_commit_worker_default_disabled_override_serial
    (config : GuestPcTraceSegmentCommitWorkerDefaultConfig) :
    config.configuredCommitPipeline = some false ->
      GuestPcTraceSegmentCommitWorkerDefaultDecisionMatches config ->
        config.defaultWorkerCount = 1 := by
  intro configuredDisabled decision
  rcases decision with
    ⟨_thresholdPositive, _pipelinePositive, defaultMatches⟩
  have selectedFalse :
      GuestPcTraceSegmentCommitWorkerDefaultPipelineSelected config = false := by
    simp [GuestPcTraceSegmentCommitWorkerDefaultPipelineSelected,
      configuredDisabled]
  simpa [selectedFalse] using defaultMatches

theorem guest_pc_trace_parallel_lower_explicit_selects_parallel_lower
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredParallelLower = true ->
      GuestPcTraceParallelLowerDecisionMatches config ->
        config.effectiveParallelLower = true := by
  intro parallelLowerEnabled decision
  rcases decision with ⟨parallelMatches, _replayOnlyMatches,
    _replaySnapshotMatches⟩
  simpa [parallelLowerEnabled] using parallelMatches

theorem guest_pc_trace_parallel_lower_work_units_selects_parallel_lower
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredWorkUnits = true ->
      GuestPcTraceParallelLowerDecisionMatches config ->
        config.effectiveParallelLower = true := by
  intro workUnitsEnabled decision
  rcases decision with ⟨parallelMatches, _replayOnlyMatches,
    _replaySnapshotMatches⟩
  simpa [workUnitsEnabled] using parallelMatches

theorem guest_pc_trace_parallel_lower_replay_only_selects_replay_only
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredReplayOnly = true ->
      GuestPcTraceParallelLowerDecisionMatches config ->
        config.effectiveReplayOnly = true := by
  intro replayOnlyEnabled decision
  rcases decision with ⟨_parallelMatches, replayOnlyMatches,
    _replaySnapshotMatches⟩
  simpa [replayOnlyEnabled] using replayOnlyMatches

theorem guest_pc_trace_parallel_lower_replay_only_selects_replay_snapshot
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredReplayOnly = true ->
      GuestPcTraceParallelLowerDecisionMatches config ->
        config.effectiveReplaySnapshot = true := by
  intro replayOnlyEnabled decision
  rcases decision with ⟨_parallelMatches, _replayOnlyMatches,
    replaySnapshotMatches⟩
  simpa [replayOnlyEnabled] using replaySnapshotMatches

theorem guest_pc_trace_parallel_lower_replay_snapshot_selects_replay_snapshot
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredReplaySnapshot = true ->
      GuestPcTraceParallelLowerDecisionMatches config ->
        config.effectiveReplaySnapshot = true := by
  intro replaySnapshotEnabled decision
  rcases decision with ⟨_parallelMatches, _replayOnlyMatches,
    replaySnapshotMatches⟩
  simpa [replaySnapshotEnabled] using replaySnapshotMatches

theorem guest_pc_trace_parallel_lower_work_units_keeps_replay_only_separate
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredWorkUnits = true ->
      config.configuredReplayOnly = false ->
        GuestPcTraceParallelLowerDecisionMatches config ->
          config.effectiveReplayOnly = false := by
  intro _workUnitsEnabled replayOnlyDisabled decision
  rcases decision with ⟨_parallelMatches, replayOnlyMatches,
    _replaySnapshotMatches⟩
  simpa [replayOnlyDisabled] using replayOnlyMatches

theorem guest_pc_trace_parallel_lower_work_units_keeps_replay_snapshot_separate
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredWorkUnits = true ->
      config.configuredReplayOnly = false ->
        config.configuredReplaySnapshot = false ->
          GuestPcTraceParallelLowerDecisionMatches config ->
            config.effectiveReplaySnapshot = false := by
  intro _workUnitsEnabled replayOnlyDisabled replaySnapshotDisabled decision
  rcases decision with ⟨_parallelMatches, _replayOnlyMatches,
    replaySnapshotMatches⟩
  simpa [replayOnlyDisabled, replaySnapshotDisabled] using
    replaySnapshotMatches

theorem guest_pc_trace_parallel_lower_checked_acceptance_explicit_selects_parallel_lower
    {system : VerifierModel}
    (validation : GuestPcTraceParallelLowerValidation)
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredParallelLower = true ->
      forall publicInput proof,
        GuestPcTraceParallelLowerCheckedAcceptance
            system
            validation
            config
            publicInput
            proof ->
          config.effectiveParallelLower = true := by
  intro parallelLowerEnabled publicInput proof checked
  exact
    guest_pc_trace_parallel_lower_explicit_selects_parallel_lower
      config
      parallelLowerEnabled
      (guest_pc_trace_parallel_lower_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_parallel_lower_checked_acceptance_work_units_selects_parallel_lower
    {system : VerifierModel}
    (validation : GuestPcTraceParallelLowerValidation)
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredWorkUnits = true ->
      forall publicInput proof,
        GuestPcTraceParallelLowerCheckedAcceptance
            system
            validation
            config
            publicInput
            proof ->
          config.effectiveParallelLower = true := by
  intro workUnitsEnabled publicInput proof checked
  exact
    guest_pc_trace_parallel_lower_work_units_selects_parallel_lower
      config
      workUnitsEnabled
      (guest_pc_trace_parallel_lower_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_parallel_lower_checked_acceptance_replay_only_selects_replay_only
    {system : VerifierModel}
    (validation : GuestPcTraceParallelLowerValidation)
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredReplayOnly = true ->
      forall publicInput proof,
        GuestPcTraceParallelLowerCheckedAcceptance
            system
            validation
            config
            publicInput
            proof ->
          config.effectiveReplayOnly = true := by
  intro replayOnlyEnabled publicInput proof checked
  exact
    guest_pc_trace_parallel_lower_replay_only_selects_replay_only
      config
      replayOnlyEnabled
      (guest_pc_trace_parallel_lower_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_parallel_lower_checked_acceptance_replay_only_selects_replay_snapshot
    {system : VerifierModel}
    (validation : GuestPcTraceParallelLowerValidation)
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredReplayOnly = true ->
      forall publicInput proof,
        GuestPcTraceParallelLowerCheckedAcceptance
            system
            validation
            config
            publicInput
            proof ->
          config.effectiveReplaySnapshot = true := by
  intro replayOnlyEnabled publicInput proof checked
  exact
    guest_pc_trace_parallel_lower_replay_only_selects_replay_snapshot
      config
      replayOnlyEnabled
      (guest_pc_trace_parallel_lower_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_parallel_lower_checked_acceptance_replay_snapshot_selects_replay_snapshot
    {system : VerifierModel}
    (validation : GuestPcTraceParallelLowerValidation)
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredReplaySnapshot = true ->
      forall publicInput proof,
        GuestPcTraceParallelLowerCheckedAcceptance
            system
            validation
            config
            publicInput
            proof ->
          config.effectiveReplaySnapshot = true := by
  intro replaySnapshotEnabled publicInput proof checked
  exact
    guest_pc_trace_parallel_lower_replay_snapshot_selects_replay_snapshot
      config
      replaySnapshotEnabled
      (guest_pc_trace_parallel_lower_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_parallel_lower_checked_acceptance_work_units_keeps_replay_only_separate
    {system : VerifierModel}
    (validation : GuestPcTraceParallelLowerValidation)
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredWorkUnits = true ->
      config.configuredReplayOnly = false ->
        forall publicInput proof,
          GuestPcTraceParallelLowerCheckedAcceptance
              system
              validation
              config
              publicInput
              proof ->
            config.effectiveReplayOnly = false := by
  intro workUnitsEnabled replayOnlyDisabled publicInput proof checked
  exact
    guest_pc_trace_parallel_lower_work_units_keeps_replay_only_separate
      config
      workUnitsEnabled
      replayOnlyDisabled
      (guest_pc_trace_parallel_lower_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

theorem guest_pc_trace_parallel_lower_checked_acceptance_work_units_keeps_replay_snapshot_separate
    {system : VerifierModel}
    (validation : GuestPcTraceParallelLowerValidation)
    (config : GuestPcTraceParallelLowerConfig) :
    config.configuredWorkUnits = true ->
      config.configuredReplayOnly = false ->
        config.configuredReplaySnapshot = false ->
          forall publicInput proof,
            GuestPcTraceParallelLowerCheckedAcceptance
                system
                validation
                config
                publicInput
                proof ->
              config.effectiveReplaySnapshot = false := by
  intro workUnitsEnabled replayOnlyDisabled replaySnapshotDisabled
    publicInput proof checked
  exact
    guest_pc_trace_parallel_lower_work_units_keeps_replay_snapshot_separate
      config
      workUnitsEnabled
      replayOnlyDisabled
      replaySnapshotDisabled
      (guest_pc_trace_parallel_lower_checked_acceptance_projects_decision
        validation
        config
        publicInput
        proof
        checked)

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

theorem guest_pc_trace_descriptor_buffer_retention_explicit_override_matches
    (config : GuestPcTraceDescriptorBufferRetentionConfig)
    (configured : Bool) :
    config.configuredDescriptorBufferRetention = some configured ->
      GuestPcTraceDescriptorBufferRetentionDecisionMatches config ->
        config.effectiveDescriptorBufferRetention = configured := by
  intro configuredSome decision
  rcases decision with ⟨_limitPositive, retentionMatches⟩
  simpa [configuredSome] using retentionMatches

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

theorem guest_pc_trace_commit_mode_checked_acceptance_core_and_sound
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
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro decision coreAndSound




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

theorem guest_pc_trace_device_trace_source_checked_acceptance_core_and_sound
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
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro decision coreAndSound

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

theorem guest_pc_trace_sparse_source_checked_acceptance_core_and_sound
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
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro decision coreAndSound

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

theorem guest_pc_trace_terminal_sparse_source_checked_acceptance_core_and_sound
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
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro decision coreAndSound

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

theorem fri_retained_stage_source_checked_acceptance_core_and_sound
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
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro decision coreAndSound

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

theorem guest_pc_trace_cuda_run_checked_acceptance_core_and_sound
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
  have coreAndSound :=
    GpuRuntimeInternal.checked_acceptance_core_and_sound
      assumptions
      publicInput
      proof
      checked
  exact And.intro decision
    (And.intro sparse
      (And.intro sparseDebug
        (And.intro terminalSparse
          (And.intro retainedStage
            (And.intro retainedDebug
              (And.intro retainedDebugRequiresRetention
                (And.intro descriptorRetention coreAndSound)))))))

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



end Lzvm
