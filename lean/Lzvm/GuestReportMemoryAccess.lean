/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Model

/-!
Canonical model for folded guest report memory effects.

Runtime reports store the common normal memory-access list inline. Rare
precompile effects are represented as a folded variant that still exposes two
separate logical views: normal memory accesses and precompile memory accesses.
The theorems below state that folding preserves those views.
-/

namespace Lzvm

inductive GuestMemoryAccessKind where
  | read
  | write
deriving DecidableEq, Repr

structure GuestMemoryAccess where
  kind : GuestMemoryAccessKind
  address : Nat
  byteLen : Nat
  value : Nat
deriving DecidableEq, Repr

structure GuestPrecompileReportEffects where
  normalMemoryAccesses : List GuestMemoryAccess
  precompileMemoryAccesses : List GuestMemoryAccess
  result : Option Nat
deriving DecidableEq, Repr

inductive GuestMemoryAccessStorage where
  | empty
  | one (access : GuestMemoryAccess)
  | many (accesses : List GuestMemoryAccess)
  | precompile (effects : GuestPrecompileReportEffects)
deriving DecidableEq, Repr

def GuestMemoryAccessStorage.normalAccesses :
    GuestMemoryAccessStorage -> List GuestMemoryAccess
  | GuestMemoryAccessStorage.empty => []
  | GuestMemoryAccessStorage.one access => [access]
  | GuestMemoryAccessStorage.many accesses => accesses
  | GuestMemoryAccessStorage.precompile effects => effects.normalMemoryAccesses

def GuestMemoryAccessStorage.precompileAccesses :
    GuestMemoryAccessStorage -> List GuestMemoryAccess
  | GuestMemoryAccessStorage.precompile effects => effects.precompileMemoryAccesses
  | _ => []

def GuestMemoryAccessStorage.precompileResult :
    GuestMemoryAccessStorage -> Option Nat
  | GuestMemoryAccessStorage.precompile effects => effects.result
  | _ => none

def FoldedGuestMemoryEffectsCanonical
    (storage : GuestMemoryAccessStorage)
    (normal precompile : List GuestMemoryAccess)
    (result : Option Nat) : Prop :=
  storage.normalAccesses = normal
    /\ storage.precompileAccesses = precompile
    /\ storage.precompileResult = result

theorem folded_guest_memory_empty_views :
    FoldedGuestMemoryEffectsCanonical
      GuestMemoryAccessStorage.empty
      []
      []
      none := by
  exact And.intro rfl (And.intro rfl rfl)

theorem folded_guest_memory_one_normal_view
    (access : GuestMemoryAccess) :
    FoldedGuestMemoryEffectsCanonical
      (GuestMemoryAccessStorage.one access)
      [access]
      []
      none := by
  exact And.intro rfl (And.intro rfl rfl)

theorem folded_guest_memory_many_normal_view
    (accesses : List GuestMemoryAccess) :
    FoldedGuestMemoryEffectsCanonical
      (GuestMemoryAccessStorage.many accesses)
      accesses
      []
      none := by
  exact And.intro rfl (And.intro rfl rfl)

theorem folded_guest_memory_precompile_views
    (effects : GuestPrecompileReportEffects) :
    FoldedGuestMemoryEffectsCanonical
      (GuestMemoryAccessStorage.precompile effects)
      effects.normalMemoryAccesses
      effects.precompileMemoryAccesses
      effects.result := by
  exact And.intro rfl (And.intro rfl rfl)

theorem folded_guest_memory_nonprecompile_has_no_precompile_accesses
    (storage : GuestMemoryAccessStorage)
    (notFolded :
      forall effects, storage ≠ GuestMemoryAccessStorage.precompile effects) :
    storage.precompileAccesses = [] := by
  cases storage with
  | empty => rfl
  | one _ => rfl
  | many _ => rfl
  | precompile effects =>
      exact False.elim (notFolded effects rfl)

theorem folded_guest_memory_nonprecompile_has_no_precompile_result
    (storage : GuestMemoryAccessStorage)
    (notFolded :
      forall effects, storage ≠ GuestMemoryAccessStorage.precompile effects) :
    storage.precompileResult = none := by
  cases storage with
  | empty => rfl
  | one _ => rfl
  | many _ => rfl
  | precompile effects =>
      exact False.elim (notFolded effects rfl)

theorem folded_guest_memory_precompile_preserves_normal_accesses
    (normal precompile : List GuestMemoryAccess)
    (result : Option Nat) :
    (GuestMemoryAccessStorage.precompile
      { normalMemoryAccesses := normal
        precompileMemoryAccesses := precompile
        result := result }).normalAccesses = normal := by
  rfl

theorem folded_guest_memory_precompile_preserves_precompile_accesses
    (normal precompile : List GuestMemoryAccess)
    (result : Option Nat) :
    (GuestMemoryAccessStorage.precompile
      { normalMemoryAccesses := normal
        precompileMemoryAccesses := precompile
        result := result }).precompileAccesses = precompile := by
  rfl

theorem folded_guest_memory_precompile_preserves_result
    (normal precompile : List GuestMemoryAccess)
    (result : Option Nat) :
    (GuestMemoryAccessStorage.precompile
      { normalMemoryAccesses := normal
        precompileMemoryAccesses := precompile
        result := result }).precompileResult = result := by
  rfl

end Lzvm
