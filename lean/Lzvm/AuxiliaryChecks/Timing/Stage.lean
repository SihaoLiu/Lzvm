/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.Timing.Trace

/-!
Runtime retention and stage timing acceptance projections for auxiliary verifier metadata.
-/

namespace Lzvm

theorem guest_pc_trace_source_retention_byte_counts_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (retainedBytes rejectedBytes limitBytes : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageSourceRetentionRetainedByteCount := retainedBytes
            guestStageSourceRetentionRejectedByteCount := rejectedBytes
            guestStageSourceRetentionLimitByteCount := limitBytes })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestStageSourceRetentionRetainedByteCount := retainedBytes
          guestStageSourceRetentionRejectedByteCount := rejectedBytes
          guestStageSourceRetentionLimitByteCount := limitBytes })
      publicInput
      proof
      observed

theorem guest_pc_trace_source_retention_byte_counts_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (retainedBytes rejectedBytes limitBytes : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageSourceRetentionRetainedByteCount := retainedBytes
            guestStageSourceRetentionRejectedByteCount := rejectedBytes
            guestStageSourceRetentionLimitByteCount := limitBytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestStageSourceRetentionRetainedByteCount := retainedBytes
            guestStageSourceRetentionRejectedByteCount := rejectedBytes
            guestStageSourceRetentionLimitByteCount := limitBytes }
      publicInput
      proof
      observed

theorem guest_pc_trace_source_retention_byte_counts_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (retainedBytes rejectedBytes limitBytes : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageSourceRetentionRetainedByteCount := retainedBytes
            guestStageSourceRetentionRejectedByteCount := rejectedBytes
            guestStageSourceRetentionLimitByteCount := limitBytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_source_retention_byte_counts_acceptance_verifier_core_contract
      assumptions
      summary
      retainedBytes
      rejectedBytes
      limitBytes
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_source_retention_byte_counts_acceptance_sound
      assumptions
      summary
      retainedBytes
      rejectedBytes
      limitBytes
      publicInput
      proof
      observed
  exact And.intro core sound

theorem guest_pc_trace_source_retention_counts_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (attemptCount retainedCount rejectedCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageSourceRetentionAttemptCount := attemptCount
            guestStageSourceRetentionRetainedCount := retainedCount
            guestStageSourceRetentionRejectedCount := rejectedCount })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestStageSourceRetentionAttemptCount := attemptCount
          guestStageSourceRetentionRetainedCount := retainedCount
          guestStageSourceRetentionRejectedCount := rejectedCount })
      publicInput
      proof
      observed

theorem guest_pc_trace_source_retention_counts_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (attemptCount retainedCount rejectedCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageSourceRetentionAttemptCount := attemptCount
            guestStageSourceRetentionRetainedCount := retainedCount
            guestStageSourceRetentionRejectedCount := rejectedCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestStageSourceRetentionAttemptCount := attemptCount
            guestStageSourceRetentionRetainedCount := retainedCount
            guestStageSourceRetentionRejectedCount := rejectedCount }
      publicInput
      proof
      observed

theorem guest_pc_trace_source_retention_counts_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (attemptCount retainedCount rejectedCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageSourceRetentionAttemptCount := attemptCount
            guestStageSourceRetentionRetainedCount := retainedCount
            guestStageSourceRetentionRejectedCount := rejectedCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_source_retention_counts_acceptance_verifier_core_contract
      assumptions
      summary
      attemptCount
      retainedCount
      rejectedCount
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_source_retention_counts_acceptance_sound
      assumptions
      summary
      attemptCount
      retainedCount
      rejectedCount
      publicInput
      proof
      observed
  exact And.intro core sound

theorem guest_pc_trace_descriptor_buffer_retention_byte_counts_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (retainedBytes rejectedBytes limitBytes : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDescriptorBufferRetentionRetainedByteCount := retainedBytes
            guestDescriptorBufferRetentionRejectedByteCount := rejectedBytes
            guestDescriptorBufferRetentionLimitByteCount := limitBytes })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestDescriptorBufferRetentionRetainedByteCount := retainedBytes
          guestDescriptorBufferRetentionRejectedByteCount := rejectedBytes
          guestDescriptorBufferRetentionLimitByteCount := limitBytes })
      publicInput
      proof
      observed

