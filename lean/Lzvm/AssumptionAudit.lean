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

structure RequiredCryptographicAssumptionStatements
    {system : VerifierModel}
    (assumptions : CryptographicAssumptions system) : Prop where
  merkleHashCollisionResistance :
    assumptions.hashCollisionResistance.merkleHashCollisionResistanceStatement
  transcriptHashCollisionResistance :
    assumptions.hashCollisionResistance.transcriptHashCollisionResistanceStatement
  randomOracleModel :
    assumptions.randomOracleFiatShamir.randomOracleModelStatement
  fiatShamirTranscriptBinding :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.transcriptBound publicInput proof
  pcsBinding :
    assumptions.pcsSoundness.pcsBindingStatement
  pcsOpeningSoundness :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.pcsOpeningsValid publicInput proof
  friLowDegreeSoundness :
    assumptions.friSoundness.friLowDegreeSoundnessStatement
  friQuerySoundness :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.friQueriesValid publicInput proof

def cryptographic_assumptions_required_groups
    {system : VerifierModel}
    (assumptions : CryptographicAssumptions system) :
    RequiredCryptographicAssumptionGroups system := by
  exact
    { hashCollisionResistance := assumptions.hashCollisionResistance
      randomOracleFiatShamir := assumptions.randomOracleFiatShamir
      pcsSoundness := assumptions.pcsSoundness
      friSoundness := assumptions.friSoundness }

theorem cryptographic_assumptions_required_groups_fields
    {system : VerifierModel}
    (assumptions : CryptographicAssumptions system) :
    (cryptographic_assumptions_required_groups assumptions).hashCollisionResistance =
        assumptions.hashCollisionResistance
      /\ (cryptographic_assumptions_required_groups assumptions).randomOracleFiatShamir =
        assumptions.randomOracleFiatShamir
      /\ (cryptographic_assumptions_required_groups assumptions).pcsSoundness =
        assumptions.pcsSoundness
      /\ (cryptographic_assumptions_required_groups assumptions).friSoundness =
        assumptions.friSoundness := by
  exact And.intro rfl (And.intro rfl (And.intro rfl rfl))

theorem required_crypto_assumptions_merkle_hash_collision_resistance
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    assumptions.hashCollisionResistance.merkleHashCollisionResistanceStatement := by
  exact required.merkleHashCollisionResistance

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
  exact required.transcriptHashCollisionResistance

theorem required_crypto_assumptions_random_oracle_model
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    assumptions.randomOracleFiatShamir.randomOracleModelStatement := by
  exact required.randomOracleModel

theorem required_crypto_assumptions_fiat_shamir_transcript_binding
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.transcriptBound publicInput proof := by
  exact required.fiatShamirTranscriptBinding

theorem required_crypto_assumptions_pcs_binding
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    assumptions.pcsSoundness.pcsBindingStatement := by
  exact required.pcsBinding

theorem required_crypto_assumptions_pcs_opening_soundness
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.pcsOpeningsValid publicInput proof := by
  exact required.pcsOpeningSoundness

theorem required_crypto_assumptions_fri_low_degree_soundness
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    assumptions.friSoundness.friLowDegreeSoundnessStatement := by
  exact required.friLowDegreeSoundness

theorem required_crypto_assumptions_fri_query_soundness
    {system : VerifierModel}
    {assumptions : CryptographicAssumptions system}
    (required : RequiredCryptographicAssumptionStatements assumptions) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.friQueriesValid publicInput proof := by
  exact required.friQuerySoundness

