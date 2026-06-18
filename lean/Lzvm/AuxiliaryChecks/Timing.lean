/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks
import Lzvm.AuxiliaryChecks.ProofTiming
import Lzvm.AuxiliaryChecks.ProofTimingVerifier

/-!
Runtime timing acceptance projections for auxiliary verifier metadata.
-/

namespace Lzvm

def TimingObservedAcceptance
    (system : VerifierModel)
    (observations : List TimingObservation)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system observations publicInput proof

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
    ignored_metadata_acceptance_sound
      assumptions
      observations
      publicInput
      proof
      acceptedWithTimings

theorem timing_observation_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (observations : List TimingObservation) :
    forall publicInput proof,
      TimingObservedAcceptance system observations publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      observations
      publicInput
      proof
      observed

def GuestPcTraceTimingObservedAcceptance
    (system : VerifierModel)
    (summary : Option GuestPcTraceTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system summary publicInput proof

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
    ignored_metadata_acceptance_sound
      assumptions
      summary
      publicInput
      proof
      acceptedWithGuestPcTraceTimings

theorem guest_pc_trace_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

theorem guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system (some summary) publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_verifier_core_contract
      assumptions
      (some summary)
      publicInput
      proof
      observed

theorem guest_pc_trace_stream_elapsed_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (elapsedMilliseconds : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some { summary with guestTraceStreamElapsedMilliseconds := elapsedMilliseconds })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some { summary with guestTraceStreamElapsedMilliseconds := elapsedMilliseconds })
      publicInput
      proof
      observed

theorem guest_pc_trace_stream_elapsed_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (elapsedMilliseconds : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some { summary with guestTraceStreamElapsedMilliseconds := elapsedMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with guestTraceStreamElapsedMilliseconds := elapsedMilliseconds }
      publicInput
      proof
      observed

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
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestTraceDescriptorCompactRowCount := compactRows
            guestTraceDescriptorWideRowCount := wideRows }
      publicInput
      proof
      observed

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
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestTraceReportMilliseconds := reportMilliseconds
            guestTraceReportValidationMilliseconds := validationMilliseconds
            guestTraceReportCount := reportCount
            guestTraceReportRowCount := reportRows }
      publicInput
      proof
      observed

theorem guest_pc_trace_report_subtiming_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (rowValidationMilliseconds sourceValuesMilliseconds precompileMemoryMilliseconds
      instructionResultMilliseconds nextPcMilliseconds registerAccessMilliseconds
      memoryAccessMilliseconds storeApplyMilliseconds visitMilliseconds : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceReportRowValidationMilliseconds := rowValidationMilliseconds
            guestTraceReportSourceValuesMilliseconds := sourceValuesMilliseconds
            guestTraceReportPrecompileMemoryMilliseconds := precompileMemoryMilliseconds
            guestTraceReportInstructionResultMilliseconds := instructionResultMilliseconds
            guestTraceReportNextPcMilliseconds := nextPcMilliseconds
            guestTraceReportRegisterAccessMilliseconds := registerAccessMilliseconds
            guestTraceReportMemoryAccessMilliseconds := memoryAccessMilliseconds
            guestTraceReportStoreApplyMilliseconds := storeApplyMilliseconds
            guestTraceReportVisitMilliseconds := visitMilliseconds })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestTraceReportRowValidationMilliseconds := rowValidationMilliseconds
          guestTraceReportSourceValuesMilliseconds := sourceValuesMilliseconds
          guestTraceReportPrecompileMemoryMilliseconds := precompileMemoryMilliseconds
          guestTraceReportInstructionResultMilliseconds := instructionResultMilliseconds
          guestTraceReportNextPcMilliseconds := nextPcMilliseconds
          guestTraceReportRegisterAccessMilliseconds := registerAccessMilliseconds
          guestTraceReportMemoryAccessMilliseconds := memoryAccessMilliseconds
          guestTraceReportStoreApplyMilliseconds := storeApplyMilliseconds
          guestTraceReportVisitMilliseconds := visitMilliseconds })
      publicInput
      proof
      observed

