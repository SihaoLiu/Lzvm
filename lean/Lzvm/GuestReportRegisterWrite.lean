/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Model

/-!
Canonical model for compact guest report register-write storage.

The runtime report stores only the written value. The destination register is
derived from the decoded instruction when a report row is lowered. This module
records the invariant needed by the conformance layer: the compact value and
derived destination reconstruct exactly the same zero-or-one write list used by
the abstract execution model.
-/

namespace Lzvm

structure GuestRegisterWrite where
  index : Nat
  value : Nat
deriving DecidableEq, Repr

inductive GuestRegisterWriteDestination where
  | none
  | register (index : Nat)
deriving DecidableEq, Repr

structure CompactGuestRegisterWrite where
  value : Nat
deriving DecidableEq, Repr

def reconstructGuestRegisterWrites
    (destination : GuestRegisterWriteDestination)
    (compact : CompactGuestRegisterWrite) : List GuestRegisterWrite :=
  match destination with
  | GuestRegisterWriteDestination.none => []
  | GuestRegisterWriteDestination.register index =>
      [{ index := index, value := compact.value }]

def CompactGuestRegisterWriteCanonical
    (destination : GuestRegisterWriteDestination)
    (compact : CompactGuestRegisterWrite)
    (writes : List GuestRegisterWrite) : Prop :=
  writes = reconstructGuestRegisterWrites destination compact

theorem compact_guest_register_write_none_empty
    (compact : CompactGuestRegisterWrite) :
    reconstructGuestRegisterWrites GuestRegisterWriteDestination.none compact = [] := by
  rfl

theorem compact_guest_register_write_register_singleton
    (index : Nat)
    (compact : CompactGuestRegisterWrite) :
    reconstructGuestRegisterWrites
      (GuestRegisterWriteDestination.register index)
      compact = [{ index := index, value := compact.value }] := by
  rfl

theorem compact_guest_register_write_list_length_le_one
    (destination : GuestRegisterWriteDestination)
    (compact : CompactGuestRegisterWrite) :
    (reconstructGuestRegisterWrites destination compact).length <= 1 := by
  cases destination <;> simp [reconstructGuestRegisterWrites]

theorem compact_guest_register_write_canonical_length_le_one
    {destination : GuestRegisterWriteDestination}
    {compact : CompactGuestRegisterWrite}
    {writes : List GuestRegisterWrite}
    (canonical : CompactGuestRegisterWriteCanonical destination compact writes) :
    writes.length <= 1 := by
  rw [canonical]
  exact compact_guest_register_write_list_length_le_one destination compact

theorem compact_guest_register_write_canonical_none
    {compact : CompactGuestRegisterWrite}
    {writes : List GuestRegisterWrite}
    (canonical :
      CompactGuestRegisterWriteCanonical
        GuestRegisterWriteDestination.none
        compact
        writes) :
    writes = [] := by
  exact canonical

theorem compact_guest_register_write_canonical_register
    {index : Nat}
    {compact : CompactGuestRegisterWrite}
    {writes : List GuestRegisterWrite}
    (canonical :
      CompactGuestRegisterWriteCanonical
        (GuestRegisterWriteDestination.register index)
        compact
        writes) :
    writes = [{ index := index, value := compact.value }] := by
  exact canonical

theorem compact_guest_register_write_register_value
    (index : Nat)
    (compact : CompactGuestRegisterWrite) :
    (reconstructGuestRegisterWrites
      (GuestRegisterWriteDestination.register index)
      compact).map GuestRegisterWrite.value = [compact.value] := by
  simp [reconstructGuestRegisterWrites]

theorem compact_guest_register_write_register_index
    (index : Nat)
    (compact : CompactGuestRegisterWrite) :
    (reconstructGuestRegisterWrites
      (GuestRegisterWriteDestination.register index)
      compact).map GuestRegisterWrite.index = [index] := by
  simp [reconstructGuestRegisterWrites]

end Lzvm
