/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.TimingCore

/-!
Runtime timing acceptance projections for auxiliary verifier metadata.
-/

namespace Lzvm

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

theorem guest_pc_trace_stream_elapsed_timing_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_stream_elapsed_timing_acceptance_verifier_core_contract
      assumptions
      summary
      elapsedMilliseconds
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_stream_elapsed_timing_acceptance_sound
      assumptions
      summary
      elapsedMilliseconds
      publicInput
      proof
      observed
  exact And.intro core sound

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

theorem guest_pc_trace_descriptor_width_counts_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_descriptor_width_counts_acceptance_verifier_core_contract
      assumptions
      summary
      compactRows
      wideRows
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_descriptor_width_counts_acceptance_sound
      assumptions
      summary
      compactRows
      wideRows
      publicInput
      proof
      observed
  exact And.intro core sound

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

theorem guest_pc_trace_report_timing_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_report_timing_acceptance_verifier_core_contract
      assumptions
      summary
      reportMilliseconds
      validationMilliseconds
      reportCount
      reportRows
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_report_timing_acceptance_sound
      assumptions
      summary
      reportMilliseconds
      validationMilliseconds
      reportCount
      reportRows
      publicInput
      proof
      observed
  exact And.intro core sound

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

theorem guest_pc_trace_report_subtiming_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_report_subtiming_acceptance_verifier_core_contract
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
      observed
  have sound :=
    guest_pc_trace_report_subtiming_acceptance_sound
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
      observed
  exact And.intro core sound

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

theorem guest_pc_trace_report_lower_subtiming_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_report_lower_subtiming_acceptance_verifier_core_contract
      assumptions
      summary
      singleRowMilliseconds
      multiRowMilliseconds
      pendingDmaMilliseconds
      amoMilliseconds
      storeConditionalMilliseconds
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_report_lower_subtiming_acceptance_sound
      assumptions
      summary
      singleRowMilliseconds
      multiRowMilliseconds
      pendingDmaMilliseconds
      amoMilliseconds
      storeConditionalMilliseconds
      publicInput
      proof
      observed
  exact And.intro core sound

