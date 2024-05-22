// //
// // use std::net::ToSocketAddrs;
// // use engine::net::tcp::{TcpListener, TcpStream};
// // use engine::{coro, io_yield, run_on_all_cores, spawn_local};
// // use engine::sync::{Mutex};
// //
// // static L: Mutex<i32> = Mutex::new(0);
// //
// // pub fn local_test() {
// //     tcp_benchmark();
// //
// //     // run_on_all_cores!({
// //     //     for i in 0..10 {
// //     //         let res = L.lock();
// //     //         println!("was locked: {}", res.is_ok());
// //     //         if res.is_ok() {
// //     //             let mut r = res.unwrap();
// //     //             *r += i;
// //     //         }
// //     //     }
// //     // });
// // }
// //

#![feature(coroutines)]
#![feature(coroutine_trait)]
#![feature(stmt_expr_attributes)]
#![feature(gen_blocks)]

use std::net::ToSocketAddrs;
use engine::{coro, run_on_all_cores, spawn_local};
use engine::net::{TcpListener, TcpStream};

fn tcp_benchmark() {
    #[coro]
    fn handle_tcp_client(mut stream: TcpStream) {
        loop {
            let slice = (yield stream.read()).unwrap();

            if slice.is_empty() {
                break;
            }

            let mut buf = engine::utils::buffer();
            buf.append(slice);

            let res = yield TcpStream::write_all(&mut stream, buf);

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
//
// #![feature(coroutines)]
// #![feature(coroutine_trait)]
// #![feature(stmt_expr_attributes)]
// #![feature(gen_blocks)]
//
// use std::net::ToSocketAddrs;
// use engine::{coro, run_on_all_cores, spawn_local, wait};
// use engine::net::{TcpListener, TcpStream};
// use engine::utils::Buffer;
//
// #[coro]
// fn difficult_write(mut stream: TcpStream, buf: Buffer) {
//     let res = yield stream.write_all(buf);
//     if res.is_err() {
//         println!("write failed, reason: {}", res.err().unwrap());
//     }
// }
//
// #[coro]
// fn handle_tcp_client(stream: TcpStream) {
//     let mut buf = engine::utils::buffer();
//     buf.append("hello".as_bytes());
//
//     // Here we need to wait to avoid dropping the stream before the coroutine is finished.
//     wait!(difficult_write(stream, buf));
// }
//
// #[coro]
// fn start_server() {
//     let mut listener = yield TcpListener::new("engine:8081".to_socket_addrs().unwrap().next().unwrap());
//     loop {
//         // We needn't wait here, because we want to handle all connections in parallel
//         let stream = yield TcpListener::accept(&mut listener);
//         spawn_local!(handle_tcp_client(stream.unwrap()));
//     }
// }
//
// fn main() {
//     run_on_all_cores(start_server);
// }

// #![feature(coroutines)]
// #![feature(coroutine_trait)]
// #![feature(stmt_expr_attributes)]
// #![feature(gen_blocks)]
//
// use std::time::Duration;
// use engine::{coro, run_on_all_cores};
// use engine::sleep::sleep;
//
// #[coro]
// fn start_coro() {
//     let mut c = 0;
//     loop {
//         println!("hello, c {c}");
//         c += 1;
//         yield sleep(Duration::from_millis(1 << c));
//         if c == 16 {
//             return;
//         }
//     }
// }
//
// fn main() {
//     run_on_all_cores(start_coro);
// }