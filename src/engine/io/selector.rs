use std::io::Error;
use std::os::fd::RawFd;
use crate::engine::io::Token;
use crate::engine::local::Scheduler;
use crate::utils::Buffer;

pub(crate) trait Selector {
    fn insert_token(&mut self, token: Token) -> usize;
    fn get_token_mut_ref(&mut self, token_id: usize) -> &mut Token;
    fn poll(&mut self, scheduler: &mut Scheduler) -> Result<(), ()>;
    fn register(&mut self, fd: RawFd, token_id: usize);
    fn deregister(&mut self, token_id: usize) -> Token;

    /// write is async. it will wake task up later.
    fn write(&mut self, token_id: usize);
    /// write_all is async. it will wake task up later.
    fn write_all(&mut self, token_id: usize);
    fn close_connection(token: Token);
}