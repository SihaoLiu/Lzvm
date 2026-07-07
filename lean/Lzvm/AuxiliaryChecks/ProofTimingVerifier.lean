/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.ProofTiming

/-!
Verifier-core contracts for proof-finish timing observations.
-/

namespace Lzvm

theorem proof_artifact_finish_verifier_descriptor_upload_shape_acceptance_verifier_core_contract
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
    proof_artifact_finish_descriptor_upload_shape_acceptance_verifier_core_contract
      assumptions
      summary
      byteCount
      wordCount
      rowCount
      publicInput
      proof
      observed

theorem proof_artifact_finish_verifier_descriptor_upload_shape_acceptance_core_and_sound
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
    proof_artifact_finish_descriptor_upload_shape_acceptance_core_and_sound
      assumptions
      summary
      byteCount
      wordCount
      rowCount
      publicInput
      proof
      observed

theorem proof_artifact_finish_verifier_descriptor_upload_shape_acceptance_audited_core_contract
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
    proof_artifact_finish_descriptor_upload_shape_acceptance_audited_core_contract
      assumptions
      summary
      byteCount
      wordCount
      rowCount
      publicInput
      proof
      observed

theorem proof_artifact_finish_verifier_retained_source_row_values_acceptance_verifier_core_contract
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
    proof_artifact_finish_retained_source_row_values_acceptance_verifier_core_contract
      assumptions
      summary
      retainedSourceCount
      retainedParentCheckpointOpeningCount
      retainedParentCheckpointOpeningRowCount
      rowValuesMilliseconds
      sourceExtendMilliseconds
      sourceDownloadMilliseconds
      sourceExtendCalls
      sourceExtendMaxRows
      sourceRows
      words
      bytes
      publicInput
      proof
      observed

theorem proof_artifact_finish_verifier_retained_source_row_values_acceptance_core_and_sound
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
    proof_artifact_finish_retained_source_row_values_acceptance_core_and_sound
      assumptions
      summary
      retainedSourceCount
      retainedParentCheckpointOpeningCount
      retainedParentCheckpointOpeningRowCount
      rowValuesMilliseconds
      sourceExtendMilliseconds
      sourceDownloadMilliseconds
      sourceExtendCalls
      sourceExtendMaxRows
      sourceRows
      words
      bytes
      publicInput
      proof
      observed

theorem proof_artifact_finish_verifier_retained_source_row_values_acceptance_audited_core_contract
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
    proof_artifact_finish_retained_source_row_values_acceptance_audited_core_contract
      assumptions
      summary
      retainedSourceCount
      retainedParentCheckpointOpeningCount
      retainedParentCheckpointOpeningRowCount
      rowValuesMilliseconds
      sourceExtendMilliseconds
      sourceDownloadMilliseconds
      sourceExtendCalls
      sourceExtendMaxRows
      sourceRows
      words
      bytes
      publicInput
      proof
      observed

theorem proof_artifact_finish_aggregate_timing_acceptance_verifier_core_contract
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
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
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

theorem proof_artifact_finish_aggregate_timing_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_core_and_sound
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

theorem proof_artifact_finish_aggregate_timing_acceptance_audited_core_contract
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_audited_core_contract
      assumptions
      _
      publicInput
      proof
      observed

