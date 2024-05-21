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

use std::io::Error;
use std::mem::MaybeUninit;
use std::net::ToSocketAddrs;
use engine::{coro, ret_yield, run_on_all_cores, spawn_local, wait};
use engine::coroutine::CoroutineImpl;
use engine::net::{TcpListener, TcpStream};
use engine::utils::{Buffer, buffer};

#[coro]
fn with_ret2(mut stream: TcpStream) -> Result<&'static [u8], Error> {
    yield stream.read()
}

#[coro]
fn with_ret(mut stream: TcpStream) -> Result<&'static [u8], Error> {
    let r = (yield stream.write(buffer()))?;

    // while 0 < (yield stream.write(buffer())).unwrap() {
    //     let r = yield stream.read();
    //     if r.is_err() {
    //         return r;
    //     }
    // }
    //
    // loop {
    //     let r = yield stream.read();
    //     if r.is_err() {
    //         return r;
    //     }
    // }

    // let s = [0, 2, 3];
    // let r = s[yield stream.write(buffer())];

    // for i in 0..(yield stream.write(buffer())).unwrap() {
    //     let r = yield stream.read();
    // }

    // let r = unsafe {
    //     yield stream.read()
    // };

    //let r = *(yield stream.read());

    //let r = [(yield stream.write(buffer())).unwrap(); 10];

    // struct A {
    //     res: Result<&'static [u8], Error>
    // }
    // let a = A {
    //     res: yield stream.read()
    // };
    // let r = a.res;

    //let r = (yield stream.read()) as Result<&'static [u8], Error>;

    //let r = [yield stream.read(), yield stream.read()];

    //let res = yield stream.write(yield stream.read());

    // struct a {
    //     cor: CoroutineImpl
    // }
    // let mut res = std::mem::MaybeUninit::uninit();
    // let aa = a {
    //     cor: with_ret2(stream, res.as_mut_ptr())
    // };
    //
    // let r = yield aa.cor();

    //let r = yield aa.cor;

    //let r = &(yield stream.read()).unwrap();

    //let r = ((yield stream.write(buffer())).unwrap(), (yield stream.write(buffer())).unwrap());

    //let r = ((yield stream.write(buffer())).unwrap() + (yield stream.write(buffer())).unwrap());

    //let r = (yield stream.write(buffer())).unwrap();

    // match yield stream.read() {
    //     Ok(slice) => {
    //         let mut buf = buffer();
    //         buf.append(slice);
    //         yield stream.write_all(buf);
    //     }
    //     Err(err) => {
    //         return yield stream.read();
    //     }
    // };

    // let res = yield stream.read();
    // let res;
    // res = yield stream.read();

    // if yield stream.read() {
    //     let res = yield stream.read();
    // } else {
    //     let buf = buffer();
    //     let res = yield stream.write_all(buf);
    // }

    // // without ret
    // yield stream.read();
    //
    // // with ret
    // yield stream.read()

    // return yield stream.read();
}

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
//     // Here we need to wait to avoid dropping the stream before the coroutine is finished.
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
    a();
}