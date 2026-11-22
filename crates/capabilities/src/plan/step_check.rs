use std::{future::Future, pin::Pin};

pub type CheckFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

pub trait StepCheck: Send + Sync {
    fn run(&self) -> CheckFuture<'_>;
}