theorem guest_pc_trace_descriptor_buffer_retention_byte_counts_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (retainedBytes rejectedBytes limitBytes : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDescriptorBufferRetentionRetainedByteCount := retainedBytes
            guestDescriptorBufferRetentionRejectedByteCount := rejectedBytes
            guestDescriptorBufferRetentionLimitByteCount := limitBytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestDescriptorBufferRetentionRetainedByteCount := retainedBytes
            guestDescriptorBufferRetentionRejectedByteCount := rejectedBytes
            guestDescriptorBufferRetentionLimitByteCount := limitBytes }
      publicInput
      proof
      observed

theorem guest_pc_trace_descriptor_buffer_retention_byte_counts_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (retainedBytes rejectedBytes limitBytes : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDescriptorBufferRetentionRetainedByteCount := retainedBytes
            guestDescriptorBufferRetentionRejectedByteCount := rejectedBytes
            guestDescriptorBufferRetentionLimitByteCount := limitBytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_descriptor_buffer_retention_byte_counts_acceptance_verifier_core_contract
      assumptions
      summary
      retainedBytes
      rejectedBytes
      limitBytes
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_descriptor_buffer_retention_byte_counts_acceptance_sound
      assumptions
      summary
      retainedBytes
      rejectedBytes
      limitBytes
      publicInput
      proof
      observed
  exact And.intro core sound

theorem guest_pc_trace_descriptor_buffer_retention_counts_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (attemptCount retainedCount rejectedCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDescriptorBufferRetentionAttemptCount := attemptCount
            guestDescriptorBufferRetentionRetainedCount := retainedCount
            guestDescriptorBufferRetentionRejectedCount := rejectedCount })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestDescriptorBufferRetentionAttemptCount := attemptCount
          guestDescriptorBufferRetentionRetainedCount := retainedCount
          guestDescriptorBufferRetentionRejectedCount := rejectedCount })
      publicInput
      proof
      observed

theorem guest_pc_trace_descriptor_buffer_retention_counts_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (attemptCount retainedCount rejectedCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDescriptorBufferRetentionAttemptCount := attemptCount
            guestDescriptorBufferRetentionRetainedCount := retainedCount
            guestDescriptorBufferRetentionRejectedCount := rejectedCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestDescriptorBufferRetentionAttemptCount := attemptCount
            guestDescriptorBufferRetentionRetainedCount := retainedCount
            guestDescriptorBufferRetentionRejectedCount := rejectedCount }
      publicInput
      proof
      observed

theorem guest_pc_trace_descriptor_buffer_retention_counts_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (attemptCount retainedCount rejectedCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDescriptorBufferRetentionAttemptCount := attemptCount
            guestDescriptorBufferRetentionRetainedCount := retainedCount
            guestDescriptorBufferRetentionRejectedCount := rejectedCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_descriptor_buffer_retention_counts_acceptance_verifier_core_contract
      assumptions
      summary
      attemptCount
      retainedCount
      rejectedCount
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_descriptor_buffer_retention_counts_acceptance_sound
      assumptions
      summary
      attemptCount
      retainedCount
      rejectedCount
      publicInput
      proof
      observed
  exact And.intro core sound

theorem guest_pc_trace_leaf_output_cache_counts_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (hitCount missCount : Nat)
    (stageTimings : List GuestPcTraceStageTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafOutputCacheHitCount := hitCount
            guestStageLeafOutputCacheMissCount := missCount
            stageTimings := stageTimings })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestStageLeafOutputCacheHitCount := hitCount
          guestStageLeafOutputCacheMissCount := missCount
          stageTimings := stageTimings })
      publicInput
      proof
      observed

