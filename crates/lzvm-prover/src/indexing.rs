use std::collections::{BTreeMap, BTreeSet};

use lzvm_artifacts::pcs_query_segment::PcsQueryPlanUnit;

pub(crate) fn index_first_by_key<'a, T, K, F>(items: &'a [T], key: F) -> BTreeMap<K, &'a T>
where
    K: Ord,
    F: Fn(&T) -> K,
{
    let mut indexed = BTreeMap::new();
    for item in items {
        indexed.entry(key(item)).or_insert(item);
    }
    indexed
}

pub(crate) fn index_first_position_by_key<T, K, F>(items: &[T], key: F) -> BTreeMap<K, usize>
where
    K: Ord,
    F: Fn(&T) -> K,
{
    let mut indexed = BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        indexed.entry(key(item)).or_insert(index);
    }
    indexed
}

pub(crate) fn collect_unique_query_identities<E, O, D>(
    query_units: &[PcsQueryPlanUnit],
    mut overflow: O,
    mut duplicate: D,
) -> Result<BTreeSet<(u32, u32)>, E>
where
    O: FnMut() -> E,
    D: FnMut(usize) -> E,
{
    let mut identities = BTreeSet::new();
    for unit in query_units {
        let unit_index = usize::try_from(unit.unit_index).map_err(|_| overflow())?;
        let identity = (unit.unit_index, unit.trace_instance_index);
        if !identities.insert(identity) {
            return Err(duplicate(unit_index));
        }
    }
    Ok(identities)
}
