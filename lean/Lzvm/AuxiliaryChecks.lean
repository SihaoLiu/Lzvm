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

structure GuestPcTraceTimingSummary where
  segmentCount : Nat
  guestTraceStreamMilliseconds : Nat
  guestSegmentCommitMilliseconds : Nat
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
  guestStageTreeCommitRetainWorkMilliseconds : Nat
  stageTimings : List GuestPcTraceStageTimingSummary
deriving DecidableEq, Repr

structure WitnessOpeningStageRowValueTimingSummary where
  stageIndex : Nat
  sourceExtendMilliseconds : Nat
  sourceDownloadMilliseconds : Nat
  deviceDownloadMilliseconds : Nat
  deviceRowCount : Nat
  sourceRowCount : Nat
  wordCount : Nat
  byteCount : Nat
deriving DecidableEq, Repr

structure WitnessOpeningRowValueTimingSummary where
  rowValueSourceExtendMilliseconds : Nat
  rowValueSourceDownloadMilliseconds : Nat
  rowValueDeviceDownloadMilliseconds : Nat
  deviceRowCount : Nat
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
  finishWitnessExternalSourceDescriptorUploadByteCount : Nat
  finishWitnessExternalSourceDescriptorUploadWordCount : Nat
  finishWitnessExternalSourceDescriptorUploadRowCount : Nat
  finishFriOpeningMilliseconds : Nat
  finishProofEncodeMilliseconds : Nat
  finishContributionSegmentMilliseconds : Nat
  finishContributionVerifyMilliseconds : Nat
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

structure GpuRetainedDeviceCacheBudget where
  sourceBytes : Nat
  leafDigestBytes : Nat
  sourceLimit : Nat
  leafDigestLimit : Nat
  combinedLimit : Option Nat
deriving DecidableEq, Repr

def GpuRetainedDeviceCacheBudgetWithinLimits
    (budget : GpuRetainedDeviceCacheBudget) : Prop :=
  budget.sourceBytes <= budget.sourceLimit
    /\ budget.leafDigestBytes <= budget.leafDigestLimit
    /\ match budget.combinedLimit with
      | some limit => budget.sourceBytes + budget.leafDigestBytes <= limit
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

theorem source_lookup_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (auxiliary : AuxiliaryValidation system) :
    forall publicInput proof,
      SourceLookupCheckedAcceptance system auxiliary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem source_lookup_checked_acceptance_projects_auxiliary_evidence
    {system : VerifierModel}
    (auxiliary : AuxiliaryValidation system) :
    forall publicInput proof,
      SourceLookupCheckedAcceptance system auxiliary publicInput proof ->
        SourceLookupAuxiliaryEvidence system auxiliary publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem source_lookup_auxiliary_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (auxiliary : AuxiliaryValidation system) :
    forall publicInput proof,
      SourceLookupCheckedAcceptance system auxiliary publicInput proof ->
        SourceLookupAuxiliaryEvidence system auxiliary publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithLookupChecks
  exact
    And.intro
      (source_lookup_checked_acceptance_projects_auxiliary_evidence
        auxiliary
        publicInput
        proof
        acceptedWithLookupChecks)
      (abstract_verifier_sound
        assumptions
        publicInput
        proof
        (source_lookup_checked_acceptance_projects_verifier_acceptance
          auxiliary
          publicInput
          proof
          acceptedWithLookupChecks))

theorem source_lookup_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (auxiliary : AuxiliaryValidation system) :
    forall publicInput proof,
      SourceLookupCheckedAcceptance system auxiliary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    source_lookup_auxiliary_acceptance_sound
      assumptions
      auxiliary
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

theorem witness_leaf_digest_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem witness_leaf_digest_checked_acceptance_projects_evidence
    {system : VerifierModel}
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        WitnessLeafDigestEvidence system validation publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem witness_leaf_digest_checked_acceptance_projects_canonical_leaf_bytes
    {system : VerifierModel}
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        validation.canonicalExtendedLeafBytes publicInput proof := by
  intro publicInput proof checked
  exact checked.right.left

theorem witness_leaf_digest_checked_acceptance_projects_narrow_padded_digest_rows
    {system : VerifierModel}
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        validation.narrowPaddedDigestsBindRows publicInput proof := by
  intro publicInput proof checked
  exact checked.right.right.left

theorem witness_leaf_digest_checked_acceptance_projects_wide_linear_digest_rows
    {system : VerifierModel}
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        validation.wideLinearDigestsBindRows publicInput proof := by
  intro publicInput proof checked
  exact checked.right.right.right