theorem guest_pc_trace_leaf_output_cache_counts_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (hitCount missCount : Nat)
    (stageTimings : List GuestPcTraceStageTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafOutputCacheHitCount := hitCount
            guestStageLeafOutputCacheMissCount := missCount
            stageTimings := stageTimings })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestStageLeafOutputCacheHitCount := hitCount
            guestStageLeafOutputCacheMissCount := missCount
            stageTimings := stageTimings }
      publicInput
      proof
      observed

theorem guest_pc_trace_leaf_output_cache_counts_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (hitCount missCount : Nat)
    (stageTimings : List GuestPcTraceStageTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafOutputCacheHitCount := hitCount
            guestStageLeafOutputCacheMissCount := missCount
            stageTimings := stageTimings })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_leaf_output_cache_counts_acceptance_verifier_core_contract
      assumptions
      summary
      hitCount
      missCount
      stageTimings
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_leaf_output_cache_counts_acceptance_sound
      assumptions
      summary
      hitCount
      missCount
      stageTimings
      publicInput
      proof
      observed
  exact And.intro core sound

theorem guest_pc_trace_leaf_extend_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (extendMilliseconds : Nat)
    (stageTimings : List GuestPcTraceStageTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafExtendWorkMilliseconds := extendMilliseconds
            stageTimings := stageTimings })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestStageLeafExtendWorkMilliseconds := extendMilliseconds
          stageTimings := stageTimings })
      publicInput
      proof
      observed

theorem guest_pc_trace_leaf_extend_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (extendMilliseconds : Nat)
    (stageTimings : List GuestPcTraceStageTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafExtendWorkMilliseconds := extendMilliseconds
            stageTimings := stageTimings })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestStageLeafExtendWorkMilliseconds := extendMilliseconds
            stageTimings := stageTimings }
      publicInput
      proof
      observed

theorem guest_pc_trace_leaf_extend_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (extendMilliseconds : Nat)
    (stageTimings : List GuestPcTraceStageTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafExtendWorkMilliseconds := extendMilliseconds
            stageTimings := stageTimings })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_leaf_extend_timing_acceptance_verifier_core_contract
      assumptions
      summary
      extendMilliseconds
      stageTimings
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_leaf_extend_timing_acceptance_sound
      assumptions
      summary
      extendMilliseconds
      stageTimings
      publicInput
      proof
      observed
  exact And.intro core sound

theorem guest_pc_trace_leaf_setup_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (setupMilliseconds prepareMilliseconds outputAllocMilliseconds
      workspaceAllocMilliseconds outputAllocByteCount workspaceAllocByteCount
      outputAllocCount workspaceAllocCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafSetupWorkMilliseconds := setupMilliseconds
            guestStageLeafSetupPrepareMilliseconds := prepareMilliseconds
            guestStageLeafSetupOutputAllocMilliseconds := outputAllocMilliseconds
            guestStageLeafSetupWorkspaceAllocMilliseconds := workspaceAllocMilliseconds
            guestStageLeafSetupOutputAllocByteCount := outputAllocByteCount
            guestStageLeafSetupWorkspaceAllocByteCount := workspaceAllocByteCount
            guestStageLeafSetupOutputAllocCount := outputAllocCount
            guestStageLeafSetupWorkspaceAllocCount := workspaceAllocCount })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestStageLeafSetupWorkMilliseconds := setupMilliseconds
          guestStageLeafSetupPrepareMilliseconds := prepareMilliseconds
          guestStageLeafSetupOutputAllocMilliseconds := outputAllocMilliseconds
          guestStageLeafSetupWorkspaceAllocMilliseconds := workspaceAllocMilliseconds
          guestStageLeafSetupOutputAllocByteCount := outputAllocByteCount
          guestStageLeafSetupWorkspaceAllocByteCount := workspaceAllocByteCount
          guestStageLeafSetupOutputAllocCount := outputAllocCount
          guestStageLeafSetupWorkspaceAllocCount := workspaceAllocCount })
      publicInput
      proof
      observed

