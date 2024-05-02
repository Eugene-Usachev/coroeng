#![feature(core_intrinsics)]
#![feature(trait_alias)]
#![feature(coroutines, coroutine_trait)]
#![feature(fn_traits)]

use std::io::Read;
use std::net::ToSocketAddrs;
use crate::engine::net::tcp::{TcpListener, TcpStream};

mod engine;
mod utils;

pub fn local_test() {
    tcp_benchmark();
}

fn tcp_benchmark() {
    run_on_all_cores!({
        let mut listener = io_yield!(TcpListener::new, "engine:8081".to_socket_addrs().unwrap().next().unwrap());
        loop {
            let stream_ = io_yield!(TcpListener::accept, &mut listener);

            if stream_.is_err() {
                println!("accept failed, reason: {}", stream_.err().unwrap());
                continue;
            }

            let mut stream: TcpStream = stream_.unwrap();
            spawn_local_move!({
                loop {
                    let mut slice = io_yield!(TcpStream::read, &mut stream).unwrap();

                    if slice.is_empty() {
                        break;
                    }

                    let mut buf = utils::buffer();
                    buf.append(slice);

                    let res = io_yield!(TcpStream::write_all, &mut stream, buf);
                }
            });
        }
    });
}


fn main() {
    local_test();
}