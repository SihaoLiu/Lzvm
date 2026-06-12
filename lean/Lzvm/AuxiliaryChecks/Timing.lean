/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks

/-!
Runtime timing acceptance projections for auxiliary verifier metadata.
-/

namespace Lzvm

def TimingObservedAcceptance
    (system : VerifierModel)
    (_observations : List TimingObservation)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem timing_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (observations : List TimingObservation) :
    forall publicInput proof,
      TimingObservedAcceptance system observations publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem timing_observation_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (observations : List TimingObservation) :
    forall publicInput proof,
      TimingObservedAcceptance system observations publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithTimings
  exact
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      (timing_observed_acceptance_projects_verifier_acceptance
        observations
        publicInput
        proof
        acceptedWithTimings)

theorem timing_observation_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (observations : List TimingObservation) :
    forall publicInput proof,
      TimingObservedAcceptance system observations publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (timing_observation_acceptance_sound
        assumptions
        observations
        publicInput
        proof
        observed)

def GuestPcTraceTimingObservedAcceptance
    (system : VerifierModel)
    (_summary : Option GuestPcTraceTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem guest_pc_trace_timing_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem guest_pc_trace_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithGuestPcTraceTimings
  exact
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      (guest_pc_trace_timing_observed_acceptance_projects_verifier_acceptance
        summary
        publicInput
        proof
        acceptedWithGuestPcTraceTimings)

theorem guest_pc_trace_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_timing_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

theorem guest_pc_trace_descriptor_width_counts_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (compactRows wideRows : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceDescriptorCompactRowCount := compactRows
            guestTraceDescriptorWideRowCount := wideRows })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestTraceDescriptorCompactRowCount := compactRows
          guestTraceDescriptorWideRowCount := wideRows })
      publicInput
      proof
      observed

theorem guest_pc_trace_descriptor_width_counts_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (compactRows wideRows : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceDescriptorCompactRowCount := compactRows
            guestTraceDescriptorWideRowCount := wideRows })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_descriptor_width_counts_acceptance_sound
        assumptions
        summary
        compactRows
        wideRows
        publicInput
        proof
        observed)

theorem guest_pc_trace_report_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (reportMilliseconds validationMilliseconds reportCount reportRows : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceReportMilliseconds := reportMilliseconds
            guestTraceReportValidationMilliseconds := validationMilliseconds
            guestTraceReportCount := reportCount
            guestTraceReportRowCount := reportRows })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestTraceReportMilliseconds := reportMilliseconds
          guestTraceReportValidationMilliseconds := validationMilliseconds
          guestTraceReportCount := reportCount
          guestTraceReportRowCount := reportRows })
      publicInput
      proof
      observed

theorem guest_pc_trace_report_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (reportMilliseconds validationMilliseconds reportCount reportRows : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceReportMilliseconds := reportMilliseconds
            guestTraceReportValidationMilliseconds := validationMilliseconds
            guestTraceReportCount := reportCount
            guestTraceReportRowCount := reportRows })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_report_timing_acceptance_sound
        assumptions
        summary
        reportMilliseconds
        validationMilliseconds
        reportCount
        reportRows
        publicInput
        proof
        observed)