theorem guest_pc_trace_leaf_setup_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (setupMilliseconds prepareMilliseconds outputAllocMilliseconds
      workspaceAllocMilliseconds outputAllocByteCount workspaceAllocByteCount
      outputAllocCount workspaceAllocCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafSetupWorkMilliseconds := setupMilliseconds
            guestStageLeafSetupPrepareMilliseconds := prepareMilliseconds
            guestStageLeafSetupOutputAllocMilliseconds := outputAllocMilliseconds
            guestStageLeafSetupWorkspaceAllocMilliseconds := workspaceAllocMilliseconds
            guestStageLeafSetupOutputAllocByteCount := outputAllocByteCount
            guestStageLeafSetupWorkspaceAllocByteCount := workspaceAllocByteCount
            guestStageLeafSetupOutputAllocCount := outputAllocCount
            guestStageLeafSetupWorkspaceAllocCount := workspaceAllocCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestStageLeafSetupWorkMilliseconds := setupMilliseconds
            guestStageLeafSetupPrepareMilliseconds := prepareMilliseconds
            guestStageLeafSetupOutputAllocMilliseconds := outputAllocMilliseconds
            guestStageLeafSetupWorkspaceAllocMilliseconds := workspaceAllocMilliseconds
            guestStageLeafSetupOutputAllocByteCount := outputAllocByteCount
            guestStageLeafSetupWorkspaceAllocByteCount := workspaceAllocByteCount
            guestStageLeafSetupOutputAllocCount := outputAllocCount
            guestStageLeafSetupWorkspaceAllocCount := workspaceAllocCount }
      publicInput
      proof
      observed

theorem guest_pc_trace_leaf_setup_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (setupMilliseconds prepareMilliseconds outputAllocMilliseconds
      workspaceAllocMilliseconds outputAllocByteCount workspaceAllocByteCount
      outputAllocCount workspaceAllocCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafSetupWorkMilliseconds := setupMilliseconds
            guestStageLeafSetupPrepareMilliseconds := prepareMilliseconds
            guestStageLeafSetupOutputAllocMilliseconds := outputAllocMilliseconds
            guestStageLeafSetupWorkspaceAllocMilliseconds := workspaceAllocMilliseconds
            guestStageLeafSetupOutputAllocByteCount := outputAllocByteCount
            guestStageLeafSetupWorkspaceAllocByteCount := workspaceAllocByteCount
            guestStageLeafSetupOutputAllocCount := outputAllocCount
            guestStageLeafSetupWorkspaceAllocCount := workspaceAllocCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_leaf_setup_timing_acceptance_verifier_core_contract
      assumptions
      summary
      setupMilliseconds
      prepareMilliseconds
      outputAllocMilliseconds
      workspaceAllocMilliseconds
      outputAllocByteCount
      workspaceAllocByteCount
      outputAllocCount
      workspaceAllocCount
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_leaf_setup_timing_acceptance_sound
      assumptions
      summary
      setupMilliseconds
      prepareMilliseconds
      outputAllocMilliseconds
      workspaceAllocMilliseconds
      outputAllocByteCount
      workspaceAllocByteCount
      outputAllocCount
      workspaceAllocCount
      publicInput
      proof
      observed
  exact And.intro core sound

theorem guest_pc_trace_leaf_work_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (uploadMilliseconds kernelMilliseconds downloadMilliseconds
      validateMilliseconds hashMilliseconds hashRows hashBytes hashArity2Rows
      hashArity2Bytes hashArity4Rows hashArity4Bytes : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafUploadWorkMilliseconds := uploadMilliseconds
            guestStageLeafKernelWorkMilliseconds := kernelMilliseconds
            guestStageLeafDownloadWorkMilliseconds := downloadMilliseconds
            guestStageLeafValidateWorkMilliseconds := validateMilliseconds
            guestStageLeafHashWorkMilliseconds := hashMilliseconds
            guestStageLeafHashRowCount := hashRows
            guestStageLeafHashByteCount := hashBytes
            guestStageLeafHashArity2RowCount := hashArity2Rows
            guestStageLeafHashArity2ByteCount := hashArity2Bytes
            guestStageLeafHashArity4RowCount := hashArity4Rows
            guestStageLeafHashArity4ByteCount := hashArity4Bytes })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestStageLeafUploadWorkMilliseconds := uploadMilliseconds
          guestStageLeafKernelWorkMilliseconds := kernelMilliseconds
          guestStageLeafDownloadWorkMilliseconds := downloadMilliseconds
          guestStageLeafValidateWorkMilliseconds := validateMilliseconds
          guestStageLeafHashWorkMilliseconds := hashMilliseconds
          guestStageLeafHashRowCount := hashRows
          guestStageLeafHashByteCount := hashBytes
          guestStageLeafHashArity2RowCount := hashArity2Rows
          guestStageLeafHashArity2ByteCount := hashArity2Bytes
          guestStageLeafHashArity4RowCount := hashArity4Rows
          guestStageLeafHashArity4ByteCount := hashArity4Bytes })
      publicInput
      proof
      observed

