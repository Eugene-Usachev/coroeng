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
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::time::Duration;
use engine::{coro, run_on_all_cores, spawn_local, wait};
use engine::net::{TcpListener, TcpStream};
use engine::sleep::sleep;
use engine::buf::Buffer;

fn docs() {
    #[coro]
    fn difficult_write(mut stream: TcpStream, mut buf: Buffer) -> usize {
        loop {
            let res: Result<Option<Buffer>, Error> = yield stream.write(buf);
            if res.is_err() {
                println!("write failed, reason: {}", res.err().unwrap());
                break;
            }
            if let Some(new_buf) = res.unwrap() {
                buf = new_buf;
            } else {
                break;
            }
        }
        42
    }
    #[coro]
    fn handle_tcp_stream(mut stream: TcpStream) {
        let res = wait!(difficult_write(stream, engine::buf::buffer()));
        println!("{}", res);
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
            spawn_local!(handle_tcp_stream(stream));
        }
    }

    run_on_all_cores(start_server);
}

fn tcp_benchmark() {
    #[coro]
    fn handle_tcp_client(mut stream: TcpStream) {
        loop {
            let slice: &[u8] = (yield stream.read()).unwrap();

            if slice.is_empty() {
                break;
            }

            let mut buf = engine::buf::buffer();
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

fn benchmark_sleep() {
    #[coro]
    fn spawn_sleep() {
        println!("spawned {}", SPAWNED.fetch_add(1, SeqCst) + 1);
        yield sleep(Duration::from_secs(1000000));
    }

    const N: usize = 10_000_000;
    const PAR: usize = 6;

    static SPAWNED: AtomicUsize = AtomicUsize::new(0);

    #[coro]
    fn benchmark() {
        for _ in 0..N / PAR {
            spawn_local!(spawn_sleep());
        }
    }

    run_on_all_cores(benchmark);
}

fn main() {
    docs();
}

