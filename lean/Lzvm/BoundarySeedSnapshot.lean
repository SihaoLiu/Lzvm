/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Conformance

/-!
Runtime obligations for boundary seed snapshots.
-/

namespace Lzvm

structure RuntimeBoundarySeedSnapshotValidation (system : VerifierModel) where
  snapshotShortcutAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  snapshotShortcutAuthorized : RuntimeArtifact -> PublicInput -> Proof -> Prop
  boundarySegment : RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  finalBoundary : RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  directBoundarySeedPresent : RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  directBoundarySeedMatchesNormalTransition :
    RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  optimizedBoundarySeedUsed : RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  boundarySeedMissRejected : RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  snapshotShortcutAcceptedImpliesAuthorized :
    forall artifact publicInput proof,
      snapshotShortcutAccepted artifact publicInput proof ->
        snapshotShortcutAuthorized artifact publicInput proof
  snapshotShortcutAcceptedImpliesNonFinalBoundaryHasDirectSeed :
    forall artifact publicInput proof segment,
      snapshotShortcutAccepted artifact publicInput proof ->
        boundarySegment artifact publicInput proof segment ->
          ¬ finalBoundary artifact publicInput proof segment ->
            directBoundarySeedPresent artifact publicInput proof segment
  snapshotShortcutAcceptedAndDirectSeedImpliesNormalTransitionMatch :
    forall artifact publicInput proof segment,
      snapshotShortcutAccepted artifact publicInput proof ->
        boundarySegment artifact publicInput proof segment ->
          directBoundarySeedPresent artifact publicInput proof segment ->
            directBoundarySeedMatchesNormalTransition
              artifact
              publicInput
              proof
              segment
  snapshotShortcutAcceptedAndDirectSeedImpliesOptimizedSeedUsed :
    forall artifact publicInput proof segment,
      snapshotShortcutAccepted artifact publicInput proof ->
        boundarySegment artifact publicInput proof segment ->
          directBoundarySeedPresent artifact publicInput proof segment ->
            optimizedBoundarySeedUsed artifact publicInput proof segment
  snapshotShortcutAcceptedAndMissingDirectSeedImpliesRejected :
    forall artifact publicInput proof segment,
      snapshotShortcutAccepted artifact publicInput proof ->
        boundarySegment artifact publicInput proof segment ->
          ¬ finalBoundary artifact publicInput proof segment ->
            ¬ directBoundarySeedPresent artifact publicInput proof segment ->
              boundarySeedMissRejected artifact publicInput proof segment

def RuntimeBoundarySeedSnapshotCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeBoundarySeedSnapshotValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.snapshotShortcutAccepted artifact publicInput proof

def RuntimeBoundarySeedSnapshotContract
    (_system : VerifierModel)
    (validation : RuntimeBoundarySeedSnapshotValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.snapshotShortcutAuthorized artifact publicInput proof
    /\ forall segment,
      validation.boundarySegment artifact publicInput proof segment ->
        ¬ validation.finalBoundary artifact publicInput proof segment ->
          validation.directBoundarySeedPresent artifact publicInput proof segment
            /\ validation.directBoundarySeedMatchesNormalTransition
              artifact
              publicInput
              proof
              segment
            /\ validation.optimizedBoundarySeedUsed artifact publicInput proof segment

def RuntimeBoundarySeedSnapshotMissContract
    (_system : VerifierModel)
    (validation : RuntimeBoundarySeedSnapshotValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  forall segment,
    validation.boundarySegment artifact publicInput proof segment ->
      ¬ validation.finalBoundary artifact publicInput proof segment ->
        ¬ validation.directBoundarySeedPresent artifact publicInput proof segment ->
          validation.boundarySeedMissRejected artifact publicInput proof segment

structure RuntimeBoundaryPendingState where
  lastPending : Option Nat
  nextPending : Option Nat
deriving DecidableEq, Repr

def RuntimeBoundaryPendingState.recordPlainShape
    (state : RuntimeBoundaryPendingState) : RuntimeBoundaryPendingState :=
  match state.lastPending, state.nextPending with
  | none, none => state
  | _, nextPending => { lastPending := nextPending, nextPending := none }

def RuntimeBoundaryPendingState.recordShape
    (state : RuntimeBoundaryPendingState)
    (newPending : Option Nat) : RuntimeBoundaryPendingState :=
  match newPending with
  | some pending => { lastPending := state.nextPending, nextPending := some pending }
  | none => state.recordPlainShape

theorem runtime_boundary_pending_state_idle_plain_shape_skip_preserves_state
    (state : RuntimeBoundaryPendingState)
    (lastEmpty : state.lastPending = none)
    (nextEmpty : state.nextPending = none) :
    state.recordShape none = state := by
  cases state
  cases lastEmpty
  cases nextEmpty
  rfl

theorem runtime_boundary_pending_state_plain_shape_active_step_shifts_next
    (state : RuntimeBoundaryPendingState)
    (active : state.lastPending ≠ none \/ state.nextPending ≠ none) :
    state.recordShape none = { lastPending := state.nextPending, nextPending := none } := by
  cases state with
  | mk lastPending nextPending =>
      cases lastPending <;> cases nextPending <;> simp [RuntimeBoundaryPendingState.recordShape,
        RuntimeBoundaryPendingState.recordPlainShape] at *

theorem runtime_boundary_pending_state_new_shape_records_pending
    (state : RuntimeBoundaryPendingState)
    (pending : Nat) :
    state.recordShape (some pending) =
      { lastPending := state.nextPending, nextPending := some pending } := by
  rfl

theorem runtime_boundary_seed_snapshot_checked_acceptance_contract
    {system : VerifierModel}
    (validation : RuntimeBoundarySeedSnapshotValidation system) :
    forall artifact publicInput proof,
      RuntimeBoundarySeedSnapshotCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeBoundarySeedSnapshotContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  refine And.intro ?authorized ?segments
  · exact
      validation.snapshotShortcutAcceptedImpliesAuthorized
        artifact
        publicInput
        proof
        accepted
  · intro segment boundary notFinal
    have direct :=
      validation.snapshotShortcutAcceptedImpliesNonFinalBoundaryHasDirectSeed
        artifact
        publicInput
        proof
        segment
        accepted
        boundary
        notFinal
    exact
      And.intro
        direct
        (And.intro
          (validation.snapshotShortcutAcceptedAndDirectSeedImpliesNormalTransitionMatch
            artifact
            publicInput
            proof
            segment
            accepted
            boundary
            direct)
          (validation.snapshotShortcutAcceptedAndDirectSeedImpliesOptimizedSeedUsed
            artifact
            publicInput
            proof
            segment
            accepted
            boundary
            direct))

theorem runtime_boundary_seed_snapshot_checked_acceptance_miss_contract
    {system : VerifierModel}
    (validation : RuntimeBoundarySeedSnapshotValidation system) :
    forall artifact publicInput proof,
      RuntimeBoundarySeedSnapshotCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeBoundarySeedSnapshotMissContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted segment boundary notFinal missingDirect
  exact
    validation.snapshotShortcutAcceptedAndMissingDirectSeedImpliesRejected
      artifact
      publicInput
      proof
      segment
      accepted
      boundary
      notFinal
      missingDirect

end Lzvm
