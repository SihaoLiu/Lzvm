/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.ProofTiming.Finish

/-!
External-source proof artifact finish timing contracts.
-/

namespace Lzvm

theorem proof_artifact_finish_retained_source_row_values_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (retainedSourceCount retainedParentCheckpointOpeningCount
      retainedParentCheckpointOpeningRowCount rowValuesMilliseconds
      sourceExtendMilliseconds sourceDownloadMilliseconds sourceExtendCalls
      sourceExtendMaxRows sourceRows words bytes : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningRetainedSourceCount := retainedSourceCount
            finishWitnessOpeningExternalSourceCount := 0
            finishWitnessOpeningEmbeddedSourceCount := 0
            finishWitnessOpeningMissingSourceCount := 0
            finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
              retainedParentCheckpointOpeningCount
            finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
              retainedParentCheckpointOpeningRowCount
            finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
            finishWitnessOpeningRowValueSourceExtendMilliseconds :=
              sourceExtendMilliseconds
            finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
              sourceDownloadMilliseconds
            finishWitnessOpeningRowValueDeviceDownloadMilliseconds := 0
            finishWitnessOpeningRowValuesDeviceRowCount := 0
            finishWitnessOpeningRowValuesDeviceDownloadBatchCount := 0
            finishWitnessOpeningRowValuesDeviceSingleDownloadCount := 0
            finishWitnessOpeningRowValuesSourceExtendCallCount := sourceExtendCalls
            finishWitnessOpeningRowValuesSourceExtendMaxRowCount := sourceExtendMaxRows
            finishWitnessOpeningRowValuesSourceRowCount := sourceRows
            finishWitnessOpeningRowValuesWordCount := words
            finishWitnessOpeningRowValuesByteCount := bytes })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
      assumptions
      { summary with
        finishWitnessOpeningRetainedSourceCount := retainedSourceCount
        finishWitnessOpeningExternalSourceCount := 0
        finishWitnessOpeningEmbeddedSourceCount := 0
        finishWitnessOpeningMissingSourceCount := 0
        finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
          retainedParentCheckpointOpeningCount
        finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
          retainedParentCheckpointOpeningRowCount
        finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
        finishWitnessOpeningRowValueSourceExtendMilliseconds :=
          sourceExtendMilliseconds
        finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
          sourceDownloadMilliseconds
        finishWitnessOpeningRowValueDeviceDownloadMilliseconds := 0
        finishWitnessOpeningRowValuesDeviceRowCount := 0
        finishWitnessOpeningRowValuesDeviceDownloadBatchCount := 0
        finishWitnessOpeningRowValuesDeviceSingleDownloadCount := 0
        finishWitnessOpeningRowValuesSourceExtendCallCount := sourceExtendCalls
        finishWitnessOpeningRowValuesSourceExtendMaxRowCount := sourceExtendMaxRows
        finishWitnessOpeningRowValuesSourceRowCount := sourceRows
        finishWitnessOpeningRowValuesWordCount := words
        finishWitnessOpeningRowValuesByteCount := bytes }
      publicInput
      proof
      observed

theorem proof_artifact_finish_retained_source_row_values_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (retainedSourceCount retainedParentCheckpointOpeningCount
      retainedParentCheckpointOpeningRowCount rowValuesMilliseconds
      sourceExtendMilliseconds sourceDownloadMilliseconds sourceExtendCalls
      sourceExtendMaxRows sourceRows words bytes : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningRetainedSourceCount := retainedSourceCount
            finishWitnessOpeningExternalSourceCount := 0
            finishWitnessOpeningEmbeddedSourceCount := 0
            finishWitnessOpeningMissingSourceCount := 0
            finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
              retainedParentCheckpointOpeningCount
            finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
              retainedParentCheckpointOpeningRowCount
            finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
            finishWitnessOpeningRowValueSourceExtendMilliseconds :=
              sourceExtendMilliseconds
            finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
              sourceDownloadMilliseconds
            finishWitnessOpeningRowValueDeviceDownloadMilliseconds := 0
            finishWitnessOpeningRowValuesDeviceRowCount := 0
            finishWitnessOpeningRowValuesDeviceDownloadBatchCount := 0
            finishWitnessOpeningRowValuesDeviceSingleDownloadCount := 0
            finishWitnessOpeningRowValuesSourceExtendCallCount := sourceExtendCalls
            finishWitnessOpeningRowValuesSourceExtendMaxRowCount := sourceExtendMaxRows
            finishWitnessOpeningRowValuesSourceRowCount := sourceRows
            finishWitnessOpeningRowValuesWordCount := words
            finishWitnessOpeningRowValuesByteCount := bytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
        finishWitnessOpeningRetainedSourceCount := retainedSourceCount
        finishWitnessOpeningExternalSourceCount := 0
        finishWitnessOpeningEmbeddedSourceCount := 0
        finishWitnessOpeningMissingSourceCount := 0
        finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
          retainedParentCheckpointOpeningCount
        finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
          retainedParentCheckpointOpeningRowCount
        finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
        finishWitnessOpeningRowValueSourceExtendMilliseconds :=
          sourceExtendMilliseconds
        finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
          sourceDownloadMilliseconds
        finishWitnessOpeningRowValueDeviceDownloadMilliseconds := 0
        finishWitnessOpeningRowValuesDeviceRowCount := 0
        finishWitnessOpeningRowValuesDeviceDownloadBatchCount := 0
        finishWitnessOpeningRowValuesDeviceSingleDownloadCount := 0
        finishWitnessOpeningRowValuesSourceExtendCallCount := sourceExtendCalls
        finishWitnessOpeningRowValuesSourceExtendMaxRowCount := sourceExtendMaxRows
        finishWitnessOpeningRowValuesSourceRowCount := sourceRows
        finishWitnessOpeningRowValuesWordCount := words
        finishWitnessOpeningRowValuesByteCount := bytes }
      publicInput
      proof
      observed

