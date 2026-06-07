use std::time::Duration;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WitnessProofArtifactTiming {
    pub query_plan: Duration,
    pub constant_opening: Duration,
    pub witness_opening: Duration,
    pub witness_opening_query_count: usize,
    pub witness_opening_query_unit_count: usize,
    pub witness_opening_single_query_unit_count: usize,
    pub witness_opening_max_queries_per_unit: usize,
    pub witness_opening_stage_count: usize,
    pub witness_opening_retained_source_count: usize,
    pub witness_opening_external_source_count: usize,
    pub witness_opening_embedded_source_count: usize,
    pub witness_opening_missing_source_count: usize,
    pub witness_opening_retained_leaf_digest_opening_count: usize,
    pub witness_opening_retained_leaf_digest_opening_row_count: usize,
    pub witness_opening_retained_parent_checkpoint_opening_count: usize,
    pub witness_opening_retained_parent_checkpoint_opening_row_count: usize,
    pub witness_external_source: Duration,
    pub witness_external_source_descriptor_upload: Duration,
    pub witness_external_source_descriptor_upload_byte_count: usize,
    pub witness_external_source_descriptor_upload_row_count: usize,
    pub witness_external_source_trace_expand: Duration,
    pub witness_opening_setup: Duration,
    pub witness_opening_leaf_extend: Duration,
    pub witness_opening_leaf_hash: Duration,
    pub witness_opening_leaf_hash_row_count: usize,
    pub witness_opening_leaf_hash_byte_count: usize,
    pub witness_opening_leaf_hash_arity2_row_count: usize,
    pub witness_opening_leaf_hash_arity2_byte_count: usize,
    pub witness_opening_leaf_hash_arity4_row_count: usize,
    pub witness_opening_leaf_hash_arity4_byte_count: usize,
    pub witness_opening_leaf_coset_extend_call_count: usize,
    pub witness_opening_leaf_coset_extend_output_byte_count: usize,
    pub witness_opening_leaf_coset_extend_column_count: usize,
    pub witness_opening_leaf_coset_extend_max_column_count: usize,
    pub witness_opening_leaf_coset_extend_ntt_launch_count: usize,
    pub witness_opening_leaf_coset_extend_bit_reverse_launch_count: usize,
    pub witness_opening_leaf_coset_extend_ntt_stage_launch_count: usize,
    pub witness_opening_leaf_coset_extend_ntt_block_twiddle_launch_count: usize,
    pub witness_opening_leaf_coset_extend_normalize_launch_count: usize,
    pub witness_opening_leaf_coset_extend_pack_launch_count: usize,
    pub witness_opening_leaf_coset_extend_unpack_launch_count: usize,
    pub witness_opening_path_parent_hash_row_count: usize,
    pub witness_opening_path_parent_hash_byte_count: usize,
    pub witness_opening_path_parent_hash_launch_count: usize,
    pub witness_opening_path: Duration,
    pub witness_opening_row_values: Duration,
    pub witness_opening_row_values_source_extend: Duration,
    pub witness_opening_row_values_source_download: Duration,
    pub witness_opening_row_values_device_download: Duration,
    pub witness_opening_row_values_device_row_count: usize,
    pub witness_opening_row_values_source_row_count: usize,
    pub witness_opening_row_values_word_count: usize,
    pub witness_opening_row_values_byte_count: usize,
    pub witness_stage_external_source: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening_setup: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening_leaf_extend: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening_leaf_hash: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening_work: Vec<WitnessProofStageOpeningWork>,
    pub witness_stage_opening_path: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening_row_values: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening_row_value_source_extend: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening_row_value_source_download: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening_row_value_device_download: Vec<WitnessProofStageOpeningTiming>,
    pub fri_opening: Duration,
    pub fri_opening_unit_build: Duration,
    pub fri_opening_layer_tree: Duration,
    pub fri_opening_query: Duration,
    pub fri_opening_fold: Duration,
    pub fri_opening_unit_count: usize,
    pub fri_opening_layer_count: usize,
    pub fri_opening_query_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WitnessProofStageOpeningTiming {
    pub stage_index: usize,
    pub duration: Duration,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WitnessProofStageOpeningWork {
    pub stage_index: usize,
    pub retained_source_count: usize,
    pub external_source_count: usize,
    pub embedded_source_count: usize,
    pub missing_source_count: usize,
    pub retained_leaf_digest_opening_count: usize,
    pub retained_leaf_digest_opening_row_count: usize,
    pub retained_parent_checkpoint_opening_count: usize,
    pub retained_parent_checkpoint_opening_row_count: usize,
    pub leaf_hash_row_count: usize,
    pub leaf_hash_byte_count: usize,
    pub leaf_hash_arity2_row_count: usize,
    pub leaf_hash_arity2_byte_count: usize,
    pub leaf_hash_arity4_row_count: usize,
    pub leaf_hash_arity4_byte_count: usize,
    pub leaf_coset_extend_call_count: usize,
    pub leaf_coset_extend_output_byte_count: usize,
    pub leaf_coset_extend_column_count: usize,
    pub leaf_coset_extend_max_column_count: usize,
    pub leaf_coset_extend_ntt_launch_count: usize,
    pub leaf_coset_extend_bit_reverse_launch_count: usize,
    pub leaf_coset_extend_ntt_stage_launch_count: usize,
    pub leaf_coset_extend_ntt_block_twiddle_launch_count: usize,
    pub leaf_coset_extend_normalize_launch_count: usize,
    pub leaf_coset_extend_pack_launch_count: usize,
    pub leaf_coset_extend_unpack_launch_count: usize,
    pub path_parent_hash_row_count: usize,
    pub path_parent_hash_byte_count: usize,
    pub path_parent_hash_launch_count: usize,
    pub row_values_device_row_count: usize,
    pub row_values_source_row_count: usize,
    pub row_values_word_count: usize,
    pub row_values_byte_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(feature = "cuda"), allow(dead_code))]
pub(crate) enum WitnessOpeningSourceKind {
    Retained,
    External,
    Embedded,
    Missing,
}

impl WitnessOpeningSourceKind {
    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Retained => "retained",
            Self::External => "external",
            Self::Embedded => "embedded",
            Self::Missing => "missing",
        }
    }
}

