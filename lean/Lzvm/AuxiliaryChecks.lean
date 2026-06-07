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

structure TimingObservation where
  label : Nat
  milliseconds : Nat
deriving DecidableEq, Repr

structure GuestPcTraceTimingSummary where
  segmentCount : Nat
  guestTraceStreamMilliseconds : Nat
  guestSegmentCommitMilliseconds : Nat
  guestRegularConstraintsMilliseconds : Nat
  guestRegularHintsMilliseconds : Nat
  guestStageCommitMilliseconds : Nat
  guestStageTraceExtractMilliseconds : Nat
  guestStageLeafExtendWorkMilliseconds : Nat
  guestStageLeafSetupWorkMilliseconds : Nat
  guestStageLeafUploadWorkMilliseconds : Nat
  guestStageLeafKernelWorkMilliseconds : Nat
  guestStageLeafDownloadWorkMilliseconds : Nat
  guestStageLeafValidateWorkMilliseconds : Nat
  guestStageLeafHashWorkMilliseconds : Nat
  guestStageTreeCommitWorkMilliseconds : Nat
deriving DecidableEq, Repr

structure WitnessOpeningStageRowValueTimingSummary where
  stageIndex : Nat
  deviceRowCount : Nat
  sourceRowCount : Nat
  wordCount : Nat
  byteCount : Nat
deriving DecidableEq, Repr

structure WitnessOpeningRowValueTimingSummary where
  deviceRowCount : Nat
  sourceRowCount : Nat
  wordCount : Nat
  byteCount : Nat
  stages : List WitnessOpeningStageRowValueTimingSummary
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

def GpuAllocationCheckedAcceptance
    (system : VerifierModel)
    (validation : GpuAllocationCacheValidation)
    (allocation : GpuAllocationSource)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ validation.writtenContentsBound allocation publicInput proof

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

end Lzvm