theorem guest_pc_trace_shape_counts_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (singleRowReports multiRowReports pendingDmaReports amoReports
      storeConditionalReports externalOpRows copyRows flagRows precompileRows
      indirectMemoryRows registerSourceReads memorySourceReads registerStoreRows
      memoryStoreRows noStoreRows : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceSingleRowReportCount := singleRowReports
            guestTraceMultiRowReportCount := multiRowReports
            guestTracePendingDmaReportCount := pendingDmaReports
            guestTraceAmoReportCount := amoReports
            guestTraceStoreConditionalReportCount := storeConditionalReports
            guestTraceExternalOpRowCount := externalOpRows
            guestTraceCopyRowCount := copyRows
            guestTraceFlagRowCount := flagRows
            guestTracePrecompileRowCount := precompileRows
            guestTraceIndirectMemoryRowCount := indirectMemoryRows
            guestTraceRegisterSourceReadCount := registerSourceReads
            guestTraceMemorySourceReadCount := memorySourceReads
            guestTraceRegisterStoreRowCount := registerStoreRows
            guestTraceMemoryStoreRowCount := memoryStoreRows
            guestTraceNoStoreRowCount := noStoreRows })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestTraceSingleRowReportCount := singleRowReports
          guestTraceMultiRowReportCount := multiRowReports
          guestTracePendingDmaReportCount := pendingDmaReports
          guestTraceAmoReportCount := amoReports
          guestTraceStoreConditionalReportCount := storeConditionalReports
          guestTraceExternalOpRowCount := externalOpRows
          guestTraceCopyRowCount := copyRows
          guestTraceFlagRowCount := flagRows
          guestTracePrecompileRowCount := precompileRows
          guestTraceIndirectMemoryRowCount := indirectMemoryRows
          guestTraceRegisterSourceReadCount := registerSourceReads
          guestTraceMemorySourceReadCount := memorySourceReads
          guestTraceRegisterStoreRowCount := registerStoreRows
          guestTraceMemoryStoreRowCount := memoryStoreRows
          guestTraceNoStoreRowCount := noStoreRows })
      publicInput
      proof
      observed

theorem guest_pc_trace_shape_counts_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (singleRowReports multiRowReports pendingDmaReports amoReports
      storeConditionalReports externalOpRows copyRows flagRows precompileRows
      indirectMemoryRows registerSourceReads memorySourceReads registerStoreRows
      memoryStoreRows noStoreRows : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceSingleRowReportCount := singleRowReports
            guestTraceMultiRowReportCount := multiRowReports
            guestTracePendingDmaReportCount := pendingDmaReports
            guestTraceAmoReportCount := amoReports
            guestTraceStoreConditionalReportCount := storeConditionalReports
            guestTraceExternalOpRowCount := externalOpRows
            guestTraceCopyRowCount := copyRows
            guestTraceFlagRowCount := flagRows
            guestTracePrecompileRowCount := precompileRows
            guestTraceIndirectMemoryRowCount := indirectMemoryRows
            guestTraceRegisterSourceReadCount := registerSourceReads
            guestTraceMemorySourceReadCount := memorySourceReads
            guestTraceRegisterStoreRowCount := registerStoreRows
            guestTraceMemoryStoreRowCount := memoryStoreRows
            guestTraceNoStoreRowCount := noStoreRows })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_shape_counts_acceptance_sound
        assumptions
        summary
        singleRowReports
        multiRowReports
        pendingDmaReports
        amoReports
        storeConditionalReports
        externalOpRows
        copyRows
        flagRows
        precompileRows
        indirectMemoryRows
        registerSourceReads
        memorySourceReads
        registerStoreRows
        memoryStoreRows
        noStoreRows
        publicInput
        proof
        observed)

theorem guest_pc_trace_memory_access_shape_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (indirectMemoryRows memorySourceReads memoryStoreRows : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceIndirectMemoryRowCount := indirectMemoryRows
            guestTraceMemorySourceReadCount := memorySourceReads
            guestTraceMemoryStoreRowCount := memoryStoreRows })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestTraceIndirectMemoryRowCount := indirectMemoryRows
          guestTraceMemorySourceReadCount := memorySourceReads
          guestTraceMemoryStoreRowCount := memoryStoreRows })
      publicInput
      proof
      observed

theorem guest_pc_trace_memory_access_shape_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (indirectMemoryRows memorySourceReads memoryStoreRows : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceIndirectMemoryRowCount := indirectMemoryRows
            guestTraceMemorySourceReadCount := memorySourceReads
            guestTraceMemoryStoreRowCount := memoryStoreRows })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_memory_access_shape_acceptance_sound
        assumptions
        summary
        indirectMemoryRows
        memorySourceReads
        memoryStoreRows
        publicInput
        proof
        observed)

theorem guest_pc_trace_report_buffer_capacity_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (capacity maxCapacity excessCapacity : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceReportBufferCapacity := capacity
            guestTraceReportBufferMaxCapacity := maxCapacity
            guestTraceReportBufferExcessCapacity := excessCapacity })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestTraceReportBufferCapacity := capacity
          guestTraceReportBufferMaxCapacity := maxCapacity
          guestTraceReportBufferExcessCapacity := excessCapacity })
      publicInput
      proof
      observed

