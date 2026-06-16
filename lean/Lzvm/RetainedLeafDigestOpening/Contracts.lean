/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.RetainedLeafDigestOpening

/-!
Compact source/core contracts for retained leaf digest opening checks.
-/

namespace Lzvm

universe uDigest

theorem runtime_retained_leaf_digest_nary_opening_source_and_core_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedLeafDigestOpeningDigestContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedLeafDigestOpeningShiftedRowSourceContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedLeafDigestOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_nary_opening_digest_contract_from_bundle
        assumptions
        validation
        centralized
        binding
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (runtime_retained_leaf_digest_opening_checked_acceptance_shifted_row_source_contract
          validation
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (runtime_retained_leaf_digest_opening_checked_acceptance_retained_rows_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)
          (runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)))

theorem runtime_retained_leaf_digest_opening_checked_acceptance_source_and_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedLeafDigestOpeningShiftedRowSourceContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedLeafDigestOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_opening_checked_acceptance_shifted_row_source_contract
        validation
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (runtime_retained_leaf_digest_opening_checked_acceptance_retained_rows_contract
          assumptions
          validation
          artifact
          publicInput
          proof
          accepted)
        (runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract
          assumptions
          validation
          artifact
          publicInput
          proof
          accepted))

end Lzvm
