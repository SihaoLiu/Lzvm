/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Soundness

/-!
Auxiliary implementation checks that are useful for diagnostics but are not
part of the abstract verifier soundness theorem.
-/

namespace Lzvm

structure AuxiliaryValidation (system : VerifierModel) where
  exactSourceLookupBalance : PublicInput -> Proof -> Prop
  dynamicSourceLookupConstrained : PublicInput -> Proof -> Prop

structure WitnessLeafDigestValidation (system : VerifierModel) where
  canonicalExtendedLeafBytes : PublicInput -> Proof -> Prop
  narrowPaddedDigestsBindRows : PublicInput -> Proof -> Prop
  wideLinearDigestsBindRows : PublicInput -> Proof -> Prop

structure GpuCanonicalLeafValidation (system : VerifierModel) where
  leafValidation : WitnessLeafDigestValidation system
  gpuCanonicalFlagClear : PublicInput -> Proof -> Prop
  flagClearImpliesCanonicalExtendedLeafBytes :
    forall publicInput proof,
      gpuCanonicalFlagClear publicInput proof ->
        leafValidation.canonicalExtendedLeafBytes publicInput proof

structure GpuLeafOutputBufferReuseValidation (system : VerifierModel) where
  leafValidation : WitnessLeafDigestValidation system
  leafOutputBufferLengthMatches : PublicInput -> Proof -> Prop
  leafOutputBufferFullyOverwritten : PublicInput -> Proof -> Prop
  leafOutputBufferReuseImpliesCanonicalLeafBytes :
    forall publicInput proof,
      leafOutputBufferLengthMatches publicInput proof ->
      leafOutputBufferFullyOverwritten publicInput proof ->
        leafValidation.canonicalExtendedLeafBytes publicInput proof

structure GpuCosetExtensionValidation (system : VerifierModel) where
  leafValidation : WitnessLeafDigestValidation system
  cosetExtensionMatchesHost : PublicInput -> Proof -> Prop
  cosetExtensionImpliesCanonicalLeafBytes :
    forall publicInput proof,
      cosetExtensionMatchesHost publicInput proof ->
        leafValidation.canonicalExtendedLeafBytes publicInput proof

structure GpuFriFoldInterpolationValidation (system : VerifierModel) where
  gpuFriInterpolationMatchesHost : PublicInput -> Proof -> Prop
  friFoldsValid : PublicInput -> Proof -> Prop
  gpuFriInterpolationImpliesFriFoldsValid :
    forall publicInput proof,
      gpuFriInterpolationMatchesHost publicInput proof ->
        friFoldsValid publicInput proof

structure GpuMerkleDigestPrefixBatchValidation (system : VerifierModel) where
  gpuMerkleDigestPrefixBatchMatchesSinglePaths : PublicInput -> Proof -> Prop
  lowerPrefixesBound : PublicInput -> Proof -> Prop
  gpuMerkleDigestPrefixBatchImpliesLowerPrefixesBound :
    forall publicInput proof,
      gpuMerkleDigestPrefixBatchMatchesSinglePaths publicInput proof ->
        lowerPrefixesBound publicInput proof

universe u

def IgnoredMetadataObservedAcceptance
    (system : VerifierModel)
    {Metadata : Type u}
    (_metadata : Metadata)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem ignored_metadata_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    {Metadata : Type u}
    (metadata : Metadata) :
    forall publicInput proof,
      IgnoredMetadataObservedAcceptance system metadata publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem ignored_metadata_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {Metadata : Type u}
    (metadata : Metadata) :
    forall publicInput proof,
      IgnoredMetadataObservedAcceptance system metadata publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      (ignored_metadata_observed_acceptance_projects_verifier_acceptance
        metadata
        publicInput
        proof
        observed)

theorem auxiliary_checked_acceptance_sound_witness
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {auxiliaryAccepted : PublicInput -> Proof -> Prop} :
    forall publicInput proof,
      system.accepts publicInput proof
        /\ auxiliaryAccepted publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact abstract_verifier_sound assumptions publicInput proof checked.left

theorem ignored_metadata_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {Metadata : Type u}
    (metadata : Metadata) :
    forall publicInput proof,
      IgnoredMetadataObservedAcceptance system metadata publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  have accepted :=
    ignored_metadata_observed_acceptance_projects_verifier_acceptance
      metadata
      publicInput
      proof
      observed
  exact
    And.intro
      (assumption_bundle_fiat_shamir_transcript_binding assumptions publicInput proof accepted)
      (And.intro
        (assumption_bundle_public_input_binding assumptions publicInput proof accepted)
        (And.intro
          (assumption_bundle_pcs_opening_soundness assumptions publicInput proof accepted)
          (assumption_bundle_fri_query_soundness assumptions publicInput proof accepted)))

theorem ignored_metadata_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {Metadata : Type u}
    (metadata : Metadata) :
    forall publicInput proof,
      IgnoredMetadataObservedAcceptance system metadata publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  have core :=
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      metadata
      publicInput
      proof
      observed
  have sound :=
    ignored_metadata_acceptance_sound
      assumptions
      metadata
      publicInput
      proof
      observed
  exact And.intro core sound

theorem auxiliary_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {auxiliaryAccepted : PublicInput -> Proof -> Prop} :
    forall publicInput proof,
      system.accepts publicInput proof
        /\ auxiliaryAccepted publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (assumption_bundle_fiat_shamir_transcript_binding assumptions publicInput proof checked.left)
      (And.intro
        (assumption_bundle_public_input_binding assumptions publicInput proof checked.left)
        (And.intro
          (assumption_bundle_pcs_opening_soundness assumptions publicInput proof checked.left)
          (assumption_bundle_fri_query_soundness assumptions publicInput proof checked.left)))

structure TimingObservation where
  label : Nat
  milliseconds : Nat
deriving DecidableEq, Repr

structure GuestPcTraceStageTimingSummary where
  stageIndex : Nat
  leafExtendWorkMilliseconds : Nat
  leafSetupWorkMilliseconds : Nat
  leafSetupPrepareMilliseconds : Nat
  leafSetupOutputAllocMilliseconds : Nat
  leafSetupWorkspaceAllocMilliseconds : Nat
  leafSetupOutputAllocByteCount : Nat
  leafSetupWorkspaceAllocByteCount : Nat
  leafSetupOutputAllocCount : Nat
  leafOutputCacheHitCount : Nat
  leafOutputCacheMissCount : Nat
  leafSetupWorkspaceAllocCount : Nat
  leafUploadWorkMilliseconds : Nat
  leafKernelWorkMilliseconds : Nat
  leafDownloadWorkMilliseconds : Nat
  leafValidateWorkMilliseconds : Nat
  leafHashWorkMilliseconds : Nat
  leafHashRowCount : Nat
  leafHashByteCount : Nat
  leafHashArity2RowCount : Nat
  leafHashArity2ByteCount : Nat
  leafHashArity4RowCount : Nat
  leafHashArity4ByteCount : Nat
  leafCosetExtendCallCount : Nat
  leafCosetExtendOutputByteCount : Nat
  leafCosetExtendColumnCount : Nat
  leafCosetExtendMaxColumnCount : Nat
  leafCosetExtendNttLaunchCount : Nat
  leafCosetExtendBitReverseLaunchCount : Nat
  leafCosetExtendNttStageLaunchCount : Nat
  leafCosetExtendNttBlockTwiddleLaunchCount : Nat
  leafCosetExtendNormalizeLaunchCount : Nat
  leafCosetExtendPackLaunchCount : Nat
  leafCosetExtendUnpackLaunchCount : Nat
  treeCommitWorkMilliseconds : Nat
  treeCommitCheckpointWorkMilliseconds : Nat
  treeCommitRootWorkMilliseconds : Nat
  treeCommitRetainWorkMilliseconds : Nat
deriving DecidableEq, Repr

