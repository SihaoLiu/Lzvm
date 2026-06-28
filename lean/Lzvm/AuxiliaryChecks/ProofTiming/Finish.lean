/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.ProofTiming.Core

/-!
Proof artifact finish timing observation contracts.
-/

namespace Lzvm

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
  have core :=
    proof_artifact_finish_external_source_timing_acceptance_verifier_core_contract
      assumptions
      summary
      externalSourceMilliseconds
      descriptorUploadMilliseconds
      traceExpandMilliseconds
      publicInput
      proof
      observed
  have sound :=
    proof_artifact_finish_external_source_timing_acceptance_sound
      assumptions
      summary
      externalSourceMilliseconds
      descriptorUploadMilliseconds
      traceExpandMilliseconds
      publicInput
      proof
      observed
  exact And.intro core sound

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
  have core :=
    proof_artifact_finish_witness_opening_subtiming_acceptance_verifier_core_contract
      assumptions
      summary
      setupMilliseconds
      leafExtendMilliseconds
      leafHashMilliseconds
      pathMilliseconds
      publicInput
      proof
      observed
  have sound :=
    proof_artifact_finish_witness_opening_subtiming_acceptance_sound
      assumptions
      summary
      setupMilliseconds
      leafExtendMilliseconds
      leafHashMilliseconds
      pathMilliseconds
      publicInput
      proof
      observed
  exact And.intro core sound

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
  have core :=
    proof_artifact_finish_descriptor_upload_word_count_acceptance_verifier_core_contract
      assumptions
      summary
      wordCount
      publicInput
      proof
      observed
  have sound :=
    proof_artifact_finish_descriptor_upload_word_count_acceptance_sound
      assumptions
      summary
      wordCount
      publicInput
      proof
      observed
  exact And.intro core sound

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
  have core :=
    proof_artifact_finish_descriptor_upload_shape_acceptance_verifier_core_contract
      assumptions
      summary
      byteCount
      wordCount
      rowCount
      publicInput
      proof
      observed
  have sound :=
    proof_artifact_finish_descriptor_upload_shape_acceptance_sound
      assumptions
      summary
      byteCount
      wordCount
      rowCount
      publicInput
      proof
      observed
  exact And.intro core sound

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


end Lzvm
