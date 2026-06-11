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
  guestDeviceSourceDescriptorUploadRowCount : Nat
  guestDeviceSourceTraceExpandMilliseconds : Nat
  guestStageSourceRetentionAttemptCount : Nat
  guestStageSourceRetentionRetainedCount : Nat
  guestStageSourceRetentionRejectedCount : Nat
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
    And.intro acceptedWithLookupChecks.right
      (abstract_verifier_sound assumptions publicInput proof acceptedWithLookupChecks.left)

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
    And.intro acceptedWithLeafDigestChecks.right
      (abstract_verifier_sound assumptions publicInput proof acceptedWithLeafDigestChecks.left)

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
      (validation.flagClearImpliesCanonicalExtendedLeafBytes
        publicInput
        proof
        acceptedWithCanonicalFlag.right)
      (abstract_verifier_sound assumptions publicInput proof acceptedWithCanonicalFlag.left)

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
      (gpu_coset_extension_matches_host_implies_leaf_bytes
        validation
        publicInput
        proof
        checked.right)
      (abstract_verifier_sound assumptions publicInput proof checked.left)

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
      (gpu_fri_interpolation_matches_host_implies_fri_folds_valid
        validation
        publicInput
        proof
        checked.right)
      (abstract_verifier_sound assumptions publicInput proof checked.left)

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
      (gpu_merkle_digest_prefix_batch_matches_single_paths_implies_lower_prefixes_bound
        validation
        publicInput
        proof
        checked.right)
      (abstract_verifier_sound assumptions publicInput proof checked.left)

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

def TimingObservedAcceptance
    (system : VerifierModel)
    (_observations : List TimingObservation)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem timing_observation_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (observations : List TimingObservation) :
    forall publicInput proof,
      TimingObservedAcceptance system observations publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithTimings
  exact abstract_verifier_sound assumptions publicInput proof acceptedWithTimings

theorem timing_observation_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (observations : List TimingObservation) :
    forall publicInput proof,
      TimingObservedAcceptance system observations publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (timing_observation_acceptance_sound
        assumptions
        observations
        publicInput
        proof
        observed)