theorem guest_pc_trace_leaf_work_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (uploadMilliseconds kernelMilliseconds downloadMilliseconds
      validateMilliseconds hashMilliseconds hashRows hashBytes hashArity2Rows
      hashArity2Bytes hashArity4Rows hashArity4Bytes : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafUploadWorkMilliseconds := uploadMilliseconds
            guestStageLeafKernelWorkMilliseconds := kernelMilliseconds
            guestStageLeafDownloadWorkMilliseconds := downloadMilliseconds
            guestStageLeafValidateWorkMilliseconds := validateMilliseconds
            guestStageLeafHashWorkMilliseconds := hashMilliseconds
            guestStageLeafHashRowCount := hashRows
            guestStageLeafHashByteCount := hashBytes
            guestStageLeafHashArity2RowCount := hashArity2Rows
            guestStageLeafHashArity2ByteCount := hashArity2Bytes
            guestStageLeafHashArity4RowCount := hashArity4Rows
            guestStageLeafHashArity4ByteCount := hashArity4Bytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestStageLeafUploadWorkMilliseconds := uploadMilliseconds
            guestStageLeafKernelWorkMilliseconds := kernelMilliseconds
            guestStageLeafDownloadWorkMilliseconds := downloadMilliseconds
            guestStageLeafValidateWorkMilliseconds := validateMilliseconds
            guestStageLeafHashWorkMilliseconds := hashMilliseconds
            guestStageLeafHashRowCount := hashRows
            guestStageLeafHashByteCount := hashBytes
            guestStageLeafHashArity2RowCount := hashArity2Rows
            guestStageLeafHashArity2ByteCount := hashArity2Bytes
            guestStageLeafHashArity4RowCount := hashArity4Rows
            guestStageLeafHashArity4ByteCount := hashArity4Bytes }
      publicInput
      proof
      observed

