use std::io::Error;
use std::os::fd::RawFd;
use crate::engine::coroutine::YieldStatus;
use crate::{spawn_local_move};
use crate::engine::io::Token;
use crate::utils::{Buffer, Ptr};

pub struct TcpStream {
    /// TODO: new docs
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
    is_registered: bool,
    data: Ptr<Token>
}

impl TcpStream {
    pub fn new(fd: RawFd) -> Self {
        Self {
            is_registered: false,
            data: Ptr::new(unsafe { Token::new_empty(fd) })
        }
    }

    #[inline(always)]
    pub fn token_ptr(&mut self) -> Ptr<Token> {
        self.data
    }

    pub fn is_registered(&self) -> bool {
        self.is_registered
    }

    pub fn set_registered(&mut self, is_registered: bool) {
        self.is_registered = is_registered;
    }

    pub fn read(&mut self, res: *mut Result<&'static [u8], Error>) -> YieldStatus {
        let is_registered = self.is_registered();
        if !is_registered {
            self.set_registered(true);
        }
        YieldStatus::tcp_read(is_registered, self.data, res)
    }

    pub fn write(&mut self, buf: Buffer, res: *mut Result<usize, Error>) -> YieldStatus {
        YieldStatus::tcp_write(self.data, buf, res)
    }

    pub fn write_all(&mut self, buf: Buffer, res: *mut Result<(), Error>) -> YieldStatus {
        YieldStatus::tcp_write_all(self.data, buf, res)
    }

    fn close(token_ref: Ptr<Token>) -> YieldStatus {
        YieldStatus::tcp_close(token_ref)
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        let token_ptr = self.data;
        if self.is_registered() {
            spawn_local_move!({
                yield TcpStream::close(token_ptr);
                unsafe { token_ptr.drop_in_place(); }
            });
        } else {
            unsafe { token_ptr.drop_in_place(); }
        }
    }
}