def GuestPcTraceTimingObservedAcceptance
    (system : VerifierModel)
    (_summary : Option GuestPcTraceTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem guest_pc_trace_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithGuestPcTraceTimings
  exact abstract_verifier_sound assumptions publicInput proof acceptedWithGuestPcTraceTimings

theorem guest_pc_trace_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_timing_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

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
    sound_witness_implies_verifier_core_contract
      (guest_pc_trace_descriptor_width_counts_acceptance_sound
        assumptions
        summary
        compactRows
        wideRows
        publicInput
        proof
        observed)

def WitnessOpeningRowValueTimingObservedAcceptance
    (system : VerifierModel)
    (_summary : Option WitnessOpeningRowValueTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem witness_opening_row_value_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option WitnessOpeningRowValueTimingSummary) :
    forall publicInput proof,
      WitnessOpeningRowValueTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithRowValueTimings
  exact abstract_verifier_sound assumptions publicInput proof acceptedWithRowValueTimings

theorem witness_opening_row_value_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option WitnessOpeningRowValueTimingSummary) :
    forall publicInput proof,
      WitnessOpeningRowValueTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (witness_opening_row_value_timing_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

def ConstantMaterialValidationTimingObservedAcceptance
    (system : VerifierModel)
    (_summary : Option ConstantMaterialValidationTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem constant_material_validation_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ConstantMaterialValidationTimingSummary) :
    forall publicInput proof,
      ConstantMaterialValidationTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithConstantMaterialTimings
  exact abstract_verifier_sound assumptions publicInput proof acceptedWithConstantMaterialTimings

theorem constant_material_validation_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ConstantMaterialValidationTimingSummary) :
    forall publicInput proof,
      ConstantMaterialValidationTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (constant_material_validation_timing_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

def ProverGpuModeObservedAcceptance
    (system : VerifierModel)
    (_summary : Option ProverGpuModeSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem prover_gpu_mode_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProverGpuModeSummary) :
    forall publicInput proof,
      ProverGpuModeObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithGpuMode
  exact abstract_verifier_sound assumptions publicInput proof acceptedWithGpuMode

theorem prover_gpu_mode_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProverGpuModeSummary) :
    forall publicInput proof,
      ProverGpuModeObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (prover_gpu_mode_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

def GpuRunOptionsObservedAcceptance
    (system : VerifierModel)
    (_summary : Option GpuRunOptionsSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem gpu_run_options_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GpuRunOptionsSummary) :
    forall publicInput proof,
      GpuRunOptionsObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithGpuRunOptions
  exact abstract_verifier_sound assumptions publicInput proof acceptedWithGpuRunOptions

theorem gpu_run_options_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GpuRunOptionsSummary) :
    forall publicInput proof,
      GpuRunOptionsObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (gpu_run_options_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

def CudaBackendObservedAcceptance
    (system : VerifierModel)
    (_summary : Option CudaBackendSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem cuda_backend_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option CudaBackendSummary) :
    forall publicInput proof,
      CudaBackendObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithCudaBackend
  exact abstract_verifier_sound assumptions publicInput proof acceptedWithCudaBackend

theorem cuda_backend_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option CudaBackendSummary) :
    forall publicInput proof,
      CudaBackendObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (cuda_backend_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

def CudaAllocatorTimingObservedAcceptance
    (system : VerifierModel)
    (_summary : Option CudaAllocatorTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem cuda_allocator_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option CudaAllocatorTimingSummary) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithAllocatorTimings
  exact abstract_verifier_sound assumptions publicInput proof acceptedWithAllocatorTimings

theorem cuda_allocator_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option CudaAllocatorTimingSummary) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (cuda_allocator_timing_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

def ProofArtifactFinishTimingObservedAcceptance
    (system : VerifierModel)
    (_summary : Option ProofArtifactFinishTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem proof_artifact_finish_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithProofFinishTimings
  exact abstract_verifier_sound assumptions publicInput proof acceptedWithProofFinishTimings

theorem proof_artifact_finish_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (proof_artifact_finish_timing_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

def RuntimePerformanceObservedAcceptance
    (system : VerifierModel)
    (_summary : RuntimePerformanceObservationSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof

theorem runtime_performance_observation_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithPerformanceObservations
  exact abstract_verifier_sound assumptions publicInput proof
    acceptedWithPerformanceObservations

theorem runtime_performance_observation_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    sound_witness_implies_verifier_core_contract
      (runtime_performance_observation_acceptance_sound
        assumptions
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_projects_timing_observations
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        TimingObservedAcceptance
          system
          summary.timingObservations
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_projects_guest_pc_trace_timing
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        GuestPcTraceTimingObservedAcceptance
          system
          summary.guestPcTraceTiming
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_projects_witness_opening_row_value_timing
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        WitnessOpeningRowValueTimingObservedAcceptance
          system
          summary.witnessOpeningRowValueTiming
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_projects_constant_material_validation_timing
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        ConstantMaterialValidationTimingObservedAcceptance
          system
          summary.constantMaterialValidationTiming
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_projects_prover_gpu_mode
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        ProverGpuModeObservedAcceptance
          system
          summary.proverGpuMode
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_projects_gpu_run_options
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        GpuRunOptionsObservedAcceptance
          system
          summary.gpuRunOptions
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_projects_cuda_backend
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        CudaBackendObservedAcceptance
          system
          summary.cudaBackend
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_projects_cuda_allocator_timing
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        CudaAllocatorTimingObservedAcceptance
          system
          summary.cudaAllocatorTiming
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_projects_proof_artifact_finish_timing
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        ProofArtifactFinishTimingObservedAcceptance
          system
          summary.proofArtifactFinishTiming
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

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

theorem gpu_setup_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuSetupCacheValidation)
    (request : GpuSetupRequest) :
    forall publicInput proof,
      GpuSetupCheckedAcceptance system validation request publicInput proof ->
        validation.constantsSoundFor request.device request.requiredBits
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithSetup
  exact
    And.intro acceptedWithSetup.right
      (abstract_verifier_sound assumptions publicInput proof acceptedWithSetup.left)

theorem gpu_setup_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuSetupCacheValidation)
    (request : GpuSetupRequest) :
    forall publicInput proof,
      GpuSetupCheckedAcceptance system validation request publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_setup_checked_acceptance_sound
      assumptions
      validation
      request
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

theorem gpu_allocation_cache_reuse_preserves_written_contents
    (validation : GpuAllocationCacheValidation)
    (cached fresh : GpuAllocationSource) :
    GpuAllocationSameRequest cached fresh ->
      (forall publicInput proof,
        validation.writtenContentsBound fresh publicInput proof ->
          validation.writtenContentsBound cached publicInput proof) := by
  intro sameRequest publicInput proof freshBound
  exact
    validation.cachedReusePreservesWrittenContents
      cached
      fresh
      publicInput
      proof
      sameRequest
      freshBound

theorem gpu_allocation_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuAllocationCacheValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocationCheckedAcceptance system validation allocation publicInput proof ->
        validation.writtenContentsBound allocation publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithAllocation
  exact
    And.intro acceptedWithAllocation.right
      (abstract_verifier_sound assumptions publicInput proof acceptedWithAllocation.left)

theorem gpu_allocation_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuAllocationCacheValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocationCheckedAcceptance system validation allocation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_allocation_checked_acceptance_sound
      assumptions
      validation
      allocation
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

theorem gpu_host_device_copy_round_trip_implies_written_contents
    (validation : GpuHostDeviceCopyRoundTripValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      validation.uploadedBytesRoundTrip allocation publicInput proof ->
        validation.allocationValidation.writtenContentsBound allocation publicInput proof := by
  intro publicInput proof roundTrip
  exact
    validation.roundTripImpliesWrittenContents
      allocation
      publicInput
      proof
      roundTrip

theorem gpu_host_device_copy_round_trip_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuHostDeviceCopyRoundTripValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuHostDeviceCopyRoundTripCheckedAcceptance
          system
          validation
          allocation
          publicInput
          proof ->
        validation.allocationValidation.writtenContentsBound allocation publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (gpu_host_device_copy_round_trip_implies_written_contents
        validation
        allocation
        publicInput
        proof
        checked.right)
      (abstract_verifier_sound assumptions publicInput proof checked.left)

theorem gpu_host_device_copy_round_trip_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuHostDeviceCopyRoundTripValidation)
    (allocation : GpuAllocationSource) :
    forall publicInput proof,
      GpuHostDeviceCopyRoundTripCheckedAcceptance
          system
          validation
          allocation
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_host_device_copy_round_trip_checked_acceptance_sound
      assumptions
      validation
      allocation
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

theorem gpu_temporary_buffer_reuse_implies_same_request
    (validation : GpuTemporaryBufferReuseValidation)
    (previous next : GpuAllocationSource) :
    forall publicInput proof,
      validation.temporaryBufferReuseAllowed previous next publicInput proof ->
        GpuAllocationSameRequest previous next := by
  intro publicInput proof reuseAllowed
  exact
    validation.temporaryBufferReuseImpliesSameRequest
      previous
      next
      publicInput
      proof
      reuseAllowed

theorem gpu_temporary_buffer_reuse_implies_pending_reads_complete
    (validation : GpuTemporaryBufferReuseValidation)
    (previous next : GpuAllocationSource) :
    forall publicInput proof,
      validation.temporaryBufferReuseAllowed previous next publicInput proof ->
        validation.pendingDeviceReadsComplete previous publicInput proof := by
  intro publicInput proof reuseAllowed
  exact
    validation.temporaryBufferReuseImpliesPendingReadsComplete
      previous
      next
      publicInput
      proof
      reuseAllowed

theorem gpu_temporary_buffer_reuse_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuTemporaryBufferReuseValidation)
    (previous next : GpuAllocationSource) :
    forall publicInput proof,
      GpuTemporaryBufferReuseCheckedAcceptance
          system
          validation
          previous
          next
          publicInput
          proof ->
        GpuAllocationSameRequest previous next
          /\ validation.pendingDeviceReadsComplete previous publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have sameRequest :=
    gpu_temporary_buffer_reuse_implies_same_request
      validation
      previous
      next
      publicInput
      proof
      checked.right
  have pendingComplete :=
    gpu_temporary_buffer_reuse_implies_pending_reads_complete
      validation
      previous
      next
      publicInput
      proof
      checked.right
  exact
    And.intro sameRequest
      (And.intro pendingComplete
        (abstract_verifier_sound assumptions publicInput proof checked.left))

theorem gpu_temporary_buffer_reuse_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuTemporaryBufferReuseValidation)
    (previous next : GpuAllocationSource) :
    forall publicInput proof,
      GpuTemporaryBufferReuseCheckedAcceptance
          system
          validation
          previous
          next
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_temporary_buffer_reuse_checked_acceptance_sound
      assumptions
      validation
      previous
      next
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right.right

theorem gpu_allocator_no_wait_bypass_implies_same_request
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource) :
    forall publicInput proof,
      validation.noWaitBypassAllowed pending fresh publicInput proof ->
        GpuAllocationSameRequest pending fresh := by
  intro publicInput proof bypassAllowed
  exact
    validation.noWaitBypassImpliesSameRequest
      pending
      fresh
      publicInput
      proof
      bypassAllowed

