use std::fmt::Debug;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn run_each<T: Debug + Copy>(items: impl IntoIterator<Item = T>, body: impl Fn(T)) {
    for item in items {
        assert!(
            catch_unwind(AssertUnwindSafe(|| body(item))).is_ok(),
            "failed case: {item:?}"
        );
    }
}
