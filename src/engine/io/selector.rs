use std::io::Error;
use std::os::fd::RawFd;
use crate::engine::io::Token;
use crate::engine::local::Scheduler;

pub(crate) trait Selector {
    fn insert_token(&mut self, token: Token) -> usize;
    fn get_token_mut_ref(&mut self, token_id: usize) -> &mut Token;
    fn poll(&mut self, scheduler: &mut Scheduler) -> Result<(), ()>;
    fn register(&mut self, fd: RawFd, token_id: usize);
    fn deregister(&mut self, token_id: usize) -> Token;

    fn write(&mut self, token_id: usize, buf: &[u8]) -> Result<usize, Error>;
    fn close_connection(token: Token);
}