set_option maxRecDepth 2048 in
structure GuestPcTraceTimingSummary where
  segmentCount : Nat
  guestTraceStreamElapsedMilliseconds : Nat
  guestTraceStreamMilliseconds : Nat
  guestTraceProofValuePrerunMilliseconds : Nat
  guestSegmentCommitMilliseconds : Nat
  guestSegmentCommitInitialWorkerCount : Nat
  guestSegmentCommitEffectiveWorkerCount : Nat
  guestSegmentCommitOomRetryCount : Nat
  guestSegmentCommitCudaMemoryTotalByteCount : Nat
  guestSegmentCommitCudaMemoryInitialFreeByteCount : Nat
  guestSegmentCommitCudaMemoryEffectiveFreeByteCount : Nat
  guestSegmentCommitCudaMemoryMinFreeByteCount : Nat
  guestSegmentCommitCudaAllocatorInitialCachedByteCount : Nat
  guestSegmentCommitCudaAllocatorEffectiveCachedByteCount : Nat
  guestTraceRunnerMilliseconds : Nat
  guestTraceLowererMilliseconds : Nat
  guestTraceLowerMilliseconds : Nat
  guestTraceReportMilliseconds : Nat
  guestTraceReportValidationMilliseconds : Nat
  guestTraceReportLoweringMilliseconds : Nat
  guestTraceReportRowValidationMilliseconds : Nat
  guestTraceReportSourceValuesMilliseconds : Nat
  guestTraceReportPrecompileMemoryMilliseconds : Nat
  guestTraceReportInstructionResultMilliseconds : Nat
  guestTraceReportNextPcMilliseconds : Nat
  guestTraceReportRegisterAccessMilliseconds : Nat
  guestTraceReportMemoryAccessMilliseconds : Nat
  guestTraceReportStoreApplyMilliseconds : Nat
  guestTraceReportVisitMilliseconds : Nat
  guestTraceSingleRowReportLowerMilliseconds : Nat
  guestTraceMultiRowReportLowerMilliseconds : Nat
  guestTracePendingDmaReportLowerMilliseconds : Nat
  guestTraceAmoReportLowerMilliseconds : Nat
  guestTraceStoreConditionalReportLowerMilliseconds : Nat
  guestTraceReportCount : Nat
  guestTraceReportRowCount : Nat
  guestTraceReportBufferCapacity : Nat
  guestTraceReportBufferMaxCapacity : Nat
  guestTraceReportBufferExcessCapacity : Nat
  guestTraceSingleRowReportCount : Nat
  guestTraceMultiRowReportCount : Nat
  guestTracePendingDmaReportCount : Nat
  guestTraceAmoReportCount : Nat
  guestTraceStoreConditionalReportCount : Nat
  guestTraceExternalOpRowCount : Nat
  guestTraceCopyRowCount : Nat
  guestTraceFlagRowCount : Nat
  guestTracePrecompileRowCount : Nat
  guestTraceIndirectMemoryRowCount : Nat
  guestTraceRegisterSourceReadCount : Nat
  guestTraceMemorySourceReadCount : Nat
  guestTraceRegisterStoreRowCount : Nat
  guestTraceMemoryStoreRowCount : Nat
  guestTraceNoStoreRowCount : Nat
  guestTraceEmitMilliseconds : Nat
  guestTraceDescriptorMilliseconds : Nat
  guestTraceDescriptorRowCount : Nat
  guestTraceDescriptorCompactRowCount : Nat
  guestTraceDescriptorWideRowCount : Nat
  guestTracePendingSendWaitMilliseconds : Nat
  guestTracePendingReceiveWaitMilliseconds : Nat
  guestTraceSegmentSendWaitMilliseconds : Nat
  guestTraceSegmentReceiveWaitMilliseconds : Nat
  guestTraceParallelLowerWorkerCount : Nat
  guestTraceParallelLowerDispatchedCount : Nat
  guestTraceParallelLowerReceivedCount : Nat
  guestTraceParallelLowerEmittedCount : Nat
  guestTraceParallelLowerMaxReorderCount : Nat
  guestTraceOwnedStreamingLowerSegmentCount : Nat
  guestTraceParallelLowerStreamStartDispatchWaitMilliseconds : Nat
  guestTraceParallelLowerStreamChunkDispatchWaitMilliseconds : Nat
  guestTraceParallelLowerStreamSegmentDispatchWaitMilliseconds : Nat
  guestTraceParallelLowerStreamFinishDispatchWaitMilliseconds : Nat
  guestDeviceSourceBuildMilliseconds : Nat
  guestDeviceSourceDescriptorUploadMilliseconds : Nat
  guestDeviceSourceDescriptorUploadByteCount : Nat
  guestDeviceSourceDescriptorUploadWordCount : Nat
  guestDeviceSourceDescriptorUploadRowCount : Nat
  guestDeviceSourceTraceExpandMilliseconds : Nat
  guestStageSourceRetentionAttemptCount : Nat
  guestStageSourceRetentionRetainedCount : Nat
  guestStageSourceRetentionRejectedCount : Nat
  guestStageSourceRetentionRetainedByteCount : Nat
  guestStageSourceRetentionRejectedByteCount : Nat
  guestStageSourceRetentionLimitByteCount : Nat
  guestDescriptorBufferRetentionAttemptCount : Nat
  guestDescriptorBufferRetentionRetainedCount : Nat
  guestDescriptorBufferRetentionRejectedCount : Nat
  guestDescriptorBufferRetentionRetainedByteCount : Nat
  guestDescriptorBufferRetentionRejectedByteCount : Nat
  guestDescriptorBufferRetentionLimitByteCount : Nat
  guestRegularConstraintsMilliseconds : Nat
  guestRegularHintsMilliseconds : Nat
  guestStageCommitMilliseconds : Nat
  guestStageTraceExtractMilliseconds : Nat
  guestStageLeafExtendWorkMilliseconds : Nat
  guestStageLeafSetupWorkMilliseconds : Nat
  guestStageLeafSetupPrepareMilliseconds : Nat
  guestStageLeafSetupOutputAllocMilliseconds : Nat
  guestStageLeafSetupWorkspaceAllocMilliseconds : Nat
  guestStageLeafSetupOutputAllocByteCount : Nat
  guestStageLeafSetupWorkspaceAllocByteCount : Nat
  guestStageLeafSetupOutputAllocCount : Nat
  guestStageLeafOutputCacheHitCount : Nat
  guestStageLeafOutputCacheMissCount : Nat
  guestStageLeafSetupWorkspaceAllocCount : Nat
  guestStageLeafUploadWorkMilliseconds : Nat
  guestStageLeafKernelWorkMilliseconds : Nat
  guestStageLeafDownloadWorkMilliseconds : Nat
  guestStageLeafValidateWorkMilliseconds : Nat
  guestStageLeafHashWorkMilliseconds : Nat
  guestStageLeafHashRowCount : Nat
  guestStageLeafHashByteCount : Nat
  guestStageLeafHashArity2RowCount : Nat
  guestStageLeafHashArity2ByteCount : Nat
  guestStageLeafHashArity4RowCount : Nat
  guestStageLeafHashArity4ByteCount : Nat
  guestStageLeafCosetExtendCallCount : Nat
  guestStageLeafCosetExtendOutputByteCount : Nat
  guestStageLeafCosetExtendColumnCount : Nat
  guestStageLeafCosetExtendMaxColumnCount : Nat
  guestStageLeafCosetExtendNttLaunchCount : Nat
  guestStageLeafCosetExtendBitReverseLaunchCount : Nat
  guestStageLeafCosetExtendNttStageLaunchCount : Nat
  guestStageLeafCosetExtendNttBlockTwiddleLaunchCount : Nat
  guestStageLeafCosetExtendNormalizeLaunchCount : Nat
  guestStageLeafCosetExtendPackLaunchCount : Nat
  guestStageLeafCosetExtendUnpackLaunchCount : Nat
  guestStageTreeCommitWorkMilliseconds : Nat
  guestStageTreeCommitCheckpointWorkMilliseconds : Nat
  guestStageTreeCommitRootWorkMilliseconds : Nat
  guestStageTreeCommitRootCount : Nat
  guestStageTreeCommitRootByteCount : Nat
  guestStageTreeCommitRootMaterializationGroupCount : Nat
  guestStageTreeCommitRootMaterializationMaxGroupSize : Nat
  guestStageTreeCommitRetainWorkMilliseconds : Nat
  stageTimings : List GuestPcTraceStageTimingSummary
