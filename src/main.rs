#![feature(coroutines)]
#![feature(coroutine_trait)]
#![feature(stmt_expr_attributes)]
#![feature(gen_blocks)]

use std::net::ToSocketAddrs;
use engine::net::tcp::{TcpListener, TcpStream};
use engine::{io_yield, run_on_all_cores, spawn_local_move, coro};
use engine::sync::{Mutex};
use engine::utils::{set_panic_hook};

static L: Mutex<i32> = Mutex::new(0);

pub fn local_test() {
    tcp_benchmark();

    // run_on_all_cores!({
    //     for i in 0..10 {
    //         let res = L.lock();
    //         println!("was locked: {}", res.is_ok());
    //         if res.is_ok() {
    //             let mut r = res.unwrap();
    //             *r += i;
    //         }
    //     }
    // });
}

fn tcp_benchmark() {
    #[coro]
    fn handle_tcp_client(mut stream: TcpStream) {
        loop {
            let slice = io_yield!(TcpStream::read, &mut stream).unwrap();

            if slice.is_empty() {
                break;
            }

            let mut buf = engine::utils::buffer();
            buf.append(slice);

            let res = io_yield!(TcpStream::write_all, &mut stream, buf);

            if res.is_err() {
                println!("write failed, reason: {}", res.err().unwrap());
                break;
            }
        }
    }

    run_on_all_cores!({
        let mut listener = io_yield!(TcpListener::new, "engine:8081".to_socket_addrs().unwrap().next().unwrap());
        loop {
            let stream_ = io_yield!(TcpListener::accept, &mut listener);

            if stream_.is_err() {
                println!("accept failed, reason: {}", stream_.err().unwrap());
                continue;
            }

            let mut stream: TcpStream = stream_.unwrap();
            spawn_local_move!(handle_tcp_client(stream));
        }
    });
}

fn main() {
    set_panic_hook("worker on core 0".to_string());
    local_test();
}