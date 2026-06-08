/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks
import Lzvm.AssumptionAudit
import Lzvm.BatchOpeningBinding
import Lzvm.ChallengeSegmentBinding
import Lzvm.Conformance
import Lzvm.DigestPrefix
import Lzvm.EthBlockPublicInputBinding
import Lzvm.ExternalSource
import Lzvm.ProofArtifactBinding
import Lzvm.OpeningValidation
import Lzvm.OpeningSegmentBinding
import Lzvm.PipelineBinding
import Lzvm.QueryPlanBinding
import Lzvm.RetainedLeafDigestOpening
import Lzvm.RetainedParentCheckpointOpening
import Lzvm.RequiredExternalSource
import Lzvm.RuntimeExternalSource
import Lzvm.RuntimeSoundness
import Lzvm.Soundness
import Lzvm.TranscriptBinding
import Lzvm.TraceConstraintArtifactBinding
import Lzvm.TraceConstraintValidation

/-!
Top-level module for the Lzvm formal soundness model.
-/