impl WitnessProofStageOpeningWork {
    fn add(&mut self, timing: &crate::witness_commitment::WitnessStageOpeningWorkTiming) {
        self.leaf_hash_row_count += timing.leaf_hash_rows;
        self.leaf_hash_byte_count += timing.leaf_hash_bytes;
        self.leaf_hash_arity2_row_count += timing.leaf_hash_arity2_row_count;
        self.leaf_hash_arity2_byte_count += timing.leaf_hash_arity2_byte_count;
        self.leaf_hash_arity4_row_count += timing.leaf_hash_arity4_row_count;
        self.leaf_hash_arity4_byte_count += timing.leaf_hash_arity4_byte_count;
        self.leaf_coset_extend_call_count += timing.leaf_coset_extend_call_count;
        self.leaf_coset_extend_output_byte_count += timing.leaf_coset_extend_output_byte_count;
        self.leaf_coset_extend_column_count += timing.leaf_coset_extend_column_count;
        self.leaf_coset_extend_max_column_count = self
            .leaf_coset_extend_max_column_count
            .max(timing.leaf_coset_extend_max_column_count);
        self.leaf_coset_extend_ntt_launch_count += timing.leaf_coset_extend_ntt_launch_count;
        self.leaf_coset_extend_bit_reverse_launch_count +=
            timing.leaf_coset_extend_bit_reverse_launch_count;
        self.leaf_coset_extend_ntt_stage_launch_count +=
            timing.leaf_coset_extend_ntt_stage_launch_count;
        self.leaf_coset_extend_ntt_block_twiddle_launch_count +=
            timing.leaf_coset_extend_ntt_block_twiddle_launch_count;
        self.leaf_coset_extend_normalize_launch_count +=
            timing.leaf_coset_extend_normalize_launch_count;
        self.leaf_coset_extend_pack_launch_count += timing.leaf_coset_extend_pack_launch_count;
        self.leaf_coset_extend_unpack_launch_count += timing.leaf_coset_extend_unpack_launch_count;
        self.path_parent_hash_row_count += timing.path_parent_hash_row_count;
        self.path_parent_hash_byte_count += timing.path_parent_hash_byte_count;
        self.path_parent_hash_launch_count += timing.path_parent_hash_launch_count;
        self.row_values_device_row_count += timing.row_values_device_row_count;
        self.row_values_source_row_count += timing.row_values_source_row_count;
        self.row_values_word_count += timing.row_values_word_count;
        self.row_values_byte_count += timing.row_values_byte_count;
        self.retained_leaf_digest_opening_count += timing.retained_leaf_digest_opening_count;
        self.retained_leaf_digest_opening_row_count +=
            timing.retained_leaf_digest_opening_row_count;
        self.retained_parent_checkpoint_opening_count +=
            timing.retained_parent_checkpoint_opening_count;
        self.retained_parent_checkpoint_opening_row_count +=
            timing.retained_parent_checkpoint_opening_row_count;
    }

