/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.Core

/-!
Auxiliary source lookup and leaf digest checked-acceptance contracts.
-/

namespace Lzvm

theorem source_lookup_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (auxiliary : AuxiliaryValidation system) :
    forall publicInput proof,
      SourceLookupCheckedAcceptance system auxiliary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem source_lookup_checked_acceptance_projects_auxiliary_evidence
    {system : VerifierModel}
    (auxiliary : AuxiliaryValidation system) :
    forall publicInput proof,
      SourceLookupCheckedAcceptance system auxiliary publicInput proof ->
        SourceLookupAuxiliaryEvidence system auxiliary publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem source_lookup_auxiliary_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (auxiliary : AuxiliaryValidation system) :
    forall publicInput proof,
      SourceLookupCheckedAcceptance system auxiliary publicInput proof ->
        SourceLookupAuxiliaryEvidence system auxiliary publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithLookupChecks
  exact
    And.intro
      (source_lookup_checked_acceptance_projects_auxiliary_evidence
        auxiliary
        publicInput
        proof
        acceptedWithLookupChecks)
      (auxiliary_checked_acceptance_sound_witness
        assumptions
        publicInput
        proof
        acceptedWithLookupChecks)

theorem source_lookup_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (auxiliary : AuxiliaryValidation system) :
    forall publicInput proof,
      SourceLookupCheckedAcceptance system auxiliary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    auxiliary_checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem witness_leaf_digest_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem witness_leaf_digest_checked_acceptance_projects_evidence
    {system : VerifierModel}
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        WitnessLeafDigestEvidence system validation publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem witness_leaf_digest_checked_acceptance_projects_canonical_leaf_bytes
    {system : VerifierModel}
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        validation.canonicalExtendedLeafBytes publicInput proof := by
  intro publicInput proof checked
  exact checked.right.left

theorem witness_leaf_digest_checked_acceptance_projects_narrow_padded_digest_rows
    {system : VerifierModel}
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        validation.narrowPaddedDigestsBindRows publicInput proof := by
  intro publicInput proof checked
  exact checked.right.right.left

theorem witness_leaf_digest_checked_acceptance_projects_wide_linear_digest_rows
    {system : VerifierModel}
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        validation.wideLinearDigestsBindRows publicInput proof := by
  intro publicInput proof checked
  exact checked.right.right.right

theorem witness_leaf_digest_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        WitnessLeafDigestEvidence system validation publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithLeafDigestChecks
  exact
    And.intro
      (witness_leaf_digest_checked_acceptance_projects_evidence
        validation
        publicInput
        proof
        acceptedWithLeafDigestChecks)
      (auxiliary_checked_acceptance_sound_witness
        assumptions
        publicInput
        proof
        acceptedWithLeafDigestChecks)

theorem witness_leaf_digest_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : WitnessLeafDigestValidation system) :
    forall publicInput proof,
      WitnessLeafDigestCheckedAcceptance system validation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    auxiliary_checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem gpu_canonical_leaf_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (validation : GpuCanonicalLeafValidation system) :
    forall publicInput proof,
      GpuCanonicalLeafCheckedAcceptance system validation publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem gpu_canonical_leaf_checked_acceptance_projects_flag_clear
    {system : VerifierModel}
    (validation : GpuCanonicalLeafValidation system) :
    forall publicInput proof,
      GpuCanonicalLeafCheckedAcceptance system validation publicInput proof ->
        validation.gpuCanonicalFlagClear publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem gpu_canonical_leaf_checked_acceptance_projects_leaf_bytes
    {system : VerifierModel}
    (validation : GpuCanonicalLeafValidation system) :
    forall publicInput proof,
      GpuCanonicalLeafCheckedAcceptance system validation publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof := by
  intro publicInput proof checked
  exact
    validation.flagClearImpliesCanonicalExtendedLeafBytes
      publicInput
      proof
      (gpu_canonical_leaf_checked_acceptance_projects_flag_clear
        validation
        publicInput
        proof
        checked)