theorem proof_artifact_finish_retained_source_row_values_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (retainedSourceCount retainedParentCheckpointOpeningCount
      retainedParentCheckpointOpeningRowCount rowValuesMilliseconds
      sourceExtendMilliseconds sourceDownloadMilliseconds sourceExtendCalls
      sourceExtendMaxRows sourceRows words bytes : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningRetainedSourceCount := retainedSourceCount
            finishWitnessOpeningExternalSourceCount := 0
            finishWitnessOpeningEmbeddedSourceCount := 0
            finishWitnessOpeningMissingSourceCount := 0
            finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
              retainedParentCheckpointOpeningCount
            finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
              retainedParentCheckpointOpeningRowCount
            finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
            finishWitnessOpeningRowValueSourceExtendMilliseconds :=
              sourceExtendMilliseconds
            finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
              sourceDownloadMilliseconds
            finishWitnessOpeningRowValueDeviceDownloadMilliseconds := 0
            finishWitnessOpeningRowValuesDeviceRowCount := 0
            finishWitnessOpeningRowValuesDeviceDownloadBatchCount := 0
            finishWitnessOpeningRowValuesDeviceSingleDownloadCount := 0
            finishWitnessOpeningRowValuesSourceExtendCallCount := sourceExtendCalls
            finishWitnessOpeningRowValuesSourceExtendMaxRowCount := sourceExtendMaxRows
            finishWitnessOpeningRowValuesSourceRowCount := sourceRows
            finishWitnessOpeningRowValuesWordCount := words
            finishWitnessOpeningRowValuesByteCount := bytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_core_and_sound
      assumptions
      { summary with
        finishWitnessOpeningRetainedSourceCount := retainedSourceCount
        finishWitnessOpeningExternalSourceCount := 0
        finishWitnessOpeningEmbeddedSourceCount := 0
        finishWitnessOpeningMissingSourceCount := 0
        finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
          retainedParentCheckpointOpeningCount
        finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
          retainedParentCheckpointOpeningRowCount
        finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
        finishWitnessOpeningRowValueSourceExtendMilliseconds :=
          sourceExtendMilliseconds
        finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
          sourceDownloadMilliseconds
        finishWitnessOpeningRowValueDeviceDownloadMilliseconds := 0
        finishWitnessOpeningRowValuesDeviceRowCount := 0
        finishWitnessOpeningRowValuesDeviceDownloadBatchCount := 0
        finishWitnessOpeningRowValuesDeviceSingleDownloadCount := 0
        finishWitnessOpeningRowValuesSourceExtendCallCount := sourceExtendCalls
        finishWitnessOpeningRowValuesSourceExtendMaxRowCount := sourceExtendMaxRows
        finishWitnessOpeningRowValuesSourceRowCount := sourceRows
        finishWitnessOpeningRowValuesWordCount := words
        finishWitnessOpeningRowValuesByteCount := bytes }
      publicInput
      proof
      observed

theorem proof_artifact_finish_retained_source_row_values_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (retainedSourceCount retainedParentCheckpointOpeningCount
      retainedParentCheckpointOpeningRowCount rowValuesMilliseconds
      sourceExtendMilliseconds sourceDownloadMilliseconds sourceExtendCalls
      sourceExtendMaxRows sourceRows words bytes : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningRetainedSourceCount := retainedSourceCount
            finishWitnessOpeningExternalSourceCount := 0
            finishWitnessOpeningEmbeddedSourceCount := 0
            finishWitnessOpeningMissingSourceCount := 0
            finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
              retainedParentCheckpointOpeningCount
            finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
              retainedParentCheckpointOpeningRowCount
            finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
            finishWitnessOpeningRowValueSourceExtendMilliseconds :=
              sourceExtendMilliseconds
            finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
              sourceDownloadMilliseconds
            finishWitnessOpeningRowValueDeviceDownloadMilliseconds := 0
            finishWitnessOpeningRowValuesDeviceRowCount := 0
            finishWitnessOpeningRowValuesDeviceDownloadBatchCount := 0
            finishWitnessOpeningRowValuesDeviceSingleDownloadCount := 0
            finishWitnessOpeningRowValuesSourceExtendCallCount := sourceExtendCalls
            finishWitnessOpeningRowValuesSourceExtendMaxRowCount := sourceExtendMaxRows
            finishWitnessOpeningRowValuesSourceRowCount := sourceRows
            finishWitnessOpeningRowValuesWordCount := words
            finishWitnessOpeningRowValuesByteCount := bytes })
        publicInput
        proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_audited_core_contract
      assumptions
      { summary with
        finishWitnessOpeningRetainedSourceCount := retainedSourceCount
        finishWitnessOpeningExternalSourceCount := 0
        finishWitnessOpeningEmbeddedSourceCount := 0
        finishWitnessOpeningMissingSourceCount := 0
        finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
          retainedParentCheckpointOpeningCount
        finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
          retainedParentCheckpointOpeningRowCount
        finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
        finishWitnessOpeningRowValueSourceExtendMilliseconds :=
          sourceExtendMilliseconds
        finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
          sourceDownloadMilliseconds
        finishWitnessOpeningRowValueDeviceDownloadMilliseconds := 0
        finishWitnessOpeningRowValuesDeviceRowCount := 0
        finishWitnessOpeningRowValuesDeviceDownloadBatchCount := 0
        finishWitnessOpeningRowValuesDeviceSingleDownloadCount := 0
        finishWitnessOpeningRowValuesSourceExtendCallCount := sourceExtendCalls
        finishWitnessOpeningRowValuesSourceExtendMaxRowCount := sourceExtendMaxRows
        finishWitnessOpeningRowValuesSourceRowCount := sourceRows
        finishWitnessOpeningRowValuesWordCount := words
        finishWitnessOpeningRowValuesByteCount := bytes }
      publicInput
      proof
      observed

