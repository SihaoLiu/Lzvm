/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Assumptions
import Lzvm.MerklePathSoundness.Binary
import Lzvm.MerklePathSoundness.NAry

/-!
Auditable accessors for the centralized cryptographic assumption bundle.
-/

namespace Lzvm

universe uDigest

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

theorem required_crypto_assumptions_merkle_hash_collision_resistance
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    assumptions.hashCollisionResistance.merkleHashCollisionResistanceStatement := by
  rcases required with
    ⟨merkleHashCollisionResistance, _transcriptHashCollisionResistance,
      _randomOracleModel, _fiatShamirTranscriptBinding, _pcsBinding,
      _pcsOpeningSoundness, _friLowDegreeSoundness, _friQuerySoundness⟩
  exact merkleHashCollisionResistance

theorem required_crypto_assumptions_merkle_compression_no_collision
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (required : RequiredCryptographicAssumptionStatements assumptions)
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.hashCollisionResistance
        compress) :
    MerkleCompressionNoCollision compress := by
  exact
    Eq.mp
      centralized
      (required_crypto_assumptions_merkle_hash_collision_resistance
        required)

theorem required_crypto_assumptions_merkle_compression_collision_free
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (required : RequiredCryptographicAssumptionStatements assumptions)
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.hashCollisionResistance
        compress) :
    MerkleCompressionCollisionFree compress := by
  exact
    merkle_compression_collision_free_of_no_collision
      (required_crypto_assumptions_merkle_compression_no_collision
        required
        centralized)

theorem required_crypto_assumptions_nary_merkle_compression_no_collision
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (required : RequiredCryptographicAssumptionStatements assumptions)
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.hashCollisionResistance
        compress) :
    NAryMerkleCompressionNoCollision compress := by
  exact
    Eq.mp
      centralized
      (required_crypto_assumptions_merkle_hash_collision_resistance
        required)

theorem required_crypto_assumptions_nary_merkle_compression_collision_free
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (required : RequiredCryptographicAssumptionStatements assumptions)
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.hashCollisionResistance
        compress) :
    NAryMerkleCompressionCollisionFree compress := by
  exact
    nary_merkle_compression_collision_free_of_no_collision
      (required_crypto_assumptions_nary_merkle_compression_no_collision
        required
        centralized)

theorem required_crypto_assumptions_transcript_hash_collision_resistance
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    assumptions.hashCollisionResistance.transcriptHashCollisionResistanceStatement := by
  rcases required with
    ⟨_merkleHashCollisionResistance, transcriptHashCollisionResistance,
      _randomOracleModel, _fiatShamirTranscriptBinding, _pcsBinding,
      _pcsOpeningSoundness, _friLowDegreeSoundness, _friQuerySoundness⟩
  exact transcriptHashCollisionResistance

theorem required_crypto_assumptions_random_oracle_model
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    assumptions.randomOracleFiatShamir.randomOracleModelStatement := by
  rcases required with
    ⟨_merkleHashCollisionResistance, _transcriptHashCollisionResistance,
      randomOracleModel, _fiatShamirTranscriptBinding, _pcsBinding,
      _pcsOpeningSoundness, _friLowDegreeSoundness, _friQuerySoundness⟩
  exact randomOracleModel

theorem required_crypto_assumptions_fiat_shamir_transcript_binding
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.transcriptBound publicInput proof := by
  rcases required with
    ⟨_merkleHashCollisionResistance, _transcriptHashCollisionResistance,
      _randomOracleModel, fiatShamirTranscriptBinding, _pcsBinding,
      _pcsOpeningSoundness, _friLowDegreeSoundness, _friQuerySoundness⟩
  exact fiatShamirTranscriptBinding

theorem required_crypto_assumptions_pcs_binding
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    assumptions.pcsSoundness.pcsBindingStatement := by
  rcases required with
    ⟨_merkleHashCollisionResistance, _transcriptHashCollisionResistance,
      _randomOracleModel, _fiatShamirTranscriptBinding, pcsBinding,
      _pcsOpeningSoundness, _friLowDegreeSoundness, _friQuerySoundness⟩
  exact pcsBinding

theorem required_crypto_assumptions_pcs_opening_soundness
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.pcsOpeningsValid publicInput proof := by
  rcases required with
    ⟨_merkleHashCollisionResistance, _transcriptHashCollisionResistance,
      _randomOracleModel, _fiatShamirTranscriptBinding, _pcsBinding,
      pcsOpeningSoundness, _friLowDegreeSoundness, _friQuerySoundness⟩
  exact pcsOpeningSoundness

theorem required_crypto_assumptions_fri_low_degree_soundness
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    assumptions.friSoundness.friLowDegreeSoundnessStatement := by
  rcases required with
    ⟨_merkleHashCollisionResistance, _transcriptHashCollisionResistance,
      _randomOracleModel, _fiatShamirTranscriptBinding, _pcsBinding,
      _pcsOpeningSoundness, friLowDegreeSoundness, _friQuerySoundness⟩
  exact friLowDegreeSoundness

