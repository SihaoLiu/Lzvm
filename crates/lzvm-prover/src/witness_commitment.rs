mod errors;
mod extend;
mod load;
mod segment;
mod trace;
mod tree;
mod values;

pub use errors::*;
pub use extend::*;
pub use load::*;
pub use segment::*;
pub use trace::*;
pub use tree::*;
pub use values::*;

const HASH_WORDS: usize = 4;
const WORD_BYTES: usize = 8;
const NTT_COLUMN_GROUP_SIZE: usize = 4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CosetExtendLaunchWork {
    pub(crate) ntt_launch_count: usize,
    pub(crate) bit_reverse_launch_count: usize,
    pub(crate) ntt_stage_launch_count: usize,
    pub(crate) ntt_block_twiddle_launch_count: usize,
    pub(crate) normalize_launch_count: usize,
    pub(crate) pack_launch_count: usize,
    pub(crate) unpack_launch_count: usize,
}

pub(crate) fn coset_extend_launch_work(
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
) -> CosetExtendLaunchWork {
    let column_group_count = column_count.div_ceil(NTT_COLUMN_GROUP_SIZE);
    let bit_reverse_launch_count = column_group_count.saturating_mul(2);
    let ntt_stage_launch_count = column_group_count.saturating_mul(
        coset_extend_ntt_stage_launch_count(source_bits)
            .saturating_add(coset_extend_ntt_stage_launch_count(target_bits)),
    );
    let ntt_block_twiddle_launch_count = column_group_count.saturating_mul(
        coset_extend_ntt_block_twiddle_launch_count(source_bits)
            .saturating_add(coset_extend_ntt_block_twiddle_launch_count(target_bits)),
    );
    CosetExtendLaunchWork {
        ntt_launch_count: bit_reverse_launch_count
            .saturating_add(ntt_stage_launch_count)
            .saturating_add(ntt_block_twiddle_launch_count),
        bit_reverse_launch_count,
        ntt_stage_launch_count,
        ntt_block_twiddle_launch_count,
        normalize_launch_count: column_group_count,
        pack_launch_count: 1,
        unpack_launch_count: 1,
    }
}

fn coset_extend_ntt_stage_launch_count(bits: usize) -> usize {
    // Canonical CUDA NTTs fuse stages 1 through 9 into one shared-memory launch.
    usize::from(bits != 0)
}

fn coset_extend_ntt_block_twiddle_launch_count(bits: usize) -> usize {
    bits.saturating_sub(9)
}

#[cfg(test)]
mod launch_work_tests {
    use super::coset_extend_launch_work;

    #[test]
    fn coset_extend_launch_work_splits_ntt_and_memory_launches() {
        let work = coset_extend_launch_work(3, 22, 25);

        assert_eq!(work.bit_reverse_launch_count, 2);
        assert_eq!(work.ntt_stage_launch_count, 2);
        assert_eq!(work.ntt_block_twiddle_launch_count, 29);
        assert_eq!(work.ntt_launch_count, 33);
        assert_eq!(work.normalize_launch_count, 1);
        assert_eq!(work.pack_launch_count, 1);
        assert_eq!(work.unpack_launch_count, 1);

        let two_groups = coset_extend_launch_work(5, 22, 25);
        assert_eq!(two_groups.bit_reverse_launch_count, 4);
        assert_eq!(two_groups.ntt_stage_launch_count, 4);
        assert_eq!(two_groups.ntt_block_twiddle_launch_count, 58);
        assert_eq!(two_groups.ntt_launch_count, 66);
        assert_eq!(two_groups.normalize_launch_count, 2);
    }
}
