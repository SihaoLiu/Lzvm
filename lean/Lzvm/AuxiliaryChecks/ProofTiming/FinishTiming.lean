/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.ProofTiming.Core

/-!
Proof artifact finish timing acceptance base contracts.
-/

namespace Lzvm

def ProofArtifactFinishTimingObservedAcceptance
    (system : VerifierModel)
    (summary : Option ProofArtifactFinishTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system summary publicInput proof

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
    ignored_metadata_acceptance_sound
      assumptions
      summary
      publicInput
      proof
      acceptedWithProofFinishTimings

theorem proof_artifact_finish_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

theorem proof_artifact_finish_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_core_and_sound
      assumptions
      summary
      publicInput
      proof
      observed

theorem proof_artifact_finish_timing_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system summary publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_audited_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

theorem proof_artifact_finish_timing_some_summary_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system (some summary) publicInput proof ->
        SoundWitness system publicInput proof :=
  proof_artifact_finish_timing_acceptance_sound assumptions (some summary)

theorem proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system (some summary) publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_acceptance_verifier_core_contract
      assumptions
      (some summary)
      publicInput
      proof
      observed

theorem proof_artifact_finish_timing_some_summary_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system (some summary) publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_core_and_sound
      assumptions
      (some summary)
      publicInput
      proof
      observed

theorem proof_artifact_finish_timing_some_summary_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system (some summary) publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_acceptance_audited_core_contract
      assumptions
      (some summary)
      publicInput
      proof
      observed

theorem proof_artifact_finish_top_level_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (queryPlanMilliseconds constantOpeningMilliseconds witnessOpeningMilliseconds
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

theorem proof_artifact_finish_top_level_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (queryPlanMilliseconds constantOpeningMilliseconds witnessOpeningMilliseconds
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

theorem proof_artifact_finish_top_level_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (queryPlanMilliseconds constantOpeningMilliseconds witnessOpeningMilliseconds
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

theorem proof_artifact_finish_top_level_timing_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (queryPlanMilliseconds constantOpeningMilliseconds witnessOpeningMilliseconds
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
      { summary with
        finishQueryPlanMilliseconds := queryPlanMilliseconds
        finishConstantOpeningMilliseconds := constantOpeningMilliseconds
        finishWitnessOpeningMilliseconds := witnessOpeningMilliseconds
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
