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
use engine::{coro, run_on_all_cores, run_on_core, spawn_local, wait};
use engine::net::{TcpListener, TcpStream};
use engine::sleep::sleep;
use engine::buf::{Buffer, buffer};
use engine::utils::get_core_ids;

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
    fn handle_tcp_stream(stream: TcpStream) {
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

#[coro]
fn ping_pong() {
    static C: AtomicUsize = AtomicUsize::new(0);
    #[coro]
    fn client() {
        let mut stream_ = yield TcpStream::connect("engine:8082".to_socket_addrs().unwrap().next().unwrap());
        if stream_.is_err() {
            println!("connect failed, reason: {}", stream_.err().unwrap());
            return;
        }

        let mut stream: TcpStream = stream_.unwrap();
        unsafe {
            println!("connected: {}, fd: {}", C.fetch_add(1, SeqCst) + 1, stream.state_ptr().as_ref().fd());
        }
        let mut buf: Buffer;
        let mut res: Result<&[u8], Error>;

        loop {
            buf = buffer();
            buf.append(b"ping");
            yield stream.write(buf);

            res = yield stream.read();
            if res.is_err() {
                println!("read failed, reason: {:?}", res.unwrap_err());
                break;
            }

            let res = res.unwrap();

            if res == b"pong" {
                println!("Pong has been received");
                yield sleep(Duration::from_secs(2));
            } else {
                println!("Pong has not been received!, received: {:?}", String::from_utf8(res.to_vec()).unwrap());
                break;
            }
        }
    }

    #[coro]
    fn server() {
        #[coro]
        fn handle_tcp_client(mut stream: TcpStream) {
            loop {
                let slice: &[u8] = (yield stream.read()).unwrap();

                if slice != b"ping" {
                    println!("received: {:?}", slice);
                    break;
                }

                let mut buf = engine::buf::buffer();
                buf.append(b"pong");

                let res: Result<(), Error> = yield stream.write_all(buf);

                if res.is_err() {
                    println!("write failed, reason: {}", res.err().unwrap());
                    break;
                }
            }
        }

        let mut listener: TcpListener = yield TcpListener::new("engine:8082".to_socket_addrs().unwrap().next().unwrap());
        unsafe {
            println!("listener is created, fd: {}", listener.state_ptr().as_ref().fd());
        }

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

    spawn_local!(server());
    yield sleep(Duration::from_secs(1));

    for i in 0..20000 {
        spawn_local!(client());
    }
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
    run_on_core(ping_pong, get_core_ids().unwrap()[0]);
}