theorem guest_pc_trace_report_buffer_capacity_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (capacity maxCapacity excessCapacity : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceReportBufferCapacity := capacity
            guestTraceReportBufferMaxCapacity := maxCapacity
            guestTraceReportBufferExcessCapacity := excessCapacity })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_report_buffer_capacity_acceptance_sound
        assumptions
        summary
        capacity
        maxCapacity
        excessCapacity
        publicInput
        proof
        observed)

theorem guest_pc_trace_descriptor_upload_word_count_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (wordCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDeviceSourceDescriptorUploadWordCount := wordCount })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestDeviceSourceDescriptorUploadWordCount := wordCount })
      publicInput
      proof
      observed

theorem guest_pc_trace_descriptor_upload_word_count_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (wordCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDeviceSourceDescriptorUploadWordCount := wordCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_descriptor_upload_word_count_acceptance_sound
        assumptions
        summary
        wordCount
        publicInput
        proof
        observed)

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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_leaf_output_cache_counts_acceptance_sound
        assumptions
        summary
        hitCount
        missCount
        stageTimings
        publicInput
        proof
        observed)

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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_leaf_setup_timing_acceptance_sound
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
        observed)

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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_leaf_work_timing_acceptance_sound
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
        observed)

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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_leaf_coset_timing_acceptance_sound
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
      observed)

theorem guest_pc_trace_tree_commit_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (workMilliseconds checkpointMilliseconds rootMilliseconds rootCount
      rootByteCount retainMilliseconds : Nat) :
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
          guestStageTreeCommitRetainWorkMilliseconds := retainMilliseconds })
      publicInput
      proof
      observed

theorem guest_pc_trace_tree_commit_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (workMilliseconds checkpointMilliseconds rootMilliseconds rootCount
      rootByteCount retainMilliseconds : Nat) :
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
            guestStageTreeCommitRetainWorkMilliseconds := retainMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_tree_commit_timing_acceptance_sound
        assumptions
        summary
        workMilliseconds
        checkpointMilliseconds
        rootMilliseconds
        rootCount
        rootByteCount
        retainMilliseconds
        publicInput
        proof
        observed)

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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_stage_timing_acceptance_sound
        assumptions
        summary
        stageTimings
        publicInput
        proof
        observed)

def WitnessOpeningRowValueTimingObservedAcceptance
    (system : VerifierModel)
    (_summary : Option WitnessOpeningRowValueTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem witness_opening_row_value_timing_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option WitnessOpeningRowValueTimingSummary) :
    forall publicInput proof,
      WitnessOpeningRowValueTimingObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem witness_opening_row_value_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option WitnessOpeningRowValueTimingSummary) :
    forall publicInput proof,
      WitnessOpeningRowValueTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithRowValueTimings
  exact
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      (witness_opening_row_value_timing_observed_acceptance_projects_verifier_acceptance
        summary
        publicInput
        proof
        acceptedWithRowValueTimings)

theorem witness_opening_row_value_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option WitnessOpeningRowValueTimingSummary) :
    forall publicInput proof,
      WitnessOpeningRowValueTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (witness_opening_row_value_timing_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

theorem witness_opening_row_value_aggregate_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : WitnessOpeningRowValueTimingSummary)
    (sourceExtendMilliseconds sourceDownloadMilliseconds deviceDownloadMilliseconds
      deviceRows sourceRows words bytes : Nat) :
    forall publicInput proof,
      WitnessOpeningRowValueTimingObservedAcceptance
        system
        (some
          { summary with
            rowValueSourceExtendMilliseconds := sourceExtendMilliseconds
            rowValueSourceDownloadMilliseconds := sourceDownloadMilliseconds
            rowValueDeviceDownloadMilliseconds := deviceDownloadMilliseconds
            deviceRowCount := deviceRows
            sourceRowCount := sourceRows
            wordCount := words
            byteCount := bytes })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    witness_opening_row_value_timing_acceptance_sound
      assumptions
      (some
        { summary with
          rowValueSourceExtendMilliseconds := sourceExtendMilliseconds
          rowValueSourceDownloadMilliseconds := sourceDownloadMilliseconds
          rowValueDeviceDownloadMilliseconds := deviceDownloadMilliseconds
          deviceRowCount := deviceRows
          sourceRowCount := sourceRows
          wordCount := words
          byteCount := bytes })
      publicInput
      proof
      observed