theorem witness_leaf_digest_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        WitnessLeafDigestEvidence system validation publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithLeafDigestChecks
  exact
    And.intro
      (witness_leaf_digest_checked_acceptance_projects_evidence
        validation
        publicInput
        proof
        acceptedWithLeafDigestChecks)
      (abstract_verifier_sound
        assumptions
        publicInput
        proof
        (witness_leaf_digest_checked_acceptance_projects_verifier_acceptance
          validation
          publicInput
          proof
          acceptedWithLeafDigestChecks))

theorem witness_leaf_digest_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    witness_leaf_digest_acceptance_sound
      assumptions
      validation
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

theorem gpu_canonical_leaf_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (validation : GpuCanonicalLeafValidation system) :
    forall publicInput proof,
      GpuCanonicalLeafCheckedAcceptance system validation publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem gpu_canonical_leaf_checked_acceptance_projects_flag_clear
    {system : VerifierModel}
    (validation : GpuCanonicalLeafValidation system) :
    forall publicInput proof,
      GpuCanonicalLeafCheckedAcceptance system validation publicInput proof ->
        validation.gpuCanonicalFlagClear publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem gpu_canonical_leaf_checked_acceptance_projects_leaf_bytes
    {system : VerifierModel}
    (validation : GpuCanonicalLeafValidation system) :
    forall publicInput proof,
      GpuCanonicalLeafCheckedAcceptance system validation publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof := by
  intro publicInput proof checked
  exact
    validation.flagClearImpliesCanonicalExtendedLeafBytes
      publicInput
      proof
      (gpu_canonical_leaf_checked_acceptance_projects_flag_clear
        validation
        publicInput
        proof
        checked)

theorem gpu_canonical_leaf_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuCanonicalLeafValidation system) :
    forall publicInput proof,
      GpuCanonicalLeafCheckedAcceptance system validation publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithCanonicalFlag
  exact
    And.intro
      (gpu_canonical_leaf_checked_acceptance_projects_leaf_bytes
        validation
        publicInput
        proof
        acceptedWithCanonicalFlag)
      (abstract_verifier_sound
        assumptions
        publicInput
        proof
        (gpu_canonical_leaf_checked_acceptance_projects_verifier_acceptance
          validation
          publicInput
          proof
          acceptedWithCanonicalFlag))

theorem gpu_canonical_leaf_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuCanonicalLeafValidation system) :
    forall publicInput proof,
      GpuCanonicalLeafCheckedAcceptance system validation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_canonical_leaf_checked_acceptance_sound
      assumptions
      validation
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

theorem gpu_leaf_output_buffer_reuse_implies_canonical_leaf_bytes
    {system : VerifierModel}
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      validation.leafOutputBufferLengthMatches publicInput proof ->
      validation.leafOutputBufferFullyOverwritten publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof := by
  intro publicInput proof outputLengthMatches outputFullyOverwritten
  exact
    validation.leafOutputBufferReuseImpliesCanonicalLeafBytes
      publicInput
      proof
      outputLengthMatches
      outputFullyOverwritten

theorem gpu_leaf_output_buffer_reuse_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      GpuLeafOutputBufferReuseCheckedAcceptance system validation publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem gpu_leaf_output_buffer_reuse_checked_acceptance_projects_length_match
    {system : VerifierModel}
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      GpuLeafOutputBufferReuseCheckedAcceptance system validation publicInput proof ->
        validation.leafOutputBufferLengthMatches publicInput proof := by
  intro publicInput proof checked
  exact checked.right.left

theorem gpu_leaf_output_buffer_reuse_checked_acceptance_projects_fully_overwritten
    {system : VerifierModel}
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      GpuLeafOutputBufferReuseCheckedAcceptance system validation publicInput proof ->
        validation.leafOutputBufferFullyOverwritten publicInput proof := by
  intro publicInput proof checked
  exact checked.right.right

theorem gpu_leaf_output_buffer_reuse_checked_acceptance_projects_leaf_bytes
    {system : VerifierModel}
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      GpuLeafOutputBufferReuseCheckedAcceptance system validation publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof := by
  intro publicInput proof checked
  exact
    gpu_leaf_output_buffer_reuse_implies_canonical_leaf_bytes
      validation
      publicInput
      proof
      checked.right.left
      checked.right.right

theorem gpu_leaf_output_buffer_reuse_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      GpuLeafOutputBufferReuseCheckedAcceptance system validation publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (gpu_leaf_output_buffer_reuse_checked_acceptance_projects_leaf_bytes
        validation
        publicInput
        proof
        checked)
      (abstract_verifier_sound
        assumptions
        publicInput
        proof
        (gpu_leaf_output_buffer_reuse_checked_acceptance_projects_verifier_acceptance
          validation
          publicInput
          proof
          checked))

