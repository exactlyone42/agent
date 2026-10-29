use std::collections::BTreeMap;

pub fn all_passed<K>(results: &BTreeMap<K, i32>) -> bool {
    results.values().all(|code| *code == 0)
}