theorem guest_pc_trace_emit_descriptor_wait_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (emitMilliseconds descriptorMilliseconds descriptorRows pendingSendWaitMilliseconds
      pendingReceiveWaitMilliseconds segmentSendWaitMilliseconds
      segmentReceiveWaitMilliseconds parallelWorkerCount parallelDispatchedCount
      parallelReceivedCount parallelEmittedCount parallelMaxReorderCount
      ownedStreamingLowerSegmentCount
      parallelStreamStartDispatchWaitMilliseconds
      parallelStreamChunkDispatchWaitMilliseconds
      parallelStreamSegmentDispatchWaitMilliseconds
      parallelStreamFinishDispatchWaitMilliseconds : Nat) :
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
            guestTraceParallelLowerMaxReorderCount := parallelMaxReorderCount
            guestTraceOwnedStreamingLowerSegmentCount := ownedStreamingLowerSegmentCount
            guestTraceParallelLowerStreamStartDispatchWaitMilliseconds :=
              parallelStreamStartDispatchWaitMilliseconds
            guestTraceParallelLowerStreamChunkDispatchWaitMilliseconds :=
              parallelStreamChunkDispatchWaitMilliseconds
            guestTraceParallelLowerStreamSegmentDispatchWaitMilliseconds :=
              parallelStreamSegmentDispatchWaitMilliseconds
            guestTraceParallelLowerStreamFinishDispatchWaitMilliseconds :=
              parallelStreamFinishDispatchWaitMilliseconds })
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
          guestTraceParallelLowerMaxReorderCount := parallelMaxReorderCount
          guestTraceOwnedStreamingLowerSegmentCount := ownedStreamingLowerSegmentCount
          guestTraceParallelLowerStreamStartDispatchWaitMilliseconds :=
            parallelStreamStartDispatchWaitMilliseconds
          guestTraceParallelLowerStreamChunkDispatchWaitMilliseconds :=
            parallelStreamChunkDispatchWaitMilliseconds
          guestTraceParallelLowerStreamSegmentDispatchWaitMilliseconds :=
            parallelStreamSegmentDispatchWaitMilliseconds
          guestTraceParallelLowerStreamFinishDispatchWaitMilliseconds :=
            parallelStreamFinishDispatchWaitMilliseconds })
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
      parallelReceivedCount parallelEmittedCount parallelMaxReorderCount
      ownedStreamingLowerSegmentCount
      parallelStreamStartDispatchWaitMilliseconds
      parallelStreamChunkDispatchWaitMilliseconds
      parallelStreamSegmentDispatchWaitMilliseconds
      parallelStreamFinishDispatchWaitMilliseconds : Nat) :
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
            guestTraceParallelLowerMaxReorderCount := parallelMaxReorderCount
            guestTraceOwnedStreamingLowerSegmentCount := ownedStreamingLowerSegmentCount
            guestTraceParallelLowerStreamStartDispatchWaitMilliseconds :=
              parallelStreamStartDispatchWaitMilliseconds
            guestTraceParallelLowerStreamChunkDispatchWaitMilliseconds :=
              parallelStreamChunkDispatchWaitMilliseconds
            guestTraceParallelLowerStreamSegmentDispatchWaitMilliseconds :=
              parallelStreamSegmentDispatchWaitMilliseconds
            guestTraceParallelLowerStreamFinishDispatchWaitMilliseconds :=
              parallelStreamFinishDispatchWaitMilliseconds })
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
            guestTraceParallelLowerMaxReorderCount := parallelMaxReorderCount
            guestTraceOwnedStreamingLowerSegmentCount := ownedStreamingLowerSegmentCount
            guestTraceParallelLowerStreamStartDispatchWaitMilliseconds :=
              parallelStreamStartDispatchWaitMilliseconds
            guestTraceParallelLowerStreamChunkDispatchWaitMilliseconds :=
              parallelStreamChunkDispatchWaitMilliseconds
            guestTraceParallelLowerStreamSegmentDispatchWaitMilliseconds :=
              parallelStreamSegmentDispatchWaitMilliseconds
            guestTraceParallelLowerStreamFinishDispatchWaitMilliseconds :=
              parallelStreamFinishDispatchWaitMilliseconds }
      publicInput
      proof
      observed

theorem guest_pc_trace_emit_descriptor_wait_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary)
    (emitMilliseconds descriptorMilliseconds descriptorRows pendingSendWaitMilliseconds
      pendingReceiveWaitMilliseconds segmentSendWaitMilliseconds
      segmentReceiveWaitMilliseconds parallelWorkerCount parallelDispatchedCount
      parallelReceivedCount parallelEmittedCount parallelMaxReorderCount
      ownedStreamingLowerSegmentCount
      parallelStreamStartDispatchWaitMilliseconds
      parallelStreamChunkDispatchWaitMilliseconds
      parallelStreamSegmentDispatchWaitMilliseconds
      parallelStreamFinishDispatchWaitMilliseconds : Nat) :
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
            guestTraceParallelLowerMaxReorderCount := parallelMaxReorderCount
            guestTraceOwnedStreamingLowerSegmentCount := ownedStreamingLowerSegmentCount
            guestTraceParallelLowerStreamStartDispatchWaitMilliseconds :=
              parallelStreamStartDispatchWaitMilliseconds
            guestTraceParallelLowerStreamChunkDispatchWaitMilliseconds :=
              parallelStreamChunkDispatchWaitMilliseconds
            guestTraceParallelLowerStreamSegmentDispatchWaitMilliseconds :=
              parallelStreamSegmentDispatchWaitMilliseconds
            guestTraceParallelLowerStreamFinishDispatchWaitMilliseconds :=
              parallelStreamFinishDispatchWaitMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_emit_descriptor_wait_timing_acceptance_verifier_core_contract
      assumptions
      summary
      emitMilliseconds
      descriptorMilliseconds
      descriptorRows
      pendingSendWaitMilliseconds
      pendingReceiveWaitMilliseconds
      segmentSendWaitMilliseconds
      segmentReceiveWaitMilliseconds
      parallelWorkerCount
      parallelDispatchedCount
      parallelReceivedCount
      parallelEmittedCount
      parallelMaxReorderCount
      ownedStreamingLowerSegmentCount
      parallelStreamStartDispatchWaitMilliseconds
      parallelStreamChunkDispatchWaitMilliseconds
      parallelStreamSegmentDispatchWaitMilliseconds
      parallelStreamFinishDispatchWaitMilliseconds
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_emit_descriptor_wait_timing_acceptance_sound
      assumptions
      summary
      emitMilliseconds
      descriptorMilliseconds
      descriptorRows
      pendingSendWaitMilliseconds
      pendingReceiveWaitMilliseconds
      segmentSendWaitMilliseconds
      segmentReceiveWaitMilliseconds
      parallelWorkerCount
      parallelDispatchedCount
      parallelReceivedCount
      parallelEmittedCount
      parallelMaxReorderCount
      ownedStreamingLowerSegmentCount
      parallelStreamStartDispatchWaitMilliseconds
      parallelStreamChunkDispatchWaitMilliseconds
      parallelStreamSegmentDispatchWaitMilliseconds
      parallelStreamFinishDispatchWaitMilliseconds
      publicInput
      proof
      observed
  exact And.intro core sound

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

