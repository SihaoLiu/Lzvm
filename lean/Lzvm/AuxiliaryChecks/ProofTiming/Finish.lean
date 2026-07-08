/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.ProofTiming.FinishTiming

/-!
Proof artifact finish timing observation contracts.
-/

namespace Lzvm

def ProofArtifactFinishWitnessOpeningRowDedupAccounting
    (summary : ProofArtifactFinishTimingSummary) : Prop :=
  summary.finishWitnessOpeningRowDedupInputRowCount =
    summary.finishWitnessOpeningRowDedupUniqueRowCount
      + summary.finishWitnessOpeningRowDedupElidedRowCount

theorem proof_artifact_finish_witness_opening_row_dedup_accounting_update
    (summary : ProofArtifactFinishTimingSummary)
    {rowDedupInputRowCount rowDedupUniqueRowCount rowDedupElidedRowCount : Nat}
    (accounting :
      rowDedupInputRowCount =
        rowDedupUniqueRowCount + rowDedupElidedRowCount) :
    ProofArtifactFinishWitnessOpeningRowDedupAccounting
      { summary with
        finishWitnessOpeningRowDedupInputRowCount := rowDedupInputRowCount
        finishWitnessOpeningRowDedupUniqueRowCount := rowDedupUniqueRowCount
        finishWitnessOpeningRowDedupElidedRowCount := rowDedupElidedRowCount } := by
  unfold ProofArtifactFinishWitnessOpeningRowDedupAccounting
  simp [accounting]

theorem proof_artifact_finish_witness_opening_shape_acceptance_sound
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
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
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

theorem proof_artifact_finish_witness_opening_shape_acceptance_verifier_core_contract
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
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
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

theorem proof_artifact_finish_witness_opening_shape_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_core_and_sound
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

theorem proof_artifact_finish_leaf_work_shape_acceptance_sound
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
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
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
        finishWitnessOpeningLeafCosetExtendNormalizeLaunchCount := leafCosetNormalizeLaunches
        finishWitnessOpeningLeafCosetExtendPackLaunchCount := leafCosetPackLaunches
        finishWitnessOpeningLeafCosetExtendUnpackLaunchCount := leafCosetUnpackLaunches }
      publicInput
      proof
      observed

theorem proof_artifact_finish_leaf_work_shape_acceptance_verifier_core_contract
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
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
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
        finishWitnessOpeningLeafCosetExtendNormalizeLaunchCount := leafCosetNormalizeLaunches
        finishWitnessOpeningLeafCosetExtendPackLaunchCount := leafCosetPackLaunches
        finishWitnessOpeningLeafCosetExtendUnpackLaunchCount := leafCosetUnpackLaunches }
      publicInput
      proof
      observed

theorem proof_artifact_finish_leaf_work_shape_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_core_and_sound
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

theorem proof_artifact_finish_path_parent_hash_shape_acceptance_sound
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
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
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

theorem proof_artifact_finish_path_parent_hash_shape_acceptance_verifier_core_contract
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
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
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

theorem proof_artifact_finish_path_parent_hash_shape_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_core_and_sound
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

theorem proof_artifact_finish_path_parent_hash_per_unit_shape_acceptance_sound
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
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
      assumptions
      { summary with
        finishWitnessOpeningPathParentHashRowsPerQuery := rowsPerQuery
        finishWitnessOpeningPathParentHashRowsPerStage := rowsPerStage
        finishWitnessOpeningPathParentHashLaunchesPerStage := launchesPerStage }
      publicInput
      proof
      observed

theorem proof_artifact_finish_path_parent_hash_per_unit_shape_acceptance_verifier_core_contract
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
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
        finishWitnessOpeningPathParentHashRowsPerQuery := rowsPerQuery
        finishWitnessOpeningPathParentHashRowsPerStage := rowsPerStage
        finishWitnessOpeningPathParentHashLaunchesPerStage := launchesPerStage }
      publicInput
      proof
      observed

theorem proof_artifact_finish_path_parent_hash_per_unit_shape_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_core_and_sound
      assumptions
      { summary with
        finishWitnessOpeningPathParentHashRowsPerQuery := rowsPerQuery
        finishWitnessOpeningPathParentHashRowsPerStage := rowsPerStage
        finishWitnessOpeningPathParentHashLaunchesPerStage := launchesPerStage }
      publicInput
      proof
      observed

theorem proof_artifact_finish_row_values_shape_acceptance_sound
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
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
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

theorem proof_artifact_finish_row_values_shape_acceptance_verifier_core_contract
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
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
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

theorem proof_artifact_finish_row_values_shape_acceptance_core_and_sound
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
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_core_and_sound
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

end Lzvm
