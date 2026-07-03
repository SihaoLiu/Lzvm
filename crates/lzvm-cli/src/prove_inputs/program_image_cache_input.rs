use std::path::Path;

use lzvm_artifacts::program_image::{
    read_program_image_commitment_cache_file, ProgramImageCommitmentCache,
};

pub(super) fn read_optional_program_image_cache(
    path: Option<&Path>,
) -> Result<Option<ProgramImageCommitmentCache>, String> {
    path.map(|path| {
        read_program_image_commitment_cache_file(path).map_err(|error| {
            format!(
                "read program-image cache failed: {}: {error}",
                path.display()
            )
        })
    })
    .transpose()
}