theorem guest_pc_trace_report_subtiming_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (rowValidationMilliseconds sourceValuesMilliseconds precompileMemoryMilliseconds
      instructionResultMilliseconds nextPcMilliseconds registerAccessMilliseconds
      memoryAccessMilliseconds storeApplyMilliseconds visitMilliseconds : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceReportRowValidationMilliseconds := rowValidationMilliseconds
            guestTraceReportSourceValuesMilliseconds := sourceValuesMilliseconds
            guestTraceReportPrecompileMemoryMilliseconds := precompileMemoryMilliseconds
            guestTraceReportInstructionResultMilliseconds := instructionResultMilliseconds
            guestTraceReportNextPcMilliseconds := nextPcMilliseconds
            guestTraceReportRegisterAccessMilliseconds := registerAccessMilliseconds
            guestTraceReportMemoryAccessMilliseconds := memoryAccessMilliseconds
            guestTraceReportStoreApplyMilliseconds := storeApplyMilliseconds
            guestTraceReportVisitMilliseconds := visitMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestTraceReportRowValidationMilliseconds := rowValidationMilliseconds
            guestTraceReportSourceValuesMilliseconds := sourceValuesMilliseconds
            guestTraceReportPrecompileMemoryMilliseconds := precompileMemoryMilliseconds
            guestTraceReportInstructionResultMilliseconds := instructionResultMilliseconds
            guestTraceReportNextPcMilliseconds := nextPcMilliseconds
            guestTraceReportRegisterAccessMilliseconds := registerAccessMilliseconds
            guestTraceReportMemoryAccessMilliseconds := memoryAccessMilliseconds
            guestTraceReportStoreApplyMilliseconds := storeApplyMilliseconds
            guestTraceReportVisitMilliseconds := visitMilliseconds }
      publicInput
      proof
      observed

theorem guest_pc_trace_report_lower_subtiming_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (singleRowMilliseconds multiRowMilliseconds pendingDmaMilliseconds amoMilliseconds
      storeConditionalMilliseconds : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceSingleRowReportLowerMilliseconds := singleRowMilliseconds
            guestTraceMultiRowReportLowerMilliseconds := multiRowMilliseconds
            guestTracePendingDmaReportLowerMilliseconds := pendingDmaMilliseconds
            guestTraceAmoReportLowerMilliseconds := amoMilliseconds
            guestTraceStoreConditionalReportLowerMilliseconds :=
              storeConditionalMilliseconds })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestTraceSingleRowReportLowerMilliseconds := singleRowMilliseconds
          guestTraceMultiRowReportLowerMilliseconds := multiRowMilliseconds
          guestTracePendingDmaReportLowerMilliseconds := pendingDmaMilliseconds
          guestTraceAmoReportLowerMilliseconds := amoMilliseconds
          guestTraceStoreConditionalReportLowerMilliseconds :=
            storeConditionalMilliseconds })
      publicInput
      proof
      observed

theorem guest_pc_trace_report_lower_subtiming_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (singleRowMilliseconds multiRowMilliseconds pendingDmaMilliseconds amoMilliseconds
      storeConditionalMilliseconds : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceSingleRowReportLowerMilliseconds := singleRowMilliseconds
            guestTraceMultiRowReportLowerMilliseconds := multiRowMilliseconds
            guestTracePendingDmaReportLowerMilliseconds := pendingDmaMilliseconds
            guestTraceAmoReportLowerMilliseconds := amoMilliseconds
            guestTraceStoreConditionalReportLowerMilliseconds :=
              storeConditionalMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestTraceSingleRowReportLowerMilliseconds := singleRowMilliseconds
            guestTraceMultiRowReportLowerMilliseconds := multiRowMilliseconds
            guestTracePendingDmaReportLowerMilliseconds := pendingDmaMilliseconds
            guestTraceAmoReportLowerMilliseconds := amoMilliseconds
            guestTraceStoreConditionalReportLowerMilliseconds :=
              storeConditionalMilliseconds }
      publicInput
      proof
      observed