theorem proof_artifact_finish_external_source_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (externalSourceMilliseconds descriptorUploadMilliseconds
      traceExpandMilliseconds : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessExternalSourceMilliseconds := externalSourceMilliseconds
            finishWitnessExternalSourceDescriptorUploadMilliseconds :=
              descriptorUploadMilliseconds
            finishWitnessExternalSourceTraceExpandMilliseconds := traceExpandMilliseconds })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
      assumptions
      { summary with
        finishWitnessExternalSourceMilliseconds := externalSourceMilliseconds
        finishWitnessExternalSourceDescriptorUploadMilliseconds :=
          descriptorUploadMilliseconds
        finishWitnessExternalSourceTraceExpandMilliseconds := traceExpandMilliseconds }
      publicInput
      proof
      observed

theorem proof_artifact_finish_external_source_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (externalSourceMilliseconds descriptorUploadMilliseconds
      traceExpandMilliseconds : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessExternalSourceMilliseconds := externalSourceMilliseconds
            finishWitnessExternalSourceDescriptorUploadMilliseconds :=
              descriptorUploadMilliseconds
            finishWitnessExternalSourceTraceExpandMilliseconds := traceExpandMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
        finishWitnessExternalSourceMilliseconds := externalSourceMilliseconds
        finishWitnessExternalSourceDescriptorUploadMilliseconds :=
          descriptorUploadMilliseconds
        finishWitnessExternalSourceTraceExpandMilliseconds := traceExpandMilliseconds }
      publicInput
      proof
      observed

theorem proof_artifact_finish_external_source_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (externalSourceMilliseconds descriptorUploadMilliseconds
      traceExpandMilliseconds : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessExternalSourceMilliseconds := externalSourceMilliseconds
            finishWitnessExternalSourceDescriptorUploadMilliseconds :=
              descriptorUploadMilliseconds
            finishWitnessExternalSourceTraceExpandMilliseconds := traceExpandMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_core_and_sound
      assumptions
      { summary with
        finishWitnessExternalSourceMilliseconds := externalSourceMilliseconds
        finishWitnessExternalSourceDescriptorUploadMilliseconds :=
          descriptorUploadMilliseconds
        finishWitnessExternalSourceTraceExpandMilliseconds := traceExpandMilliseconds }
      publicInput
      proof
      observed

theorem proof_artifact_finish_witness_opening_subtiming_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (setupMilliseconds leafExtendMilliseconds leafHashMilliseconds
      pathMilliseconds : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningSetupMilliseconds := setupMilliseconds
            finishWitnessOpeningLeafExtendMilliseconds := leafExtendMilliseconds
            finishWitnessOpeningLeafHashMilliseconds := leafHashMilliseconds
            finishWitnessOpeningPathMilliseconds := pathMilliseconds })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
      assumptions
      { summary with
        finishWitnessOpeningSetupMilliseconds := setupMilliseconds
        finishWitnessOpeningLeafExtendMilliseconds := leafExtendMilliseconds
        finishWitnessOpeningLeafHashMilliseconds := leafHashMilliseconds
        finishWitnessOpeningPathMilliseconds := pathMilliseconds }
      publicInput
      proof
      observed

theorem proof_artifact_finish_witness_opening_subtiming_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (setupMilliseconds leafExtendMilliseconds leafHashMilliseconds
      pathMilliseconds : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningSetupMilliseconds := setupMilliseconds
            finishWitnessOpeningLeafExtendMilliseconds := leafExtendMilliseconds
            finishWitnessOpeningLeafHashMilliseconds := leafHashMilliseconds
            finishWitnessOpeningPathMilliseconds := pathMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
        finishWitnessOpeningSetupMilliseconds := setupMilliseconds
        finishWitnessOpeningLeafExtendMilliseconds := leafExtendMilliseconds
        finishWitnessOpeningLeafHashMilliseconds := leafHashMilliseconds
        finishWitnessOpeningPathMilliseconds := pathMilliseconds }
      publicInput
      proof
      observed

