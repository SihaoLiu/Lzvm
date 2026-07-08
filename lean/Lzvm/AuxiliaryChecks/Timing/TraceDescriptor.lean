/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.Timing.Trace

/-!
Descriptor upload timing acceptance projections.
-/

namespace Lzvm

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
  exact
    guest_pc_trace_timing_acceptance_core_and_sound
      assumptions
      (some
        { summary with
          guestDeviceSourceDescriptorUploadWordCount := wordCount })
      publicInput
      proof
      observed

theorem guest_pc_trace_descriptor_upload_word_count_acceptance_audited_core_contract
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_audited_core_contract
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
  exact
    guest_pc_trace_timing_acceptance_core_and_sound
      assumptions
      (some
        { summary with
          guestDeviceSourceDescriptorUploadByteCount := byteCount
          guestDeviceSourceDescriptorUploadWordCount := wordCount
          guestDeviceSourceDescriptorUploadRowCount := rowCount })
      publicInput
      proof
      observed

theorem guest_pc_trace_descriptor_upload_shape_acceptance_audited_core_contract
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_some_summary_acceptance_audited_core_contract
      assumptions
      { summary with
            guestDeviceSourceDescriptorUploadByteCount := byteCount
            guestDeviceSourceDescriptorUploadWordCount := wordCount
            guestDeviceSourceDescriptorUploadRowCount := rowCount }
      publicInput
      proof
      observed

end Lzvm