theorem gpu_canonical_leaf_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuCanonicalLeafValidation system) :
    forall publicInput proof,
      GpuCanonicalLeafCheckedAcceptance system validation publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithCanonicalFlag
  exact
    And.intro
      (gpu_canonical_leaf_checked_acceptance_projects_leaf_bytes
        validation
        publicInput
        proof
        acceptedWithCanonicalFlag)
      (auxiliary_checked_acceptance_sound_witness
        assumptions
        publicInput
        proof
        acceptedWithCanonicalFlag)

theorem gpu_canonical_leaf_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuCanonicalLeafValidation system) :
    forall publicInput proof,
      GpuCanonicalLeafCheckedAcceptance system validation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    auxiliary_checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem gpu_leaf_output_buffer_reuse_implies_canonical_leaf_bytes
    {system : VerifierModel}
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      validation.leafOutputBufferLengthMatches publicInput proof ->
      validation.leafOutputBufferFullyOverwritten publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof := by
  intro publicInput proof outputLengthMatches outputFullyOverwritten
  exact
    validation.leafOutputBufferReuseImpliesCanonicalLeafBytes
      publicInput
      proof
      outputLengthMatches
      outputFullyOverwritten

theorem gpu_leaf_output_buffer_reuse_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      GpuLeafOutputBufferReuseCheckedAcceptance system validation publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem gpu_leaf_output_buffer_reuse_checked_acceptance_projects_length_match
    {system : VerifierModel}
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      GpuLeafOutputBufferReuseCheckedAcceptance system validation publicInput proof ->
        validation.leafOutputBufferLengthMatches publicInput proof := by
  intro publicInput proof checked
  exact checked.right.left

theorem gpu_leaf_output_buffer_reuse_checked_acceptance_projects_fully_overwritten
    {system : VerifierModel}
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      GpuLeafOutputBufferReuseCheckedAcceptance system validation publicInput proof ->
        validation.leafOutputBufferFullyOverwritten publicInput proof := by
  intro publicInput proof checked
  exact checked.right.right

theorem gpu_leaf_output_buffer_reuse_checked_acceptance_projects_leaf_bytes
    {system : VerifierModel}
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      GpuLeafOutputBufferReuseCheckedAcceptance system validation publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof := by
  intro publicInput proof checked
  exact
    gpu_leaf_output_buffer_reuse_implies_canonical_leaf_bytes
      validation
      publicInput
      proof
      checked.right.left
      checked.right.right

theorem gpu_leaf_output_buffer_reuse_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      GpuLeafOutputBufferReuseCheckedAcceptance system validation publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (gpu_leaf_output_buffer_reuse_checked_acceptance_projects_leaf_bytes
        validation
        publicInput
        proof
        checked)
      (auxiliary_checked_acceptance_sound_witness
        assumptions
        (auxiliaryAccepted := fun publicInput proof =>
          validation.leafOutputBufferLengthMatches publicInput proof
            /\ validation.leafOutputBufferFullyOverwritten publicInput proof)
        publicInput
        proof
        (And.intro checked.left (And.intro checked.right.left checked.right.right)))

theorem gpu_leaf_output_buffer_reuse_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuLeafOutputBufferReuseValidation system) :
    forall publicInput proof,
      GpuLeafOutputBufferReuseCheckedAcceptance system validation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    auxiliary_checked_acceptance_verifier_core_contract
      assumptions
      (auxiliaryAccepted := fun publicInput proof =>
        validation.leafOutputBufferLengthMatches publicInput proof
          /\ validation.leafOutputBufferFullyOverwritten publicInput proof)
      publicInput
      proof
      checked

theorem gpu_coset_extension_matches_host_implies_leaf_bytes
    {system : VerifierModel}
    (validation : GpuCosetExtensionValidation system) :
    forall publicInput proof,
      validation.cosetExtensionMatchesHost publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof := by
  intro publicInput proof cosetMatchesHost
  exact
    validation.cosetExtensionImpliesCanonicalLeafBytes
      publicInput
      proof
      cosetMatchesHost

