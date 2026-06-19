/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.Timing

/-!
Batched runtime timing core-contract projections.
-/

namespace Lzvm

universe u

theorem timing_projected_metadata_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {Metadata : Type u}
    (metadata : Metadata) :
    forall publicInput proof,
      IgnoredMetadataObservedAcceptance system metadata publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      metadata
      publicInput
      proof
      observed

structure TimingProjectedCoreContracts
    (system : VerifierModel)
    (publicInput : PublicInput)
    (proof : Proof) : Prop where
  timingObservations :
    RuntimeVerifierCoreContract system publicInput proof
  guestPcTraceTiming :
    RuntimeVerifierCoreContract system publicInput proof

theorem timing_projected_core_contracts
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (observations : List TimingObservation)
    (summary : Option GuestPcTraceTimingSummary) :
    forall publicInput proof,
      TimingObservedAcceptance system observations publicInput proof ->
      GuestPcTraceTimingObservedAcceptance system summary publicInput proof ->
        TimingProjectedCoreContracts system publicInput proof := by
  intro publicInput proof timingObserved guestPcTraceObserved
  exact
    { timingObservations :=
        timing_projected_metadata_acceptance_verifier_core_contract
          assumptions
          observations
          publicInput
          proof
          timingObserved
      guestPcTraceTiming :=
        timing_projected_metadata_acceptance_verifier_core_contract
          assumptions
          summary
          publicInput
          proof
          guestPcTraceObserved }

end Lzvm
