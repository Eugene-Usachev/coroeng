#![feature(coroutines)]
#![feature(coroutine_trait)]
#![feature(stmt_expr_attributes)]
#![feature(gen_blocks)]
//
// use std::net::ToSocketAddrs;
// use engine::net::tcp::{TcpListener, TcpStream};
// use engine::{coro, io_yield, run_on_all_cores, spawn_local};
// use engine::sync::{Mutex};
//
// static L: Mutex<i32> = Mutex::new(0);
//
// pub fn local_test() {
//     tcp_benchmark();
//
//     // run_on_all_cores!({
//     //     for i in 0..10 {
//     //         let res = L.lock();
//     //         println!("was locked: {}", res.is_ok());
//     //         if res.is_ok() {
//     //             let mut r = res.unwrap();
//     //             *r += i;
//     //         }
//     //     }
//     // });
// }
//
// fn tcp_benchmark() {
//     #[coro]
//     fn handle_tcp_client(mut stream: TcpStream) {
//         loop {
//             let slice = io_yield!(TcpStream::read, &mut stream).unwrap();
//
//             if slice.is_empty() {
//                 break;
//             }
//
//             let mut buf = engine::utils::buffer();
//             buf.append(slice);
//
//             let res = io_yield!(TcpStream::write_all, &mut stream, buf);
//
//             if res.is_err() {
//                 println!("write failed, reason: {}", res.err().unwrap());
//                 break;
//             }
//         }
//     }
//
//     #[coro]
//     fn start_server() {
//         let mut listener = io_yield!(TcpListener::new, "engine:8081".to_socket_addrs().unwrap().next().unwrap());
//         loop {
//             let stream_ = io_yield!(TcpListener::accept, &mut listener);
//
//             if stream_.is_err() {
//                 println!("accept failed, reason: {}", stream_.err().unwrap());
//                 continue;
//             }
//
//             let stream: TcpStream = stream_.unwrap();
//             spawn_local!(handle_tcp_client(stream));
//         }
//     }
//
//     run_on_all_cores(start_server);
// }

use std::mem::MaybeUninit;
use std::net::ToSocketAddrs;
use engine::{coro, print_ret, ret_yield, run_on_all_cores, spawn_local, wait};
use engine::net::{TcpListener, TcpStream};
use engine::utils::Buffer;

fn bar(a: usize, res: *mut usize) {
    if a % 2 == 0 {
        unsafe { *res = 0; }
        return;
    }
    unsafe { *res = 1; }
}

#[print_ret]
fn a(b: usize) -> usize {
    if b % 2 == 0 {
        return 0;
    }
    1
}

// #[coro]
// fn with_ret(mut stream: TcpStream) -> usize {
//     let slice = ret_yield!(TcpStream::read, &mut stream).unwrap();
//     return slice.len();
// }

// #[coro]
// fn difficult_write(mut stream: TcpStream, buf: Buffer) {
//     let res = ret_yield!(TcpStream::write_all, &mut stream, buf);
//     if res.is_err() {
//         println!("write failed, reason: {}", res.err().unwrap());
//     }
// }
//
// #[coro]
// fn handle_tcp_client(mut stream: TcpStream) {
//     let mut buf = engine::utils::buffer();
//     buf.append("hello".as_bytes());
//
//     // Here we need to wait for difficult_write to finish
//     wait!(difficult_write(stream, buf));
// }
//
// #[coro]
// fn start_server() {
//     let mut listener = ret_yield!(TcpListener::new, "engine:8081".to_socket_addrs().unwrap().next().unwrap());
//     loop {
//         // We needn't wait here, because we want to handle all connections in parallel
//         spawn_local!(handle_tcp_client(ret_yield!(TcpListener::accept, &mut listener).unwrap()));
//     }
// }

fn main() {
    run_on_all_cores(with_ret);
}