theorem guest_pc_trace_leaf_work_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (uploadMilliseconds kernelMilliseconds downloadMilliseconds
      validateMilliseconds hashMilliseconds hashRows hashBytes hashArity2Rows
      hashArity2Bytes hashArity4Rows hashArity4Bytes : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafUploadWorkMilliseconds := uploadMilliseconds
            guestStageLeafKernelWorkMilliseconds := kernelMilliseconds
            guestStageLeafDownloadWorkMilliseconds := downloadMilliseconds
            guestStageLeafValidateWorkMilliseconds := validateMilliseconds
            guestStageLeafHashWorkMilliseconds := hashMilliseconds
            guestStageLeafHashRowCount := hashRows
            guestStageLeafHashByteCount := hashBytes
            guestStageLeafHashArity2RowCount := hashArity2Rows
            guestStageLeafHashArity2ByteCount := hashArity2Bytes
            guestStageLeafHashArity4RowCount := hashArity4Rows
            guestStageLeafHashArity4ByteCount := hashArity4Bytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_leaf_work_timing_acceptance_verifier_core_contract
      assumptions
      summary
      uploadMilliseconds
      kernelMilliseconds
      downloadMilliseconds
      validateMilliseconds
      hashMilliseconds
      hashRows
      hashBytes
      hashArity2Rows
      hashArity2Bytes
      hashArity4Rows
      hashArity4Bytes
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_leaf_work_timing_acceptance_sound
      assumptions
      summary
      uploadMilliseconds
      kernelMilliseconds
      downloadMilliseconds
      validateMilliseconds
      hashMilliseconds
      hashRows
      hashBytes
      hashArity2Rows
      hashArity2Bytes
      hashArity4Rows
      hashArity4Bytes
      publicInput
      proof
      observed
  exact And.intro core sound

theorem guest_pc_trace_leaf_coset_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (callCount outputByteCount columnCount maxColumnCount nttLaunchCount
      bitReverseLaunchCount nttStageLaunchCount nttBlockTwiddleLaunchCount
      normalizeLaunchCount packLaunchCount unpackLaunchCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafCosetExtendCallCount := callCount
            guestStageLeafCosetExtendOutputByteCount := outputByteCount
            guestStageLeafCosetExtendColumnCount := columnCount
            guestStageLeafCosetExtendMaxColumnCount := maxColumnCount
            guestStageLeafCosetExtendNttLaunchCount := nttLaunchCount
            guestStageLeafCosetExtendBitReverseLaunchCount := bitReverseLaunchCount
            guestStageLeafCosetExtendNttStageLaunchCount := nttStageLaunchCount
            guestStageLeafCosetExtendNttBlockTwiddleLaunchCount := nttBlockTwiddleLaunchCount
            guestStageLeafCosetExtendNormalizeLaunchCount := normalizeLaunchCount
            guestStageLeafCosetExtendPackLaunchCount := packLaunchCount
            guestStageLeafCosetExtendUnpackLaunchCount := unpackLaunchCount })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestStageLeafCosetExtendCallCount := callCount
          guestStageLeafCosetExtendOutputByteCount := outputByteCount
          guestStageLeafCosetExtendColumnCount := columnCount
          guestStageLeafCosetExtendMaxColumnCount := maxColumnCount
          guestStageLeafCosetExtendNttLaunchCount := nttLaunchCount
          guestStageLeafCosetExtendBitReverseLaunchCount := bitReverseLaunchCount
          guestStageLeafCosetExtendNttStageLaunchCount := nttStageLaunchCount
          guestStageLeafCosetExtendNttBlockTwiddleLaunchCount := nttBlockTwiddleLaunchCount
          guestStageLeafCosetExtendNormalizeLaunchCount := normalizeLaunchCount
          guestStageLeafCosetExtendPackLaunchCount := packLaunchCount
          guestStageLeafCosetExtendUnpackLaunchCount := unpackLaunchCount })
      publicInput
      proof
      observed

