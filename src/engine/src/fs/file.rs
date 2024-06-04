use crate::coroutine::YieldStatus;
// TODO docs
use crate::io::PollState;
use crate::utils::Ptr;

pub struct File {
    state: Ptr<PollState>
}

#[cfg(test)]
mod tests {
    use proc::test_local;
    use super::*;

    #[test_local]
    fn new() {

    }
}