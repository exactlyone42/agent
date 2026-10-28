#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Claim {
    Pending,
    Granted,
    Denied,
}