theorem witness_opening_row_value_aggregate_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : WitnessOpeningRowValueTimingSummary)
    (sourceExtendMilliseconds sourceDownloadMilliseconds deviceDownloadMilliseconds
      deviceRows sourceRows words bytes : Nat) :
    forall publicInput proof,
      WitnessOpeningRowValueTimingObservedAcceptance
        system
        (some
          { summary with
            rowValueSourceExtendMilliseconds := sourceExtendMilliseconds
            rowValueSourceDownloadMilliseconds := sourceDownloadMilliseconds
            rowValueDeviceDownloadMilliseconds := deviceDownloadMilliseconds
            deviceRowCount := deviceRows
            sourceRowCount := sourceRows
            wordCount := words
            byteCount := bytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (witness_opening_row_value_aggregate_timing_acceptance_sound
        assumptions
        summary
        sourceExtendMilliseconds
        sourceDownloadMilliseconds
        deviceDownloadMilliseconds
        deviceRows
        sourceRows
        words
        bytes
        publicInput
        proof
        observed)

def ConstantMaterialValidationTimingObservedAcceptance
    (system : VerifierModel)
    (_summary : Option ConstantMaterialValidationTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem constant_material_validation_timing_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option ConstantMaterialValidationTimingSummary) :
    forall publicInput proof,
      ConstantMaterialValidationTimingObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem constant_material_validation_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ConstantMaterialValidationTimingSummary) :
    forall publicInput proof,
      ConstantMaterialValidationTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithConstantMaterialTimings
  exact
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      (constant_material_validation_timing_observed_acceptance_projects_verifier_acceptance
        summary
        publicInput
        proof
        acceptedWithConstantMaterialTimings)

theorem constant_material_validation_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ConstantMaterialValidationTimingSummary) :
    forall publicInput proof,
      ConstantMaterialValidationTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (constant_material_validation_timing_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

theorem constant_material_validation_aggregate_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ConstantMaterialValidationTimingSummary)
    (elapsedMilliseconds joinWaitMilliseconds unitCount byteCount : Nat) :
    forall publicInput proof,
      ConstantMaterialValidationTimingObservedAcceptance
        system
        (some
          { summary with
            constantMaterialValidationElapsedMilliseconds := elapsedMilliseconds
            constantMaterialValidationJoinWaitMilliseconds := joinWaitMilliseconds
            constantMaterialValidationUnitCount := unitCount
            constantMaterialValidationByteCount := byteCount })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    constant_material_validation_timing_acceptance_sound
      assumptions
      (some
        { summary with
          constantMaterialValidationElapsedMilliseconds := elapsedMilliseconds
          constantMaterialValidationJoinWaitMilliseconds := joinWaitMilliseconds
          constantMaterialValidationUnitCount := unitCount
          constantMaterialValidationByteCount := byteCount })
      publicInput
      proof
      observed

theorem constant_material_validation_aggregate_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ConstantMaterialValidationTimingSummary)
    (elapsedMilliseconds joinWaitMilliseconds unitCount byteCount : Nat) :
    forall publicInput proof,
      ConstantMaterialValidationTimingObservedAcceptance
        system
        (some
          { summary with
            constantMaterialValidationElapsedMilliseconds := elapsedMilliseconds
            constantMaterialValidationJoinWaitMilliseconds := joinWaitMilliseconds
            constantMaterialValidationUnitCount := unitCount
            constantMaterialValidationByteCount := byteCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (constant_material_validation_aggregate_timing_acceptance_sound
        assumptions
        summary
        elapsedMilliseconds
        joinWaitMilliseconds
        unitCount
        byteCount
        publicInput
        proof
        observed)

def ProverGpuModeObservedAcceptance
    (system : VerifierModel)
    (_summary : Option ProverGpuModeSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem prover_gpu_mode_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option ProverGpuModeSummary) :
    forall publicInput proof,
      ProverGpuModeObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem prover_gpu_mode_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProverGpuModeSummary) :
    forall publicInput proof,
      ProverGpuModeObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithGpuMode
  exact
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      (prover_gpu_mode_observed_acceptance_projects_verifier_acceptance
        summary
        publicInput
        proof
        acceptedWithGpuMode)

theorem prover_gpu_mode_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProverGpuModeSummary) :
    forall publicInput proof,
      ProverGpuModeObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (prover_gpu_mode_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

def GpuRunOptionsObservedAcceptance
    (system : VerifierModel)
    (_summary : Option GpuRunOptionsSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem gpu_run_options_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option GpuRunOptionsSummary) :
    forall publicInput proof,
      GpuRunOptionsObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem gpu_run_options_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GpuRunOptionsSummary) :
    forall publicInput proof,
      GpuRunOptionsObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithGpuRunOptions
  exact
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      (gpu_run_options_observed_acceptance_projects_verifier_acceptance
        summary
        publicInput
        proof
        acceptedWithGpuRunOptions)