theorem guest_pc_trace_emit_descriptor_wait_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (emitMilliseconds descriptorMilliseconds descriptorRows pendingSendWaitMilliseconds
      pendingReceiveWaitMilliseconds segmentSendWaitMilliseconds
      segmentReceiveWaitMilliseconds parallelWorkerCount parallelDispatchedCount
      parallelReceivedCount parallelEmittedCount parallelMaxReorderCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceEmitMilliseconds := emitMilliseconds
            guestTraceDescriptorMilliseconds := descriptorMilliseconds
            guestTraceDescriptorRowCount := descriptorRows
            guestTracePendingSendWaitMilliseconds := pendingSendWaitMilliseconds
            guestTracePendingReceiveWaitMilliseconds := pendingReceiveWaitMilliseconds
            guestTraceSegmentSendWaitMilliseconds := segmentSendWaitMilliseconds
            guestTraceSegmentReceiveWaitMilliseconds := segmentReceiveWaitMilliseconds
            guestTraceParallelLowerWorkerCount := parallelWorkerCount
            guestTraceParallelLowerDispatchedCount := parallelDispatchedCount
            guestTraceParallelLowerReceivedCount := parallelReceivedCount
            guestTraceParallelLowerEmittedCount := parallelEmittedCount
            guestTraceParallelLowerMaxReorderCount := parallelMaxReorderCount })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestTraceEmitMilliseconds := emitMilliseconds
          guestTraceDescriptorMilliseconds := descriptorMilliseconds
          guestTraceDescriptorRowCount := descriptorRows
          guestTracePendingSendWaitMilliseconds := pendingSendWaitMilliseconds
          guestTracePendingReceiveWaitMilliseconds := pendingReceiveWaitMilliseconds
          guestTraceSegmentSendWaitMilliseconds := segmentSendWaitMilliseconds
          guestTraceSegmentReceiveWaitMilliseconds := segmentReceiveWaitMilliseconds
          guestTraceParallelLowerWorkerCount := parallelWorkerCount
          guestTraceParallelLowerDispatchedCount := parallelDispatchedCount
          guestTraceParallelLowerReceivedCount := parallelReceivedCount
          guestTraceParallelLowerEmittedCount := parallelEmittedCount
          guestTraceParallelLowerMaxReorderCount := parallelMaxReorderCount })
      publicInput
      proof
      observed

theorem guest_pc_trace_emit_descriptor_wait_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (emitMilliseconds descriptorMilliseconds descriptorRows pendingSendWaitMilliseconds
      pendingReceiveWaitMilliseconds segmentSendWaitMilliseconds
      segmentReceiveWaitMilliseconds parallelWorkerCount parallelDispatchedCount
      parallelReceivedCount parallelEmittedCount parallelMaxReorderCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestTraceEmitMilliseconds := emitMilliseconds
            guestTraceDescriptorMilliseconds := descriptorMilliseconds
            guestTraceDescriptorRowCount := descriptorRows
            guestTracePendingSendWaitMilliseconds := pendingSendWaitMilliseconds
            guestTracePendingReceiveWaitMilliseconds := pendingReceiveWaitMilliseconds
            guestTraceSegmentSendWaitMilliseconds := segmentSendWaitMilliseconds
            guestTraceSegmentReceiveWaitMilliseconds := segmentReceiveWaitMilliseconds
            guestTraceParallelLowerWorkerCount := parallelWorkerCount
            guestTraceParallelLowerDispatchedCount := parallelDispatchedCount
            guestTraceParallelLowerReceivedCount := parallelReceivedCount
            guestTraceParallelLowerEmittedCount := parallelEmittedCount
            guestTraceParallelLowerMaxReorderCount := parallelMaxReorderCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestTraceEmitMilliseconds := emitMilliseconds
            guestTraceDescriptorMilliseconds := descriptorMilliseconds
            guestTraceDescriptorRowCount := descriptorRows
            guestTracePendingSendWaitMilliseconds := pendingSendWaitMilliseconds
            guestTracePendingReceiveWaitMilliseconds := pendingReceiveWaitMilliseconds
            guestTraceSegmentSendWaitMilliseconds := segmentSendWaitMilliseconds
            guestTraceSegmentReceiveWaitMilliseconds := segmentReceiveWaitMilliseconds
            guestTraceParallelLowerWorkerCount := parallelWorkerCount
            guestTraceParallelLowerDispatchedCount := parallelDispatchedCount
            guestTraceParallelLowerReceivedCount := parallelReceivedCount
            guestTraceParallelLowerEmittedCount := parallelEmittedCount
            guestTraceParallelLowerMaxReorderCount := parallelMaxReorderCount }
      publicInput
      proof
      observed

theorem guest_pc_trace_device_source_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (buildMilliseconds descriptorUploadMilliseconds traceExpandMilliseconds : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDeviceSourceBuildMilliseconds := buildMilliseconds
            guestDeviceSourceDescriptorUploadMilliseconds := descriptorUploadMilliseconds
            guestDeviceSourceTraceExpandMilliseconds := traceExpandMilliseconds })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestDeviceSourceBuildMilliseconds := buildMilliseconds
          guestDeviceSourceDescriptorUploadMilliseconds := descriptorUploadMilliseconds
          guestDeviceSourceTraceExpandMilliseconds := traceExpandMilliseconds })
      publicInput
      proof
      observed

