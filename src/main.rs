#![feature(core_intrinsics)]
#![feature(trait_alias)]
#![feature(coroutines, coroutine_trait)]
#![feature(fn_traits)]
#![feature(panic_info_message)]

use std::io::Read;
use std::net::ToSocketAddrs;
use std::time::Duration;
use crate::engine::net::tcp::{TcpListener, TcpStream};
use crate::engine::sleep::sleep::sleep;
use crate::engine::utils::{set_panic_hook};

mod engine;

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

                    let mut buf = engine::utils::buffer();
                    buf.append(slice);

                    let res = io_yield!(TcpStream::write_all, &mut stream, buf);
                }
            });
        }

        yield sleep(Duration::from_secs(123213213));
    });
}

fn main() {
    set_panic_hook("worker on core 0".to_string());
    local_test();
}