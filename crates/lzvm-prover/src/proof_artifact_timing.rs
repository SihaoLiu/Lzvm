use std::time::Duration;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WitnessProofArtifactTiming {
    pub query_plan: Duration,
    pub constant_opening: Duration,
    pub witness_opening: Duration,
    pub witness_opening_query_count: usize,
    pub witness_opening_stage_count: usize,
    pub witness_external_source: Duration,
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
    pub witness_opening_path: Duration,
    pub witness_opening_row_values: Duration,
    pub witness_stage_external_source: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening_setup: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening_leaf_extend: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening_leaf_hash: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening_work: Vec<WitnessProofStageOpeningWork>,
    pub witness_stage_opening_path: Vec<WitnessProofStageOpeningTiming>,
    pub witness_stage_opening_row_values: Vec<WitnessProofStageOpeningTiming>,
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
    }

    #[cfg_attr(not(feature = "cuda"), allow(dead_code))]
    pub(crate) fn add_witness_external_source(&mut self, duration: Duration) {
        self.witness_external_source += duration;
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
        add_stage_opening_work(&mut self.witness_stage_opening_work, stage_index, timing);
        add_stage_duration(
            &mut self.witness_stage_opening_leaf_hash,
            stage_index,
            timing.leaf_hash,
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