theorem guest_pc_trace_leaf_coset_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (callCount outputByteCount columnCount maxColumnCount nttLaunchCount
      bitReverseLaunchCount nttStageLaunchCount nttBlockTwiddleLaunchCount
      normalizeLaunchCount packLaunchCount unpackLaunchCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafCosetExtendCallCount := callCount
            guestStageLeafCosetExtendOutputByteCount := outputByteCount
            guestStageLeafCosetExtendColumnCount := columnCount
            guestStageLeafCosetExtendMaxColumnCount := maxColumnCount
            guestStageLeafCosetExtendNttLaunchCount := nttLaunchCount
            guestStageLeafCosetExtendBitReverseLaunchCount := bitReverseLaunchCount
            guestStageLeafCosetExtendNttStageLaunchCount := nttStageLaunchCount
            guestStageLeafCosetExtendNttBlockTwiddleLaunchCount := nttBlockTwiddleLaunchCount
            guestStageLeafCosetExtendNormalizeLaunchCount := normalizeLaunchCount
            guestStageLeafCosetExtendPackLaunchCount := packLaunchCount
            guestStageLeafCosetExtendUnpackLaunchCount := unpackLaunchCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestStageLeafCosetExtendCallCount := callCount
            guestStageLeafCosetExtendOutputByteCount := outputByteCount
            guestStageLeafCosetExtendColumnCount := columnCount
            guestStageLeafCosetExtendMaxColumnCount := maxColumnCount
            guestStageLeafCosetExtendNttLaunchCount := nttLaunchCount
            guestStageLeafCosetExtendBitReverseLaunchCount := bitReverseLaunchCount
            guestStageLeafCosetExtendNttStageLaunchCount := nttStageLaunchCount
            guestStageLeafCosetExtendNttBlockTwiddleLaunchCount := nttBlockTwiddleLaunchCount
            guestStageLeafCosetExtendNormalizeLaunchCount := normalizeLaunchCount
            guestStageLeafCosetExtendPackLaunchCount := packLaunchCount
            guestStageLeafCosetExtendUnpackLaunchCount := unpackLaunchCount }
      publicInput
      proof
      observed

theorem guest_pc_trace_leaf_coset_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (callCount outputByteCount columnCount maxColumnCount nttLaunchCount
      bitReverseLaunchCount nttStageLaunchCount nttBlockTwiddleLaunchCount
      normalizeLaunchCount packLaunchCount unpackLaunchCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageLeafCosetExtendCallCount := callCount
            guestStageLeafCosetExtendOutputByteCount := outputByteCount
            guestStageLeafCosetExtendColumnCount := columnCount
            guestStageLeafCosetExtendMaxColumnCount := maxColumnCount
            guestStageLeafCosetExtendNttLaunchCount := nttLaunchCount
            guestStageLeafCosetExtendBitReverseLaunchCount := bitReverseLaunchCount
            guestStageLeafCosetExtendNttStageLaunchCount := nttStageLaunchCount
            guestStageLeafCosetExtendNttBlockTwiddleLaunchCount := nttBlockTwiddleLaunchCount
            guestStageLeafCosetExtendNormalizeLaunchCount := normalizeLaunchCount
            guestStageLeafCosetExtendPackLaunchCount := packLaunchCount
            guestStageLeafCosetExtendUnpackLaunchCount := unpackLaunchCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_leaf_coset_timing_acceptance_verifier_core_contract
      assumptions
      summary
      callCount
      outputByteCount
      columnCount
      maxColumnCount
      nttLaunchCount
      bitReverseLaunchCount
      nttStageLaunchCount
      nttBlockTwiddleLaunchCount
      normalizeLaunchCount
      packLaunchCount
      unpackLaunchCount
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_leaf_coset_timing_acceptance_sound
      assumptions
      summary
      callCount
      outputByteCount
      columnCount
      maxColumnCount
      nttLaunchCount
      bitReverseLaunchCount
      nttStageLaunchCount
      nttBlockTwiddleLaunchCount
      normalizeLaunchCount
      packLaunchCount
      unpackLaunchCount
      publicInput
      proof
      observed
  exact And.intro core sound

theorem guest_pc_trace_tree_commit_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (workMilliseconds checkpointMilliseconds rootMilliseconds rootCount
      rootByteCount rootMaterializationGroupCount rootMaterializationMaxGroupSize
      retainMilliseconds : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageTreeCommitWorkMilliseconds := workMilliseconds
            guestStageTreeCommitCheckpointWorkMilliseconds := checkpointMilliseconds
            guestStageTreeCommitRootWorkMilliseconds := rootMilliseconds
            guestStageTreeCommitRootCount := rootCount
            guestStageTreeCommitRootByteCount := rootByteCount
            guestStageTreeCommitRootMaterializationGroupCount :=
              rootMaterializationGroupCount
            guestStageTreeCommitRootMaterializationMaxGroupSize :=
              rootMaterializationMaxGroupSize
            guestStageTreeCommitRetainWorkMilliseconds := retainMilliseconds })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestStageTreeCommitWorkMilliseconds := workMilliseconds
          guestStageTreeCommitCheckpointWorkMilliseconds := checkpointMilliseconds
          guestStageTreeCommitRootWorkMilliseconds := rootMilliseconds
          guestStageTreeCommitRootCount := rootCount
          guestStageTreeCommitRootByteCount := rootByteCount
          guestStageTreeCommitRootMaterializationGroupCount :=
            rootMaterializationGroupCount
          guestStageTreeCommitRootMaterializationMaxGroupSize :=
            rootMaterializationMaxGroupSize
          guestStageTreeCommitRetainWorkMilliseconds := retainMilliseconds })
      publicInput
      proof
      observed