theorem cryptographic_assumptions_carry_required_evidence
    {system : VerifierModel}
    (assumptions : CryptographicAssumptions system) :
    RequiredCryptographicAssumptionStatements assumptions := by
  exact
    { merkleHashCollisionResistance :=
        HashCollisionResistanceAssumption.merkle_hash_collision_resistance
          assumptions.hashCollisionResistance
      transcriptHashCollisionResistance :=
        HashCollisionResistanceAssumption.transcript_hash_collision_resistance
          assumptions.hashCollisionResistance
      randomOracleModel :=
        FiatShamirRandomOracleAssumption.random_oracle_model
          assumptions.randomOracleFiatShamir
      fiatShamirTranscriptBinding :=
        CryptographicAssumptions.transcript_binding assumptions
      pcsBinding :=
        PcsOpeningSoundnessAssumption.pcs_binding assumptions.pcsSoundness
      pcsOpeningSoundness :=
        CryptographicAssumptions.pcs_opening_sound assumptions
      friLowDegreeSoundness :=
        FriQuerySoundnessAssumption.fri_low_degree_soundness
          assumptions.friSoundness
      friQuerySoundness :=
        CryptographicAssumptions.fri_query_sound assumptions }

theorem assumption_bundle_carries_required_crypto_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    RequiredCryptographicAssumptionStatements assumptions.crypto := by
  exact cryptographic_assumptions_carry_required_evidence assumptions.crypto

theorem assumption_bundle_merkle_hash_collision_resistance
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    assumptions.crypto.hashCollisionResistance.merkleHashCollisionResistanceStatement := by
  exact
    required_crypto_assumptions_merkle_hash_collision_resistance
      (assumption_bundle_carries_required_crypto_evidence assumptions)

theorem assumption_bundle_transcript_hash_collision_resistance
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    assumptions.crypto.hashCollisionResistance.transcriptHashCollisionResistanceStatement := by
  exact
    required_crypto_assumptions_transcript_hash_collision_resistance
      (assumption_bundle_carries_required_crypto_evidence assumptions)

theorem assumption_bundle_random_oracle_model
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    assumptions.crypto.randomOracleFiatShamir.randomOracleModelStatement := by
  exact
    required_crypto_assumptions_random_oracle_model
      (assumption_bundle_carries_required_crypto_evidence assumptions)

theorem assumption_bundle_fiat_shamir_transcript_binding
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.transcriptBound publicInput proof := by
  exact
    required_crypto_assumptions_fiat_shamir_transcript_binding
      (assumption_bundle_carries_required_crypto_evidence assumptions)

theorem assumption_bundle_pcs_binding
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    assumptions.crypto.pcsSoundness.pcsBindingStatement := by
  exact
    required_crypto_assumptions_pcs_binding
      (assumption_bundle_carries_required_crypto_evidence assumptions)

theorem assumption_bundle_pcs_opening_soundness
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.pcsOpeningsValid publicInput proof := by
  exact
    required_crypto_assumptions_pcs_opening_soundness
      (assumption_bundle_carries_required_crypto_evidence assumptions)

theorem assumption_bundle_fri_low_degree_soundness
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    assumptions.crypto.friSoundness.friLowDegreeSoundnessStatement := by
  exact
    required_crypto_assumptions_fri_low_degree_soundness
      (assumption_bundle_carries_required_crypto_evidence assumptions)

theorem assumption_bundle_fri_query_soundness
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.friQueriesValid publicInput proof := by
  exact
    required_crypto_assumptions_fri_query_soundness
      (assumption_bundle_carries_required_crypto_evidence assumptions)

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

structure RequiredSemanticAssumptionStatements
    {system : VerifierModel}
    (_assumptions : SemanticAssumptions system) : Prop where
  publicInputBinding :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.publicInputBound publicInput proof
  traceExtraction :
    forall publicInput proof,
      system.accepts publicInput proof ->
        exists trace, system.traceConsistent publicInput proof trace
  constraintSatisfaction :
    forall publicInput proof trace,
      system.accepts publicInput proof ->
        system.traceConsistent publicInput proof trace ->
          exists constraints, system.constraintsSatisfied constraints trace
  witnessExtraction :
    forall publicInput proof trace constraints,
      system.accepts publicInput proof ->
        system.publicInputBound publicInput proof ->
          system.traceConsistent publicInput proof trace ->
            system.constraintsSatisfied constraints trace ->
              exists witness, system.witnessMatchesTrace witness trace

theorem required_semantic_assumptions_public_input_binding
    {system : VerifierModel}
    {assumptions : SemanticAssumptions system}
    (required : RequiredSemanticAssumptionStatements assumptions) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.publicInputBound publicInput proof := by
  exact required.publicInputBinding

