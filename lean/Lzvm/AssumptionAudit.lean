/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Assumptions

/-!
Auditable accessors for the centralized cryptographic assumption bundle.
-/

namespace Lzvm

structure RequiredCryptographicAssumptionGroups (system : VerifierModel) where
  hashCollisionResistance : HashCollisionResistanceAssumption
  randomOracleFiatShamir : FiatShamirRandomOracleAssumption system
  pcsSoundness : PcsOpeningSoundnessAssumption system
  friSoundness : FriQuerySoundnessAssumption system

def RequiredCryptographicAssumptionStatements
    {system : VerifierModel}
    (assumptions : CryptographicAssumptions system) : Prop :=
  assumptions.hashCollisionResistance.merkleHashCollisionResistanceStatement
    /\ assumptions.hashCollisionResistance.transcriptHashCollisionResistanceStatement
    /\ assumptions.randomOracleFiatShamir.randomOracleModelStatement
    /\ (forall publicInput proof,
      system.accepts publicInput proof ->
        system.transcriptBound publicInput proof)
    /\ assumptions.pcsSoundness.pcsBindingStatement
    /\ (forall publicInput proof,
      system.accepts publicInput proof ->
        system.pcsOpeningsValid publicInput proof)
    /\ assumptions.friSoundness.friLowDegreeSoundnessStatement
    /\ (forall publicInput proof,
      system.accepts publicInput proof ->
        system.friQueriesValid publicInput proof)

def cryptographic_assumptions_required_groups
    {system : VerifierModel}
    (assumptions : CryptographicAssumptions system) :
    RequiredCryptographicAssumptionGroups system := by
  exact
    { hashCollisionResistance := assumptions.hashCollisionResistance
      randomOracleFiatShamir := assumptions.randomOracleFiatShamir
      pcsSoundness := assumptions.pcsSoundness
      friSoundness := assumptions.friSoundness }

theorem cryptographic_assumptions_carry_required_evidence
    {system : VerifierModel}
    (assumptions : CryptographicAssumptions system) :
    RequiredCryptographicAssumptionStatements assumptions := by
  exact
    And.intro assumptions.hashCollisionResistance.merkleHashCollisionResistance.evidence
      (And.intro assumptions.hashCollisionResistance.transcriptHashCollisionResistance.evidence
        (And.intro assumptions.randomOracleFiatShamir.randomOracleModel.evidence
          (And.intro assumptions.randomOracleFiatShamir.fiatShamirTranscriptBinding.evidence
            (And.intro assumptions.pcsSoundness.pcsBinding.evidence
              (And.intro assumptions.pcsSoundness.pcsOpeningSoundness.evidence
                (And.intro assumptions.friSoundness.friLowDegreeSoundness.evidence
                  assumptions.friSoundness.friQuerySoundness.evidence))))))

theorem assumption_bundle_carries_required_crypto_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    RequiredCryptographicAssumptionStatements assumptions.crypto := by
  exact cryptographic_assumptions_carry_required_evidence assumptions.crypto

end Lzvm