deriving DecidableEq, Repr

structure WitnessOpeningStageRowValueTimingSummary where
  stageIndex : Nat
  sourceExtendMilliseconds : Nat
  sourceDownloadMilliseconds : Nat
  deviceDownloadMilliseconds : Nat
  deviceRowCount : Nat
  deviceDownloadBatchCount : Nat
  deviceSingleDownloadCount : Nat
  sourceRowCount : Nat
  wordCount : Nat
  byteCount : Nat
deriving DecidableEq, Repr

structure WitnessOpeningRowValueTimingSummary where
  rowValueSourceExtendMilliseconds : Nat
  rowValueSourceDownloadMilliseconds : Nat
  rowValueDeviceDownloadMilliseconds : Nat
  deviceRowCount : Nat
  deviceDownloadBatchCount : Nat
  deviceSingleDownloadCount : Nat
  sourceRowCount : Nat
  wordCount : Nat
  byteCount : Nat
  stages : List WitnessOpeningStageRowValueTimingSummary
deriving DecidableEq, Repr

structure ConstantMaterialValidationTimingSummary where
  constantMaterialValidationElapsedMilliseconds : Nat
  constantMaterialValidationJoinWaitMilliseconds : Nat
  constantMaterialValidationUnitCount : Nat
  constantMaterialValidationByteCount : Nat
deriving DecidableEq, Repr

structure ProverGpuModeSummary where
  proverGpuModeName : Nat
deriving DecidableEq, Repr

structure GpuRunOptionsSummary where
  gpuPreallocateRequested : Bool
  gpuStreamLimit : Nat
  witnessThreadPoolCount : Nat
  storedWitnessLimit : Nat
  packTraceEnabled : Bool
deriving DecidableEq, Repr

structure CudaBackendSummary where
  cudaBackendEnabled : Bool
deriving DecidableEq, Repr

structure CudaAllocatorTimingSummary where
  cudaAllocatorMallocCallCount : Nat
  cudaAllocatorMallocByteCount : Nat
  cudaAllocatorMallocWaitNanoseconds : Nat
  cudaAllocatorMallocMaxWaitNanoseconds : Nat
  cudaAllocatorHostRegisterCallCount : Nat
  cudaAllocatorHostRegisterByteCount : Nat
  cudaAllocatorHostRegisterWaitNanoseconds : Nat
  cudaAllocatorHostRegisterMaxWaitNanoseconds : Nat
  cudaAllocatorHostUnregisterCallCount : Nat
  cudaAllocatorHostUnregisterWaitNanoseconds : Nat
  cudaAllocatorHostUnregisterMaxWaitNanoseconds : Nat
  cudaAllocatorCopyH2DCallCount : Nat
  cudaAllocatorCopyH2DByteCount : Nat
  cudaAllocatorCopyH2DWaitNanoseconds : Nat
  cudaAllocatorCopyH2DMaxWaitNanoseconds : Nat
  cudaAllocatorCopyH2DHotByteCount : Nat
  cudaAllocatorCopyH2DHotCount : Nat
  cudaAllocatorCopyH2DHotWaitNanoseconds : Nat
  cudaAllocatorCopyH2DSecondHotByteCount : Nat
  cudaAllocatorCopyH2DSecondHotCount : Nat
  cudaAllocatorCopyH2DSecondHotWaitNanoseconds : Nat
  cudaAllocatorCopyD2HCallCount : Nat
  cudaAllocatorCopyD2HByteCount : Nat
  cudaAllocatorCopyD2HWaitNanoseconds : Nat
  cudaAllocatorCopyD2HMaxWaitNanoseconds : Nat
  cudaAllocatorCopyD2DCallCount : Nat
  cudaAllocatorCopyD2DByteCount : Nat
  cudaAllocatorCopyD2DWaitNanoseconds : Nat
  cudaAllocatorCopyD2DMaxWaitNanoseconds : Nat
  cudaAllocatorDeviceSynchronizeCallCount : Nat
  cudaAllocatorDeviceSynchronizeWaitNanoseconds : Nat
  cudaAllocatorDeviceSynchronizeMaxWaitNanoseconds : Nat
  cudaAllocatorCachedBlockCount : Nat
  cudaAllocatorCachedByteCount : Nat
  cudaAllocatorEventQueryCallCount : Nat
  cudaAllocatorEventQueryReadyCount : Nat
  cudaAllocatorEventQueryNotReadyCount : Nat
  cudaAllocatorEventSynchronizeCallCount : Nat
  cudaAllocatorEventSynchronizeByteCount : Nat
  cudaAllocatorEventSynchronizeMaxByteCount : Nat
  cudaAllocatorEventSynchronizeWaitNanoseconds : Nat
  cudaAllocatorEventSynchronizeMaxWaitNanoseconds : Nat
  cudaAllocatorEventSynchronizeHotByteCount : Nat
  cudaAllocatorEventSynchronizeHotCount : Nat
  cudaAllocatorEventSynchronizeHotWaitNanoseconds : Nat
  cudaAllocatorCachedReuseCount : Nat
  cudaAllocatorPendingReuseCount : Nat
  cudaAllocatorNoWaitBypassCount : Nat
  cudaAllocatorNoWaitBypassByteCount : Nat
deriving DecidableEq, Repr

