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

structure NamedCryptographicAssumption (statement : Prop) where
  evidence : statement

structure HashCollisionResistanceAssumption where
  merkleHashCollisionResistanceStatement : Prop
  transcriptHashCollisionResistanceStatement : Prop
  merkleHashCollisionResistance :
    NamedCryptographicAssumption merkleHashCollisionResistanceStatement
  transcriptHashCollisionResistance :
    NamedCryptographicAssumption transcriptHashCollisionResistanceStatement

namespace HashCollisionResistanceAssumption

theorem merkle_hash_collision_resistance
    (assumptions : HashCollisionResistanceAssumption) :
    assumptions.merkleHashCollisionResistanceStatement :=
  assumptions.merkleHashCollisionResistance.evidence

end HashCollisionResistanceAssumption

structure FiatShamirRandomOracleAssumption (system : VerifierModel) where
  randomOracleModelStatement : Prop
  randomOracleModel :
    NamedCryptographicAssumption randomOracleModelStatement
  fiatShamirTranscriptBinding :
    NamedCryptographicAssumption
      (forall publicInput proof,
        system.accepts publicInput proof ->
          system.transcriptBound publicInput proof)

structure PcsOpeningSoundnessAssumption (system : VerifierModel) where
  pcsBindingStatement : Prop
  pcsBinding : NamedCryptographicAssumption pcsBindingStatement
  pcsOpeningSoundness :
    NamedCryptographicAssumption
      (forall publicInput proof,
        system.accepts publicInput proof ->
          system.pcsOpeningsValid publicInput proof)

structure FriQuerySoundnessAssumption (system : VerifierModel) where
  friLowDegreeSoundnessStatement : Prop
  friLowDegreeSoundness :
    NamedCryptographicAssumption friLowDegreeSoundnessStatement
  friQuerySoundness :
    NamedCryptographicAssumption
      (forall publicInput proof,
        system.accepts publicInput proof ->
          system.friQueriesValid publicInput proof)

structure CryptographicAssumptions (system : VerifierModel) where
  hashCollisionResistance : HashCollisionResistanceAssumption
  randomOracleFiatShamir : FiatShamirRandomOracleAssumption system
  pcsSoundness : PcsOpeningSoundnessAssumption system
  friSoundness : FriQuerySoundnessAssumption system

namespace CryptographicAssumptions

def transcript_binding
    {system : VerifierModel}
    (assumptions : CryptographicAssumptions system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.transcriptBound publicInput proof :=
  assumptions.randomOracleFiatShamir.fiatShamirTranscriptBinding.evidence

def pcs_opening_sound
    {system : VerifierModel}
    (assumptions : CryptographicAssumptions system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.pcsOpeningsValid publicInput proof :=
  assumptions.pcsSoundness.pcsOpeningSoundness.evidence

def fri_query_sound
    {system : VerifierModel}
    (assumptions : CryptographicAssumptions system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.friQueriesValid publicInput proof :=
  assumptions.friSoundness.friQuerySoundness.evidence

end CryptographicAssumptions

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

structure AssumptionBundle (system : VerifierModel) where
  crypto : CryptographicAssumptions system
  semantic : SemanticAssumptions system

end Lzvm
