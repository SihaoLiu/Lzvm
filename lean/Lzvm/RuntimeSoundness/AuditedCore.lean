/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.RuntimeSoundness.Core

/-!
Audited runtime soundness contracts derived from finalized core evidence.
-/

namespace Lzvm

theorem
  runtime_soundness_checked_acceptance_audited_core_sound_witness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have finalizedCore :=
    runtime_soundness_checked_acceptance_audited_finalized_core_sound_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  rcases finalizedCore with
    ⟨cryptoEvidence,
      semanticEvidence,
      _artifactFinalized,
      coreContract,
      executionObligations,
      soundWitness⟩
  exact
    ⟨cryptoEvidence,
      semanticEvidence,
      coreContract,
      executionObligations,
      soundWitness⟩

end Lzvm