theorem gpu_coset_extension_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (validation : GpuCosetExtensionValidation system) :
    forall publicInput proof,
      GpuCosetExtensionCheckedAcceptance system validation publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem gpu_coset_extension_checked_acceptance_projects_matches_host
    {system : VerifierModel}
    (validation : GpuCosetExtensionValidation system) :
    forall publicInput proof,
      GpuCosetExtensionCheckedAcceptance system validation publicInput proof ->
        validation.cosetExtensionMatchesHost publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem gpu_coset_extension_checked_acceptance_projects_leaf_bytes
    {system : VerifierModel}
    (validation : GpuCosetExtensionValidation system) :
    forall publicInput proof,
      GpuCosetExtensionCheckedAcceptance system validation publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof := by
  intro publicInput proof checked
  exact
    gpu_coset_extension_matches_host_implies_leaf_bytes
      validation
      publicInput
      proof
      (gpu_coset_extension_checked_acceptance_projects_matches_host
        validation
        publicInput
        proof
        checked)

theorem gpu_coset_extension_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuCosetExtensionValidation system) :
    forall publicInput proof,
      GpuCosetExtensionCheckedAcceptance system validation publicInput proof ->
        validation.leafValidation.canonicalExtendedLeafBytes publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (gpu_coset_extension_checked_acceptance_projects_leaf_bytes
        validation
        publicInput
        proof
        checked)
      (auxiliary_checked_acceptance_sound_witness
        assumptions
        publicInput
        proof
        checked)

theorem gpu_coset_extension_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuCosetExtensionValidation system) :
    forall publicInput proof,
      GpuCosetExtensionCheckedAcceptance system validation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    auxiliary_checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem gpu_fri_interpolation_matches_host_implies_fri_folds_valid
    {system : VerifierModel}
    (validation : GpuFriFoldInterpolationValidation system) :
    forall publicInput proof,
      validation.gpuFriInterpolationMatchesHost publicInput proof ->
        validation.friFoldsValid publicInput proof := by
  intro publicInput proof interpolationMatchesHost
  exact
    validation.gpuFriInterpolationImpliesFriFoldsValid
      publicInput
      proof
      interpolationMatchesHost

theorem gpu_fri_fold_interpolation_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (validation : GpuFriFoldInterpolationValidation system) :
    forall publicInput proof,
      GpuFriFoldInterpolationCheckedAcceptance system validation publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem gpu_fri_fold_interpolation_checked_acceptance_projects_matches_host
    {system : VerifierModel}
    (validation : GpuFriFoldInterpolationValidation system) :
    forall publicInput proof,
      GpuFriFoldInterpolationCheckedAcceptance system validation publicInput proof ->
        validation.gpuFriInterpolationMatchesHost publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem gpu_fri_fold_interpolation_checked_acceptance_projects_fri_folds_valid
    {system : VerifierModel}
    (validation : GpuFriFoldInterpolationValidation system) :
    forall publicInput proof,
      GpuFriFoldInterpolationCheckedAcceptance system validation publicInput proof ->
        validation.friFoldsValid publicInput proof := by
  intro publicInput proof checked
  exact
    gpu_fri_interpolation_matches_host_implies_fri_folds_valid
      validation
      publicInput
      proof
      (gpu_fri_fold_interpolation_checked_acceptance_projects_matches_host
        validation
        publicInput
        proof
        checked)

theorem gpu_fri_fold_interpolation_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuFriFoldInterpolationValidation system) :
    forall publicInput proof,
      GpuFriFoldInterpolationCheckedAcceptance system validation publicInput proof ->
        validation.friFoldsValid publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (gpu_fri_fold_interpolation_checked_acceptance_projects_fri_folds_valid
        validation
        publicInput
        proof
        checked)
      (auxiliary_checked_acceptance_sound_witness
        assumptions
        publicInput
        proof
        checked)

