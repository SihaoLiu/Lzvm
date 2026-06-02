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

end Lzvm
