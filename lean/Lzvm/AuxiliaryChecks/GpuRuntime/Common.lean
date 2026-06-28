/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks

/-!
Shared GPU auxiliary runtime checked-acceptance chokepoints.
-/

namespace Lzvm

theorem gpu_runtime_checked_acceptance_sound_witness
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {auxiliaryAccepted : PublicInput -> Proof -> Prop} :
    forall publicInput proof,
      system.accepts publicInput proof
        /\ auxiliaryAccepted publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    auxiliary_checked_acceptance_sound_witness
      assumptions
      publicInput
      proof
      checked

theorem gpu_runtime_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {auxiliaryAccepted : PublicInput -> Proof -> Prop} :
    forall publicInput proof,
      system.accepts publicInput proof
        /\ auxiliaryAccepted publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    auxiliary_checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

end Lzvm
