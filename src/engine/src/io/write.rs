use std::io::Error;
use crate::buf::Buffer;
use crate::coroutine::YieldStatus;

/// The AsyncWrite trait provides asynchronous write functionality for various types of data.
/// It defines methods for writing data either [`partially`](AsyncWrite::write) or [`completely`](AsyncWrite::write_all).
/// 
/// # Position based write
/// 
/// [`AsyncPWrite`] is a position based write.
pub trait AsyncWrite {
    /// Writes a data to this writer.
    ///
    /// It returns [`Buffer`] with new `offset` field (read [`Buffer::offset`] and [`Buffer`] for more info).
    /// So, `offset` contains the number of bytes written. You can check if all bytes were written by [`Buffer::written_full`].
    ///
    /// # Example
    ///
    /// ```ignore
    /// use engine::coro;
    /// use engine::net::TcpStream;
    /// use engine::buf::Buffer;
    /// use std::io::Error;
    /// use engine::io::AsyncWrite;
    ///
    ///  #[coro]
    ///  fn write_all_to_stream(mut stream: TcpStream, mut buf: Buffer) {
    ///      loop {
    ///          let res: Result<Buffer, Error> = yield stream.write(buf);
    ///          if res.is_err() {
    ///              println!("write failed, reason: {}", res.err().unwrap());
    ///              break;
    ///          }
    ///          
    ///          buf = res.unwrap();
    ///          if buf.written_full() {
    ///              break;
    ///          }
    ///      }
    ///  }
    /// ```
    fn write(&mut self, data: Buffer, res: *mut Result<Buffer, Error>) -> YieldStatus;
    
    /// Writes all data to this writer or returns an error.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use engine::coro;
    /// use engine::net::TcpStream;
    /// use engine::buf::Buffer;
    /// use std::io::Error;
    /// use engine::io::AsyncWrite;
    ///
    /// #[coro]
    /// fn write_to_stream(mut stream: TcpStream, buf: Buffer) {
    ///     let res: Result<Buffer, Error> = yield stream.write_all(buf);
    ///     if res.is_err() {
    ///         println!("write failed, reason: {}", res.err().unwrap());
    ///         break;
    ///     }
    ///     // all data has been written
    /// }
    /// ```
    fn write_all(&mut self, data: Buffer, res: *mut Result<Buffer, Error>) -> YieldStatus;
}

// TODO docs
pub trait AsyncPWrite {
    fn pwrite(&mut self, data: Buffer, offset: usize, res: *mut Result<Buffer, Error>) -> YieldStatus;
    fn pwrite_all(&mut self, data: Buffer, offset: usize, res: *mut Result<Buffer, Error>) -> YieldStatus;
}