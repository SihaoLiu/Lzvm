/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Model

/-!
Reusable ordered-slot lookup model for implementation fast paths.
-/

namespace Lzvm

def listGet? (entries : List α) (index : Nat) : Option α :=
  match entries, index with
  | [], _ => none
  | entry :: _, 0 => some entry
  | _ :: rest, index + 1 => listGet? rest index

def firstStageMatch
    (entries : List α)
    (stageIndexOf : α -> Nat)
    (stageIndex : Nat) : Option α :=
  entries.find? fun entry => stageIndexOf entry == stageIndex

def orderedStageSlotMatch
    (entries : List α)
    (stageIndexOf : α -> Nat)
    (stageIndex : Nat) : Option α :=
  match stageIndex with
  | 0 => none
  | stageSlot + 1 =>
      match listGet? entries stageSlot with
      | none => none
      | some entry =>
          if stageIndexOf entry == stageIndex
            && (entries.take stageSlot).all
                (fun prior => stageIndexOf prior != stageIndex) then
            some entry
          else
            none

def guardedStageLookup
    (entries : List α)
    (stageIndexOf : α -> Nat)
    (stageIndex : Nat) : Option α :=
  match orderedStageSlotMatch entries stageIndexOf stageIndex with
  | none => firstStageMatch entries stageIndexOf stageIndex
  | some entry => some entry

theorem guarded_stage_lookup_uses_fallback_when_fast_path_declines
    (entries : List α)
    (stageIndexOf : α -> Nat)
    (stageIndex : Nat)
    (fastPathDeclines :
      orderedStageSlotMatch entries stageIndexOf stageIndex = none) :
    guardedStageLookup entries stageIndexOf stageIndex =
      firstStageMatch entries stageIndexOf stageIndex := by
  unfold guardedStageLookup
  rw [fastPathDeclines]

theorem guarded_stage_lookup_preserves_first_match_when_fast_path_matches
    (entries : List α)
    (stageIndexOf : α -> Nat)
    (stageIndex : Nat)
    (fastPathMatches :
      orderedStageSlotMatch entries stageIndexOf stageIndex =
        firstStageMatch entries stageIndexOf stageIndex) :
    guardedStageLookup entries stageIndexOf stageIndex =
      firstStageMatch entries stageIndexOf stageIndex := by
  unfold guardedStageLookup
  rw [fastPathMatches]
  cases firstStageMatch entries stageIndexOf stageIndex <;> rfl

theorem guarded_stage_lookup_preserves_first_match
    (entries : List α)
    (stageIndexOf : α -> Nat)
    (stageIndex : Nat)
    (fastPathDeclinesOrMatches :
      orderedStageSlotMatch entries stageIndexOf stageIndex = none
        \/ orderedStageSlotMatch entries stageIndexOf stageIndex =
          firstStageMatch entries stageIndexOf stageIndex) :
    guardedStageLookup entries stageIndexOf stageIndex =
      firstStageMatch entries stageIndexOf stageIndex := by
  cases fastPathDeclinesOrMatches with
  | inl fastPathDeclines =>
      exact
        guarded_stage_lookup_uses_fallback_when_fast_path_declines
          entries
          stageIndexOf
          stageIndex
          fastPathDeclines
  | inr fastPathMatches =>
      exact
        guarded_stage_lookup_preserves_first_match_when_fast_path_matches
          entries
          stageIndexOf
          stageIndex
          fastPathMatches

end Lzvm