structure ProofArtifactFinishTimingSummary where
  finishQueryPlanMilliseconds : Nat
  finishConstantOpeningMilliseconds : Nat
  finishWitnessOpeningMilliseconds : Nat
  finishWitnessOpeningQueryCount : Nat
  finishWitnessOpeningQueryUnitCount : Nat
  finishWitnessOpeningSingleQueryUnitCount : Nat
  finishWitnessOpeningMaxQueriesPerUnit : Nat
  finishWitnessOpeningStageCount : Nat
  finishWitnessOpeningRetainedSourceCount : Nat
  finishWitnessOpeningExternalSourceCount : Nat
  finishWitnessOpeningEmbeddedSourceCount : Nat
  finishWitnessOpeningMissingSourceCount : Nat
  finishWitnessOpeningRetainedLeafDigestOpeningCount : Nat
  finishWitnessOpeningRetainedLeafDigestOpeningRowCount : Nat
  finishWitnessOpeningRetainedParentCheckpointOpeningCount : Nat
  finishWitnessOpeningRetainedParentCheckpointOpeningRowCount : Nat
  finishWitnessOpeningRowDedupInputRowCount : Nat
  finishWitnessOpeningRowDedupUniqueRowCount : Nat
  finishWitnessOpeningRowDedupElidedRowCount : Nat
  finishWitnessExternalSourceMilliseconds : Nat
  finishWitnessExternalSourceDescriptorUploadMilliseconds : Nat
  finishWitnessExternalSourceDescriptorUploadByteCount : Nat
  finishWitnessExternalSourceDescriptorUploadWordCount : Nat
  finishWitnessExternalSourceDescriptorUploadRowCount : Nat
  finishWitnessExternalSourceTraceExpandMilliseconds : Nat
  finishWitnessOpeningSetupMilliseconds : Nat
  finishWitnessOpeningLeafExtendMilliseconds : Nat
  finishWitnessOpeningLeafHashMilliseconds : Nat
  finishWitnessOpeningLeafHashRowCount : Nat
  finishWitnessOpeningLeafHashByteCount : Nat
  finishWitnessOpeningLeafHashArity2RowCount : Nat
  finishWitnessOpeningLeafHashArity2ByteCount : Nat
  finishWitnessOpeningLeafHashArity4RowCount : Nat
  finishWitnessOpeningLeafHashArity4ByteCount : Nat
  finishWitnessOpeningLeafCosetExtendCallCount : Nat
  finishWitnessOpeningLeafCosetExtendOutputByteCount : Nat
  finishWitnessOpeningLeafCosetExtendColumnCount : Nat
  finishWitnessOpeningLeafCosetExtendMaxColumnCount : Nat
  finishWitnessOpeningLeafCosetExtendNttLaunchCount : Nat
  finishWitnessOpeningLeafCosetExtendBitReverseLaunchCount : Nat
  finishWitnessOpeningLeafCosetExtendNttStageLaunchCount : Nat
  finishWitnessOpeningLeafCosetExtendNttBlockTwiddleLaunchCount : Nat
  finishWitnessOpeningLeafCosetExtendNormalizeLaunchCount : Nat
  finishWitnessOpeningLeafCosetExtendPackLaunchCount : Nat
  finishWitnessOpeningLeafCosetExtendUnpackLaunchCount : Nat
  finishWitnessOpeningPathParentHashRowCount : Nat
  finishWitnessOpeningPathParentHashByteCount : Nat
  finishWitnessOpeningPathParentHashLaunchCount : Nat
  finishWitnessOpeningPathParentHashRecomputedRowCount : Nat
  finishWitnessOpeningPathParentHashRecomputedByteCount : Nat
  finishWitnessOpeningPathParentHashRecomputedLaunchCount : Nat
  finishWitnessOpeningPathParentHashRetainedLeafDigestRowCount : Nat
  finishWitnessOpeningPathParentHashRetainedLeafDigestByteCount : Nat
  finishWitnessOpeningPathParentHashRetainedLeafDigestLaunchCount : Nat
  finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixRowCount : Nat
  finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixByteCount : Nat
  finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixLaunchCount : Nat
  finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixRowCount : Nat
  finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixByteCount : Nat
  finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixLaunchCount : Nat
  finishWitnessOpeningPathParentHashRowsPerQuery : Nat
  finishWitnessOpeningPathParentHashRowsPerStage : Nat
  finishWitnessOpeningPathParentHashLaunchesPerStage : Nat
  finishWitnessOpeningRowValuesMilliseconds : Nat
  finishWitnessOpeningRowValueSourceExtendMilliseconds : Nat
  finishWitnessOpeningRowValueSourceDownloadMilliseconds : Nat
  finishWitnessOpeningRowValueDeviceDownloadMilliseconds : Nat
  finishWitnessOpeningRowValuesDeviceRowCount : Nat
  finishWitnessOpeningRowValuesDeviceDownloadBatchCount : Nat
  finishWitnessOpeningRowValuesDeviceSingleDownloadCount : Nat
  finishWitnessOpeningRowValuesSourceRowCount : Nat
  finishWitnessOpeningRowValuesWordCount : Nat
  finishWitnessOpeningRowValuesByteCount : Nat
  finishWitnessOpeningPathMilliseconds : Nat
  finishFriOpeningMilliseconds : Nat
  finishFriOpeningUnitBuildMilliseconds : Nat
  finishFriOpeningLayerTreeMilliseconds : Nat
  finishFriOpeningQueryMilliseconds : Nat
  finishFriOpeningFoldMilliseconds : Nat
  finishFriOpeningUnitCount : Nat
  finishFriOpeningLayerCount : Nat
  finishFriOpeningQueryCount : Nat
  finishFriTranscriptUnitBuildMilliseconds : Nat
  finishFriTranscriptLayerTreeMilliseconds : Nat
  finishFriTranscriptFoldMilliseconds : Nat
  finishFriTranscriptUnitCount : Nat
  finishFriTranscriptLayerCount : Nat
  finishProofEncodeMilliseconds : Nat
  finishContributionSegmentMilliseconds : Nat
  finishContributionVerifyMilliseconds : Nat
  finishContributionChallengeMilliseconds : Nat
deriving DecidableEq, Repr

structure ProofTimingBatchSummary where
  smallRunCount : Nat
  largeRunCount : Nat
  smallStableRunCount : Nat
  largeStableRunCount : Nat
  smallStableAverageMilliseconds : Nat
  largeStableAverageMilliseconds : Nat
  smallStableSpreadMilliseconds : Nat
  largeStableSpreadMilliseconds : Nat
  smallTimingParseFailedCount : Nat
  largeTimingParseFailedCount : Nat
deriving DecidableEq, Repr

structure RuntimePerformanceObservationSummary where
  timingObservations : List TimingObservation
  guestPcTraceTiming : Option GuestPcTraceTimingSummary
  witnessOpeningRowValueTiming : Option WitnessOpeningRowValueTimingSummary
  constantMaterialValidationTiming : Option ConstantMaterialValidationTimingSummary
  proverGpuMode : Option ProverGpuModeSummary
  gpuRunOptions : Option GpuRunOptionsSummary
  cudaBackend : Option CudaBackendSummary
  cudaAllocatorTiming : Option CudaAllocatorTimingSummary
  proofArtifactFinishTiming : Option ProofArtifactFinishTimingSummary
  proofTimingBatch : Option ProofTimingBatchSummary
deriving DecidableEq, Repr

structure GpuSetupCacheState where
  device : Nat
  initializedBits : Nat
deriving DecidableEq, Repr

structure GpuSetupRequest where
  device : Nat
  requiredBits : Nat
deriving DecidableEq, Repr

def GpuSetupCacheCovers
    (state : GpuSetupCacheState)
    (request : GpuSetupRequest) : Prop :=
  request.device = state.device /\ request.requiredBits <= state.initializedBits

structure GpuSetupCacheValidation where
  constantsSoundFor : Nat -> Nat -> Prop
  coveredConstantsSound :
    forall device initializedBits requiredBits,
      requiredBits <= initializedBits ->
        constantsSoundFor device initializedBits ->
          constantsSoundFor device requiredBits

def GpuSetupCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuSetupCacheValidation)
    (request : GpuSetupRequest)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.constantsSoundFor request.device request.requiredBits

structure GpuAllocationSource where
  device : Nat
  byteCount : Nat
  fromCache : Bool
deriving DecidableEq, Repr

def GpuAllocationSameRequest
    (cached fresh : GpuAllocationSource) : Prop :=
  cached.device = fresh.device /\ cached.byteCount = fresh.byteCount

structure GpuAllocationCacheValidation where
  writtenContentsBound : GpuAllocationSource -> PublicInput -> Proof -> Prop
  cachedReusePreservesWrittenContents :
    forall cached fresh publicInput proof,
      GpuAllocationSameRequest cached fresh ->
        writtenContentsBound fresh publicInput proof ->
          writtenContentsBound cached publicInput proof

structure GpuTemporaryBufferReuseValidation where
  temporaryBufferReuseAllowed :
    GpuAllocationSource -> GpuAllocationSource -> PublicInput -> Proof -> Prop
  pendingDeviceReadsComplete : GpuAllocationSource -> PublicInput -> Proof -> Prop
  temporaryBufferReuseImpliesSameRequest :
    forall previous next publicInput proof,
      temporaryBufferReuseAllowed previous next publicInput proof ->
        GpuAllocationSameRequest previous next
  temporaryBufferReuseImpliesPendingReadsComplete :
    forall previous next publicInput proof,
      temporaryBufferReuseAllowed previous next publicInput proof ->
        pendingDeviceReadsComplete previous publicInput proof

structure GpuAllocatorNoWaitBypassValidation where
  noWaitBypassAllowed :
    GpuAllocationSource -> GpuAllocationSource -> PublicInput -> Proof -> Prop
  pendingAllocationNotReused : GpuAllocationSource -> PublicInput -> Proof -> Prop
  freshAllocationIssued : GpuAllocationSource -> PublicInput -> Proof -> Prop
  noWaitBypassImpliesSameRequest :
    forall pending fresh publicInput proof,
      noWaitBypassAllowed pending fresh publicInput proof ->
        GpuAllocationSameRequest pending fresh
  noWaitBypassImpliesPendingNotReused :
    forall pending fresh publicInput proof,
      noWaitBypassAllowed pending fresh publicInput proof ->
        pendingAllocationNotReused pending publicInput proof
  noWaitBypassImpliesFreshAllocation :
    forall pending fresh publicInput proof,
      noWaitBypassAllowed pending fresh publicInput proof ->
        freshAllocationIssued fresh publicInput proof

