use std::collections::BTreeMap;

use crate::source_fixed_columns::SourceFixedColumnsWriteError;

use super::SourceFixedDynamicOperation;

pub(super) fn source_fixed_dynamic_merge_values(
    staged_columns: &mut BTreeMap<String, Vec<Option<u64>>>,
    operation: &SourceFixedDynamicOperation,
    values: Vec<Option<u64>>,
) -> Result<(), SourceFixedColumnsWriteError> {
    let target = staged_columns
        .entry(operation.target_column.clone())
        .or_insert_with(|| vec![None; values.len()]);
    for (row, value) in values.into_iter().enumerate() {
        let Some(value) = value else {
            continue;
        };
        match target.get_mut(row) {
            Some(slot @ None) => *slot = Some(value),
            Some(Some(existing)) if *existing == value => {}
            Some(Some(_)) | None => {
                return Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
                    source_name: operation.source_name.clone(),
                    column: operation.target_column.clone(),
                });
            }
        }
    }
    Ok(())
}