theorem gpu_run_options_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GpuRunOptionsSummary) :
    forall publicInput proof,
      GpuRunOptionsObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (gpu_run_options_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

def CudaBackendObservedAcceptance
    (system : VerifierModel)
    (_summary : Option CudaBackendSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem cuda_backend_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option CudaBackendSummary) :
    forall publicInput proof,
      CudaBackendObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem cuda_backend_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option CudaBackendSummary) :
    forall publicInput proof,
      CudaBackendObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithCudaBackend
  exact
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      (cuda_backend_observed_acceptance_projects_verifier_acceptance
        summary
        publicInput
        proof
        acceptedWithCudaBackend)

theorem cuda_backend_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option CudaBackendSummary) :
    forall publicInput proof,
      CudaBackendObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (cuda_backend_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

def CudaAllocatorTimingObservedAcceptance
    (system : VerifierModel)
    (_summary : Option CudaAllocatorTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem cuda_allocator_timing_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option CudaAllocatorTimingSummary) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem cuda_allocator_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option CudaAllocatorTimingSummary) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithAllocatorTimings
  exact
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      (cuda_allocator_timing_observed_acceptance_projects_verifier_acceptance
        summary
        publicInput
        proof
        acceptedWithAllocatorTimings)

theorem cuda_allocator_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option CudaAllocatorTimingSummary) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (cuda_allocator_timing_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

theorem cuda_allocator_aggregate_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : CudaAllocatorTimingSummary)
    (mallocCalls mallocBytes cachedBlocks cachedBytes eventQueryCalls
      eventQueryReadyCount eventQueryNotReadyCount eventSynchronizeCalls
      eventSynchronizeBytes eventSynchronizeMaxBytes eventSynchronizeWaitNanoseconds
      eventSynchronizeMaxWaitNanoseconds eventSynchronizeHotBytes
      eventSynchronizeHotCount eventSynchronizeHotWaitNanoseconds cachedReuseCount
      pendingReuseCount noWaitBypassCount noWaitBypassBytes : Nat) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance
        system
        (some
          { summary with
            cudaAllocatorMallocCallCount := mallocCalls
            cudaAllocatorMallocByteCount := mallocBytes
            cudaAllocatorCachedBlockCount := cachedBlocks
            cudaAllocatorCachedByteCount := cachedBytes
            cudaAllocatorEventQueryCallCount := eventQueryCalls
            cudaAllocatorEventQueryReadyCount := eventQueryReadyCount
            cudaAllocatorEventQueryNotReadyCount := eventQueryNotReadyCount
            cudaAllocatorEventSynchronizeCallCount := eventSynchronizeCalls
            cudaAllocatorEventSynchronizeByteCount := eventSynchronizeBytes
            cudaAllocatorEventSynchronizeMaxByteCount := eventSynchronizeMaxBytes
            cudaAllocatorEventSynchronizeWaitNanoseconds := eventSynchronizeWaitNanoseconds
            cudaAllocatorEventSynchronizeMaxWaitNanoseconds :=
              eventSynchronizeMaxWaitNanoseconds
            cudaAllocatorEventSynchronizeHotByteCount := eventSynchronizeHotBytes
            cudaAllocatorEventSynchronizeHotCount := eventSynchronizeHotCount
            cudaAllocatorEventSynchronizeHotWaitNanoseconds :=
              eventSynchronizeHotWaitNanoseconds
            cudaAllocatorCachedReuseCount := cachedReuseCount
            cudaAllocatorPendingReuseCount := pendingReuseCount
            cudaAllocatorNoWaitBypassCount := noWaitBypassCount
            cudaAllocatorNoWaitBypassByteCount := noWaitBypassBytes })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    cuda_allocator_timing_acceptance_sound
      assumptions
      (some
        { summary with
          cudaAllocatorMallocCallCount := mallocCalls
          cudaAllocatorMallocByteCount := mallocBytes
          cudaAllocatorCachedBlockCount := cachedBlocks
          cudaAllocatorCachedByteCount := cachedBytes
          cudaAllocatorEventQueryCallCount := eventQueryCalls
          cudaAllocatorEventQueryReadyCount := eventQueryReadyCount
          cudaAllocatorEventQueryNotReadyCount := eventQueryNotReadyCount
          cudaAllocatorEventSynchronizeCallCount := eventSynchronizeCalls
          cudaAllocatorEventSynchronizeByteCount := eventSynchronizeBytes
          cudaAllocatorEventSynchronizeMaxByteCount := eventSynchronizeMaxBytes
          cudaAllocatorEventSynchronizeWaitNanoseconds := eventSynchronizeWaitNanoseconds
          cudaAllocatorEventSynchronizeMaxWaitNanoseconds :=
            eventSynchronizeMaxWaitNanoseconds
          cudaAllocatorEventSynchronizeHotByteCount := eventSynchronizeHotBytes
          cudaAllocatorEventSynchronizeHotCount := eventSynchronizeHotCount
          cudaAllocatorEventSynchronizeHotWaitNanoseconds :=
            eventSynchronizeHotWaitNanoseconds
          cudaAllocatorCachedReuseCount := cachedReuseCount
          cudaAllocatorPendingReuseCount := pendingReuseCount
          cudaAllocatorNoWaitBypassCount := noWaitBypassCount
          cudaAllocatorNoWaitBypassByteCount := noWaitBypassBytes })
      publicInput
      proof
      observed