theorem proof_artifact_finish_aggregate_timing_accounting_acceptance_core_and_sound
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
      contributionChallengeMilliseconds : Nat)
    (rowDedupAccounting :
      witnessOpeningRowDedupInputRowCount =
        witnessOpeningRowDedupUniqueRowCount + witnessOpeningRowDedupElidedRowCount) :
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
        ProofArtifactFinishWitnessOpeningRowDedupAccounting
            { summary with
              finishWitnessOpeningRowDedupInputRowCount :=
                witnessOpeningRowDedupInputRowCount
              finishWitnessOpeningRowDedupUniqueRowCount :=
                witnessOpeningRowDedupUniqueRowCount
              finishWitnessOpeningRowDedupElidedRowCount :=
                witnessOpeningRowDedupElidedRowCount }
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have rowAccounting :=
    proof_artifact_finish_witness_opening_row_dedup_accounting_update
      summary
      rowDedupAccounting
  have coreAndSound :=
    proof_artifact_finish_aggregate_timing_acceptance_core_and_sound
      assumptions
      summary
      queryPlanMilliseconds
      constantOpeningMilliseconds
      witnessOpeningMilliseconds
      witnessOpeningQueryCount
      witnessOpeningQueryUnitCount
      witnessOpeningSingleQueryUnitCount
      witnessOpeningMaxQueriesPerUnit
      witnessOpeningStageCount
      witnessOpeningRetainedSourceCount
      witnessOpeningExternalSourceCount
      witnessOpeningEmbeddedSourceCount
      witnessOpeningMissingSourceCount
      witnessOpeningRetainedLeafDigestOpeningCount
      witnessOpeningRetainedLeafDigestOpeningRowCount
      witnessOpeningRetainedParentCheckpointOpeningCount
      witnessOpeningRetainedParentCheckpointOpeningRowCount
      witnessOpeningRowDedupInputRowCount
      witnessOpeningRowDedupUniqueRowCount
      witnessOpeningRowDedupElidedRowCount
      descriptorUploadByteCount
      descriptorUploadWordCount
      descriptorUploadRowCount
      friOpeningMilliseconds
      friOpeningUnitBuildMilliseconds
      friOpeningLayerTreeMilliseconds
      friOpeningQueryMilliseconds
      friOpeningFoldMilliseconds
      friOpeningUnitCount
      friOpeningLayerCount
      friOpeningQueryCount
      friTranscriptUnitBuildMilliseconds
      friTranscriptLayerTreeMilliseconds
      friTranscriptFoldMilliseconds
      friTranscriptUnitCount
      friTranscriptLayerCount
      proofEncodeMilliseconds
      contributionSegmentMilliseconds
      contributionVerifyMilliseconds
      contributionChallengeMilliseconds
      publicInput
      proof
      observed
  exact And.intro rowAccounting coreAndSound

theorem proof_artifact_finish_aggregate_timing_accounting_acceptance_audited_core_contract
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
      contributionChallengeMilliseconds : Nat)
    (rowDedupAccounting :
      witnessOpeningRowDedupInputRowCount =
        witnessOpeningRowDedupUniqueRowCount + witnessOpeningRowDedupElidedRowCount) :
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
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ ProofArtifactFinishWitnessOpeningRowDedupAccounting
            { summary with
              finishWitnessOpeningRowDedupInputRowCount :=
                witnessOpeningRowDedupInputRowCount
              finishWitnessOpeningRowDedupUniqueRowCount :=
                witnessOpeningRowDedupUniqueRowCount
              finishWitnessOpeningRowDedupElidedRowCount :=
                witnessOpeningRowDedupElidedRowCount }
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have rowAccounting :=
    proof_artifact_finish_witness_opening_row_dedup_accounting_update
      summary
      rowDedupAccounting
  have audited :=
    proof_artifact_finish_aggregate_timing_acceptance_audited_core_contract
      assumptions
      summary
      queryPlanMilliseconds
      constantOpeningMilliseconds
      witnessOpeningMilliseconds
      witnessOpeningQueryCount
      witnessOpeningQueryUnitCount
      witnessOpeningSingleQueryUnitCount
      witnessOpeningMaxQueriesPerUnit
      witnessOpeningStageCount
      witnessOpeningRetainedSourceCount
      witnessOpeningExternalSourceCount
      witnessOpeningEmbeddedSourceCount
      witnessOpeningMissingSourceCount
      witnessOpeningRetainedLeafDigestOpeningCount
      witnessOpeningRetainedLeafDigestOpeningRowCount
      witnessOpeningRetainedParentCheckpointOpeningCount
      witnessOpeningRetainedParentCheckpointOpeningRowCount
      witnessOpeningRowDedupInputRowCount
      witnessOpeningRowDedupUniqueRowCount
      witnessOpeningRowDedupElidedRowCount
      descriptorUploadByteCount
      descriptorUploadWordCount
      descriptorUploadRowCount
      friOpeningMilliseconds
      friOpeningUnitBuildMilliseconds
      friOpeningLayerTreeMilliseconds
      friOpeningQueryMilliseconds
      friOpeningFoldMilliseconds
      friOpeningUnitCount
      friOpeningLayerCount
      friOpeningQueryCount
      friTranscriptUnitBuildMilliseconds
      friTranscriptLayerTreeMilliseconds
      friTranscriptFoldMilliseconds
      friTranscriptUnitCount
      friTranscriptLayerCount
      proofEncodeMilliseconds
      contributionSegmentMilliseconds
      contributionVerifyMilliseconds
      contributionChallengeMilliseconds
      publicInput
      proof
      observed
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro rowAccounting audited.right.right))

end Lzvm
