#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fault {
    Invalid,
    Busy,
    Failed,
}