theorem gpu_leaf_output_buffer_reuse_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      GpuLeafOutputBufferReuseCheckedAcceptance system validation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_leaf_output_buffer_reuse_checked_acceptance_sound
      assumptions
      validation
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

theorem gpu_coset_extension_matches_host_implies_leaf_bytes
    {system : VerifierModel}
    (validation : GpuCosetExtensionValidation system) :
    forall publicInput proof,
      validation.cosetExtensionMatchesHost publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof := by
  intro publicInput proof cosetMatchesHost
  exact
    validation.cosetExtensionImpliesCanonicalLeafBytes
      publicInput
      proof
      cosetMatchesHost

theorem gpu_coset_extension_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (validation : GpuCosetExtensionValidation system) :
    forall publicInput proof,
      GpuCosetExtensionCheckedAcceptance system validation publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem gpu_coset_extension_checked_acceptance_projects_matches_host
    {system : VerifierModel}
    (validation : GpuCosetExtensionValidation system) :
    forall publicInput proof,
      GpuCosetExtensionCheckedAcceptance system validation publicInput proof ->
        validation.cosetExtensionMatchesHost publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem gpu_coset_extension_checked_acceptance_projects_leaf_bytes
    {system : VerifierModel}
    (validation : GpuCosetExtensionValidation system) :
    forall publicInput proof,
      GpuCosetExtensionCheckedAcceptance system validation publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof := by
  intro publicInput proof checked
  exact
    gpu_coset_extension_matches_host_implies_leaf_bytes
      validation
      publicInput
      proof
      (gpu_coset_extension_checked_acceptance_projects_matches_host
        validation
        publicInput
        proof
        checked)

theorem gpu_coset_extension_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuCosetExtensionValidation system) :
    forall publicInput proof,
      GpuCosetExtensionCheckedAcceptance system validation publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (gpu_coset_extension_checked_acceptance_projects_leaf_bytes
        validation
        publicInput
        proof
        checked)
      (abstract_verifier_sound
        assumptions
        publicInput
        proof
        (gpu_coset_extension_checked_acceptance_projects_verifier_acceptance
          validation
          publicInput
          proof
          checked))

theorem gpu_coset_extension_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuCosetExtensionValidation system) :
    forall publicInput proof,
      GpuCosetExtensionCheckedAcceptance system validation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_coset_extension_checked_acceptance_sound
      assumptions
      validation
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

theorem gpu_fri_interpolation_matches_host_implies_fri_folds_valid
    {system : VerifierModel}
    (validation : GpuFriFoldInterpolationValidation system) :
    forall publicInput proof,
      validation.gpuFriInterpolationMatchesHost publicInput proof ->
        validation.friFoldsValid publicInput proof := by
  intro publicInput proof interpolationMatchesHost
  exact
    validation.gpuFriInterpolationImpliesFriFoldsValid
      publicInput
      proof
      interpolationMatchesHost

theorem gpu_fri_fold_interpolation_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (validation : GpuFriFoldInterpolationValidation system) :
    forall publicInput proof,
      GpuFriFoldInterpolationCheckedAcceptance system validation publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem gpu_fri_fold_interpolation_checked_acceptance_projects_matches_host
    {system : VerifierModel}
    (validation : GpuFriFoldInterpolationValidation system) :
    forall publicInput proof,
      GpuFriFoldInterpolationCheckedAcceptance system validation publicInput proof ->
        validation.gpuFriInterpolationMatchesHost publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem gpu_fri_fold_interpolation_checked_acceptance_projects_fri_folds_valid
    {system : VerifierModel}
    (validation : GpuFriFoldInterpolationValidation system) :
    forall publicInput proof,
      GpuFriFoldInterpolationCheckedAcceptance system validation publicInput proof ->
        validation.friFoldsValid publicInput proof := by
  intro publicInput proof checked
  exact
    gpu_fri_interpolation_matches_host_implies_fri_folds_valid
      validation
      publicInput
      proof
      (gpu_fri_fold_interpolation_checked_acceptance_projects_matches_host
        validation
        publicInput
        proof
        checked)

theorem gpu_fri_fold_interpolation_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuFriFoldInterpolationValidation system) :
    forall publicInput proof,
      GpuFriFoldInterpolationCheckedAcceptance system validation publicInput proof ->
        validation.friFoldsValid publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (gpu_fri_fold_interpolation_checked_acceptance_projects_fri_folds_valid
        validation
        publicInput
        proof
        checked)
      (abstract_verifier_sound
        assumptions
        publicInput
        proof
        (gpu_fri_fold_interpolation_checked_acceptance_projects_verifier_acceptance
          validation
          publicInput
          proof
          checked))

