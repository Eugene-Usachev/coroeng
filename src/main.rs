// // //
// // // use std::net::ToSocketAddrs;
// // // use engine::net::tcp::{TcpListener, TcpStream};
// // // use engine::{coro, io_yield, run_on_all_cores, spawn_local};
// // // use engine::sync::{Mutex};
// // //
// // // static L: Mutex<i32> = Mutex::new(0);
// // //
// // // pub fn local_test() {
// // //     tcp_benchmark();
// // //
// // //     // run_on_all_cores!({
// // //     //     for i in 0..10 {
// // //     //         let res = L.lock();
// // //     //         println!("was locked: {}", res.is_ok());
// // //     //         if res.is_ok() {
// // //     //             let mut r = res.unwrap();
// // //     //             *r += i;
// // //     //         }
// // //     //     }
// // //     // });
// // // }
// // //
//
#![feature(coroutines)]
#![feature(coroutine_trait)]
#![feature(stmt_expr_attributes)]
#![feature(gen_blocks)]

use std::io::Error;
use std::net::ToSocketAddrs;
use engine::{coro, run_on_all_cores, spawn_local};
use engine::net::{TcpListener, TcpStream};

fn tcp_benchmark() {
    #[coro]
    fn handle_tcp_client(mut stream: TcpStream) {
        loop {
            let slice: &[u8] = (yield stream.read()).unwrap();

            if slice.is_empty() {
                break;
            }

            let mut buf = engine::utils::buffer();
            buf.append(slice);

            let res: Result<(), Error> = yield TcpStream::write_all(&mut stream, buf);

            if res.is_err() {
                println!("write failed, reason: {}", res.err().unwrap());
                break;
            }
        }
    }

    #[coro]
    fn start_server() {
        let mut listener = yield TcpListener::new("engine:8081".to_socket_addrs().unwrap().next().unwrap());
        loop {
            let stream_ = yield listener.accept();

            if stream_.is_err() {
                println!("accept failed, reason: {}", stream_.err().unwrap());
                continue;
            }

            let stream: TcpStream = stream_.unwrap();
            spawn_local!(handle_tcp_client(stream));
        }
    }

    run_on_all_cores(start_server);
}

fn main() {
    tcp_benchmark();
}

