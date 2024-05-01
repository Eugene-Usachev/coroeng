use std::io::Error;
use crate::engine::coroutine::coroutine::YieldStatus;
use crate::{spawn_local, spawn_local_move};
use crate::utils::Buffer;

pub struct TcpStream {
    /// # Why do we not use `usize` and `bool`?
    ///
    /// We need to store two pieces of information: `token_id` and `is_registered`.
    /// We could do so by using a `token_id` field of type `usize` and a `is_registered` field of type `bool`.
    /// However, this would result in a total size of 16 bytes, as the `usize` takes up 8 bytes and the `bool` takes up another 8.
    /// This is because the `usize` is aligned to the 8-byte boundary, which means that the `bool` must also be aligned to 8 bytes.
    ///
    /// We reasoned as follows:
    /// * most of the time, the structure simply takes up memory
    /// * the rest of the time is extremely short, and is used only to extract these two fields.
    ///
    /// Therefore, we decided to sacrifice a very small portion of performance (< 1 nanosecond per operation) in order to save 12 bytes per connection.
    /// Given that one computer can handle several million connections, this saves a significant amount of memory.
    /// This approach limits the number of connections per core to 2,147,483,648. We believe this is sufficient for all purposes.
    /// As a result, we retain the first bit for `is_registered`, and the next 31 bits for `token_id`.
    data: u32,
}

impl TcpStream {
    pub fn new(token_id: usize) -> Self {
        Self { data: (token_id as u32) << 1 }
    }

    pub fn token_id(&self) -> usize {
        (self.data >> 1) as usize
    }

    pub fn is_registered(&self) -> bool {
        (self.data & 1) != 0
    }

    pub fn set_registered(&mut self, is_registered: bool) {
        if is_registered {
            self.data |= 1;
        } else {
            self.data &= !1;
        }
    }

    pub fn read(&mut self, res: *mut Result<&'static [u8], Error>) -> YieldStatus {
        let is_registered = self.is_registered();
        if !is_registered {
            self.set_registered(true);
        }
        YieldStatus::TcpRead(is_registered, self.token_id(), res)
    }

    pub fn write(&self, buf: Buffer, res: *mut Result<usize, Error>) -> YieldStatus {
        YieldStatus::TcpWrite(self.token_id(), buf, res)
    }

    pub fn write_all(&self, buf: Buffer, res: *mut Result<(), Error>) -> YieldStatus {
        YieldStatus::TcpWriteAll(self.token_id(), buf, res)
    }

    fn close(token_id: usize) -> YieldStatus {
        YieldStatus::TcpClose(token_id)
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        if !self.is_registered() {
            return;
        }
        let token_id = self.token_id();
        spawn_local_move!({
            yield TcpStream::close(token_id);
        });
    }
}