theorem guest_pc_trace_tree_commit_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (workMilliseconds checkpointMilliseconds rootMilliseconds rootCount
      rootByteCount rootMaterializationGroupCount rootMaterializationMaxGroupSize
      retainMilliseconds : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestStageTreeCommitWorkMilliseconds := workMilliseconds
            guestStageTreeCommitCheckpointWorkMilliseconds := checkpointMilliseconds
            guestStageTreeCommitRootWorkMilliseconds := rootMilliseconds
            guestStageTreeCommitRootCount := rootCount
            guestStageTreeCommitRootByteCount := rootByteCount
            guestStageTreeCommitRootMaterializationGroupCount :=
              rootMaterializationGroupCount
            guestStageTreeCommitRootMaterializationMaxGroupSize :=
              rootMaterializationMaxGroupSize
            guestStageTreeCommitRetainWorkMilliseconds := retainMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestStageTreeCommitWorkMilliseconds := workMilliseconds
            guestStageTreeCommitCheckpointWorkMilliseconds := checkpointMilliseconds
            guestStageTreeCommitRootWorkMilliseconds := rootMilliseconds
            guestStageTreeCommitRootCount := rootCount
            guestStageTreeCommitRootByteCount := rootByteCount
            guestStageTreeCommitRootMaterializationGroupCount :=
              rootMaterializationGroupCount
            guestStageTreeCommitRootMaterializationMaxGroupSize :=
              rootMaterializationMaxGroupSize
            guestStageTreeCommitRetainWorkMilliseconds := retainMilliseconds }
      publicInput
      proof
      observed

theorem guest_pc_trace_segment_commit_worker_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (initialWorkerCount effectiveWorkerCount oomRetryCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestSegmentCommitInitialWorkerCount := initialWorkerCount
            guestSegmentCommitEffectiveWorkerCount := effectiveWorkerCount
            guestSegmentCommitOomRetryCount := oomRetryCount })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestSegmentCommitInitialWorkerCount := initialWorkerCount
          guestSegmentCommitEffectiveWorkerCount := effectiveWorkerCount
          guestSegmentCommitOomRetryCount := oomRetryCount })
      publicInput
      proof
      observed

theorem guest_pc_trace_segment_commit_worker_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (initialWorkerCount effectiveWorkerCount oomRetryCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestSegmentCommitInitialWorkerCount := initialWorkerCount
            guestSegmentCommitEffectiveWorkerCount := effectiveWorkerCount
            guestSegmentCommitOomRetryCount := oomRetryCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestSegmentCommitInitialWorkerCount := initialWorkerCount
            guestSegmentCommitEffectiveWorkerCount := effectiveWorkerCount
            guestSegmentCommitOomRetryCount := oomRetryCount }
      publicInput
      proof
      observed

theorem guest_pc_trace_stage_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (stageTimings : List GuestPcTraceStageTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some { summary with stageTimings := stageTimings })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some { summary with stageTimings := stageTimings })
      publicInput
      proof
      observed

theorem guest_pc_trace_stage_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (stageTimings : List GuestPcTraceStageTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some { summary with stageTimings := stageTimings })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with stageTimings := stageTimings }
      publicInput
      proof
      observed

end Lzvm
