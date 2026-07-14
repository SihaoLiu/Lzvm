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

theorem abstract_verifier_sound_with_semantic_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    RequiredSemanticAssumptionStatements assumptions.semantic
      /\ ProofSystemSound system := by
  exact
    And.intro
      (assumption_bundle_carries_required_semantic_evidence assumptions)
      (abstract_verifier_sound assumptions)

theorem abstract_verifier_sound_with_audited_soundness_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    RequiredCryptographicAssumptionStatements assumptions.crypto
      /\ RequiredSemanticAssumptionStatements assumptions.semantic
      /\ ProofSystemSound system := by
  have cryptoEvidence :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have semanticEvidence :=
    assumption_bundle_carries_required_semantic_evidence assumptions
  have auditedSound :=
    abstract_verifier_sound_with_audited_assumptions assumptions
  rcases auditedSound with
    ⟨_cryptoEvidenceFromAuditedSound, proofSystemSound⟩
  exact
    And.intro
      cryptoEvidence
      (And.intro
        semanticEvidence
        proofSystemSound)

theorem accepted_proof_required_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof accepted
  have cryptoEvidence :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have semanticEvidence :=
    assumption_bundle_carries_required_semantic_evidence assumptions
  exact
    ⟨cryptoEvidence,
      semanticEvidence,
      required_assumption_statements_verifier_core_contract
        cryptoEvidence
        semanticEvidence
        publicInput
        proof
        accepted⟩

theorem accepted_proof_crypto_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof accepted
  rcases
    accepted_proof_required_core_contract
      assumptions
      publicInput
      proof
      accepted with
    ⟨cryptoEvidence, _semanticEvidence, coreContract⟩
  exact ⟨cryptoEvidence, coreContract⟩

theorem accepted_proof_semantic_execution_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredSemanticAssumptionStatements assumptions.semantic
          /\ exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have semanticSound :=
    abstract_verifier_sound_with_semantic_evidence assumptions
  rcases semanticSound with
    ⟨semanticEvidence, proofSystemSound⟩
  have soundWitness :=
    proofSystemSound publicInput proof accepted
  rcases soundWitness with
    ⟨witness,
      trace,
      constraints,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    And.intro
      semanticEvidence
      (Exists.intro witness
        (Exists.intro trace
          (Exists.intro constraints
            (And.intro traceConsistent
              (And.intro constraintsSatisfied witnessMatchesTrace)))))

theorem accepted_proof_audited_core_and_sound_witness
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof accepted
  have cryptoCore :=
    accepted_proof_crypto_core_contract
      assumptions
      publicInput
      proof
      accepted
  have semanticSound :=
    abstract_verifier_sound_with_semantic_evidence assumptions
  rcases cryptoCore with
    ⟨cryptoEvidence, coreContract⟩
  rcases semanticSound with
    ⟨semanticEvidence, proofSystemSound⟩
  exact
    And.intro
      cryptoEvidence
      (And.intro
        semanticEvidence
        (And.intro coreContract
          (proofSystemSound publicInput proof accepted)))

theorem accepted_proof_audited_core_execution_and_sound_witness
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof accepted
  have proofSystemSound :=
    (abstract_verifier_sound_with_semantic_evidence assumptions).2
  have cryptoCore :=
    accepted_proof_crypto_core_contract
      assumptions
      publicInput
      proof
      accepted
  have semanticExecution :=
    accepted_proof_semantic_execution_obligations
      assumptions
      publicInput
      proof
      accepted
  exact
    ⟨cryptoCore.1,
      semanticExecution.1,
      cryptoCore.2,
      semanticExecution.2,
      proofSystemSound publicInput proof accepted⟩

theorem accepted_proof_audited_core_and_execution_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have cryptoCore :=
    accepted_proof_crypto_core_contract
      assumptions
      publicInput
      proof
      accepted
  have semanticExecution :=
    accepted_proof_semantic_execution_obligations
      assumptions
      publicInput
      proof
      accepted
  exact
    ⟨cryptoCore.1,
      semanticExecution.1,
      cryptoCore.2,
      semanticExecution.2⟩

theorem accepted_proof_audited_full_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ exists witness trace constraints,
            RuntimeVerifierCoreContract system publicInput proof
              /\ system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have cryptoCore :=
    accepted_proof_crypto_core_contract
      assumptions
      publicInput
      proof
      accepted
  have semanticExecution :=
    accepted_proof_semantic_execution_obligations
      assumptions
      publicInput
      proof
      accepted
  rcases cryptoCore with
    ⟨cryptoEvidence, coreContract⟩
  rcases semanticExecution with
    ⟨semanticEvidence, executionObligations⟩
  rcases executionObligations with
    ⟨witness,
      trace,
      constraints,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    And.intro cryptoEvidence
      (And.intro semanticEvidence
        (Exists.intro witness
          (Exists.intro trace
            (Exists.intro constraints
              (And.intro coreContract
                (And.intro traceConsistent
                  (And.intro constraintsSatisfied witnessMatchesTrace)))))))