theorem required_crypto_assumptions_fri_query_soundness
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.friQueriesValid publicInput proof := by
  rcases required with
    ⟨_merkleHashCollisionResistance, _transcriptHashCollisionResistance,
      _randomOracleModel, _fiatShamirTranscriptBinding, _pcsBinding,
      _pcsOpeningSoundness, _friLowDegreeSoundness, friQuerySoundness⟩
  exact friQuerySoundness

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

theorem assumption_bundle_merkle_compression_no_collision
    {system : VerifierModel}
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (assumptions : AssumptionBundle system)
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    MerkleCompressionNoCollision compress := by
  exact
    required_crypto_assumptions_merkle_compression_no_collision
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      centralized

theorem assumption_bundle_merkle_compression_collision_free
    {system : VerifierModel}
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (assumptions : AssumptionBundle system)
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    MerkleCompressionCollisionFree compress := by
  exact
    required_crypto_assumptions_merkle_compression_collision_free
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      centralized

theorem assumption_bundle_nary_merkle_compression_no_collision
    {system : VerifierModel}
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (assumptions : AssumptionBundle system)
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    NAryMerkleCompressionNoCollision compress := by
  exact
    required_crypto_assumptions_nary_merkle_compression_no_collision
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      centralized

theorem assumption_bundle_nary_merkle_compression_collision_free
    {system : VerifierModel}
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (assumptions : AssumptionBundle system)
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress) :
    NAryMerkleCompressionCollisionFree compress := by
  exact
    required_crypto_assumptions_nary_merkle_compression_collision_free
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      centralized

def RequiredSemanticAssumptionStatements
    {system : VerifierModel}
    (_assumptions : SemanticAssumptions system) : Prop :=
  (forall publicInput proof,
      system.accepts publicInput proof ->
        system.publicInputBound publicInput proof)
    /\ (forall publicInput proof,
      system.accepts publicInput proof ->
        exists trace, system.traceConsistent publicInput proof trace)
    /\ (forall publicInput proof trace,
      system.accepts publicInput proof ->
        system.traceConsistent publicInput proof trace ->
          exists constraints, system.constraintsSatisfied constraints trace)
    /\ (forall publicInput proof trace constraints,
      system.accepts publicInput proof ->
        system.publicInputBound publicInput proof ->
          system.traceConsistent publicInput proof trace ->
            system.constraintsSatisfied constraints trace ->
              exists witness, system.witnessMatchesTrace witness trace)

theorem required_semantic_assumptions_public_input_binding
    {system : VerifierModel}
    {assumptions : SemanticAssumptions system}
    (required : RequiredSemanticAssumptionStatements assumptions) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.publicInputBound publicInput proof := by
  rcases required with
    ⟨publicInputBinding, _traceExtraction, _constraintSatisfaction,
      _witnessExtraction⟩
  exact publicInputBinding

theorem required_semantic_assumptions_trace_extraction
    {system : VerifierModel}
    {assumptions : SemanticAssumptions system}
    (required : RequiredSemanticAssumptionStatements assumptions) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        exists trace, system.traceConsistent publicInput proof trace := by
  rcases required with
    ⟨_publicInputBinding, traceExtraction, _constraintSatisfaction,
      _witnessExtraction⟩
  exact traceExtraction

theorem required_semantic_assumptions_constraint_satisfaction
    {system : VerifierModel}
    {assumptions : SemanticAssumptions system}
    (required : RequiredSemanticAssumptionStatements assumptions) :
    forall publicInput proof trace,
      system.accepts publicInput proof ->
        system.traceConsistent publicInput proof trace ->
          exists constraints, system.constraintsSatisfied constraints trace := by
  rcases required with
    ⟨_publicInputBinding, _traceExtraction, constraintSatisfaction,
      _witnessExtraction⟩
  exact constraintSatisfaction

theorem required_semantic_assumptions_witness_extraction
    {system : VerifierModel}
    {assumptions : SemanticAssumptions system}
    (required : RequiredSemanticAssumptionStatements assumptions) :
    forall publicInput proof trace constraints,
      system.accepts publicInput proof ->
        system.publicInputBound publicInput proof ->
          system.traceConsistent publicInput proof trace ->
            system.constraintsSatisfied constraints trace ->
              exists witness, system.witnessMatchesTrace witness trace := by
  rcases required with
    ⟨_publicInputBinding, _traceExtraction, _constraintSatisfaction,
      witnessExtraction⟩
  exact witnessExtraction

theorem semantic_assumptions_carry_required_evidence
    {system : VerifierModel}
    (assumptions : SemanticAssumptions system) :
    RequiredSemanticAssumptionStatements assumptions := by
  exact
    And.intro assumptions.public_input_binding
      (And.intro assumptions.trace_extraction
        (And.intro assumptions.constraint_satisfaction assumptions.witness_extraction))

theorem assumption_bundle_carries_required_semantic_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    RequiredSemanticAssumptionStatements assumptions.semantic := by
  exact semantic_assumptions_carry_required_evidence assumptions.semantic

end Lzvm
