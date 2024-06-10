use std::io::Error;
use crate::coroutine::YieldStatus;
// TODO docs
use crate::io::State;
use crate::utils::Ptr;

pub struct File {
    state: Ptr<State>
}

impl File {
    pub fn new(res: *mut Result<File, Error>) -> YieldStatus {
        YieldStatus::new_file(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc::test_local;

    #[test_local(crate="crate")]
    fn new() {

    }
}