structure GpuAllocatorNoWaitLimitConfig where
  pendingNoWaitLimitBytes : Nat
  pendingAllocationBytes : Nat
  freshAllocationBytes : Nat
  bypassSelected : Bool
deriving DecidableEq, Repr

def GpuAllocatorNoWaitLimitDecisionMatches
    (config : GpuAllocatorNoWaitLimitConfig) : Prop :=
  config.freshAllocationBytes = config.pendingAllocationBytes
    /\ config.bypassSelected =
      decide (config.pendingAllocationBytes <= config.pendingNoWaitLimitBytes)

structure GpuAllocatorNoWaitLimitValidation where
  noWaitLimitConfigAccepted :
    GpuAllocatorNoWaitLimitConfig -> PublicInput -> Proof -> Prop
  noWaitLimitConfigImpliesDecisionMatches :
    forall config publicInput proof,
      noWaitLimitConfigAccepted config publicInput proof ->
        GpuAllocatorNoWaitLimitDecisionMatches config

structure GuestPcTraceSegmentQueueConfig where
  defaultSegmentQueueCapacity : Nat
  configuredSegmentQueueCapacity : Option Nat
  effectiveSegmentQueueCapacity : Nat
deriving DecidableEq, Repr

def GuestPcTraceSegmentQueueDecisionMatches
    (config : GuestPcTraceSegmentQueueConfig) : Prop :=
  match config.configuredSegmentQueueCapacity with
  | some configured =>
      config.effectiveSegmentQueueCapacity = configured
  | none =>
      config.effectiveSegmentQueueCapacity =
        config.defaultSegmentQueueCapacity

structure GuestPcTraceSegmentQueueValidation where
  segmentQueueConfigAccepted :
    GuestPcTraceSegmentQueueConfig -> PublicInput -> Proof -> Prop
  segmentQueueConfigImpliesDecisionMatches :
    forall config publicInput proof,
      segmentQueueConfigAccepted config publicInput proof ->
        GuestPcTraceSegmentQueueDecisionMatches config

structure GuestPcTraceLargeGpuGateConfig where
  defaultLargeTraceInstructionThreshold : Nat
  defaultMinFreeGpuMemoryBytes : Nat
  requestedInstructionLimit : Option Nat
  observedFreeGpuMemoryBytes : Option Nat
  gpuBackendAvailable : Bool
  largeTraceAllowed : Bool
deriving DecidableEq, Repr

def GuestPcTraceLargeGpuGateInstructionThreshold : Nat := 1000000

def GuestPcTraceLargeGpuGateMinFreeGpuMemoryBytes : Nat := 1024 * 1024 * 1024

def GuestPcTraceLargeGpuGateMemoryCheckPasses
    (config : GuestPcTraceLargeGpuGateConfig) : Bool :=
  match config.observedFreeGpuMemoryBytes with
  | some freeBytes =>
      decide (config.defaultMinFreeGpuMemoryBytes <= freeBytes)
  | none => false

def GuestPcTraceLargeGpuGateDecisionMatches
    (config : GuestPcTraceLargeGpuGateConfig) : Prop :=
  config.defaultLargeTraceInstructionThreshold =
    GuestPcTraceLargeGpuGateInstructionThreshold
    /\ config.defaultMinFreeGpuMemoryBytes =
      GuestPcTraceLargeGpuGateMinFreeGpuMemoryBytes
    /\ match config.requestedInstructionLimit with
      | some limit =>
          config.largeTraceAllowed =
            decide (limit < config.defaultLargeTraceInstructionThreshold
              \/ (config.gpuBackendAvailable = true
                /\ GuestPcTraceLargeGpuGateMemoryCheckPasses config = true))
      | none =>
          config.largeTraceAllowed = true

structure GuestPcTraceLargeGpuGateValidation where
  largeGpuGateConfigAccepted :
    GuestPcTraceLargeGpuGateConfig -> PublicInput -> Proof -> Prop
  largeGpuGateConfigImpliesDecisionMatches :
    forall config publicInput proof,
      largeGpuGateConfigAccepted config publicInput proof ->
        GuestPcTraceLargeGpuGateDecisionMatches config

structure GuestPcTraceTracelessCommitmentInputConfig where
  configuredTracelessCommitmentInput : Option Bool
  effectiveTracelessCommitmentInput : Bool
deriving DecidableEq, Repr

def GuestPcTraceTracelessCommitmentInputDecisionMatches
    (config : GuestPcTraceTracelessCommitmentInputConfig) : Prop :=
  match config.configuredTracelessCommitmentInput with
  | some configured =>
      config.effectiveTracelessCommitmentInput = configured
  | none =>
      config.effectiveTracelessCommitmentInput = true

structure GuestPcTraceTracelessCommitmentInputValidation where
  tracelessCommitmentInputConfigAccepted :
    GuestPcTraceTracelessCommitmentInputConfig -> PublicInput -> Proof -> Prop
  tracelessCommitmentInputConfigImpliesDecisionMatches :
    forall config publicInput proof,
      tracelessCommitmentInputConfigAccepted config publicInput proof ->
        GuestPcTraceTracelessCommitmentInputDecisionMatches config

structure GuestPcTraceTracelessSegmentOutputConfig where
  configuredTracelessSegmentOutput : Option Bool
  effectiveTracelessSegmentOutput : Bool
deriving DecidableEq, Repr

def GuestPcTraceTracelessSegmentOutputDecisionMatches
    (config : GuestPcTraceTracelessSegmentOutputConfig) : Prop :=
  match config.configuredTracelessSegmentOutput with
  | some configured =>
      config.effectiveTracelessSegmentOutput = configured
  | none =>
      config.effectiveTracelessSegmentOutput = true

structure GuestPcTraceTracelessSegmentOutputValidation where
  tracelessSegmentOutputConfigAccepted :
    GuestPcTraceTracelessSegmentOutputConfig -> PublicInput -> Proof -> Prop
  tracelessSegmentOutputConfigImpliesDecisionMatches :
    forall config publicInput proof,
      tracelessSegmentOutputConfigAccepted config publicInput proof ->
        GuestPcTraceTracelessSegmentOutputDecisionMatches config

structure GuestPcTraceCrossSegmentRootMaterializationConfig where
  configuredCrossSegmentRootMaterialization : Option Bool
  effectiveCrossSegmentRootMaterialization : Bool
  inputByteCount : Nat
  supportedInputByteLimit : Nat
deriving DecidableEq, Repr

def GuestPcTraceCrossSegmentRootMaterializationDecisionMatches
    (config : GuestPcTraceCrossSegmentRootMaterializationConfig) : Prop :=
  config.supportedInputByteLimit = 8 * 1024 * 1024
    /\ match config.configuredCrossSegmentRootMaterialization with
      | some configured =>
          config.effectiveCrossSegmentRootMaterialization =
            decide (configured = true
              /\ config.inputByteCount < config.supportedInputByteLimit)
      | none =>
          config.effectiveCrossSegmentRootMaterialization =
            decide (config.inputByteCount < config.supportedInputByteLimit)

structure GuestPcTraceCrossSegmentRootMaterializationValidation where
  crossSegmentRootMaterializationConfigAccepted :
    GuestPcTraceCrossSegmentRootMaterializationConfig -> PublicInput -> Proof -> Prop
  crossSegmentRootMaterializationConfigImpliesDecisionMatches :
    forall config publicInput proof,
      crossSegmentRootMaterializationConfigAccepted config publicInput proof ->
        GuestPcTraceCrossSegmentRootMaterializationDecisionMatches config

structure GuestPcTraceDescriptorBufferRetentionConfig where
  configuredDescriptorBufferRetention : Option Bool
  parallelLowerEnabledForDescriptorRetention : Bool
  inputByteCount : Nat
  supportedInputByteLimit : Nat
  effectiveDescriptorBufferRetention : Bool