theorem accepted_proof_audited_sound_witness_components
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ exists witness trace constraints,
            system.transcriptBound publicInput proof
              /\ system.publicInputBound publicInput proof
              /\ system.pcsOpeningsValid publicInput proof
              /\ system.friQueriesValid publicInput proof
              /\ system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have auditedSoundness :=
    abstract_verifier_sound_with_audited_soundness_obligations assumptions
  rcases auditedSoundness with
    ⟨cryptoEvidence, semanticEvidence, proofSystemSound⟩
  have soundWitness :=
    proofSystemSound publicInput proof accepted
  rcases soundWitness with
    ⟨witness,
      trace,
      constraints,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    And.intro cryptoEvidence
      (And.intro semanticEvidence
        (Exists.intro witness
          (Exists.intro trace
            (Exists.intro constraints
              (And.intro transcriptBound
                (And.intro publicInputBound
                  (And.intro pcsOpeningsValid
                    (And.intro friQueriesValid
                      (And.intro traceConsistent
                        (And.intro constraintsSatisfied witnessMatchesTrace))))))))))

theorem accepted_proof_audited_core_and_sound_witness_components
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ exists witness trace constraints,
            system.transcriptBound publicInput proof
              /\ system.publicInputBound publicInput proof
              /\ system.pcsOpeningsValid publicInput proof
              /\ system.friQueriesValid publicInput proof
              /\ system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have cryptoCore :=
    accepted_proof_crypto_core_contract
      assumptions
      publicInput
      proof
      accepted
  have semanticSound :=
    abstract_verifier_sound_with_semantic_evidence assumptions
  rcases cryptoCore with
    ⟨cryptoEvidence, coreContract⟩
  rcases semanticSound with
    ⟨semanticEvidence, proofSystemSound⟩
  have soundWitness :=
    proofSystemSound publicInput proof accepted
  rcases soundWitness with
    ⟨witness,
      trace,
      constraints,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    And.intro cryptoEvidence
      (And.intro semanticEvidence
        (And.intro coreContract
          (Exists.intro witness
            (Exists.intro trace
              (Exists.intro constraints
                (And.intro transcriptBound
                  (And.intro publicInputBound
                    (And.intro pcsOpeningsValid
                      (And.intro friQueriesValid
                        (And.intro traceConsistent
                          (And.intro constraintsSatisfied witnessMatchesTrace)))))))))))

theorem accepted_proof_audited_proof_system_and_components
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ ProofSystemSound system
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ exists witness trace constraints,
            system.transcriptBound publicInput proof
              /\ system.publicInputBound publicInput proof
              /\ system.pcsOpeningsValid publicInput proof
              /\ system.friQueriesValid publicInput proof
              /\ system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have cryptoCore :=
    accepted_proof_crypto_core_contract
      assumptions
      publicInput
      proof
      accepted
  have semanticSound :=
    abstract_verifier_sound_with_semantic_evidence assumptions
  rcases cryptoCore with
    ⟨cryptoEvidence, coreContract⟩
  rcases semanticSound with
    ⟨semanticEvidence, proofSystemSound⟩
  have soundWitness := proofSystemSound publicInput proof accepted
  rcases soundWitness with
    ⟨witness,
      trace,
      constraints,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    And.intro cryptoEvidence
      (And.intro semanticEvidence
        (And.intro proofSystemSound
          (And.intro coreContract
            (Exists.intro witness
              (Exists.intro trace
                (Exists.intro constraints
                  (And.intro transcriptBound
                    (And.intro publicInputBound
                      (And.intro pcsOpeningsValid
                        (And.intro friQueriesValid
                          (And.intro traceConsistent
                            (And.intro constraintsSatisfied witnessMatchesTrace))))))))))))

theorem accepted_proof_audited_proof_system_core_and_execution_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ ProofSystemSound system
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have proofSystemSound :=
    (abstract_verifier_sound_with_semantic_evidence assumptions).2
  have cryptoCore :=
    accepted_proof_crypto_core_contract
      assumptions
      publicInput
      proof
      accepted
  have semanticExecution :=
    accepted_proof_semantic_execution_obligations
      assumptions
      publicInput
      proof
      accepted
  exact
    ⟨cryptoCore.1,
      semanticExecution.1,
      proofSystemSound,
      cryptoCore.2,
      semanticExecution.2⟩

theorem accepted_proof_audited_proof_system_core_execution_and_sound_witness
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ ProofSystemSound system
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof accepted
  have proofSystemSound :=
    (abstract_verifier_sound_with_semantic_evidence assumptions).2
  have cryptoCore :=
    accepted_proof_crypto_core_contract
      assumptions
      publicInput
      proof
      accepted
  have semanticExecution :=
    accepted_proof_semantic_execution_obligations
      assumptions
      publicInput
      proof
      accepted
  exact
    ⟨cryptoCore.1,
      semanticExecution.1,
      proofSystemSound,
      cryptoCore.2,
      semanticExecution.2,
      proofSystemSound publicInput proof accepted⟩

theorem accepted_proof_audited_flat_proof_system_components
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ ProofSystemSound system
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have proofSystemSound :=
    (abstract_verifier_sound_with_semantic_evidence assumptions).2
  have cryptoCore :=
    accepted_proof_crypto_core_contract
      assumptions
      publicInput
      proof
      accepted
  have semanticExecution :=
    accepted_proof_semantic_execution_obligations
      assumptions
      publicInput
      proof
      accepted
  rcases cryptoCore.2 with
    ⟨transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid⟩
  rcases semanticExecution.2 with
    ⟨witness,
      trace,
      constraints,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    ⟨cryptoCore.1,
      semanticExecution.1,
      proofSystemSound,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      witness,
      trace,
      constraints,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩

end Lzvm