theorem guest_pc_trace_device_source_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (buildMilliseconds descriptorUploadMilliseconds traceExpandMilliseconds : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDeviceSourceBuildMilliseconds := buildMilliseconds
            guestDeviceSourceDescriptorUploadMilliseconds := descriptorUploadMilliseconds
            guestDeviceSourceTraceExpandMilliseconds := traceExpandMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestDeviceSourceBuildMilliseconds := buildMilliseconds
            guestDeviceSourceDescriptorUploadMilliseconds := descriptorUploadMilliseconds
            guestDeviceSourceTraceExpandMilliseconds := traceExpandMilliseconds }
      publicInput
      proof
      observed

theorem guest_pc_trace_regular_stage_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (regularConstraintsMilliseconds regularHintsMilliseconds
      stageCommitMilliseconds stageTraceExtractMilliseconds : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestRegularConstraintsMilliseconds := regularConstraintsMilliseconds
            guestRegularHintsMilliseconds := regularHintsMilliseconds
            guestStageCommitMilliseconds := stageCommitMilliseconds
            guestStageTraceExtractMilliseconds := stageTraceExtractMilliseconds })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestRegularConstraintsMilliseconds := regularConstraintsMilliseconds
          guestRegularHintsMilliseconds := regularHintsMilliseconds
          guestStageCommitMilliseconds := stageCommitMilliseconds
          guestStageTraceExtractMilliseconds := stageTraceExtractMilliseconds })
      publicInput
      proof
      observed

theorem guest_pc_trace_regular_stage_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (regularConstraintsMilliseconds regularHintsMilliseconds
      stageCommitMilliseconds stageTraceExtractMilliseconds : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestRegularConstraintsMilliseconds := regularConstraintsMilliseconds
            guestRegularHintsMilliseconds := regularHintsMilliseconds
            guestStageCommitMilliseconds := stageCommitMilliseconds
            guestStageTraceExtractMilliseconds := stageTraceExtractMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestRegularConstraintsMilliseconds := regularConstraintsMilliseconds
            guestRegularHintsMilliseconds := regularHintsMilliseconds
            guestStageCommitMilliseconds := stageCommitMilliseconds
            guestStageTraceExtractMilliseconds := stageTraceExtractMilliseconds }
      publicInput
      proof
      observed

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
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
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
            guestTraceNoStoreRowCount := noStoreRows }
      publicInput
      proof
      observed

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
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestTraceIndirectMemoryRowCount := indirectMemoryRows
            guestTraceMemorySourceReadCount := memorySourceReads
            guestTraceMemoryStoreRowCount := memoryStoreRows }
      publicInput
      proof
      observed

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
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestTraceReportBufferCapacity := capacity
            guestTraceReportBufferMaxCapacity := maxCapacity
            guestTraceReportBufferExcessCapacity := excessCapacity }
      publicInput
      proof
      observed

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
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestDeviceSourceDescriptorUploadWordCount := wordCount }
      publicInput
      proof
      observed

theorem guest_pc_trace_descriptor_upload_shape_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (byteCount wordCount rowCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDeviceSourceDescriptorUploadByteCount := byteCount
            guestDeviceSourceDescriptorUploadWordCount := wordCount
            guestDeviceSourceDescriptorUploadRowCount := rowCount })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_sound
      assumptions
      (some
        { summary with
          guestDeviceSourceDescriptorUploadByteCount := byteCount
          guestDeviceSourceDescriptorUploadWordCount := wordCount
          guestDeviceSourceDescriptorUploadRowCount := rowCount })
      publicInput
      proof
      observed

theorem guest_pc_trace_descriptor_upload_shape_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (byteCount wordCount rowCount : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDeviceSourceDescriptorUploadByteCount := byteCount
            guestDeviceSourceDescriptorUploadWordCount := wordCount
            guestDeviceSourceDescriptorUploadRowCount := rowCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
            guestDeviceSourceDescriptorUploadByteCount := byteCount
            guestDeviceSourceDescriptorUploadWordCount := wordCount
            guestDeviceSourceDescriptorUploadRowCount := rowCount }
      publicInput
      proof
      observed

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
