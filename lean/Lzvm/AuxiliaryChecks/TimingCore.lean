/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks
import Lzvm.AuxiliaryChecks.ProofTiming
import Lzvm.AuxiliaryChecks.ProofTimingVerifier

/-!
Runtime timing acceptance core projections for auxiliary verifier metadata.
-/

namespace Lzvm

def TimingObservedAcceptance
    (system : VerifierModel)
    (observations : List TimingObservation)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system observations publicInput proof

theorem timing_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (observations : List TimingObservation) :
    forall publicInput proof,
      TimingObservedAcceptance system observations publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem timing_observation_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (observations : List TimingObservation) :
    forall publicInput proof,
      TimingObservedAcceptance system observations publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithTimings
  exact
    ignored_metadata_acceptance_sound
      assumptions
      observations
      publicInput
      proof
      acceptedWithTimings

theorem timing_observation_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (observations : List TimingObservation) :
    forall publicInput proof,
      TimingObservedAcceptance system observations publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      observations
      publicInput
      proof
      observed

theorem timing_observation_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (observations : List TimingObservation) :
    forall publicInput proof,
      TimingObservedAcceptance system observations publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_core_and_sound
      assumptions
      observations
      publicInput
      proof
      observed

theorem timing_observation_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (observations : List TimingObservation) :
    forall publicInput proof,
      TimingObservedAcceptance system observations publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_audited_core_contract
      assumptions
      observations
      publicInput
      proof
      observed

def GuestPcTraceTimingObservedAcceptance
    (system : VerifierModel)
    (summary : Option GuestPcTraceTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system summary publicInput proof

theorem guest_pc_trace_timing_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem guest_pc_trace_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithGuestPcTraceTimings
  exact
    ignored_metadata_acceptance_sound
      assumptions
      summary
      publicInput
      proof
      acceptedWithGuestPcTraceTimings

theorem guest_pc_trace_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

theorem guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system (some summary) publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_verifier_core_contract
      assumptions
      (some summary)
      publicInput
      proof
      observed

theorem guest_pc_trace_timing_none_summary_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system none publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_verifier_core_contract
      assumptions
      none
      publicInput
      proof
      observed

theorem guest_pc_trace_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system summary publicInput proof ->
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

theorem guest_pc_trace_timing_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system summary publicInput proof ->
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

theorem guest_pc_trace_timing_some_summary_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system (some summary) publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_core_and_sound
      assumptions
      (some summary)
      publicInput
      proof
      observed

theorem guest_pc_trace_timing_some_summary_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : GuestPcTraceTimingSummary) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system (some summary) publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_audited_core_contract
      assumptions
      (some summary)
      publicInput
      proof
      observed

theorem guest_pc_trace_timing_none_summary_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system none publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_core_and_sound
      assumptions
      none
      publicInput
      proof
      observed

theorem guest_pc_trace_timing_none_summary_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      GuestPcTraceTimingObservedAcceptance system none publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    guest_pc_trace_timing_acceptance_audited_core_contract
      assumptions
      none
      publicInput
      proof
      observed

end Lzvm