    fn add_source(&mut self, kind: WitnessOpeningSourceKind) {
        match kind {
            WitnessOpeningSourceKind::Retained => self.retained_source_count += 1,
            WitnessOpeningSourceKind::External => self.external_source_count += 1,
            WitnessOpeningSourceKind::Embedded => self.embedded_source_count += 1,
            WitnessOpeningSourceKind::Missing => self.missing_source_count += 1,
        }
    }
}

impl WitnessProofArtifactTiming {
    pub(crate) fn add_query_plan(&mut self, duration: Duration) {
        self.query_plan += duration;
    }

    pub(crate) fn add_constant_opening(&mut self, duration: Duration) {
        self.constant_opening += duration;
    }

    pub(crate) fn add_witness_opening(&mut self, duration: Duration) {
        self.witness_opening += duration;
    }

    pub(crate) fn add_witness_opening_queries(&mut self, count: usize) {
        self.witness_opening_query_count += count;
        self.witness_opening_query_unit_count += 1;
        if count == 1 {
            self.witness_opening_single_query_unit_count += 1;
        }
        self.witness_opening_max_queries_per_unit =
            self.witness_opening_max_queries_per_unit.max(count);
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn add_witness_stage_opening_source(
        &mut self,
        stage_index: usize,
        kind: WitnessOpeningSourceKind,
    ) {
        match kind {
            WitnessOpeningSourceKind::Retained => self.witness_opening_retained_source_count += 1,
            WitnessOpeningSourceKind::External => self.witness_opening_external_source_count += 1,
            WitnessOpeningSourceKind::Embedded => self.witness_opening_embedded_source_count += 1,
            WitnessOpeningSourceKind::Missing => self.witness_opening_missing_source_count += 1,
        }
        add_stage_opening_source(&mut self.witness_stage_opening_work, stage_index, kind);
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn add_witness_external_source(&mut self, duration: Duration) {
        self.witness_external_source += duration;
    }

    #[cfg(feature = "cuda")]
    pub(crate) fn add_witness_external_source_build_timing(
        &mut self,
        timing: &crate::guest_pc_trace_backend::GuestPcDeviceSourceBuildTiming,
    ) {
        self.witness_external_source_descriptor_upload += timing.descriptor_upload_duration();
        self.witness_external_source_descriptor_upload_byte_count +=
            timing.descriptor_upload_byte_count();
        self.witness_external_source_descriptor_upload_row_count +=
            timing.descriptor_upload_row_count();
        self.witness_external_source_trace_expand += timing.trace_expand_duration();
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn add_witness_stage_external_source(
        &mut self,
        stage_index: usize,
        duration: Duration,
    ) {
        add_stage_duration(
            &mut self.witness_stage_external_source,
            stage_index,
            duration,
        );
    }

    pub(crate) fn add_witness_stage_opening(&mut self, stage_index: usize, duration: Duration) {
        self.witness_opening_stage_count += 1;
        add_stage_duration(&mut self.witness_stage_opening, stage_index, duration);
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn add_witness_stage_opening_setup(
        &mut self,
        stage_index: usize,
        duration: Duration,
    ) {
        self.witness_opening_setup += duration;
        add_stage_duration(&mut self.witness_stage_opening_setup, stage_index, duration);
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn add_witness_stage_opening_leaf_extend(
        &mut self,
        stage_index: usize,
        duration: Duration,
    ) {
        self.witness_opening_leaf_extend += duration;
        add_stage_duration(
            &mut self.witness_stage_opening_leaf_extend,
            stage_index,
            duration,
        );
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn add_witness_stage_opening_leaf_hash(
        &mut self,
        stage_index: usize,
        timing: &crate::witness_commitment::WitnessStageOpeningWorkTiming,
    ) {
        self.witness_opening_leaf_hash += timing.leaf_hash;
        self.witness_opening_leaf_hash_row_count += timing.leaf_hash_rows;
        self.witness_opening_leaf_hash_byte_count += timing.leaf_hash_bytes;
        self.witness_opening_leaf_hash_arity2_row_count += timing.leaf_hash_arity2_row_count;
        self.witness_opening_leaf_hash_arity2_byte_count += timing.leaf_hash_arity2_byte_count;
        self.witness_opening_leaf_hash_arity4_row_count += timing.leaf_hash_arity4_row_count;
        self.witness_opening_leaf_hash_arity4_byte_count += timing.leaf_hash_arity4_byte_count;
        self.witness_opening_leaf_coset_extend_call_count += timing.leaf_coset_extend_call_count;
        self.witness_opening_leaf_coset_extend_output_byte_count +=
            timing.leaf_coset_extend_output_byte_count;
        self.witness_opening_leaf_coset_extend_column_count +=
            timing.leaf_coset_extend_column_count;
        self.witness_opening_leaf_coset_extend_max_column_count = self
            .witness_opening_leaf_coset_extend_max_column_count
            .max(timing.leaf_coset_extend_max_column_count);
        self.witness_opening_leaf_coset_extend_ntt_launch_count +=
            timing.leaf_coset_extend_ntt_launch_count;
        self.witness_opening_leaf_coset_extend_bit_reverse_launch_count +=
            timing.leaf_coset_extend_bit_reverse_launch_count;
        self.witness_opening_leaf_coset_extend_ntt_stage_launch_count +=
            timing.leaf_coset_extend_ntt_stage_launch_count;
        self.witness_opening_leaf_coset_extend_ntt_block_twiddle_launch_count +=
            timing.leaf_coset_extend_ntt_block_twiddle_launch_count;
        self.witness_opening_leaf_coset_extend_normalize_launch_count +=
            timing.leaf_coset_extend_normalize_launch_count;
        self.witness_opening_leaf_coset_extend_pack_launch_count +=
            timing.leaf_coset_extend_pack_launch_count;
        self.witness_opening_leaf_coset_extend_unpack_launch_count +=
            timing.leaf_coset_extend_unpack_launch_count;
        self.witness_opening_path_parent_hash_row_count += timing.path_parent_hash_row_count;
        self.witness_opening_path_parent_hash_byte_count += timing.path_parent_hash_byte_count;
        self.witness_opening_path_parent_hash_launch_count += timing.path_parent_hash_launch_count;
        self.witness_opening_row_values_source_extend += timing.row_values_source_extend;
        self.witness_opening_row_values_source_download += timing.row_values_source_download;
        self.witness_opening_row_values_device_download += timing.row_values_device_download;
        self.witness_opening_row_values_device_row_count += timing.row_values_device_row_count;
        self.witness_opening_row_values_source_row_count += timing.row_values_source_row_count;
        self.witness_opening_row_values_word_count += timing.row_values_word_count;
        self.witness_opening_row_values_byte_count += timing.row_values_byte_count;
        self.witness_opening_retained_leaf_digest_opening_count +=
            timing.retained_leaf_digest_opening_count;
        self.witness_opening_retained_leaf_digest_opening_row_count +=
            timing.retained_leaf_digest_opening_row_count;
        self.witness_opening_retained_parent_checkpoint_opening_count +=
            timing.retained_parent_checkpoint_opening_count;
        self.witness_opening_retained_parent_checkpoint_opening_row_count +=
            timing.retained_parent_checkpoint_opening_row_count;
        add_stage_opening_work(&mut self.witness_stage_opening_work, stage_index, timing);
        add_stage_duration(
            &mut self.witness_stage_opening_leaf_hash,
            stage_index,
            timing.leaf_hash,
        );
        add_stage_duration(
            &mut self.witness_stage_opening_row_value_source_extend,
            stage_index,
            timing.row_values_source_extend,
        );
        add_stage_duration(
            &mut self.witness_stage_opening_row_value_source_download,
            stage_index,
            timing.row_values_source_download,
        );
        add_stage_duration(
            &mut self.witness_stage_opening_row_value_device_download,
            stage_index,
            timing.row_values_device_download,
        );
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn add_witness_stage_opening_path(
        &mut self,
        stage_index: usize,
        duration: Duration,
    ) {
        self.witness_opening_path += duration;
        add_stage_duration(&mut self.witness_stage_opening_path, stage_index, duration);
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn add_witness_stage_opening_row_values(
        &mut self,
        stage_index: usize,
        duration: Duration,
    ) {
        self.witness_opening_row_values += duration;
        add_stage_duration(
            &mut self.witness_stage_opening_row_values,
            stage_index,
            duration,
        );
    }

    pub(crate) fn add_fri_opening(&mut self, duration: Duration) {
        self.fri_opening += duration;
    }

    pub(crate) fn add_fri_opening_build_timing(
        &mut self,
        timing: &crate::pcs_fri::PcsFriOpeningBuildTiming,
    ) {
        self.fri_opening_unit_build += timing.unit_build;
        self.fri_opening_layer_tree += timing.layer_tree;
        self.fri_opening_query += timing.query_work;
        self.fri_opening_fold += timing.fold_work;
        self.fri_opening_unit_count += timing.unit_count;
        self.fri_opening_layer_count += timing.layer_count;
        self.fri_opening_query_count += timing.query_count;
    }
}

fn add_stage_duration(
    entries: &mut Vec<WitnessProofStageOpeningTiming>,
    stage_index: usize,
    duration: Duration,
) {
    if let Some(entry) = entries
        .iter_mut()
        .find(|entry| entry.stage_index == stage_index)
    {
        entry.duration += duration;
        return;
    }
    entries.push(WitnessProofStageOpeningTiming {
        stage_index,
        duration,
    });
    entries.sort_by_key(|entry| entry.stage_index);
}

fn add_stage_opening_work(
    entries: &mut Vec<WitnessProofStageOpeningWork>,
    stage_index: usize,
    timing: &crate::witness_commitment::WitnessStageOpeningWorkTiming,
) {
    if let Some(entry) = entries
        .iter_mut()
        .find(|entry| entry.stage_index == stage_index)
    {
        entry.add(timing);
        return;
    }
    let mut entry = WitnessProofStageOpeningWork {
        stage_index,
        ..WitnessProofStageOpeningWork::default()
    };
    entry.add(timing);
    entries.push(entry);
    entries.sort_by_key(|entry| entry.stage_index);
}

fn add_stage_opening_source(
    entries: &mut Vec<WitnessProofStageOpeningWork>,
    stage_index: usize,
    kind: WitnessOpeningSourceKind,
) {
    if let Some(entry) = entries
        .iter_mut()
        .find(|entry| entry.stage_index == stage_index)
    {
        entry.add_source(kind);
        return;
    }
    let mut entry = WitnessProofStageOpeningWork {
        stage_index,
        ..WitnessProofStageOpeningWork::default()
    };
    entry.add_source(kind);
    entries.push(entry);
    entries.sort_by_key(|entry| entry.stage_index);
}