deriving DecidableEq, Repr

def GuestPcTraceDescriptorBufferRetentionDecisionMatches
    (config : GuestPcTraceDescriptorBufferRetentionConfig) : Prop :=
  0 < config.supportedInputByteLimit
    /\ match config.configuredDescriptorBufferRetention with
      | some configured =>
          config.effectiveDescriptorBufferRetention = configured
      | none =>
          config.effectiveDescriptorBufferRetention =
            decide (config.parallelLowerEnabledForDescriptorRetention = false
              /\ config.inputByteCount < config.supportedInputByteLimit)

structure GuestPcTraceSegmentCommitModeConfig where
  defaultWorkerCount : Nat
  configuredWorkerCount : Option Nat
  effectiveWorkerCount : Nat
  configuredAsyncSingleWorker : Bool
  effectiveAsyncSingleWorker : Bool
  tracelessCommitmentInputConfig :
    GuestPcTraceTracelessCommitmentInputConfig
  selectedTracelessCommitmentInput : Bool
  crossSegmentRootMaterializationConfig :
    GuestPcTraceCrossSegmentRootMaterializationConfig
  selectedCrossSegmentRootMaterialization : Bool
  descriptorBufferRetentionConfig :
    GuestPcTraceDescriptorBufferRetentionConfig
  selectedDescriptorBufferRetention : Bool
  defaultPendingRootMaterializationWindow : Nat
  configuredPendingRootMaterializationWindow : Option Nat
  effectivePendingRootMaterializationWindow : Nat
deriving DecidableEq, Repr

def GuestPcTraceSegmentCommitModeDecisionMatches
    (config : GuestPcTraceSegmentCommitModeConfig) : Prop :=
  (match config.configuredWorkerCount with
    | some configured =>
        0 < configured /\ config.effectiveWorkerCount = configured
    | none =>
        config.effectiveWorkerCount = config.defaultWorkerCount)
    /\ 0 < config.effectiveWorkerCount
    /\ config.effectiveAsyncSingleWorker =
      decide (config.effectiveWorkerCount = 1
        /\ config.configuredAsyncSingleWorker = true)
    /\ GuestPcTraceTracelessCommitmentInputDecisionMatches
      config.tracelessCommitmentInputConfig
    /\ config.selectedTracelessCommitmentInput =
      config.tracelessCommitmentInputConfig.effectiveTracelessCommitmentInput
    /\ GuestPcTraceCrossSegmentRootMaterializationDecisionMatches
      config.crossSegmentRootMaterializationConfig
    /\ config.selectedCrossSegmentRootMaterialization =
      config.crossSegmentRootMaterializationConfig.effectiveCrossSegmentRootMaterialization
    /\ GuestPcTraceDescriptorBufferRetentionDecisionMatches
      config.descriptorBufferRetentionConfig
    /\ config.selectedDescriptorBufferRetention =
      config.descriptorBufferRetentionConfig.effectiveDescriptorBufferRetention
    /\ 0 < config.defaultPendingRootMaterializationWindow
    /\ match config.configuredPendingRootMaterializationWindow with
      | some configured =>
          0 < configured
            /\ config.effectivePendingRootMaterializationWindow =
              if config.selectedCrossSegmentRootMaterialization then
                configured
              else
                1
      | none =>
          config.effectivePendingRootMaterializationWindow =
            if config.selectedCrossSegmentRootMaterialization then
              config.defaultPendingRootMaterializationWindow
            else
              1

structure GuestPcTraceSegmentCommitModeValidation where
  segmentCommitModeConfigAccepted :
    GuestPcTraceSegmentCommitModeConfig -> PublicInput -> Proof -> Prop
  segmentCommitModeConfigImpliesDecisionMatches :
    forall config publicInput proof,
      segmentCommitModeConfigAccepted config publicInput proof ->
        GuestPcTraceSegmentCommitModeDecisionMatches config

structure GuestPcTraceDeviceTraceSourceConfig where
  configuredDeviceTraceSourceEnabled : Option Bool
  effectiveDeviceTraceSourceEnabled : Bool
  configuredDeviceTraceSourceDeepValidation : Option Bool
  effectiveDeviceTraceSourceDeepValidation : Bool
deriving DecidableEq, Repr

def GuestPcTraceDeviceTraceSourceDecisionMatches
    (config : GuestPcTraceDeviceTraceSourceConfig) : Prop :=
  (match config.configuredDeviceTraceSourceEnabled with
    | some configured =>
        config.effectiveDeviceTraceSourceEnabled = configured
    | none =>
        config.effectiveDeviceTraceSourceEnabled = true)
    /\ match config.configuredDeviceTraceSourceDeepValidation with
      | some configured =>
          config.effectiveDeviceTraceSourceDeepValidation = configured
      | none =>
          config.effectiveDeviceTraceSourceDeepValidation = false

structure GuestPcTraceDeviceTraceSourceValidation where
  deviceTraceSourceConfigAccepted :
    GuestPcTraceDeviceTraceSourceConfig -> PublicInput -> Proof -> Prop
  deviceTraceSourceConfigImpliesDecisionMatches :
    forall config publicInput proof,
      deviceTraceSourceConfigAccepted config publicInput proof ->
        GuestPcTraceDeviceTraceSourceDecisionMatches config

structure GuestPcTraceSparseSourceConfig where
  configuredSparseSourceEnabled : Option Bool
  effectiveSparseSourceSelected : Bool
  defaultSparseSourceMaxPercent : Nat
  configuredSparseSourceMaxPercent : Option Nat
  effectiveSparseSourceMaxPercent : Nat
  traceWordCount : Nat
  nonzeroWordCount : Nat
  maxNonzeroWordCount : Nat
deriving DecidableEq, Repr

def GuestPcTraceSparseSourceMaxPercentMatches
    (config : GuestPcTraceSparseSourceConfig) : Prop :=
  config.defaultSparseSourceMaxPercent = 45
    /\ match config.configuredSparseSourceMaxPercent with
      | some percent =>
          1 <= percent
            /\ percent < 50
            /\ config.effectiveSparseSourceMaxPercent = percent
      | none =>
          config.effectiveSparseSourceMaxPercent =
            config.defaultSparseSourceMaxPercent

def GuestPcTraceSparseSourceWordLimitMatches
    (config : GuestPcTraceSparseSourceConfig) : Prop :=
  config.maxNonzeroWordCount =
    config.traceWordCount * config.effectiveSparseSourceMaxPercent / 100

def GuestPcTraceSparseSourceDecisionMatches
    (config : GuestPcTraceSparseSourceConfig) : Prop :=
  GuestPcTraceSparseSourceMaxPercentMatches config
    /\ GuestPcTraceSparseSourceWordLimitMatches config
    /\ match config.configuredSparseSourceEnabled with
      | some enabled =>
          config.effectiveSparseSourceSelected =
            (enabled
              && decide
                (config.nonzeroWordCount <=
                  config.maxNonzeroWordCount))
      | none =>
          config.effectiveSparseSourceSelected = false

structure GuestPcTraceSparseSourceValidation where
  sparseSourceConfigAccepted :
    GuestPcTraceSparseSourceConfig -> PublicInput -> Proof -> Prop
  sparseSourceConfigImpliesDecisionMatches :
    forall config publicInput proof,
      sparseSourceConfigAccepted config publicInput proof ->
        GuestPcTraceSparseSourceDecisionMatches config

structure GuestPcTraceSparseSourceDebugConfig where
  configuredSparseSourceDebug : Option Bool
  effectiveSparseSourceDebug : Bool
deriving DecidableEq, Repr

def GuestPcTraceSparseSourceDebugDecisionMatches
    (config : GuestPcTraceSparseSourceDebugConfig) : Prop :=
  match config.configuredSparseSourceDebug with
  | some configured =>
      config.effectiveSparseSourceDebug = configured
  | none =>
      config.effectiveSparseSourceDebug = false

