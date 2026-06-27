use std::collections::BTreeMap;

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