theorem guest_pc_trace_device_source_timing_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_device_source_timing_acceptance_verifier_core_contract
      assumptions
      summary
      buildMilliseconds
      descriptorUploadMilliseconds
      traceExpandMilliseconds
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_device_source_timing_acceptance_sound
      assumptions
      summary
      buildMilliseconds
      descriptorUploadMilliseconds
      traceExpandMilliseconds
      publicInput
      proof
      observed
  exact And.intro core sound

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

theorem guest_pc_trace_regular_stage_timing_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_regular_stage_timing_acceptance_verifier_core_contract
      assumptions
      summary
      regularConstraintsMilliseconds
      regularHintsMilliseconds
      stageCommitMilliseconds
      stageTraceExtractMilliseconds
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_regular_stage_timing_acceptance_sound
      assumptions
      summary
      regularConstraintsMilliseconds
      regularHintsMilliseconds
      stageCommitMilliseconds
      stageTraceExtractMilliseconds
      publicInput
      proof
      observed
  exact And.intro core sound

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

theorem guest_pc_trace_shape_counts_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_shape_counts_acceptance_verifier_core_contract
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
      observed
  have sound :=
    guest_pc_trace_shape_counts_acceptance_sound
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
      observed
  exact And.intro core sound

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

theorem guest_pc_trace_memory_access_shape_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_memory_access_shape_acceptance_verifier_core_contract
      assumptions
      summary
      indirectMemoryRows
      memorySourceReads
      memoryStoreRows
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_memory_access_shape_acceptance_sound
      assumptions
      summary
      indirectMemoryRows
      memorySourceReads
      memoryStoreRows
      publicInput
      proof
      observed
  exact And.intro core sound

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

theorem guest_pc_trace_report_buffer_capacity_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_report_buffer_capacity_acceptance_verifier_core_contract
      assumptions
      summary
      capacity
      maxCapacity
      excessCapacity
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_report_buffer_capacity_acceptance_sound
      assumptions
      summary
      capacity
      maxCapacity
      excessCapacity
      publicInput
      proof
      observed
  exact And.intro core sound

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

theorem guest_pc_trace_descriptor_upload_word_count_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_descriptor_upload_word_count_acceptance_verifier_core_contract
      assumptions
      summary
      wordCount
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_descriptor_upload_word_count_acceptance_sound
      assumptions
      summary
      wordCount
      publicInput
      proof
      observed
  exact And.intro core sound

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

theorem guest_pc_trace_descriptor_upload_shape_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    guest_pc_trace_descriptor_upload_shape_acceptance_verifier_core_contract
      assumptions
      summary
      byteCount
      wordCount
      rowCount
      publicInput
      proof
      observed
  have sound :=
    guest_pc_trace_descriptor_upload_shape_acceptance_sound
      assumptions
      summary
      byteCount
      wordCount
      rowCount
      publicInput
      proof
      observed
  exact And.intro core sound


end Lzvm