structure GuestPcTraceTerminalSparseSourceConfig where
  configuredTerminalSparseSourceEnabled : Option Bool
  effectiveTerminalSparseSourceSelected : Bool
  terminalTraceSourcePrefixRows : Option Nat
  terminalTraceLayoutRows : Nat
deriving DecidableEq, Repr

def GuestPcTraceTerminalSparseSourceDecisionMatches
    (config : GuestPcTraceTerminalSparseSourceConfig) : Prop :=
  match config.configuredTerminalSparseSourceEnabled with
  | some enabled =>
      config.effectiveTerminalSparseSourceSelected =
        (enabled
          && match config.terminalTraceSourcePrefixRows with
            | some prefixRows =>
                decide (prefixRows < config.terminalTraceLayoutRows)
            | none =>
                false)
  | none =>
      config.effectiveTerminalSparseSourceSelected = false

structure GuestPcTraceTerminalSparseSourceValidation where
  terminalSparseSourceConfigAccepted :
    GuestPcTraceTerminalSparseSourceConfig -> PublicInput -> Proof -> Prop
  terminalSparseSourceConfigImpliesDecisionMatches :
    forall config publicInput proof,
      terminalSparseSourceConfigAccepted config publicInput proof ->
        GuestPcTraceTerminalSparseSourceDecisionMatches config

structure FriRetainedStageSourceConfig where
  configuredRetainedStageSourceEnabled : Option Bool
  effectiveRetainedStageSourceEnabled : Bool
deriving DecidableEq, Repr

def FriRetainedStageSourceDecisionMatches
    (config : FriRetainedStageSourceConfig) : Prop :=
  match config.configuredRetainedStageSourceEnabled with
  | some configured =>
      config.effectiveRetainedStageSourceEnabled = configured
  | none =>
      config.effectiveRetainedStageSourceEnabled = true

structure FriRetainedStageSourceValidation where
  retainedStageSourceConfigAccepted :
    FriRetainedStageSourceConfig -> PublicInput -> Proof -> Prop
  retainedStageSourceConfigImpliesDecisionMatches :
    forall config publicInput proof,
      retainedStageSourceConfigAccepted config publicInput proof ->
        FriRetainedStageSourceDecisionMatches config

structure FriRetainedStageSourceDebugConfig where
  configuredRetainedStageSourceDebug : Option Bool
  selectedRetainedStageSource : Bool
  effectiveRetainedStageSourceDebug : Bool
deriving DecidableEq, Repr

def FriRetainedStageSourceDebugDecisionMatches
    (config : FriRetainedStageSourceDebugConfig) : Prop :=
  match config.configuredRetainedStageSourceDebug with
  | some configured =>
      config.effectiveRetainedStageSourceDebug =
        (config.selectedRetainedStageSource && configured)
  | none =>
      config.effectiveRetainedStageSourceDebug = false

structure GuestPcTraceCudaRunConfig where
  sparseSourceConfig : GuestPcTraceSparseSourceConfig
  selectedSparseSource : Bool
  sparseSourceDebugConfig : GuestPcTraceSparseSourceDebugConfig
  selectedSparseSourceDebug : Bool
  terminalSparseSourceConfig : GuestPcTraceTerminalSparseSourceConfig
  selectedTerminalSparseSource : Bool
  retainedStageSourceConfig : FriRetainedStageSourceConfig
  selectedRetainedStageSource : Bool
  retainedStageSourceDebugConfig : FriRetainedStageSourceDebugConfig
  selectedRetainedStageSourceDebug : Bool
  descriptorBufferRetentionConfig :
    GuestPcTraceDescriptorBufferRetentionConfig
  selectedDescriptorBufferRetention : Bool
deriving DecidableEq, Repr

structure GuestPcTraceCudaRunDecisionEvidence
    (config : GuestPcTraceCudaRunConfig) : Prop where
  sparseSourceDecision :
    GuestPcTraceSparseSourceDecisionMatches config.sparseSourceConfig
  sparseSourceSelected :
    config.selectedSparseSource =
      config.sparseSourceConfig.effectiveSparseSourceSelected
  sparseSourceDebugDecision :
    GuestPcTraceSparseSourceDebugDecisionMatches config.sparseSourceDebugConfig
  sparseSourceDebugSelected :
    config.selectedSparseSourceDebug =
      config.sparseSourceDebugConfig.effectiveSparseSourceDebug
  terminalSparseSourceDecision :
    GuestPcTraceTerminalSparseSourceDecisionMatches
      config.terminalSparseSourceConfig
  terminalSparseSourceSelected :
    config.selectedTerminalSparseSource =
      config.terminalSparseSourceConfig.effectiveTerminalSparseSourceSelected
  retainedStageSourceDecision :
    FriRetainedStageSourceDecisionMatches config.retainedStageSourceConfig
  retainedStageSourceSelected :
    config.selectedRetainedStageSource =
      config.retainedStageSourceConfig.effectiveRetainedStageSourceEnabled
  retainedStageSourceDebugUsesSelectedSource :
    config.retainedStageSourceDebugConfig.selectedRetainedStageSource =
      config.selectedRetainedStageSource
  retainedStageSourceDebugDecision :
    FriRetainedStageSourceDebugDecisionMatches
      config.retainedStageSourceDebugConfig
  retainedStageSourceDebugSelected :
    config.selectedRetainedStageSourceDebug =
      config.retainedStageSourceDebugConfig.effectiveRetainedStageSourceDebug
  descriptorBufferRetentionDecision :
    GuestPcTraceDescriptorBufferRetentionDecisionMatches
      config.descriptorBufferRetentionConfig
  descriptorBufferRetentionSelected :
    config.selectedDescriptorBufferRetention =
      config.descriptorBufferRetentionConfig.effectiveDescriptorBufferRetention

abbrev GuestPcTraceCudaRunDecisionMatches
    (config : GuestPcTraceCudaRunConfig) : Prop :=
  GuestPcTraceCudaRunDecisionEvidence config

structure GuestPcTraceCudaRunValidation where
  traceCudaRunConfigAccepted :
    GuestPcTraceCudaRunConfig -> PublicInput -> Proof -> Prop
  traceCudaRunConfigImpliesDecisionMatches :
    forall config publicInput proof,
      traceCudaRunConfigAccepted config publicInput proof ->
        GuestPcTraceCudaRunDecisionMatches config

structure GpuRetainedLeafDigestLimitConfig where
  defaultLeafDigestLimitBytes : Nat
  configuredLeafDigestLimitBytes : Option Nat
  effectiveLeafDigestLimitBytes : Nat
deriving DecidableEq, Repr

def GpuRetainedLeafDigestLimitDecisionMatches
    (config : GpuRetainedLeafDigestLimitConfig) : Prop :=
  match config.configuredLeafDigestLimitBytes with
  | some configured =>
      config.effectiveLeafDigestLimitBytes = configured
  | none =>
      config.effectiveLeafDigestLimitBytes =
        config.defaultLeafDigestLimitBytes

structure GpuRetainedLeafDigestLimitValidation where
  retainedLeafDigestLimitConfigAccepted :
    GpuRetainedLeafDigestLimitConfig -> PublicInput -> Proof -> Prop
  retainedLeafDigestLimitConfigImpliesDecisionMatches :
    forall config publicInput proof,
      retainedLeafDigestLimitConfigAccepted config publicInput proof ->
        GpuRetainedLeafDigestLimitDecisionMatches config

structure GpuRetainedDeviceCacheBudget where
  sourceBytes : Nat
  descriptorBytes : Nat
  leafDigestBytes : Nat
  sourceLimit : Nat
  descriptorLimit : Nat
  leafDigestLimit : Nat
  combinedLimit : Option Nat
deriving DecidableEq, Repr

def GpuRetainedDeviceCacheBudgetWithinLimits
    (budget : GpuRetainedDeviceCacheBudget) : Prop :=
  budget.sourceBytes <= budget.sourceLimit
    /\ budget.descriptorBytes <= budget.descriptorLimit
    /\ budget.leafDigestBytes <= budget.leafDigestLimit
    /\ match budget.combinedLimit with
      | some limit =>
          budget.sourceBytes + budget.descriptorBytes + budget.leafDigestBytes <= limit
      | none => True

