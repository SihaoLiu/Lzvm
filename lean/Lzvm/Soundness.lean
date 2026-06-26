/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AssumptionAudit

/-!
Composition theorem for the abstract Lzvm verifier soundness model.
-/

namespace Lzvm

theorem abstract_verifier_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    ProofSystemSound system := by
  intro publicInput proof accepted
  have coreContract :=
    assumption_bundle_verifier_core_contract
      assumptions
      publicInput
      proof
      accepted
  rcases coreContract with
    ⟨transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid⟩
  cases assumption_bundle_trace_extraction assumptions publicInput proof accepted with
  | intro trace traceConsistent =>
    cases assumption_bundle_constraint_satisfaction
        assumptions
        publicInput proof trace accepted traceConsistent with
    | intro constraints constraintsSatisfied =>
      cases assumption_bundle_witness_extraction
          assumptions
          publicInput
          proof
          trace
          constraints
          accepted
          publicInputBound
          traceConsistent
          constraintsSatisfied with
      | intro witness witnessMatchesTrace =>
        exact
          Exists.intro witness
            (Exists.intro trace
              (Exists.intro constraints
                (And.intro transcriptBound
                  (And.intro publicInputBound
                    (And.intro pcsOpeningsValid
                      (And.intro friQueriesValid
                        (And.intro traceConsistent
                          (And.intro constraintsSatisfied witnessMatchesTrace))))))))

theorem abstract_verifier_sound_with_audited_assumptions
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    RequiredCryptographicAssumptionStatements assumptions.crypto
      /\ ProofSystemSound system := by
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (abstract_verifier_sound assumptions)

theorem abstract_verifier_sound_with_audited_soundness_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    RequiredCryptographicAssumptionStatements assumptions.crypto
      /\ RequiredSemanticAssumptionStatements assumptions.semantic
      /\ ProofSystemSound system := by
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  have sound :=
    abstract_verifier_sound_with_audited_assumptions assumptions
  exact
    And.intro auditedAssumptions.left
      (And.intro auditedAssumptions.right sound.right)

end Lzvm