theorem required_semantic_assumptions_trace_extraction
    {system : VerifierModel}
    {assumptions : SemanticAssumptions system}
    (required : RequiredSemanticAssumptionStatements assumptions) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        exists trace, system.traceConsistent publicInput proof trace := by
  exact required.traceExtraction

theorem required_semantic_assumptions_constraint_satisfaction
    {system : VerifierModel}
    {assumptions : SemanticAssumptions system}
    (required : RequiredSemanticAssumptionStatements assumptions) :
    forall publicInput proof trace,
      system.accepts publicInput proof ->
        system.traceConsistent publicInput proof trace ->
          exists constraints, system.constraintsSatisfied constraints trace := by
  exact required.constraintSatisfaction

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
  exact required.witnessExtraction

theorem semantic_assumptions_carry_required_evidence
    {system : VerifierModel}
    (assumptions : SemanticAssumptions system) :
    RequiredSemanticAssumptionStatements assumptions := by
  exact
    { publicInputBinding := assumptions.public_input_binding
      traceExtraction := assumptions.trace_extraction
      constraintSatisfaction := assumptions.constraint_satisfaction
      witnessExtraction := assumptions.witness_extraction }

theorem assumption_bundle_carries_required_semantic_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    RequiredSemanticAssumptionStatements assumptions.semantic := by
  exact semantic_assumptions_carry_required_evidence assumptions.semantic

theorem assumption_bundle_carries_required_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    RequiredCryptographicAssumptionStatements assumptions.crypto
      /\ RequiredSemanticAssumptionStatements assumptions.semantic := by
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (assumption_bundle_carries_required_semantic_evidence assumptions)

theorem assumption_bundle_public_input_binding
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        system.publicInputBound publicInput proof := by
  exact
    required_semantic_assumptions_public_input_binding
      (assumption_bundle_carries_required_semantic_evidence assumptions)

theorem assumption_bundle_trace_extraction
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        exists trace, system.traceConsistent publicInput proof trace := by
  exact
    required_semantic_assumptions_trace_extraction
      (assumption_bundle_carries_required_semantic_evidence assumptions)

theorem assumption_bundle_constraint_satisfaction
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof trace,
      system.accepts publicInput proof ->
        system.traceConsistent publicInput proof trace ->
          exists constraints, system.constraintsSatisfied constraints trace := by
  exact
    required_semantic_assumptions_constraint_satisfaction
      (assumption_bundle_carries_required_semantic_evidence assumptions)

theorem assumption_bundle_witness_extraction
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof trace constraints,
      system.accepts publicInput proof ->
        system.publicInputBound publicInput proof ->
          system.traceConsistent publicInput proof trace ->
            system.constraintsSatisfied constraints trace ->
              exists witness, system.witnessMatchesTrace witness trace := by
  exact
    required_semantic_assumptions_witness_extraction
      (assumption_bundle_carries_required_semantic_evidence assumptions)

theorem required_assumption_statements_verifier_core_contract
    {system : VerifierModel}
    {crypto : CryptographicAssumptions system}
    {semantic : SemanticAssumptions system}
    (cryptoRequired : RequiredCryptographicAssumptionStatements crypto)
    (semanticRequired : RequiredSemanticAssumptionStatements semantic) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof accepted
  exact
    ⟨required_crypto_assumptions_fiat_shamir_transcript_binding
        cryptoRequired
        publicInput
        proof
        accepted,
      required_semantic_assumptions_public_input_binding
        semanticRequired
        publicInput
        proof
        accepted,
      required_crypto_assumptions_pcs_opening_soundness
        cryptoRequired
        publicInput
        proof
        accepted,
      required_crypto_assumptions_fri_query_soundness
        cryptoRequired
        publicInput
        proof
        accepted⟩

theorem assumption_bundle_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  exact
    required_assumption_statements_verifier_core_contract
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (assumption_bundle_carries_required_semantic_evidence assumptions)

end Lzvm
