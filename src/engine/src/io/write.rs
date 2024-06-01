use std::io::Error;
use crate::buf::Buffer;
use crate::coroutine::YieldStatus;

/// The AsyncWrite trait provides asynchronous write functionality for various types of data.
/// It defines methods for writing data either [`partially`](AsyncWrite::write) or [`completely`](AsyncWrite::write_all).
pub trait AsyncWrite<T> {
    /// Writes a data to this writer.
    ///
    /// # With [`Buffer`]
    ///
    /// Returns [`Option<Buffer>`] or an error.
    /// If Option is [`None`], all data has been written.
    /// Else `Buffer` contains new `offset` field, that indicates how many bytes have been written.
    ///
    /// # Note
    ///
    /// Don't worry about moving [`Buffer`]. In most cases buffer will be moved in [`BufPool`](crate::buf::BufPool).
    /// So using [`buffer`](crate::buf::buffer) and moving the buffer is lead to reusing memory and avoiding allocations.
    ///
    /// # Without [`Buffer`]
    ///
    /// Returns [`Option<T>`] or an error.
    /// It can be [`None`] if all data has been written.
    /// Else `T` can be changed for the next writing.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine::coro;
    /// use engine::net::TcpStream;
    /// use engine::buf::Buffer;
    /// use std::io::Error;
    /// use engine::io::AsyncWrite;
    ///
    /// #[coro]
    /// fn write_to_stream(mut stream: TcpStream, mut buf: Buffer) {
    ///     loop {
    ///         let res: Result<Option<Buffer>, Error> = yield stream.write(buf);
    ///         if res.is_err() {
    ///             println!("write failed, reason: {}", res.err().unwrap());
    ///             break;
    ///         }
    ///
    ///         if let Some(new_buf) = res.unwrap() {
    ///             buf = new_buf;
    ///         } else {
    ///             break;
    ///         }
    ///     }
    /// }
    /// ```
    fn write(&mut self, data: T, res: *mut Result<Option<T>, Error>) -> YieldStatus;

    /// Writes all data to this writer or returns an error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine::coro;
    /// use engine::net::TcpStream;
    /// use engine::buf::Buffer;
    /// use std::io::Error;
    /// use engine::io::AsyncWrite;
    ///
    /// #[coro]
    /// fn write_to_stream(mut stream: TcpStream, mut buf: Buffer) {
    ///     let res: Result<(), Error> = yield stream.write_all(buf);
    ///     if res.is_err() {
    ///         println!("write failed, reason: {}", res.err().unwrap());
    ///         break;
    ///     }
    ///     // all data has been written
    /// }
    /// ```
    fn write_all(&mut self, data: T, res: *mut Result<(), Error>) -> YieldStatus;
}