theorem gpu_fri_fold_interpolation_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuFriFoldInterpolationValidation system) :
    forall publicInput proof,
      GpuFriFoldInterpolationCheckedAcceptance system validation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_fri_fold_interpolation_checked_acceptance_sound
      assumptions
      validation
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

theorem gpu_merkle_digest_prefix_batch_matches_single_paths_implies_lower_prefixes_bound
    {system : VerifierModel}
    (validation : GpuMerkleDigestPrefixBatchValidation system) :
    forall publicInput proof,
      validation.gpuMerkleDigestPrefixBatchMatchesSinglePaths publicInput proof ->
        validation.lowerPrefixesBound publicInput proof := by
  intro publicInput proof prefixBatchMatchesSinglePaths
  exact
    validation.gpuMerkleDigestPrefixBatchImpliesLowerPrefixesBound
      publicInput
      proof
      prefixBatchMatchesSinglePaths

theorem gpu_merkle_digest_prefix_batch_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (validation : GpuMerkleDigestPrefixBatchValidation system) :
    forall publicInput proof,
      GpuMerkleDigestPrefixBatchCheckedAcceptance system validation publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem gpu_merkle_digest_prefix_batch_checked_acceptance_projects_matches_single_paths
    {system : VerifierModel}
    (validation : GpuMerkleDigestPrefixBatchValidation system) :
    forall publicInput proof,
      GpuMerkleDigestPrefixBatchCheckedAcceptance system validation publicInput proof ->
        validation.gpuMerkleDigestPrefixBatchMatchesSinglePaths publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem gpu_merkle_digest_prefix_batch_checked_acceptance_projects_lower_prefixes_bound
    {system : VerifierModel}
    (validation : GpuMerkleDigestPrefixBatchValidation system) :
    forall publicInput proof,
      GpuMerkleDigestPrefixBatchCheckedAcceptance system validation publicInput proof ->
        validation.lowerPrefixesBound publicInput proof := by
  intro publicInput proof checked
  exact
    gpu_merkle_digest_prefix_batch_matches_single_paths_implies_lower_prefixes_bound
      validation
      publicInput
      proof
      (gpu_merkle_digest_prefix_batch_checked_acceptance_projects_matches_single_paths
        validation
        publicInput
        proof
        checked)

theorem gpu_merkle_digest_prefix_batch_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuMerkleDigestPrefixBatchValidation system) :
    forall publicInput proof,
      GpuMerkleDigestPrefixBatchCheckedAcceptance system validation publicInput proof ->
        validation.lowerPrefixesBound publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (gpu_merkle_digest_prefix_batch_checked_acceptance_projects_lower_prefixes_bound
        validation
        publicInput
        proof
        checked)
      (abstract_verifier_sound
        assumptions
        publicInput
        proof
        (gpu_merkle_digest_prefix_batch_checked_acceptance_projects_verifier_acceptance
          validation
          publicInput
          proof
          checked))

theorem gpu_merkle_digest_prefix_batch_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuMerkleDigestPrefixBatchValidation system) :
    forall publicInput proof,
      GpuMerkleDigestPrefixBatchCheckedAcceptance system validation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_merkle_digest_prefix_batch_checked_acceptance_sound
      assumptions
      validation
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

theorem gpu_setup_cache_reuse_sound
    (validation : GpuSetupCacheValidation)
    (state : GpuSetupCacheState)
    (request : GpuSetupRequest) :
    GpuSetupCacheCovers state request ->
      validation.constantsSoundFor state.device state.initializedBits ->
        validation.constantsSoundFor request.device request.requiredBits := by
  intro covers initializedSound
  cases covers with
  | intro sameDevice bitCover =>
    rw [sameDevice]
    exact
      validation.coveredConstantsSound
        state.device
        state.initializedBits
        request.requiredBits
        bitCover
        initializedSound

theorem gpu_setup_cache_reuse_request_device_sound
    (validation : GpuSetupCacheValidation)
    (state : GpuSetupCacheState)
    (request : GpuSetupRequest) :
    request.device = state.device ->
      request.requiredBits <= state.initializedBits ->
        validation.constantsSoundFor state.device state.initializedBits ->
          validation.constantsSoundFor request.device request.requiredBits := by
  intro sameDevice bitCover initializedSound
  exact
    gpu_setup_cache_reuse_sound
      validation
      state
      request
      (And.intro sameDevice bitCover)
      initializedSound

end Lzvm
