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
      friOpeningMilliseconds friTranscriptUnitBuildMilliseconds
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

end Lzvm