theorem gpu_fri_fold_interpolation_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuFriFoldInterpolationValidation system) :
    forall publicInput proof,
      GpuFriFoldInterpolationCheckedAcceptance system validation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    auxiliary_checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem gpu_merkle_digest_prefix_batch_matches_single_paths_implies_lower_prefixes_bound
    {system : VerifierModel}
    (validation : GpuMerkleDigestPrefixBatchValidation system) :
    forall publicInput proof,
      validation.gpuMerkleDigestPrefixBatchMatchesSinglePaths publicInput proof ->
        validation.lowerPrefixesBound publicInput proof := by
  intro publicInput proof prefixBatchMatchesSinglePaths
  exact
    validation.gpuMerkleDigestPrefixBatchImpliesLowerPrefixesBound
      publicInput
      proof
      prefixBatchMatchesSinglePaths

theorem gpu_merkle_digest_prefix_batch_checked_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (validation : GpuMerkleDigestPrefixBatchValidation system) :
    forall publicInput proof,
      GpuMerkleDigestPrefixBatchCheckedAcceptance system validation publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof checked
  exact checked.left

theorem gpu_merkle_digest_prefix_batch_checked_acceptance_projects_matches_single_paths
    {system : VerifierModel}
    (validation : GpuMerkleDigestPrefixBatchValidation system) :
    forall publicInput proof,
      GpuMerkleDigestPrefixBatchCheckedAcceptance system validation publicInput proof ->
        validation.gpuMerkleDigestPrefixBatchMatchesSinglePaths publicInput proof := by
  intro publicInput proof checked
  exact checked.right

theorem gpu_merkle_digest_prefix_batch_checked_acceptance_projects_lower_prefixes_bound
    {system : VerifierModel}
    (validation : GpuMerkleDigestPrefixBatchValidation system) :
    forall publicInput proof,
      GpuMerkleDigestPrefixBatchCheckedAcceptance system validation publicInput proof ->
        validation.lowerPrefixesBound publicInput proof := by
  intro publicInput proof checked
  exact
    gpu_merkle_digest_prefix_batch_matches_single_paths_implies_lower_prefixes_bound
      validation
      publicInput
      proof
      (gpu_merkle_digest_prefix_batch_checked_acceptance_projects_matches_single_paths
        validation
        publicInput
        proof
        checked)

theorem gpu_merkle_digest_prefix_batch_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuMerkleDigestPrefixBatchValidation system) :
    forall publicInput proof,
      GpuMerkleDigestPrefixBatchCheckedAcceptance system validation publicInput proof ->
        validation.lowerPrefixesBound publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof checked
  exact
    And.intro
      (gpu_merkle_digest_prefix_batch_checked_acceptance_projects_lower_prefixes_bound
        validation
        publicInput
        proof
        checked)
      (auxiliary_checked_acceptance_sound_witness
        assumptions
        publicInput
        proof
        checked)

theorem gpu_merkle_digest_prefix_batch_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : GpuMerkleDigestPrefixBatchValidation system) :
    forall publicInput proof,
      GpuMerkleDigestPrefixBatchCheckedAcceptance system validation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof checked
  exact
    auxiliary_checked_acceptance_verifier_core_contract
      assumptions
      publicInput
      proof
      checked

theorem gpu_setup_cache_reuse_sound
    (validation : GpuSetupCacheValidation)
    (state : GpuSetupCacheState)
    (request : GpuSetupRequest) :
    GpuSetupCacheCovers state request ->
      validation.constantsSoundFor state.device state.initializedBits ->
        validation.constantsSoundFor request.device request.requiredBits := by
  intro covers initializedSound
  cases covers with
  | intro sameDevice bitCover =>
    rw [sameDevice]
    exact
      validation.coveredConstantsSound
        state.device
        state.initializedBits
        request.requiredBits
        bitCover
        initializedSound

theorem gpu_setup_cache_reuse_request_device_sound
    (validation : GpuSetupCacheValidation)
    (state : GpuSetupCacheState)
    (request : GpuSetupRequest) :
    request.device = state.device ->
      request.requiredBits <= state.initializedBits ->
        validation.constantsSoundFor state.device state.initializedBits ->
          validation.constantsSoundFor request.device request.requiredBits := by
  intro sameDevice bitCover initializedSound
  exact
    gpu_setup_cache_reuse_sound
      validation
      state
      request
      (And.intro sameDevice bitCover)
      initializedSound


end Lzvm