theorem proof_artifact_finish_witness_opening_subtiming_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (setupMilliseconds leafExtendMilliseconds leafHashMilliseconds
      pathMilliseconds : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningSetupMilliseconds := setupMilliseconds
            finishWitnessOpeningLeafExtendMilliseconds := leafExtendMilliseconds
            finishWitnessOpeningLeafHashMilliseconds := leafHashMilliseconds
            finishWitnessOpeningPathMilliseconds := pathMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_core_and_sound
      assumptions
      { summary with
        finishWitnessOpeningSetupMilliseconds := setupMilliseconds
        finishWitnessOpeningLeafExtendMilliseconds := leafExtendMilliseconds
        finishWitnessOpeningLeafHashMilliseconds := leafHashMilliseconds
        finishWitnessOpeningPathMilliseconds := pathMilliseconds }
      publicInput
      proof
      observed

theorem proof_artifact_finish_descriptor_upload_word_count_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (wordCount : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some { summary with finishWitnessExternalSourceDescriptorUploadWordCount := wordCount })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
      assumptions
      { summary with finishWitnessExternalSourceDescriptorUploadWordCount := wordCount }
      publicInput
      proof
      observed

theorem proof_artifact_finish_descriptor_upload_word_count_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (wordCount : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some { summary with finishWitnessExternalSourceDescriptorUploadWordCount := wordCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with finishWitnessExternalSourceDescriptorUploadWordCount := wordCount }
      publicInput
      proof
      observed

theorem proof_artifact_finish_descriptor_upload_word_count_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (wordCount : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some { summary with finishWitnessExternalSourceDescriptorUploadWordCount := wordCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_core_and_sound
      assumptions
      { summary with finishWitnessExternalSourceDescriptorUploadWordCount := wordCount }
      publicInput
      proof
      observed

theorem proof_artifact_finish_descriptor_upload_shape_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (byteCount wordCount rowCount : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessExternalSourceDescriptorUploadByteCount := byteCount
            finishWitnessExternalSourceDescriptorUploadWordCount := wordCount
            finishWitnessExternalSourceDescriptorUploadRowCount := rowCount })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
      assumptions
      { summary with
        finishWitnessExternalSourceDescriptorUploadByteCount := byteCount
        finishWitnessExternalSourceDescriptorUploadWordCount := wordCount
        finishWitnessExternalSourceDescriptorUploadRowCount := rowCount }
      publicInput
      proof
      observed

theorem proof_artifact_finish_descriptor_upload_shape_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (byteCount wordCount rowCount : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessExternalSourceDescriptorUploadByteCount := byteCount
            finishWitnessExternalSourceDescriptorUploadWordCount := wordCount
            finishWitnessExternalSourceDescriptorUploadRowCount := rowCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
        finishWitnessExternalSourceDescriptorUploadByteCount := byteCount
        finishWitnessExternalSourceDescriptorUploadWordCount := wordCount
        finishWitnessExternalSourceDescriptorUploadRowCount := rowCount }
      publicInput
      proof
      observed

theorem proof_artifact_finish_descriptor_upload_shape_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (byteCount wordCount rowCount : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessExternalSourceDescriptorUploadByteCount := byteCount
            finishWitnessExternalSourceDescriptorUploadWordCount := wordCount
            finishWitnessExternalSourceDescriptorUploadRowCount := rowCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_core_and_sound
      assumptions
      { summary with
        finishWitnessExternalSourceDescriptorUploadByteCount := byteCount
        finishWitnessExternalSourceDescriptorUploadWordCount := wordCount
        finishWitnessExternalSourceDescriptorUploadRowCount := rowCount }
      publicInput
      proof
      observed

theorem proof_artifact_finish_aggregate_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (queryPlanMilliseconds constantOpeningMilliseconds witnessOpeningMilliseconds
      witnessOpeningQueryCount witnessOpeningQueryUnitCount witnessOpeningSingleQueryUnitCount
      witnessOpeningMaxQueriesPerUnit witnessOpeningStageCount
      witnessOpeningRetainedSourceCount witnessOpeningExternalSourceCount
      witnessOpeningEmbeddedSourceCount witnessOpeningMissingSourceCount
      witnessOpeningRetainedLeafDigestOpeningCount
      witnessOpeningRetainedLeafDigestOpeningRowCount
      witnessOpeningRetainedParentCheckpointOpeningCount
      witnessOpeningRetainedParentCheckpointOpeningRowCount
      witnessOpeningRowDedupInputRowCount witnessOpeningRowDedupUniqueRowCount
      witnessOpeningRowDedupElidedRowCount
      descriptorUploadByteCount descriptorUploadWordCount descriptorUploadRowCount
      friOpeningMilliseconds friOpeningUnitBuildMilliseconds
      friOpeningLayerTreeMilliseconds friOpeningQueryMilliseconds
      friOpeningFoldMilliseconds friOpeningUnitCount friOpeningLayerCount
      friOpeningQueryCount friTranscriptUnitBuildMilliseconds
      friTranscriptLayerTreeMilliseconds friTranscriptFoldMilliseconds
      friTranscriptUnitCount friTranscriptLayerCount proofEncodeMilliseconds
      contributionSegmentMilliseconds contributionVerifyMilliseconds
      contributionChallengeMilliseconds : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishQueryPlanMilliseconds := queryPlanMilliseconds
            finishConstantOpeningMilliseconds := constantOpeningMilliseconds
            finishWitnessOpeningMilliseconds := witnessOpeningMilliseconds
            finishWitnessOpeningQueryCount := witnessOpeningQueryCount
            finishWitnessOpeningQueryUnitCount := witnessOpeningQueryUnitCount
            finishWitnessOpeningSingleQueryUnitCount := witnessOpeningSingleQueryUnitCount
            finishWitnessOpeningMaxQueriesPerUnit := witnessOpeningMaxQueriesPerUnit
            finishWitnessOpeningStageCount := witnessOpeningStageCount
            finishWitnessOpeningRetainedSourceCount := witnessOpeningRetainedSourceCount
            finishWitnessOpeningExternalSourceCount := witnessOpeningExternalSourceCount
            finishWitnessOpeningEmbeddedSourceCount := witnessOpeningEmbeddedSourceCount
            finishWitnessOpeningMissingSourceCount := witnessOpeningMissingSourceCount
            finishWitnessOpeningRetainedLeafDigestOpeningCount :=
              witnessOpeningRetainedLeafDigestOpeningCount
            finishWitnessOpeningRetainedLeafDigestOpeningRowCount :=
              witnessOpeningRetainedLeafDigestOpeningRowCount
            finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
              witnessOpeningRetainedParentCheckpointOpeningCount
            finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
              witnessOpeningRetainedParentCheckpointOpeningRowCount
            finishWitnessOpeningRowDedupInputRowCount :=
              witnessOpeningRowDedupInputRowCount
            finishWitnessOpeningRowDedupUniqueRowCount :=
              witnessOpeningRowDedupUniqueRowCount
            finishWitnessOpeningRowDedupElidedRowCount :=
              witnessOpeningRowDedupElidedRowCount
            finishWitnessExternalSourceDescriptorUploadByteCount := descriptorUploadByteCount
            finishWitnessExternalSourceDescriptorUploadWordCount := descriptorUploadWordCount
            finishWitnessExternalSourceDescriptorUploadRowCount := descriptorUploadRowCount
            finishFriOpeningMilliseconds := friOpeningMilliseconds
            finishFriOpeningUnitBuildMilliseconds := friOpeningUnitBuildMilliseconds
            finishFriOpeningLayerTreeMilliseconds := friOpeningLayerTreeMilliseconds
            finishFriOpeningQueryMilliseconds := friOpeningQueryMilliseconds
            finishFriOpeningFoldMilliseconds := friOpeningFoldMilliseconds
            finishFriOpeningUnitCount := friOpeningUnitCount
            finishFriOpeningLayerCount := friOpeningLayerCount
            finishFriOpeningQueryCount := friOpeningQueryCount
            finishFriTranscriptUnitBuildMilliseconds := friTranscriptUnitBuildMilliseconds
            finishFriTranscriptLayerTreeMilliseconds := friTranscriptLayerTreeMilliseconds
            finishFriTranscriptFoldMilliseconds := friTranscriptFoldMilliseconds
            finishFriTranscriptUnitCount := friTranscriptUnitCount
            finishFriTranscriptLayerCount := friTranscriptLayerCount
            finishProofEncodeMilliseconds := proofEncodeMilliseconds
            finishContributionSegmentMilliseconds := contributionSegmentMilliseconds
            finishContributionVerifyMilliseconds := contributionVerifyMilliseconds
            finishContributionChallengeMilliseconds := contributionChallengeMilliseconds })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
      assumptions
      { summary with
        finishQueryPlanMilliseconds := queryPlanMilliseconds
        finishConstantOpeningMilliseconds := constantOpeningMilliseconds
        finishWitnessOpeningMilliseconds := witnessOpeningMilliseconds
        finishWitnessOpeningQueryCount := witnessOpeningQueryCount
        finishWitnessOpeningQueryUnitCount := witnessOpeningQueryUnitCount
        finishWitnessOpeningSingleQueryUnitCount := witnessOpeningSingleQueryUnitCount
        finishWitnessOpeningMaxQueriesPerUnit := witnessOpeningMaxQueriesPerUnit
        finishWitnessOpeningStageCount := witnessOpeningStageCount
        finishWitnessOpeningRetainedSourceCount := witnessOpeningRetainedSourceCount
        finishWitnessOpeningExternalSourceCount := witnessOpeningExternalSourceCount
        finishWitnessOpeningEmbeddedSourceCount := witnessOpeningEmbeddedSourceCount
        finishWitnessOpeningMissingSourceCount := witnessOpeningMissingSourceCount
        finishWitnessOpeningRetainedLeafDigestOpeningCount :=
          witnessOpeningRetainedLeafDigestOpeningCount
        finishWitnessOpeningRetainedLeafDigestOpeningRowCount :=
          witnessOpeningRetainedLeafDigestOpeningRowCount
        finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
          witnessOpeningRetainedParentCheckpointOpeningCount
        finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
          witnessOpeningRetainedParentCheckpointOpeningRowCount
        finishWitnessOpeningRowDedupInputRowCount :=
          witnessOpeningRowDedupInputRowCount
        finishWitnessOpeningRowDedupUniqueRowCount :=
          witnessOpeningRowDedupUniqueRowCount
        finishWitnessOpeningRowDedupElidedRowCount :=
          witnessOpeningRowDedupElidedRowCount
        finishWitnessExternalSourceDescriptorUploadByteCount := descriptorUploadByteCount
        finishWitnessExternalSourceDescriptorUploadWordCount := descriptorUploadWordCount
        finishWitnessExternalSourceDescriptorUploadRowCount := descriptorUploadRowCount
        finishFriOpeningMilliseconds := friOpeningMilliseconds
        finishFriOpeningUnitBuildMilliseconds := friOpeningUnitBuildMilliseconds
        finishFriOpeningLayerTreeMilliseconds := friOpeningLayerTreeMilliseconds
        finishFriOpeningQueryMilliseconds := friOpeningQueryMilliseconds
        finishFriOpeningFoldMilliseconds := friOpeningFoldMilliseconds
        finishFriOpeningUnitCount := friOpeningUnitCount
        finishFriOpeningLayerCount := friOpeningLayerCount
        finishFriOpeningQueryCount := friOpeningQueryCount
        finishFriTranscriptUnitBuildMilliseconds := friTranscriptUnitBuildMilliseconds
        finishFriTranscriptLayerTreeMilliseconds := friTranscriptLayerTreeMilliseconds
        finishFriTranscriptFoldMilliseconds := friTranscriptFoldMilliseconds
        finishFriTranscriptUnitCount := friTranscriptUnitCount
        finishFriTranscriptLayerCount := friTranscriptLayerCount
        finishProofEncodeMilliseconds := proofEncodeMilliseconds
        finishContributionSegmentMilliseconds := contributionSegmentMilliseconds
        finishContributionVerifyMilliseconds := contributionVerifyMilliseconds
        finishContributionChallengeMilliseconds := contributionChallengeMilliseconds }
      publicInput
      proof
      observed

theorem proof_artifact_finish_witness_opening_shape_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (queryCount queryUnitCount singleQueryUnitCount maxQueriesPerUnit stageCount
      retainedSourceCount externalSourceCount embeddedSourceCount missingSourceCount
      retainedLeafDigestOpeningCount retainedLeafDigestOpeningRowCount
      retainedParentCheckpointOpeningCount retainedParentCheckpointOpeningRowCount
      rowDedupInputRowCount rowDedupUniqueRowCount rowDedupElidedRowCount : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningQueryCount := queryCount
            finishWitnessOpeningQueryUnitCount := queryUnitCount
            finishWitnessOpeningSingleQueryUnitCount := singleQueryUnitCount
            finishWitnessOpeningMaxQueriesPerUnit := maxQueriesPerUnit
            finishWitnessOpeningStageCount := stageCount
            finishWitnessOpeningRetainedSourceCount := retainedSourceCount
            finishWitnessOpeningExternalSourceCount := externalSourceCount
            finishWitnessOpeningEmbeddedSourceCount := embeddedSourceCount
            finishWitnessOpeningMissingSourceCount := missingSourceCount
            finishWitnessOpeningRetainedLeafDigestOpeningCount := retainedLeafDigestOpeningCount
            finishWitnessOpeningRetainedLeafDigestOpeningRowCount :=
              retainedLeafDigestOpeningRowCount
            finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
              retainedParentCheckpointOpeningCount
            finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
              retainedParentCheckpointOpeningRowCount
            finishWitnessOpeningRowDedupInputRowCount := rowDedupInputRowCount
            finishWitnessOpeningRowDedupUniqueRowCount := rowDedupUniqueRowCount
            finishWitnessOpeningRowDedupElidedRowCount := rowDedupElidedRowCount })
        publicInput
        proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_audited_core_contract
      assumptions
      { summary with
        finishWitnessOpeningQueryCount := queryCount
        finishWitnessOpeningQueryUnitCount := queryUnitCount
        finishWitnessOpeningSingleQueryUnitCount := singleQueryUnitCount
        finishWitnessOpeningMaxQueriesPerUnit := maxQueriesPerUnit
        finishWitnessOpeningStageCount := stageCount
        finishWitnessOpeningRetainedSourceCount := retainedSourceCount
        finishWitnessOpeningExternalSourceCount := externalSourceCount
        finishWitnessOpeningEmbeddedSourceCount := embeddedSourceCount
        finishWitnessOpeningMissingSourceCount := missingSourceCount
        finishWitnessOpeningRetainedLeafDigestOpeningCount := retainedLeafDigestOpeningCount
        finishWitnessOpeningRetainedLeafDigestOpeningRowCount :=
          retainedLeafDigestOpeningRowCount
        finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
          retainedParentCheckpointOpeningCount
        finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
          retainedParentCheckpointOpeningRowCount
        finishWitnessOpeningRowDedupInputRowCount := rowDedupInputRowCount
        finishWitnessOpeningRowDedupUniqueRowCount := rowDedupUniqueRowCount
        finishWitnessOpeningRowDedupElidedRowCount := rowDedupElidedRowCount }
      publicInput
      proof
      observed

theorem proof_artifact_finish_leaf_work_shape_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (leafHashRows leafHashBytes leafHashArity2Rows leafHashArity2Bytes
      leafHashArity4Rows leafHashArity4Bytes leafCosetCalls leafCosetOutputBytes
      leafCosetColumns leafCosetMaxColumns leafCosetNttLaunches
      leafCosetBitReverseLaunches leafCosetNttStageLaunches
      leafCosetNttBlockTwiddleLaunches leafCosetNormalizeLaunches
      leafCosetPackLaunches leafCosetUnpackLaunches : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningLeafHashRowCount := leafHashRows
            finishWitnessOpeningLeafHashByteCount := leafHashBytes
            finishWitnessOpeningLeafHashArity2RowCount := leafHashArity2Rows
            finishWitnessOpeningLeafHashArity2ByteCount := leafHashArity2Bytes
            finishWitnessOpeningLeafHashArity4RowCount := leafHashArity4Rows
            finishWitnessOpeningLeafHashArity4ByteCount := leafHashArity4Bytes
            finishWitnessOpeningLeafCosetExtendCallCount := leafCosetCalls
            finishWitnessOpeningLeafCosetExtendOutputByteCount := leafCosetOutputBytes
            finishWitnessOpeningLeafCosetExtendColumnCount := leafCosetColumns
            finishWitnessOpeningLeafCosetExtendMaxColumnCount := leafCosetMaxColumns
            finishWitnessOpeningLeafCosetExtendNttLaunchCount := leafCosetNttLaunches
            finishWitnessOpeningLeafCosetExtendBitReverseLaunchCount :=
              leafCosetBitReverseLaunches
            finishWitnessOpeningLeafCosetExtendNttStageLaunchCount :=
              leafCosetNttStageLaunches
            finishWitnessOpeningLeafCosetExtendNttBlockTwiddleLaunchCount :=
              leafCosetNttBlockTwiddleLaunches
            finishWitnessOpeningLeafCosetExtendNormalizeLaunchCount :=
              leafCosetNormalizeLaunches
            finishWitnessOpeningLeafCosetExtendPackLaunchCount := leafCosetPackLaunches
            finishWitnessOpeningLeafCosetExtendUnpackLaunchCount := leafCosetUnpackLaunches })
        publicInput
        proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_audited_core_contract
      assumptions
      { summary with
        finishWitnessOpeningLeafHashRowCount := leafHashRows
        finishWitnessOpeningLeafHashByteCount := leafHashBytes
        finishWitnessOpeningLeafHashArity2RowCount := leafHashArity2Rows
        finishWitnessOpeningLeafHashArity2ByteCount := leafHashArity2Bytes
        finishWitnessOpeningLeafHashArity4RowCount := leafHashArity4Rows
        finishWitnessOpeningLeafHashArity4ByteCount := leafHashArity4Bytes
        finishWitnessOpeningLeafCosetExtendCallCount := leafCosetCalls
        finishWitnessOpeningLeafCosetExtendOutputByteCount := leafCosetOutputBytes
        finishWitnessOpeningLeafCosetExtendColumnCount := leafCosetColumns
        finishWitnessOpeningLeafCosetExtendMaxColumnCount := leafCosetMaxColumns
        finishWitnessOpeningLeafCosetExtendNttLaunchCount := leafCosetNttLaunches
        finishWitnessOpeningLeafCosetExtendBitReverseLaunchCount :=
          leafCosetBitReverseLaunches
        finishWitnessOpeningLeafCosetExtendNttStageLaunchCount :=
          leafCosetNttStageLaunches
        finishWitnessOpeningLeafCosetExtendNttBlockTwiddleLaunchCount :=
          leafCosetNttBlockTwiddleLaunches
        finishWitnessOpeningLeafCosetExtendNormalizeLaunchCount :=
          leafCosetNormalizeLaunches
        finishWitnessOpeningLeafCosetExtendPackLaunchCount := leafCosetPackLaunches
        finishWitnessOpeningLeafCosetExtendUnpackLaunchCount := leafCosetUnpackLaunches }
      publicInput
      proof
      observed

theorem proof_artifact_finish_path_parent_hash_shape_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (parentHashRows parentHashBytes parentHashLaunches
      recomputedRows recomputedBytes recomputedLaunches
      retainedLeafDigestRows retainedLeafDigestBytes retainedLeafDigestLaunches
      retainedCheckpointPrefixRows retainedCheckpointPrefixBytes retainedCheckpointPrefixLaunches
      retainedCheckpointSuffixRows retainedCheckpointSuffixBytes retainedCheckpointSuffixLaunches
      : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningPathParentHashRowCount := parentHashRows
            finishWitnessOpeningPathParentHashByteCount := parentHashBytes
            finishWitnessOpeningPathParentHashLaunchCount := parentHashLaunches
            finishWitnessOpeningPathParentHashRecomputedRowCount := recomputedRows
            finishWitnessOpeningPathParentHashRecomputedByteCount := recomputedBytes
            finishWitnessOpeningPathParentHashRecomputedLaunchCount := recomputedLaunches
            finishWitnessOpeningPathParentHashRetainedLeafDigestRowCount :=
              retainedLeafDigestRows
            finishWitnessOpeningPathParentHashRetainedLeafDigestByteCount :=
              retainedLeafDigestBytes
            finishWitnessOpeningPathParentHashRetainedLeafDigestLaunchCount :=
              retainedLeafDigestLaunches
            finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixRowCount :=
              retainedCheckpointPrefixRows
            finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixByteCount :=
              retainedCheckpointPrefixBytes
            finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixLaunchCount :=
              retainedCheckpointPrefixLaunches
            finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixRowCount :=
              retainedCheckpointSuffixRows
            finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixByteCount :=
              retainedCheckpointSuffixBytes
            finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixLaunchCount :=
              retainedCheckpointSuffixLaunches })
        publicInput
        proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_audited_core_contract
      assumptions
      { summary with
        finishWitnessOpeningPathParentHashRowCount := parentHashRows
        finishWitnessOpeningPathParentHashByteCount := parentHashBytes
        finishWitnessOpeningPathParentHashLaunchCount := parentHashLaunches
        finishWitnessOpeningPathParentHashRecomputedRowCount := recomputedRows
        finishWitnessOpeningPathParentHashRecomputedByteCount := recomputedBytes
        finishWitnessOpeningPathParentHashRecomputedLaunchCount := recomputedLaunches
        finishWitnessOpeningPathParentHashRetainedLeafDigestRowCount :=
          retainedLeafDigestRows
        finishWitnessOpeningPathParentHashRetainedLeafDigestByteCount :=
          retainedLeafDigestBytes
        finishWitnessOpeningPathParentHashRetainedLeafDigestLaunchCount :=
          retainedLeafDigestLaunches
        finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixRowCount :=
          retainedCheckpointPrefixRows
        finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixByteCount :=
          retainedCheckpointPrefixBytes
        finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixLaunchCount :=
          retainedCheckpointPrefixLaunches
        finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixRowCount :=
          retainedCheckpointSuffixRows
        finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixByteCount :=
          retainedCheckpointSuffixBytes
        finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixLaunchCount :=
          retainedCheckpointSuffixLaunches }
      publicInput
      proof
      observed

