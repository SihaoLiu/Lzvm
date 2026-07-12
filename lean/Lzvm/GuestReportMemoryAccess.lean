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

inductive CompactSingleAccessShape where
  | read (byteLen : Nat)
  | write (byteLen : Nat)
deriving DecidableEq, Repr

def CompactSingleAccessShape.kind :
    CompactSingleAccessShape -> GuestMemoryAccessKind
  | CompactSingleAccessShape.read _ => GuestMemoryAccessKind.read
  | CompactSingleAccessShape.write _ => GuestMemoryAccessKind.write

def CompactSingleAccessShape.byteLen : CompactSingleAccessShape -> Nat
  | CompactSingleAccessShape.read byteLen => byteLen
  | CompactSingleAccessShape.write byteLen => byteLen

def lowBytes (value byteLen : Nat) : Nat :=
  if 8 <= byteLen then value else value % (2 ^ (8 * byteLen))

def CompactSingleAccessShape.reconstruct
    (shape : CompactSingleAccessShape)
    (address effectValue : Nat) : GuestMemoryAccess :=
  { kind := shape.kind
    address := address
    byteLen := shape.byteLen
    value := lowBytes effectValue shape.byteLen }

inductive CompactGuestMemoryAccessStorage where
  | empty
  | one
      (shape : CompactSingleAccessShape)
      (address effectValue : Nat)
  | ownedOne (access : GuestMemoryAccess)
  | pair (first second : GuestMemoryAccess)
  | many (accesses : List GuestMemoryAccess)
  | precompile (effects : GuestPrecompileReportEffects)
deriving DecidableEq, Repr

def CompactGuestMemoryAccessStorage.normalAccesses :
    CompactGuestMemoryAccessStorage -> List GuestMemoryAccess
  | CompactGuestMemoryAccessStorage.empty => []
  | CompactGuestMemoryAccessStorage.one shape address effectValue =>
      [shape.reconstruct address effectValue]
  | CompactGuestMemoryAccessStorage.ownedOne access => [access]
  | CompactGuestMemoryAccessStorage.pair first second => [first, second]
  | CompactGuestMemoryAccessStorage.many accesses => accesses
  | CompactGuestMemoryAccessStorage.precompile effects =>
      effects.normalMemoryAccesses

def CompactGuestMemoryAccessStorage.precompileAccesses :
    CompactGuestMemoryAccessStorage -> List GuestMemoryAccess
  | CompactGuestMemoryAccessStorage.precompile effects =>
      effects.precompileMemoryAccesses
  | _ => []

def CompactGuestMemoryAccessStorage.precompileResult :
    CompactGuestMemoryAccessStorage -> Option Nat
  | CompactGuestMemoryAccessStorage.precompile effects => effects.result
  | _ => none

def CompactGuestMemoryAccessStorage.normalIsEmpty
    (storage : CompactGuestMemoryAccessStorage) : Bool :=
  storage.normalAccesses.isEmpty

def compactSingleGuestMemoryAccess
    (shape : CompactSingleAccessShape)
    (effectValue : Nat)
    (access : GuestMemoryAccess) : CompactGuestMemoryAccessStorage :=
  if shape.reconstruct access.address effectValue = access then
    CompactGuestMemoryAccessStorage.one shape access.address effectValue
  else
    CompactGuestMemoryAccessStorage.ownedOne access

theorem low_bytes_full_width (value : Nat) :
    lowBytes value 8 = value := by
  simp [lowBytes]

theorem compact_guest_memory_one_reconstructs
    (shape : CompactSingleAccessShape)
    (address effectValue : Nat) :
    (CompactGuestMemoryAccessStorage.one shape address effectValue).normalAccesses =
      [shape.reconstruct address effectValue] := by
  rfl

theorem compact_guest_memory_owned_one_preserves_access
    (access : GuestMemoryAccess) :
    (CompactGuestMemoryAccessStorage.ownedOne access).normalAccesses = [access] := by
  rfl

theorem compact_guest_memory_pair_preserves_accesses
    (first second : GuestMemoryAccess) :
    (CompactGuestMemoryAccessStorage.pair first second).normalAccesses =
      [first, second] := by
  rfl

theorem compact_guest_memory_many_preserves_accesses
    (accesses : List GuestMemoryAccess) :
    (CompactGuestMemoryAccessStorage.many accesses).normalAccesses = accesses := by
  rfl

theorem compact_guest_memory_precompile_preserves_views
    (effects : GuestPrecompileReportEffects) :
    (CompactGuestMemoryAccessStorage.precompile effects).normalAccesses =
        effects.normalMemoryAccesses
      /\ (CompactGuestMemoryAccessStorage.precompile effects).precompileAccesses =
        effects.precompileMemoryAccesses
      /\ (CompactGuestMemoryAccessStorage.precompile effects).precompileResult =
        effects.result := by
  exact And.intro rfl (And.intro rfl rfl)

theorem compact_guest_memory_precompile_empty_matches_normal_view
    (effects : GuestPrecompileReportEffects) :
    (CompactGuestMemoryAccessStorage.precompile effects).normalIsEmpty =
      effects.normalMemoryAccesses.isEmpty := by
  rfl

theorem compact_single_guest_memory_access_preserves_view
    (shape : CompactSingleAccessShape)
    (effectValue : Nat)
    (access : GuestMemoryAccess) :
    (compactSingleGuestMemoryAccess shape effectValue access).normalAccesses =
      [access] := by
  unfold compactSingleGuestMemoryAccess
  split
  case isTrue exactMatch =>
    simp only [CompactGuestMemoryAccessStorage.normalAccesses, exactMatch]
  case isFalse _ =>
    rfl

theorem compact_single_guest_memory_access_uses_one_on_exact_match
    (shape : CompactSingleAccessShape)
    (effectValue : Nat)
    (access : GuestMemoryAccess)
    (exactMatch : shape.reconstruct access.address effectValue = access) :
    compactSingleGuestMemoryAccess shape effectValue access =
      CompactGuestMemoryAccessStorage.one shape access.address effectValue := by
  simp [compactSingleGuestMemoryAccess, exactMatch]

theorem compact_single_guest_memory_access_falls_back_on_mismatch
    (shape : CompactSingleAccessShape)
    (effectValue : Nat)
    (access : GuestMemoryAccess)
    (mismatch : shape.reconstruct access.address effectValue ≠ access) :
    compactSingleGuestMemoryAccess shape effectValue access =
      CompactGuestMemoryAccessStorage.ownedOne access := by
  simp [compactSingleGuestMemoryAccess, mismatch]

end Lzvm
