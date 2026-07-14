/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.OpeningSegmentBinding

/-!
Full opening-segment soundness contracts with parser and fold-order evidence.
-/

namespace Lzvm

set_option linter.style.longLine false in
theorem runtime_opening_segment_binding_checked_acceptance_full_soundness_with_fri_parser_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system)
    (boundary : RuntimeFriOpeningSegmentParserBoundary system validation) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingBoundContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeOpeningBoundContract
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof
          /\ RuntimeFriOpeningSegmentParserContract
            boundary
            artifact
            publicInput
            proof
          /\ RuntimeFriFoldTraceIdentityContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeFriFoldQueryPlanOrderContract
            system
            validation
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have fullContract :=
    runtime_opening_segment_binding_checked_acceptance_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have parserContract :=
    runtime_opening_segment_binding_checked_acceptance_fri_parser_contract
      validation
      boundary
      artifact
      publicInput
      proof
      accepted
  have foldTraceIdentityContract :=
    runtime_opening_segment_binding_checked_acceptance_fri_fold_trace_identity_contract
      validation
      artifact
      publicInput
      proof
      accepted
  have foldQueryPlanOrderContract :=
    runtime_opening_segment_binding_checked_acceptance_fri_fold_query_plan_order_contract
      validation
      artifact
      publicInput
      proof
      accepted
  rcases fullContract with
    ⟨segmentBound,
      openingEvidence,
      openingBound,
      pcsOpenings,
      friQueries,
      coreContract,
      soundWitness⟩
  exact
    And.intro segmentBound
      (And.intro openingEvidence
        (And.intro openingBound
          (And.intro pcsOpenings
            (And.intro friQueries
              (And.intro coreContract
                (And.intro soundWitness
                  (And.intro parserContract
                    (And.intro foldTraceIdentityContract foldQueryPlanOrderContract))))))))

end Lzvm