theorem proof_artifact_finish_row_values_shape_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (rowValuesMilliseconds sourceExtendMilliseconds sourceDownloadMilliseconds
      deviceDownloadMilliseconds deviceRows deviceDownloadBatches deviceSingleDownloads
      sourceExtendCalls sourceExtendMaxRows sourceRows words bytes : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
            finishWitnessOpeningRowValueSourceExtendMilliseconds :=
              sourceExtendMilliseconds
            finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
              sourceDownloadMilliseconds
            finishWitnessOpeningRowValueDeviceDownloadMilliseconds :=
              deviceDownloadMilliseconds
            finishWitnessOpeningRowValuesDeviceRowCount := deviceRows
            finishWitnessOpeningRowValuesDeviceDownloadBatchCount :=
              deviceDownloadBatches
            finishWitnessOpeningRowValuesDeviceSingleDownloadCount :=
              deviceSingleDownloads
            finishWitnessOpeningRowValuesSourceExtendCallCount := sourceExtendCalls
            finishWitnessOpeningRowValuesSourceExtendMaxRowCount := sourceExtendMaxRows
            finishWitnessOpeningRowValuesSourceRowCount := sourceRows
            finishWitnessOpeningRowValuesWordCount := words
            finishWitnessOpeningRowValuesByteCount := bytes })
        publicInput
        proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_audited_core_contract
      assumptions
      { summary with
        finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
        finishWitnessOpeningRowValueSourceExtendMilliseconds :=
          sourceExtendMilliseconds
        finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
          sourceDownloadMilliseconds
        finishWitnessOpeningRowValueDeviceDownloadMilliseconds :=
          deviceDownloadMilliseconds
        finishWitnessOpeningRowValuesDeviceRowCount := deviceRows
        finishWitnessOpeningRowValuesDeviceDownloadBatchCount :=
          deviceDownloadBatches
        finishWitnessOpeningRowValuesDeviceSingleDownloadCount :=
          deviceSingleDownloads
        finishWitnessOpeningRowValuesSourceExtendCallCount := sourceExtendCalls
        finishWitnessOpeningRowValuesSourceExtendMaxRowCount := sourceExtendMaxRows
        finishWitnessOpeningRowValuesSourceRowCount := sourceRows
        finishWitnessOpeningRowValuesWordCount := words
        finishWitnessOpeningRowValuesByteCount := bytes }
      publicInput
      proof
      observed