structure GpuRetainedDeviceCacheBudgetValidation where
  retainedDeviceCacheBudgetAccepted :
    GpuRetainedDeviceCacheBudget -> PublicInput -> Proof -> Prop
  retainedDeviceCacheBudgetImpliesWithinLimits :
    forall budget publicInput proof,
      retainedDeviceCacheBudgetAccepted budget publicInput proof ->
        GpuRetainedDeviceCacheBudgetWithinLimits budget

structure GpuHostDeviceCopyRoundTripValidation where
  allocationValidation : GpuAllocationCacheValidation
  uploadedBytesRoundTrip : GpuAllocationSource -> PublicInput -> Proof -> Prop
  roundTripImpliesWrittenContents :
    forall allocation publicInput proof,
      uploadedBytesRoundTrip allocation publicInput proof ->
        allocationValidation.writtenContentsBound allocation publicInput proof

structure FriFixedColumnCacheValidation where
  allocationValidation : GpuAllocationCacheValidation
  fixedColumnCacheRequestBound : GpuAllocationSource -> GpuAllocationSource -> Prop
  fixedColumnCacheRequestImpliesSameAllocationRequest :
    forall cached fresh,
      fixedColumnCacheRequestBound cached fresh ->
        GpuAllocationSameRequest cached fresh

def GpuAllocationCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuAllocationCacheValidation)
    (allocation : GpuAllocationSource)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.writtenContentsBound allocation publicInput proof

def GpuHostDeviceCopyRoundTripCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuHostDeviceCopyRoundTripValidation)
    (allocation : GpuAllocationSource)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.uploadedBytesRoundTrip allocation publicInput proof

def GpuTemporaryBufferReuseCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuTemporaryBufferReuseValidation)
    (previous next : GpuAllocationSource)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.temporaryBufferReuseAllowed previous next publicInput proof

def GpuAllocatorNoWaitBypassCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.noWaitBypassAllowed pending fresh publicInput proof

def GpuAllocatorNoWaitLimitCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuAllocatorNoWaitLimitValidation)
    (config : GpuAllocatorNoWaitLimitConfig)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.noWaitLimitConfigAccepted config publicInput proof

def GuestPcTraceSegmentQueueCheckedAcceptance
    (system : VerifierModel)
    (validation : GuestPcTraceSegmentQueueValidation)
    (config : GuestPcTraceSegmentQueueConfig)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.segmentQueueConfigAccepted config publicInput proof

def GuestPcTraceLargeGpuGateCheckedAcceptance
    (system : VerifierModel)
    (validation : GuestPcTraceLargeGpuGateValidation)
    (config : GuestPcTraceLargeGpuGateConfig)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.largeGpuGateConfigAccepted config publicInput proof

def GuestPcTraceTracelessCommitmentInputCheckedAcceptance
    (system : VerifierModel)
    (validation : GuestPcTraceTracelessCommitmentInputValidation)
    (config : GuestPcTraceTracelessCommitmentInputConfig)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.tracelessCommitmentInputConfigAccepted
      config
      publicInput
      proof

def GuestPcTraceTracelessSegmentOutputCheckedAcceptance
    (system : VerifierModel)
    (validation : GuestPcTraceTracelessSegmentOutputValidation)
    (config : GuestPcTraceTracelessSegmentOutputConfig)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.tracelessSegmentOutputConfigAccepted
      config
      publicInput
      proof

def GuestPcTraceCrossSegmentRootMaterializationCheckedAcceptance
    (system : VerifierModel)
    (validation : GuestPcTraceCrossSegmentRootMaterializationValidation)
    (config : GuestPcTraceCrossSegmentRootMaterializationConfig)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.crossSegmentRootMaterializationConfigAccepted
      config
      publicInput
      proof

def GuestPcTraceSegmentCommitModeCheckedAcceptance
    (system : VerifierModel)
    (validation : GuestPcTraceSegmentCommitModeValidation)
    (config : GuestPcTraceSegmentCommitModeConfig)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.segmentCommitModeConfigAccepted config publicInput proof

def GuestPcTraceDeviceTraceSourceCheckedAcceptance
    (system : VerifierModel)
    (validation : GuestPcTraceDeviceTraceSourceValidation)
    (config : GuestPcTraceDeviceTraceSourceConfig)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.deviceTraceSourceConfigAccepted config publicInput proof

def GuestPcTraceSparseSourceCheckedAcceptance
    (system : VerifierModel)
    (validation : GuestPcTraceSparseSourceValidation)
    (config : GuestPcTraceSparseSourceConfig)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.sparseSourceConfigAccepted config publicInput proof

def GuestPcTraceTerminalSparseSourceCheckedAcceptance
    (system : VerifierModel)
    (validation : GuestPcTraceTerminalSparseSourceValidation)
    (config : GuestPcTraceTerminalSparseSourceConfig)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.terminalSparseSourceConfigAccepted config publicInput proof

def FriRetainedStageSourceCheckedAcceptance
    (system : VerifierModel)
    (validation : FriRetainedStageSourceValidation)
    (config : FriRetainedStageSourceConfig)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.retainedStageSourceConfigAccepted config publicInput proof

def GuestPcTraceCudaRunCheckedAcceptance
    (system : VerifierModel)
    (validation : GuestPcTraceCudaRunValidation)
    (config : GuestPcTraceCudaRunConfig)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.traceCudaRunConfigAccepted config publicInput proof

def GpuRetainedLeafDigestLimitCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuRetainedLeafDigestLimitValidation)
    (config : GpuRetainedLeafDigestLimitConfig)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.retainedLeafDigestLimitConfigAccepted
      config
      publicInput
      proof

def GpuRetainedDeviceCacheBudgetCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuRetainedDeviceCacheBudgetValidation)
    (budget : GpuRetainedDeviceCacheBudget)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.retainedDeviceCacheBudgetAccepted budget publicInput proof

def FriFixedColumnCacheCheckedAcceptance
    (system : VerifierModel)
    (validation : FriFixedColumnCacheValidation)
    (cached fresh : GpuAllocationSource)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.fixedColumnCacheRequestBound cached fresh
    /\ validation.allocationValidation.writtenContentsBound fresh publicInput proof

def SourceLookupAuxiliaryEvidence
    (system : VerifierModel)
    (auxiliary : AuxiliaryValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  auxiliary.exactSourceLookupBalance publicInput proof
    \/ auxiliary.dynamicSourceLookupConstrained publicInput proof

def SourceLookupCheckedAcceptance
    (system : VerifierModel)
    (auxiliary : AuxiliaryValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ SourceLookupAuxiliaryEvidence system auxiliary publicInput proof

def WitnessLeafDigestEvidence
    (system : VerifierModel)
    (validation : WitnessLeafDigestValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.canonicalExtendedLeafBytes publicInput proof
    /\ validation.narrowPaddedDigestsBindRows publicInput proof
    /\ validation.wideLinearDigestsBindRows publicInput proof

def WitnessLeafDigestCheckedAcceptance
    (system : VerifierModel)
    (validation : WitnessLeafDigestValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ WitnessLeafDigestEvidence system validation publicInput proof

def GpuCanonicalLeafCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuCanonicalLeafValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.gpuCanonicalFlagClear publicInput proof

def GpuLeafOutputBufferReuseCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuLeafOutputBufferReuseValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.leafOutputBufferLengthMatches publicInput proof
    /\ validation.leafOutputBufferFullyOverwritten publicInput proof

def GpuCosetExtensionCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuCosetExtensionValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.cosetExtensionMatchesHost publicInput proof

def GpuFriFoldInterpolationCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuFriFoldInterpolationValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.gpuFriInterpolationMatchesHost publicInput proof

def GpuMerkleDigestPrefixBatchCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuMerkleDigestPrefixBatchValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.gpuMerkleDigestPrefixBatchMatchesSinglePaths publicInput proof


end Lzvm
