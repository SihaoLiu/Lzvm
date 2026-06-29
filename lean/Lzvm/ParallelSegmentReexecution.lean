/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Conformance

/-!
Runtime obligations for parallel segment reexecution.
-/

namespace Lzvm

structure RuntimeParallelSegmentReexecutionValidation (system : VerifierModel) where
  parallelReexecutionAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  serialExecutionAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  segmentIndex : RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  emittedInVerifierOrder : RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  segmentSeedMatchesPreviousTransition :
    RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  segmentOutputMatchesSerialExecution :
    RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  noSegmentDropped : RuntimeArtifact -> PublicInput -> Proof -> Prop
  noSegmentDuplicated : RuntimeArtifact -> PublicInput -> Proof -> Prop
  outOfOrderSegmentRejected : RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  missingSeedRejected : RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  parallelReexecutionAcceptedImpliesSerialExecutionAccepted :
    forall artifact publicInput proof,
      parallelReexecutionAccepted artifact publicInput proof ->
        serialExecutionAccepted artifact publicInput proof
  parallelReexecutionAcceptedImpliesNoSegmentDropped :
    forall artifact publicInput proof,
      parallelReexecutionAccepted artifact publicInput proof ->
        noSegmentDropped artifact publicInput proof
  parallelReexecutionAcceptedImpliesNoSegmentDuplicated :
    forall artifact publicInput proof,
      parallelReexecutionAccepted artifact publicInput proof ->
        noSegmentDuplicated artifact publicInput proof
  parallelReexecutionAcceptedImpliesOrderedEmission :
    forall artifact publicInput proof segment,
      parallelReexecutionAccepted artifact publicInput proof ->
        segmentIndex artifact publicInput proof segment ->
          emittedInVerifierOrder artifact publicInput proof segment
  parallelReexecutionAcceptedImpliesSeedChain :
    forall artifact publicInput proof segment,
      parallelReexecutionAccepted artifact publicInput proof ->
        segmentIndex artifact publicInput proof segment ->
          segmentSeedMatchesPreviousTransition artifact publicInput proof segment
  parallelReexecutionAcceptedImpliesSerialEquivalentSegment :
    forall artifact publicInput proof segment,
      parallelReexecutionAccepted artifact publicInput proof ->
        segmentIndex artifact publicInput proof segment ->
          segmentOutputMatchesSerialExecution artifact publicInput proof segment
  parallelReexecutionAcceptedAndNotOrderedImpliesRejected :
    forall artifact publicInput proof segment,
      parallelReexecutionAccepted artifact publicInput proof ->
        segmentIndex artifact publicInput proof segment ->
          ¬ emittedInVerifierOrder artifact publicInput proof segment ->
            outOfOrderSegmentRejected artifact publicInput proof segment
  parallelReexecutionAcceptedAndMissingSeedImpliesRejected :
    forall artifact publicInput proof segment,
      parallelReexecutionAccepted artifact publicInput proof ->
        segmentIndex artifact publicInput proof segment ->
          ¬ segmentSeedMatchesPreviousTransition artifact publicInput proof segment ->
            missingSeedRejected artifact publicInput proof segment

def RuntimeParallelSegmentReexecutionCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeParallelSegmentReexecutionValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.parallelReexecutionAccepted artifact publicInput proof

def RuntimeParallelSegmentReexecutionContract
    (_system : VerifierModel)
    (validation : RuntimeParallelSegmentReexecutionValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.serialExecutionAccepted artifact publicInput proof
    /\ validation.noSegmentDropped artifact publicInput proof
    /\ validation.noSegmentDuplicated artifact publicInput proof
    /\ forall segment,
      validation.segmentIndex artifact publicInput proof segment ->
        validation.emittedInVerifierOrder artifact publicInput proof segment
          /\ validation.segmentSeedMatchesPreviousTransition
            artifact
            publicInput
            proof
            segment
          /\ validation.segmentOutputMatchesSerialExecution
            artifact
            publicInput
            proof
            segment

def RuntimeParallelSegmentReexecutionRejectionContract
    (_system : VerifierModel)
    (validation : RuntimeParallelSegmentReexecutionValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  (forall segment,
    validation.segmentIndex artifact publicInput proof segment ->
      ¬ validation.emittedInVerifierOrder artifact publicInput proof segment ->
        validation.outOfOrderSegmentRejected artifact publicInput proof segment)
    /\ forall segment,
      validation.segmentIndex artifact publicInput proof segment ->
        ¬ validation.segmentSeedMatchesPreviousTransition
          artifact
          publicInput
          proof
          segment ->
          validation.missingSeedRejected artifact publicInput proof segment

theorem runtime_parallel_segment_reexecution_checked_acceptance_contract
    {system : VerifierModel}
    (validation : RuntimeParallelSegmentReexecutionValidation system) :
    forall artifact publicInput proof,
      RuntimeParallelSegmentReexecutionCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeParallelSegmentReexecutionContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  refine And.intro ?serial ?rest
  · exact
      validation.parallelReexecutionAcceptedImpliesSerialExecutionAccepted
        artifact
        publicInput
        proof
        accepted
  · refine And.intro ?noDropped ?restAfterDropped
    · exact
        validation.parallelReexecutionAcceptedImpliesNoSegmentDropped
          artifact
          publicInput
          proof
          accepted
    · refine And.intro ?noDuplicated ?segments
      · exact
          validation.parallelReexecutionAcceptedImpliesNoSegmentDuplicated
            artifact
            publicInput
            proof
            accepted
      · intro segment segmentKnown
        exact
          And.intro
            (validation.parallelReexecutionAcceptedImpliesOrderedEmission
              artifact
              publicInput
              proof
              segment
              accepted
              segmentKnown)
            (And.intro
              (validation.parallelReexecutionAcceptedImpliesSeedChain
                artifact
                publicInput
                proof
                segment
                accepted
                segmentKnown)
              (validation.parallelReexecutionAcceptedImpliesSerialEquivalentSegment
                artifact
                publicInput
                proof
                segment
                accepted
                segmentKnown))

theorem runtime_parallel_segment_reexecution_checked_acceptance_rejection_contract
    {system : VerifierModel}
    (validation : RuntimeParallelSegmentReexecutionValidation system) :
    forall artifact publicInput proof,
      RuntimeParallelSegmentReexecutionCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeParallelSegmentReexecutionRejectionContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  refine And.intro ?outOfOrder ?missingSeed
  · intro segment segmentKnown notOrdered
    exact
      validation.parallelReexecutionAcceptedAndNotOrderedImpliesRejected
        artifact
        publicInput
        proof
        segment
        accepted
        segmentKnown
        notOrdered
  · intro segment segmentKnown missingSeed
    exact
      validation.parallelReexecutionAcceptedAndMissingSeedImpliesRejected
        artifact
        publicInput
        proof
        segment
        accepted
        segmentKnown
        missingSeed

end Lzvm
