/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks
import Lzvm.AuxiliaryChecks.ProofTiming

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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_report_subtiming_acceptance_sound
        assumptions
        summary
        rowValidationMilliseconds
        sourceValuesMilliseconds
        precompileMemoryMilliseconds
        instructionResultMilliseconds
        nextPcMilliseconds
        registerAccessMilliseconds
        memoryAccessMilliseconds
        storeApplyMilliseconds
        visitMilliseconds
        publicInput
        proof
        observed)

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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_report_lower_subtiming_acceptance_sound
        assumptions
        summary
        singleRowMilliseconds
        multiRowMilliseconds
        pendingDmaMilliseconds
        amoMilliseconds
        storeConditionalMilliseconds
        publicInput
        proof
        observed)

theorem guest_pc_trace_emit_descriptor_wait_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (emitMilliseconds descriptorMilliseconds descriptorRows pendingSendWaitMilliseconds
      pendingReceiveWaitMilliseconds segmentSendWaitMilliseconds
      segmentReceiveWaitMilliseconds : Nat) :
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
            guestTraceSegmentReceiveWaitMilliseconds := segmentReceiveWaitMilliseconds })
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
          guestTraceSegmentReceiveWaitMilliseconds := segmentReceiveWaitMilliseconds })
      publicInput
      proof
      observed

theorem guest_pc_trace_emit_descriptor_wait_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (emitMilliseconds descriptorMilliseconds descriptorRows pendingSendWaitMilliseconds
      pendingReceiveWaitMilliseconds segmentSendWaitMilliseconds
      segmentReceiveWaitMilliseconds : Nat) :
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
            guestTraceSegmentReceiveWaitMilliseconds := segmentReceiveWaitMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_emit_descriptor_wait_timing_acceptance_sound
        assumptions
        summary
        emitMilliseconds
        descriptorMilliseconds
        descriptorRows
        pendingSendWaitMilliseconds
        pendingReceiveWaitMilliseconds
        segmentSendWaitMilliseconds
        segmentReceiveWaitMilliseconds
        publicInput
        proof
        observed)

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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_device_source_timing_acceptance_sound
        assumptions
        summary
        buildMilliseconds
        descriptorUploadMilliseconds
        traceExpandMilliseconds
        publicInput
        proof
        observed)

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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_regular_stage_timing_acceptance_sound
        assumptions
        summary
        regularConstraintsMilliseconds
        regularHintsMilliseconds
        stageCommitMilliseconds
        stageTraceExtractMilliseconds
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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_descriptor_upload_shape_acceptance_sound
        assumptions
        summary
        byteCount
        wordCount
        rowCount
        publicInput
        proof
        observed)

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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_source_retention_byte_counts_acceptance_sound
        assumptions
        summary
        retainedBytes
        rejectedBytes
        limitBytes
        publicInput
        proof
        observed)

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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_source_retention_counts_acceptance_sound
        assumptions
        summary
        attemptCount
        retainedCount
        rejectedCount
        publicInput
        proof
        observed)

theorem guest_pc_trace_descriptor_buffer_retention_byte_counts_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (retainedBytes rejectedBytes : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDescriptorBufferRetentionRetainedByteCount := retainedBytes
            guestDescriptorBufferRetentionRejectedByteCount := rejectedBytes })
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
          guestDescriptorBufferRetentionRejectedByteCount := rejectedBytes })
      publicInput
      proof
      observed

theorem guest_pc_trace_descriptor_buffer_retention_byte_counts_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (retainedBytes rejectedBytes : Nat) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance
        system
        (some
          { summary with
            guestDescriptorBufferRetentionRetainedByteCount := retainedBytes
            guestDescriptorBufferRetentionRejectedByteCount := rejectedBytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_descriptor_buffer_retention_byte_counts_acceptance_sound
        assumptions
        summary
        retainedBytes
        rejectedBytes
        publicInput
        proof
        observed)

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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_descriptor_buffer_retention_counts_acceptance_sound
        assumptions
        summary
        attemptCount
        retainedCount
        rejectedCount
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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_leaf_extend_timing_acceptance_sound
        assumptions
        summary
        extendMilliseconds
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

end Lzvm
