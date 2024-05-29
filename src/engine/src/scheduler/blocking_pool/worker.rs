use std::hint::spin_loop;
use std::os::fd::IntoRawFd;
use std::thread;
use crossbeam::queue::{SegQueue};
use crate::io::BlockingState;
use crate::net::TcpStream;
use crate::{write_err, write_ok};
use crate::coroutine::CoroutineImpl;

// TODO add thread park
pub(super) struct Worker {
    input: SegQueue<BlockingState>,
    output: SegQueue<CoroutineImpl>
}

impl Worker {
    pub(super) fn new() -> Self {
        Self {
            input: SegQueue::new(),
            output: SegQueue::new()
        }
    }

    pub(super) fn run(&self) {
        loop {
            for _ in 0..3 {
                while let Some(state) = self.input.pop() {
                    match state {
                        BlockingState::ConnectTcp(state) => {
                            let stream = std::net::TcpStream::connect(state.address);
                            if let Ok(stream) = stream {
                                write_ok!(state.result, TcpStream::new(stream.into_raw_fd()));
                            } else {
                                write_err!(state.result, stream.unwrap_err_unchecked());
                            }

                            self.output.push(state.coroutine);
                        }
                    }
                }

                for _ in 0..10 {
                    spin_loop();
                }
            }

            thread::yield_now();
        }
    }

    #[inline(always)]
    pub(super) fn get_ready(&self, ready: &mut Vec<CoroutineImpl>) {
        while let Some(coroutine) = self.output.pop() {
            ready.push(coroutine);
        }
    }

    #[inline(always)]
    pub(super) fn put_state(&self, state: BlockingState) {
        self.input.push(state);
    }
}

unsafe impl Send for Worker {}
unsafe impl Sync for Worker {}