theorem gpu_allocator_no_wait_bypass_implies_pending_not_reused
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource) :
    forall publicInput proof,
      validation.noWaitBypassAllowed pending fresh publicInput proof ->
        validation.pendingAllocationNotReused pending publicInput proof := by
  intro publicInput proof bypassAllowed
  exact
    validation.noWaitBypassImpliesPendingNotReused
      pending
      fresh
      publicInput
      proof
      bypassAllowed

theorem gpu_allocator_no_wait_bypass_implies_fresh_allocation
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource) :
    forall publicInput proof,
      validation.noWaitBypassAllowed pending fresh publicInput proof ->
        validation.freshAllocationIssued fresh publicInput proof := by
  intro publicInput proof bypassAllowed
  exact
    validation.noWaitBypassImpliesFreshAllocation
      pending
      fresh
      publicInput
      proof
      bypassAllowed

theorem gpu_allocator_no_wait_bypass_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocatorNoWaitBypassCheckedAcceptance
          system
          validation
          pending
          fresh
          publicInput
          proof ->
        GpuAllocationSameRequest pending fresh
          /\ validation.pendingAllocationNotReused pending publicInput proof
          /\ validation.freshAllocationIssued fresh publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have sameRequest :=
    gpu_allocator_no_wait_bypass_implies_same_request
      validation
      pending
      fresh
      publicInput
      proof
      checked.right
  have pendingNotReused :=
    gpu_allocator_no_wait_bypass_implies_pending_not_reused
      validation
      pending
      fresh
      publicInput
      proof
      checked.right
  have freshIssued :=
    gpu_allocator_no_wait_bypass_implies_fresh_allocation
      validation
      pending
      fresh
      publicInput
      proof
      checked.right
  exact
    And.intro sameRequest
      (And.intro pendingNotReused
        (And.intro freshIssued
          (abstract_verifier_sound assumptions publicInput proof checked.left)))

