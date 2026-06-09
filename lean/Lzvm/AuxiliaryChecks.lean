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

structure GuestPcTraceTimingSummary where
  segmentCount : Nat
  guestTraceStreamMilliseconds : Nat
  guestSegmentCommitMilliseconds : Nat
  guestTraceRunnerMilliseconds : Nat
  guestTraceLowererMilliseconds : Nat
  guestTraceLowerMilliseconds : Nat
  guestTraceReportMilliseconds : Nat
  guestTraceReportCount : Nat
  guestTraceReportRowCount : Nat
  guestTraceEmitMilliseconds : Nat
  guestTraceDescriptorMilliseconds : Nat
  guestTraceDescriptorRowCount : Nat
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
  guestStageTreeCommitWorkMilliseconds : Nat
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
  constantMaterialValidationUnitCount : Nat
  constantMaterialValidationByteCount : Nat
deriving DecidableEq, Repr

structure ProverGpuModeSummary where
  proverGpuModeName : Nat
deriving DecidableEq, Repr

structure CudaBackendSummary where
  cudaBackendEnabled : Bool
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
