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
#![allow(internal_features)]
#![feature(coroutines)]
#![feature(coroutine_trait)]
#![feature(stmt_expr_attributes)]
#![feature(gen_blocks)]
#![feature(core_intrinsics)]

use std::io::{Error};
use std::net::{ToSocketAddrs};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::{thread};
use std::ffi::CString;
use std::time::{Duration};
use io_uring::{opcode, types};
use engine::{coro, run_on_all_cores, run_on_core, spawn_local};
use engine::net::{TcpListener, TcpStream};
use engine::sleep::sleep;
use engine::buf::{Buffer, buffer};
use engine::fs::{File, OpenOptions};
use engine::io::{AsyncPRead, AsyncPWrite, AsyncRead, AsyncWrite};
use engine::scheduler::end;
use engine::utils::{get_core_ids};

#[allow(dead_code)]
fn docs() {
    #[coro]
    fn test_file() {
        let options = OpenOptions::new().read(true).write(true).create(true);
        let file: Result<File, Error> = yield File::open(Box::new("./test.txt".to_string()), options);
        if file.is_err() {
            println!("open failed, reason: {}", file.err().unwrap());
            end();
            return;
        }
        println!("file: {:?}", file.is_ok());
        let mut file = file.unwrap();
        let mut buf = buffer();
        buf.append(b"Hello, world!");
        yield file.write_all(buf);
        
        let res: Result<Buffer, Error> = yield file.read();
        if res.is_err() {
            println!("read failed, reason: {}", res.err().unwrap());
            end();
            return;
        }
        println!("read: {:?}", String::from_utf8(res.unwrap().as_ref().to_vec()).unwrap());

        let mut buf = buffer();
        buf.append(b"Hello, world!");
        yield file.pwrite_all(buf, 7);
        
        let res: Result<Buffer, Error> = yield file.pread(7);
        if res.is_err() {
            println!("read failed, reason: {}", res.err().unwrap());
            end();
            return;
        }
        println!("pread: {:?}", String::from_utf8(res.unwrap().as_ref().to_vec()).unwrap());
        
        end();
    }

    run_on_core(test_file, get_core_ids().unwrap()[0]);
}