theorem gpu_allocator_no_wait_bypass_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuAllocatorNoWaitBypassValidation)
    (pending fresh : GpuAllocationSource) :
    forall publicInput proof,
      GpuAllocatorNoWaitBypassCheckedAcceptance
          system
          validation
          pending
          fresh
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_allocator_no_wait_bypass_checked_acceptance_sound
      assumptions
      validation
      pending
      fresh
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right.right.right

theorem gpu_retained_device_cache_budget_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuRetainedDeviceCacheBudgetValidation)
    (budget : GpuRetainedDeviceCacheBudget) :
    forall publicInput proof,
      GpuRetainedDeviceCacheBudgetCheckedAcceptance
          system
          validation
          budget
          publicInput
          proof ->
        GpuRetainedDeviceCacheBudgetWithinLimits budget
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (validation.retainedDeviceCacheBudgetImpliesWithinLimits
        budget
        publicInput
        proof
        checked.right)
      (abstract_verifier_sound assumptions publicInput proof checked.left)

theorem gpu_retained_device_cache_budget_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuRetainedDeviceCacheBudgetValidation)
    (budget : GpuRetainedDeviceCacheBudget) :
    forall publicInput proof,
      GpuRetainedDeviceCacheBudgetCheckedAcceptance
          system
          validation
          budget
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    gpu_retained_device_cache_budget_checked_acceptance_sound
      assumptions
      validation
      budget
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

theorem fri_fixed_column_cache_same_request_implies_cached_contents_bound
    (validation : FriFixedColumnCacheValidation)
    (cached fresh : GpuAllocationSource) :
    validation.fixedColumnCacheRequestBound cached fresh ->
      (forall publicInput proof,
        validation.allocationValidation.writtenContentsBound fresh publicInput proof ->
          validation.allocationValidation.writtenContentsBound cached publicInput proof) := by
  intro requestBound publicInput proof freshBound
  have sameRequest :=
    validation.fixedColumnCacheRequestImpliesSameAllocationRequest
      cached
      fresh
      requestBound
  exact
    validation.allocationValidation.cachedReusePreservesWrittenContents
      cached
      fresh
      publicInput
      proof
      sameRequest
      freshBound

theorem fri_fixed_column_cache_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : FriFixedColumnCacheValidation)
    (cached fresh : GpuAllocationSource) :
    forall publicInput proof,
      FriFixedColumnCacheCheckedAcceptance
          system
          validation
          cached
          fresh
          publicInput
          proof ->
        validation.allocationValidation.writtenContentsBound cached publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  have requestBound := checked.right.left
  have freshBound := checked.right.right
  have cachedBound :=
    fri_fixed_column_cache_same_request_implies_cached_contents_bound
      validation
      cached
      fresh
      requestBound
      publicInput
      proof
      freshBound
  exact
    And.intro cachedBound
      (abstract_verifier_sound assumptions publicInput proof checked.left)

theorem fri_fixed_column_cache_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : FriFixedColumnCacheValidation)
    (cached fresh : GpuAllocationSource) :
    forall publicInput proof,
      FriFixedColumnCacheCheckedAcceptance
          system
          validation
          cached
          fresh
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  have sound :=
    fri_fixed_column_cache_checked_acceptance_sound
      assumptions
      validation
      cached
      fresh
      publicInput
      proof
      checked
  exact sound_witness_implies_verifier_core_contract sound.right

end Lzvm
