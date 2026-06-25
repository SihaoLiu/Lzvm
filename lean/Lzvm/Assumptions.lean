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

abbrev NamedCryptographicAssumption (statement : Prop) : Prop :=
  statement

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
  assumptions.merkleHashCollisionResistance

theorem transcript_hash_collision_resistance
    (assumptions : HashCollisionResistanceAssumption) :
    assumptions.transcriptHashCollisionResistanceStatement :=
  assumptions.transcriptHashCollisionResistance

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

namespace FiatShamirRandomOracleAssumption

theorem random_oracle_model
    {system : VerifierModel}
    (assumptions : FiatShamirRandomOracleAssumption system) :
    assumptions.randomOracleModelStatement :=
  assumptions.randomOracleModel

theorem fiat_shamir_transcript_binding
    {system : VerifierModel}
    (assumptions : FiatShamirRandomOracleAssumption system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.transcriptBound publicInput proof :=
  assumptions.fiatShamirTranscriptBinding

end FiatShamirRandomOracleAssumption

structure PcsOpeningSoundnessAssumption (system : VerifierModel) where
  pcsBindingStatement : Prop
  pcsBinding : NamedCryptographicAssumption pcsBindingStatement
  pcsOpeningSoundness :
    NamedCryptographicAssumption
      (forall publicInput proof,
        system.accepts publicInput proof ->
          system.pcsOpeningsValid publicInput proof)

namespace PcsOpeningSoundnessAssumption

theorem pcs_binding
    {system : VerifierModel}
    (assumptions : PcsOpeningSoundnessAssumption system) :
    assumptions.pcsBindingStatement :=
  assumptions.pcsBinding

theorem pcs_opening_soundness
    {system : VerifierModel}
    (assumptions : PcsOpeningSoundnessAssumption system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.pcsOpeningsValid publicInput proof :=
  assumptions.pcsOpeningSoundness

end PcsOpeningSoundnessAssumption

structure FriQuerySoundnessAssumption (system : VerifierModel) where
  friLowDegreeSoundnessStatement : Prop
  friLowDegreeSoundness :
    NamedCryptographicAssumption friLowDegreeSoundnessStatement
  friQuerySoundness :
    NamedCryptographicAssumption
      (forall publicInput proof,
        system.accepts publicInput proof ->
          system.friQueriesValid publicInput proof)

namespace FriQuerySoundnessAssumption

theorem fri_low_degree_soundness
    {system : VerifierModel}
    (assumptions : FriQuerySoundnessAssumption system) :
    assumptions.friLowDegreeSoundnessStatement :=
  assumptions.friLowDegreeSoundness

theorem fri_query_soundness
    {system : VerifierModel}
    (assumptions : FriQuerySoundnessAssumption system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.friQueriesValid publicInput proof :=
  assumptions.friQuerySoundness

end FriQuerySoundnessAssumption

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
  assumptions.randomOracleFiatShamir.fiat_shamir_transcript_binding

def pcs_opening_sound
    {system : VerifierModel}
    (assumptions : CryptographicAssumptions system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.pcsOpeningsValid publicInput proof :=
  assumptions.pcsSoundness.pcs_opening_soundness

def fri_query_sound
    {system : VerifierModel}
    (assumptions : CryptographicAssumptions system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.friQueriesValid publicInput proof :=
  assumptions.friSoundness.fri_query_soundness

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