theorem cuda_allocator_aggregate_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : CudaAllocatorTimingSummary)
    (mallocCalls mallocBytes cachedBlocks cachedBytes eventQueryCalls
      eventQueryReadyCount eventQueryNotReadyCount eventSynchronizeCalls
      eventSynchronizeBytes eventSynchronizeMaxBytes eventSynchronizeWaitNanoseconds
      eventSynchronizeMaxWaitNanoseconds eventSynchronizeHotBytes
      eventSynchronizeHotCount eventSynchronizeHotWaitNanoseconds cachedReuseCount
      pendingReuseCount noWaitBypassCount noWaitBypassBytes : Nat) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance
        system
        (some
          { summary with
            cudaAllocatorMallocCallCount := mallocCalls
            cudaAllocatorMallocByteCount := mallocBytes
            cudaAllocatorCachedBlockCount := cachedBlocks
            cudaAllocatorCachedByteCount := cachedBytes
            cudaAllocatorEventQueryCallCount := eventQueryCalls
            cudaAllocatorEventQueryReadyCount := eventQueryReadyCount
            cudaAllocatorEventQueryNotReadyCount := eventQueryNotReadyCount
            cudaAllocatorEventSynchronizeCallCount := eventSynchronizeCalls
            cudaAllocatorEventSynchronizeByteCount := eventSynchronizeBytes
            cudaAllocatorEventSynchronizeMaxByteCount := eventSynchronizeMaxBytes
            cudaAllocatorEventSynchronizeWaitNanoseconds := eventSynchronizeWaitNanoseconds
            cudaAllocatorEventSynchronizeMaxWaitNanoseconds :=
              eventSynchronizeMaxWaitNanoseconds
            cudaAllocatorEventSynchronizeHotByteCount := eventSynchronizeHotBytes
            cudaAllocatorEventSynchronizeHotCount := eventSynchronizeHotCount
            cudaAllocatorEventSynchronizeHotWaitNanoseconds :=
              eventSynchronizeHotWaitNanoseconds
            cudaAllocatorCachedReuseCount := cachedReuseCount
            cudaAllocatorPendingReuseCount := pendingReuseCount
            cudaAllocatorNoWaitBypassCount := noWaitBypassCount
            cudaAllocatorNoWaitBypassByteCount := noWaitBypassBytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (cuda_allocator_aggregate_timing_acceptance_sound
        assumptions
        summary
        mallocCalls
        mallocBytes
        cachedBlocks
        cachedBytes
        eventQueryCalls
        eventQueryReadyCount
        eventQueryNotReadyCount
        eventSynchronizeCalls
        eventSynchronizeBytes
        eventSynchronizeMaxBytes
        eventSynchronizeWaitNanoseconds
        eventSynchronizeMaxWaitNanoseconds
        eventSynchronizeHotBytes
        eventSynchronizeHotCount
        eventSynchronizeHotWaitNanoseconds
        cachedReuseCount
        pendingReuseCount
        noWaitBypassCount
        noWaitBypassBytes
        publicInput
        proof
        observed)

def ProofArtifactFinishTimingObservedAcceptance
    (system : VerifierModel)
    (_summary : Option ProofArtifactFinishTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem proof_artifact_finish_timing_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem proof_artifact_finish_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithProofFinishTimings
  exact
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      (proof_artifact_finish_timing_observed_acceptance_projects_verifier_acceptance
        summary
        publicInput
        proof
        acceptedWithProofFinishTimings)

theorem proof_artifact_finish_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (proof_artifact_finish_timing_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

theorem proof_artifact_finish_aggregate_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (queryPlanMilliseconds constantOpeningMilliseconds witnessOpeningMilliseconds
      friOpeningMilliseconds proofEncodeMilliseconds contributionSegmentMilliseconds
      contributionVerifyMilliseconds : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishQueryPlanMilliseconds := queryPlanMilliseconds
            finishConstantOpeningMilliseconds := constantOpeningMilliseconds
            finishWitnessOpeningMilliseconds := witnessOpeningMilliseconds
            finishFriOpeningMilliseconds := friOpeningMilliseconds
            finishProofEncodeMilliseconds := proofEncodeMilliseconds
            finishContributionSegmentMilliseconds := contributionSegmentMilliseconds
            finishContributionVerifyMilliseconds := contributionVerifyMilliseconds })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_acceptance_sound
      assumptions
      (some
        { summary with
          finishQueryPlanMilliseconds := queryPlanMilliseconds
          finishConstantOpeningMilliseconds := constantOpeningMilliseconds
          finishWitnessOpeningMilliseconds := witnessOpeningMilliseconds
          finishFriOpeningMilliseconds := friOpeningMilliseconds
          finishProofEncodeMilliseconds := proofEncodeMilliseconds
          finishContributionSegmentMilliseconds := contributionSegmentMilliseconds
          finishContributionVerifyMilliseconds := contributionVerifyMilliseconds })
      publicInput
      proof
      observed

theorem proof_artifact_finish_aggregate_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (queryPlanMilliseconds constantOpeningMilliseconds witnessOpeningMilliseconds
      friOpeningMilliseconds proofEncodeMilliseconds contributionSegmentMilliseconds
      contributionVerifyMilliseconds : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishQueryPlanMilliseconds := queryPlanMilliseconds
            finishConstantOpeningMilliseconds := constantOpeningMilliseconds
            finishWitnessOpeningMilliseconds := witnessOpeningMilliseconds
            finishFriOpeningMilliseconds := friOpeningMilliseconds
            finishProofEncodeMilliseconds := proofEncodeMilliseconds
            finishContributionSegmentMilliseconds := contributionSegmentMilliseconds
            finishContributionVerifyMilliseconds := contributionVerifyMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (proof_artifact_finish_aggregate_timing_acceptance_sound
        assumptions
        summary
        queryPlanMilliseconds
        constantOpeningMilliseconds
        witnessOpeningMilliseconds
        friOpeningMilliseconds
        proofEncodeMilliseconds
        contributionSegmentMilliseconds
        contributionVerifyMilliseconds
        publicInput
        proof
        observed)

end Lzvm