#[coro]
fn ping_pong() {
    static C: AtomicUsize = AtomicUsize::new(0);
    #[coro]
    fn client() {
        let stream_ = yield TcpStream::connect("engine:8082".to_socket_addrs().unwrap().next().unwrap());
        if stream_.is_err() {
            println!("connect failed, reason: {}", stream_.err().unwrap());
            return;
        }

        let mut stream: TcpStream = stream_.unwrap();
        unsafe {
            let c = C.fetch_add(1, SeqCst) + 1;
            if c % 100 == 0 {
                println!("connected: {}, fd: {}", c, stream.state_ptr().as_ref().fd());
            }
        }
        let mut buf: Buffer;
        let mut res: Result<Buffer, Error>;

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

            if res.as_ref() == b"pong" {
                //yield sleep(Duration::from_secs(2));
            } else {
                println!("Pong has not been received!, received: {:?}", String::from_utf8(res.as_ref().to_vec()).unwrap());
                break;
            }
        }
    }

    #[coro]
    fn server() {
        #[coro]
        fn handle_tcp_client(mut stream: TcpStream) {
            loop {
                let mut buffer: Buffer = (yield stream.read()).unwrap();

                if buffer.as_ref() != b"ping" {
                    println!("received: {:?}", buffer);
                    break;
                }

                buffer.clear();
                buffer.append(b"pong");

                let res: Result<Buffer, Error> = yield stream.write_all(buffer);

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

    for _i in 0..20000 {
        spawn_local!(client());
    }
}

fn tcp_benchmark() {
    #[coro]
    fn handle_tcp_client(mut stream: TcpStream) {
        loop {
            let slice: Buffer = (yield stream.read()).unwrap();
            
            if slice.is_empty() {
                break;
            }
            let res: Result<Buffer, Error> = yield stream.write_all(slice);

            if res.is_err() {
                println!("write failed, reason: {}", res.err().unwrap());
                break;
            }
        }
    }

    #[coro]
    fn start_server() {
        let mut listener: TcpListener = yield TcpListener::new("engine:8081".to_socket_addrs().unwrap().next().unwrap());
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
    //run_on_core(start_server, get_core_ids().unwrap()[0]);
}

fn tcp_client_benchmark() {
    #[coro]
    fn handle_connection() {
        let stream_ = yield TcpStream::connect("client:8079".to_socket_addrs().unwrap().next().unwrap());
        if stream_.is_err() {
            println!("connect failed, reason: {}", stream_.err().unwrap());
            return;
        }
        let mut stream: TcpStream = stream_.unwrap();
        yield stream.read();
        panic!("read!")
    }
    
    #[coro]
    fn client() {
        yield sleep(Duration::from_secs(3));
        println!("start");
        for _ in 0..20_000 {
            spawn_local!(handle_connection());
        }
        
        yield sleep(Duration::from_secs(1000));
    }
    
    run_on_core(client, get_core_ids().unwrap()[0]);
}

fn tcp_profile() {
    #[coro]
    fn handle_tcp_client(mut stream: TcpStream) {
        loop {
            let slice: Buffer = (yield stream.read()).unwrap();

            if slice.is_empty() {
                break;
            }

            let res: Result<Buffer, Error> = yield TcpStream::write_all(&mut stream, slice);

            if res.is_err() {
                println!("write failed, reason: {}", res.err().unwrap());
            }
        }
    }
    
    #[coro]
    fn start_server() {
        #[coro]
        fn deadline() {
            yield sleep(Duration::from_secs(60));
            end();
        }
        
        spawn_local!(deadline());
        let mut listener = yield TcpListener::new("localhost:8081".to_socket_addrs().unwrap().next().unwrap());
        loop {
            let stream_ = yield listener.accept();

            if stream_.is_err() {
                //println!("accept failed, reason: {}", stream_.err().unwrap());
                continue;
            }

            let stream: TcpStream = stream_.unwrap();
            spawn_local!(handle_tcp_client(stream));
        }
    }

    thread::spawn(|| {
        use std::io::Read;
        use std::io::Write;
        use std::net;
        use std::time::{Duration, Instant};

        const PAR: usize = 128;
        const N: usize = 2000000;
        const COUNT: usize = N / PAR;
        const TRIES: usize = 100;
        const ADDR_ENGINE: &str = "localhost:8081";

        let mut res = 0;
        thread::sleep(Duration::from_millis(2000));
        for i in 0..TRIES {
            let start = Instant::now();
            let mut joins = Vec::with_capacity(PAR);
            for _i in 0..PAR {
                joins.push(thread::spawn(move || {
                    if let Ok(mut conn) = net::TcpStream::connect(ADDR_ENGINE){
                        let mut buf = [0u8; 1024];

                        for _ in 0..COUNT {
                            let _ = conn.write_all(b"ping");
                            let _ = conn.read(&mut buf).expect("read failed");
                        }
                    } else {
                        panic!("connect failed, reason: {}", net::TcpStream::connect(ADDR_ENGINE).err().unwrap());
                    }
                }));
            }

            for join in joins {
                let _ = join.join();
            }

            let rps = (N * 1000) / start.elapsed().as_millis() as usize;
            println!("Benchmark took: {}ms, RPS: {rps}", start.elapsed().as_millis());

            res += rps;

            thread::sleep(Duration::from_millis(2000));
        }

        println!("Average RPS: {}", res / TRIES);
    });

    run_on_all_cores(start_server);
}

fn benchmark_sleep() {
    #[coro]
    fn spawn_sleep() {
        let s = SPAWNED.fetch_add(1, SeqCst) + 1;
        if s % 10000 == 0 {
            println!("spawned {}", s);
        }
        yield sleep(Duration::from_secs(1000000));
    }

    const N: usize = 10_000_000;
    const PAR: usize = 1;

    static SPAWNED: AtomicUsize = AtomicUsize::new(0);

    #[coro]
    fn benchmark() {
        for _ in 0..N / PAR {
            spawn_local!(spawn_sleep());
        }
    }

    run_on_core(benchmark, get_core_ids().unwrap()[0]);
}

#[coro]
fn leak_test() {
    for i in 0..100000000 {
        for _ in 0..200 {
            let buf = Buffer::new(1000);
            if buf.len() != 0 {
                println!("jj {i}");
            }
        }
        println!("{}", i);
        yield sleep(Duration::from_secs(1));
    }
}

fn main() {
    //benchmark_sleep();
    //run_on_core(leak_test, get_core_ids().unwrap()[0]);
    docs();
    //tcp_profile();
    //tcp_client_benchmark();
    //tcp_benchmark();
    //run_on_core(ping_pong, get_core_ids().unwrap()[0]);
    //std1();
    //io_uring();
}

fn io_uring() {
    let mut ring = io_uring::IoUring::new(16).unwrap();
    let cs_ = CString::new("./test.txt").unwrap();
    let open_how = types::OpenHow::new().flags((libc::O_CREAT | libc::O_RDWR) as u64).mode(0o777);
    let entry = opcode::OpenAt2::new(types::Fd(libc::AT_FDCWD), unsafe { cs_.as_ptr() }, &open_how)
        .build();
    unsafe { ring.submission().push(&entry).unwrap(); }
    ring.submit_and_wait(1).unwrap();
    let ret = ring.completion().next().unwrap().result();
    println!("ret {ret}");
}