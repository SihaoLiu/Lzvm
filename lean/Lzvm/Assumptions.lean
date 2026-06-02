/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Model

/-!
Explicit cryptographic and semantic assumptions for the Lzvm soundness model.
-/

namespace Lzvm

/-!
The bundle below makes each cryptographic and semantic obligation explicit.
These assumptions are intentionally abstract: later proof work should replace
them with component-level theorems tied to concrete artifacts, verifier data,
and checked execution traces.
-/

structure CryptographicAssumptions (system : VerifierModel) : Prop where
  transcript_binding :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.transcriptBound publicInput proof
  pcs_opening_sound :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.pcsOpeningsValid publicInput proof
  fri_query_sound :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.friQueriesValid publicInput proof

structure SemanticAssumptions (system : VerifierModel) : Prop where
  public_input_binding :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.publicInputBound publicInput proof
  trace_extraction :
    forall publicInput proof,
      system.accepts publicInput proof ->
        exists trace, system.traceConsistent publicInput proof trace
  constraint_satisfaction :
    forall publicInput proof trace,
      system.accepts publicInput proof ->
        system.traceConsistent publicInput proof trace ->
          exists constraints, system.constraintsSatisfied constraints trace
  witness_extraction :
    forall publicInput proof trace constraints,
      system.accepts publicInput proof ->
        system.publicInputBound publicInput proof ->
          system.traceConsistent publicInput proof trace ->
            system.constraintsSatisfied constraints trace ->
              exists witness, system.witnessMatchesTrace witness trace

structure AssumptionBundle (system : VerifierModel) : Prop where
  crypto : CryptographicAssumptions system
  semantic : SemanticAssumptions system

end Lzvm
