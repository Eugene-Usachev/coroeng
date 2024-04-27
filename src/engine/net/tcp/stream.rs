use std::io::Error;
use crate::engine::coroutine::coroutine::YieldStatus;
use crate::{spawn_local, spawn_local_move};
use crate::utils::Buffer;

pub struct TcpStream {
    pub(crate) token_id: usize,
    pub(crate) is_registered: bool
}

impl TcpStream {
    pub fn new(token_id: usize) -> Self {
        Self { token_id, is_registered: false }
    }

    pub fn read(&mut self, res: *mut Result<&'static [u8], Error>) -> YieldStatus {
        let is_registered = self.is_registered;
        if !is_registered {
            self.is_registered = true;
        }
        YieldStatus::TcpRead(is_registered, self.token_id, res)
    }

    pub fn write(&self, buf: Buffer, res: *mut Result<usize, Error>) -> YieldStatus {
        YieldStatus::TcpWrite(self.token_id, buf, res)
    }

    pub fn write_all(&self, buf: Buffer, res: *mut Result<(), Error>) -> YieldStatus {
        YieldStatus::TcpWriteAll(self.token_id, buf, res)
    }

    fn close(token_id: usize) -> YieldStatus {
        YieldStatus::TcpClose(token_id)
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        if !self.is_registered {
            return;
        }
        let token_id = self.token_id;
        spawn_local_move!({
            yield TcpStream::close(token_id);
        });
    }
}