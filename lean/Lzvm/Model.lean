/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Mathlib

/-!
Abstract proof-system objects used by the Lzvm soundness model.
-/

namespace Lzvm

/-!
This module defines an abstract verifier model. It is the entry point for a
machine-checked proof flow, not a proof that the Rust or CUDA implementation is
already sound. Concrete implementation soundness must later connect these
predicates to checked artifacts and conformance evidence.
-/

structure PublicInput where
  id : Nat
deriving DecidableEq, Repr

structure Proof where
  id : Nat
  segmentIds : List Nat := []
deriving DecidableEq, Repr

structure Witness where
  id : Nat
deriving DecidableEq, Repr

structure Trace where
  id : Nat
deriving DecidableEq, Repr

structure ConstraintSystem where
  id : Nat
deriving DecidableEq, Repr

structure VerifierModel where
  accepts : PublicInput -> Proof -> Prop
  transcriptBound : PublicInput -> Proof -> Prop
  publicInputBound : PublicInput -> Proof -> Prop
  pcsOpeningsValid : PublicInput -> Proof -> Prop
  friQueriesValid : PublicInput -> Proof -> Prop
  traceConsistent : PublicInput -> Proof -> Trace -> Prop
  constraintsSatisfied : ConstraintSystem -> Trace -> Prop
  witnessMatchesTrace : Witness -> Trace -> Prop

def RuntimeVerifierCoreContract
    (system : VerifierModel)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.transcriptBound publicInput proof
    /\ system.publicInputBound publicInput proof
    /\ system.pcsOpeningsValid publicInput proof
    /\ system.friQueriesValid publicInput proof

def SoundWitness
    (system : VerifierModel)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  exists witness trace constraints,
    system.transcriptBound publicInput proof
      /\ system.publicInputBound publicInput proof
      /\ system.pcsOpeningsValid publicInput proof
      /\ system.friQueriesValid publicInput proof
      /\ system.traceConsistent publicInput proof trace
      /\ system.constraintsSatisfied constraints trace
      /\ system.witnessMatchesTrace witness trace

theorem sound_witness_implies_verifier_core_contract
    {system : VerifierModel}
    {publicInput : PublicInput}
    {proof : Proof} :
    SoundWitness system publicInput proof ->
      RuntimeVerifierCoreContract system publicInput proof := by
  intro soundWitness
  cases soundWitness with
  | intro _witness tail =>
    cases tail with
    | intro _trace tail =>
      cases tail with
      | intro _constraints evidence =>
        exact
          And.intro evidence.left
            (And.intro evidence.right.left
              (And.intro evidence.right.right.left evidence.right.right.right.left))

theorem sound_witness_implies_execution_obligations
    {system : VerifierModel}
    {publicInput : PublicInput}
    {proof : Proof} :
    SoundWitness system publicInput proof ->
      exists witness trace constraints,
        system.traceConsistent publicInput proof trace
          /\ system.constraintsSatisfied constraints trace
          /\ system.witnessMatchesTrace witness trace := by
  intro soundWitness
  rcases soundWitness with
    ⟨witness,
      trace,
      constraints,
      _transcriptBound,
      _publicInputBound,
      _pcsOpenings,
      _friQueries,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    ⟨witness,
      trace,
      constraints,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩

def ProofSystemSound (system : VerifierModel) : Prop :=
  forall publicInput proof,
    system.accepts publicInput proof -> SoundWitness system publicInput proof

end Lzvm