theorem proof_artifact_finish_witness_opening_subtiming_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (setupMilliseconds leafExtendMilliseconds leafHashMilliseconds
      pathMilliseconds : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningSetupMilliseconds := setupMilliseconds
            finishWitnessOpeningLeafExtendMilliseconds := leafExtendMilliseconds
            finishWitnessOpeningLeafHashMilliseconds := leafHashMilliseconds
            finishWitnessOpeningPathMilliseconds := pathMilliseconds })
        publicInput
        proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_audited_core_contract
      assumptions
      { summary with
        finishWitnessOpeningSetupMilliseconds := setupMilliseconds
        finishWitnessOpeningLeafExtendMilliseconds := leafExtendMilliseconds
        finishWitnessOpeningLeafHashMilliseconds := leafHashMilliseconds
        finishWitnessOpeningPathMilliseconds := pathMilliseconds }
      publicInput
      proof
      observed

theorem proof_artifact_finish_path_parent_hash_per_unit_shape_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (rowsPerQuery rowsPerStage launchesPerStage : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningPathParentHashRowsPerQuery := rowsPerQuery
            finishWitnessOpeningPathParentHashRowsPerStage := rowsPerStage
            finishWitnessOpeningPathParentHashLaunchesPerStage := launchesPerStage })
        publicInput
        proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_audited_core_contract
      assumptions
      { summary with
        finishWitnessOpeningPathParentHashRowsPerQuery := rowsPerQuery
        finishWitnessOpeningPathParentHashRowsPerStage := rowsPerStage
        finishWitnessOpeningPathParentHashLaunchesPerStage := launchesPerStage }
      publicInput
      proof
      observed

theorem proof_artifact_finish_external_source_timing_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (externalSourceMilliseconds descriptorUploadMilliseconds
      traceExpandMilliseconds : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessExternalSourceMilliseconds := externalSourceMilliseconds
            finishWitnessExternalSourceDescriptorUploadMilliseconds :=
              descriptorUploadMilliseconds
            finishWitnessExternalSourceTraceExpandMilliseconds := traceExpandMilliseconds })
        publicInput
        proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_audited_core_contract
      assumptions
      { summary with
        finishWitnessExternalSourceMilliseconds := externalSourceMilliseconds
        finishWitnessExternalSourceDescriptorUploadMilliseconds :=
          descriptorUploadMilliseconds
        finishWitnessExternalSourceTraceExpandMilliseconds := traceExpandMilliseconds }
      publicInput
      proof
      observed

theorem proof_artifact_finish_descriptor_upload_word_count_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (wordCount : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some { summary with finishWitnessExternalSourceDescriptorUploadWordCount := wordCount })
        publicInput
        proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_audited_core_contract
      assumptions
      { summary with finishWitnessExternalSourceDescriptorUploadWordCount := wordCount }
      publicInput
      proof
      observed

theorem proof_artifact_finish_descriptor_upload_shape_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (byteCount wordCount rowCount : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessExternalSourceDescriptorUploadByteCount := byteCount
            finishWitnessExternalSourceDescriptorUploadWordCount := wordCount
            finishWitnessExternalSourceDescriptorUploadRowCount := rowCount })
        publicInput
        proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_audited_core_contract
      assumptions
      { summary with
        finishWitnessExternalSourceDescriptorUploadByteCount := byteCount
        finishWitnessExternalSourceDescriptorUploadWordCount := wordCount
        finishWitnessExternalSourceDescriptorUploadRowCount := rowCount }
      publicInput
      proof
      observed

end Lzvm
