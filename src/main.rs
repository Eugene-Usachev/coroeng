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

use std::{ptr, thread};
use std::collections::VecDeque;
use std::intrinsics::unlikely;
use std::io::{Error, Write};
use std::net::ToSocketAddrs;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::time::Duration;
use io_uring::{cqueue, IoUring, squeue};
use io_uring::types::{SubmitArgs, Timespec};
use engine::{coro, run_on_all_cores, spawn_local, wait};
use engine::net::{TcpListener, TcpStream};
use engine::sleep::sleep;
use engine::buf::{Buffer, buffer, BufPool};
use engine::io::{AsyncRead, AsyncWrite};
use engine::utils::{CoreId, get_core_ids, Ptr, set_for_current};

fn docs() {
    use engine::{run_on_core, coro, wait};
    use engine::sleep::sleep;
    use engine::utils::get_core_ids;
    use std::time::Duration;

    #[coro]
    fn print_hello(name: String) {
        let messages = ["Hello".to_string(), "world".to_string(), "from".to_string(), name, "coroutine!".to_string()];
        for msg in messages.into_iter() {
            println!("{}", msg);
            yield sleep(Duration::from_millis(500));
        }
    }

    #[coro]
    fn start_app() {
        wait!(print_hello("start_app".to_string()));
    }

    fn main() {
        let core = get_core_ids().unwrap()[0];
        run_on_core(start_app, core);
    }
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

    for _i in 0..20000 {
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
    read_test();
    //docs();
    //io_uring();
    //tcp_benchmark();
    //run_on_core(ping_pong, get_core_ids().unwrap()[0]);
    //std1();
}

fn read_test() {
    use std::os::unix::io::{AsRawFd, RawFd};
    use io_uring::{opcode, squeue, cqueue, types, IoUring, SubmissionQueue};
    println!("io_uring");

    #[derive(Debug)]
    enum Token {
        Accept,
        Read {
            fd: RawFd,
            buf: Buffer
        },
        Write {
            fd: RawFd,
            buf: Buffer,
            offset: usize,
            len: usize,
        }
    }

    struct AcceptCount {
        entry: squeue::Entry,
        count: usize,
    }

    impl AcceptCount {
        fn new(fd: RawFd, token: u64, count: usize) -> AcceptCount {
            AcceptCount {
                entry: opcode::Accept::new(types::Fd(fd), ptr::null_mut(), ptr::null_mut())
                    .build()
                    .user_data(token),
                count
            }
        }

        pub fn push_to(&mut self, sq: &mut SubmissionQueue<'_>) {
            while self.count > 0 {
                unsafe {
                    match sq.push(&self.entry) {
                        Ok(_) => self.count -= 1,
                        Err(_) => break,
                    }
                }
            }

            sq.sync();
        }
    }

    fn run(cpu: CoreId) {
        BufPool::init_in_local_thread(4096);
        set_for_current(cpu);
        const CAP: usize = 512;
        let mut ring: IoUring<squeue::Entry, cqueue::Entry> = IoUring::builder()
            .build(CAP as u32).unwrap();

        let listener_fd = TcpListener::get_fd("localhost:8081".to_socket_addrs().unwrap().next().unwrap());

        let mut backlog = VecDeque::with_capacity(CAP);

        let (submitter, mut sq, mut cq) = ring.split();
        let token_id = Ptr::new(Token::Accept).as_u64();
        let mut accept = AcceptCount::new(listener_fd.as_raw_fd(), token_id, 1);
        accept.push_to(&mut sq);

        loop {
            accept.push_to(&mut sq);

            // clean backlog
            loop {
                if sq.is_full() {
                    match submitter.submit() {
                        Ok(_) => (),
                        Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => break,
                        Err(err) => panic!("a"),
                    }
                }
                sq.sync();

                match backlog.pop_front() {
                    Some(sqe) => unsafe {
                        let _ = sq.push(&sqe);
                    },
                    None => break,
                }
            }

            match submitter.submit_with_args(1, &SubmitArgs::new().timespec(&Timespec::new().nsec(500_000))) {
                Ok(_) => (),
                Err(ref err) if err.raw_os_error() == Some(libc::ETIME) => (),
                Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => (),
                Err(err) => panic!("b"),
            }
            cq.sync();

            for cqe in &mut cq {
                let ret = cqe.result();
                let token_index = cqe.user_data();
                let token = Ptr::from(token_index);

                if ret < 0 {
                    //unsafe { token.drop_in_place() };
                    eprintln!(
                        "token {:?} error: {:?}",
                        token,
                        Error::from_raw_os_error(-ret)
                    );
                    continue;
                }

                match unsafe { token.read() } {
                    Token::Accept => {
                        accept.count += 1;

                        let fd = ret;
                        let mut buf = buffer();
                        let poll_e = opcode::Recv::new(types::Fd(fd), buf.as_mut_ptr(), buf.cap() as u32)
                            .build()
                            .user_data(Ptr::new(Token::Read { buf, fd }).as_u64());

                        unsafe {
                            if sq.push(&poll_e).is_err() {
                                backlog.push_back(poll_e);
                            }
                        }
                    }

                    Token::Read { fd, mut buf } => {
                        buf.set_written(ret as usize);
                        println!("fd: {fd}, ret: {ret}, buf: {:?}", buf.as_ref());
                        if unlikely(ret == 0) {
                            unsafe {token.drop_in_place()};
                            unsafe {
                                libc::close(fd);
                            }
                        } else {
                            let len = ret as usize;
                            let ptr = buf.as_mut_ptr();

                            unsafe {
                                token.write(Token::Write {
                                    fd,
                                    buf,
                                    len,
                                    offset: 0,
                                });
                            };

                            let write_e = opcode::Send::new(types::Fd(fd), ptr, len as _)
                                .build()
                                .user_data(token_index);

                            unsafe {
                                if sq.push(&write_e).is_err() {
                                    backlog.push_back(write_e);
                                }
                            }
                        }
                    }

                    Token::Write {
                        fd,
                        mut buf,
                        offset,
                        len,
                    } => {
                        let write_len = ret as usize;

                        let entry = if offset + write_len >= len {
                            let buf_ptr = buf.as_mut_ptr();
                            let buf_cap = buf.cap();
                            unsafe {
                                token.write(Token::Read { buf, fd });
                            }

                            opcode::Read::new(types::Fd(fd), buf_ptr, buf_cap as _)
                                .build()
                                .user_data(token_index)
                        } else {
                            let offset = offset + write_len;
                            let len = len - offset;
                            buf.set_offset(offset);
                            let ptr = buf.as_mut_ptr();

                            unsafe {
                                token.write(Token::Write {
                                    fd,
                                    buf,
                                    offset,
                                    len,
                                });
                            }

                            opcode::Write::new(types::Fd(fd), ptr, len as _)
                                .build()
                                .user_data(token_index)
                        };

                        unsafe {
                            if sq.push(&entry).is_err() {
                                backlog.push_back(entry);
                            }
                        }
                    }
                }
            }
        }
    }

    let cores = get_core_ids().unwrap();

    let core = cores[0];

    thread::spawn(|| {
        thread::sleep(Duration::from_secs(1));
        for _ in 0..500 {
            let mut stream = std::net::TcpStream::connect("localhost:8081").unwrap();
            thread::spawn(move || {
                println!("start long write");
                thread::sleep(Duration::from_secs(15));
                stream.write(&[5, 10]).expect("write failed");
                println!("end long write");
            });
        }

        for i in 0..3u8 {
            let mut stream = std::net::TcpStream::connect("localhost:8081").unwrap();
            thread::spawn(move || {
                println!("start short write");
                thread::sleep(Duration::from_secs(i as u64 + 1));
                stream.write(&[i]).expect("write failed");
                println!("end short write");
            });
        }
    });
    run(core);
}

// TODO r
fn io_uring() -> Result<(), Error> {
    use std::os::unix::io::{AsRawFd, RawFd};
    use io_uring::{opcode, squeue, cqueue, types, IoUring, SubmissionQueue};
    println!("io_uring");

    #[derive(Debug)]
    enum Token {
        Accept,
        Poll {
            fd: RawFd,
        },
        Read {
            fd: RawFd,
            buf: Buffer
        },
        Write {
            fd: RawFd,
            buf: Buffer,
            offset: usize,
            len: usize,
        }
    }

    struct AcceptCount {
        entry: squeue::Entry,
        count: usize,
    }

    impl AcceptCount {
        fn new(fd: RawFd, token: u64, count: usize) -> AcceptCount {
            AcceptCount {
                entry: opcode::Accept::new(types::Fd(fd), ptr::null_mut(), ptr::null_mut())
                    .build()
                    .user_data(token),
                count
            }
        }

        pub fn push_to(&mut self, sq: &mut SubmissionQueue<'_>) {
            while self.count > 0 {
                unsafe {
                    match sq.push(&self.entry) {
                        Ok(_) => self.count -= 1,
                        Err(_) => break,
                    }
                }
            }

            sq.sync();
        }
    }

    fn run(cpu: CoreId) -> Result<(), Error> {
        BufPool::init_in_local_thread(4096);
        set_for_current(cpu);
        const CAP: usize = 512;
        let mut ring: IoUring<squeue::Entry, cqueue::Entry> = IoUring::builder()
            .build(CAP as u32)?;

        let listener_fd = TcpListener::get_fd("engine:8081".to_socket_addrs().unwrap().next().unwrap());

        let mut backlog = VecDeque::with_capacity(CAP);

        let (submitter, mut sq, mut cq) = ring.split();
        let token_id = Ptr::new(Token::Accept).as_u64();
        let mut accept = AcceptCount::new(listener_fd.as_raw_fd(), token_id, 1);
        accept.push_to(&mut sq);

        loop {
            accept.push_to(&mut sq);

            // clean backlog
            loop {
                if sq.is_full() {
                    match submitter.submit() {
                        Ok(_) => (),
                        Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => break,
                        Err(err) => return Err(err.into()),
                    }
                }
                sq.sync();

                match backlog.pop_front() {
                    Some(sqe) => unsafe {
                        let _ = sq.push(&sqe);
                    },
                    None => break,
                }
            }

            match submitter.submit_with_args(1, &SubmitArgs::new().timespec(&Timespec::new().nsec(500_000))) {
                Ok(_) => (),
                Err(ref err) if err.raw_os_error() == Some(libc::ETIME) => (),
                Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => (),
                Err(err) => return Err(err.into()),
            }
            cq.sync();

            for cqe in &mut cq {
                let ret = cqe.result();
                let token_index = cqe.user_data();
                let token = Ptr::from(token_index);

                if ret < 0 {
                    //unsafe { token.drop_in_place() };
                    eprintln!(
                        "token {:?} error: {:?}",
                        token,
                        Error::from_raw_os_error(-ret)
                    );
                    continue;
                }

                match unsafe { token.read() } {
                    Token::Accept => {
                        accept.count += 1;

                        let fd = ret;
                        let poll_e = opcode::PollAdd::new(types::Fd(fd), libc::POLLIN as _)
                            .build()
                            .user_data(Ptr::new(Token::Poll { fd }).as_u64());

                        unsafe {
                            if sq.push(&poll_e).is_err() {
                                backlog.push_back(poll_e);
                            }
                        }
                    }

                    Token::Poll { fd } => {
                        let mut buf = buffer();
                        let ptr = buf.as_mut_ptr();
                        let cap = buf.cap();
                        unsafe { token.write(Token::Read { fd, buf }) };
                        let read_e = opcode::Recv::new(types::Fd(fd), ptr, cap as u32)
                            .build()
                            .user_data(token_index);

                        unsafe {
                            if sq.push(&read_e).is_err() {
                                backlog.push_back(read_e);
                            }
                        }
                    }

                    Token::Read { fd, mut buf } => {
                        if unlikely(ret == 0) {
                            //unsafe {token.drop_in_place()};
                            unsafe {
                                libc::close(fd);
                            }
                        } else {
                            let len = ret as usize;
                            let ptr = buf.as_mut_ptr();

                            unsafe {
                                token.write(Token::Write {
                                    fd,
                                    buf,
                                    len,
                                    offset: 0,
                                });
                            };

                            let write_e = opcode::Send::new(types::Fd(fd), ptr, len as _)
                                .build()
                                .user_data(token_index);

                            unsafe {
                                if sq.push(&write_e).is_err() {
                                    backlog.push_back(write_e);
                                }
                            }
                        }
                    }

                    Token::Write {
                        fd,
                        mut buf,
                        offset,
                        len,
                    } => {
                        let write_len = ret as usize;

                        let entry = if offset + write_len >= len {
                            unsafe {
                                token.write(Token::Poll { fd });
                            }

                            opcode::PollAdd::new(types::Fd(fd), libc::POLLIN as _)
                                .build()
                                .user_data(token_index)
                        } else {
                            let offset = offset + write_len;
                            let len = len - offset;
                            buf.set_offset(offset);
                            let ptr = buf.as_mut_ptr();

                            unsafe {
                                token.write(Token::Write {
                                    fd,
                                    buf,
                                    offset,
                                    len,
                                });
                            }

                            opcode::Write::new(types::Fd(fd), ptr, len as _)
                                .build()
                                .user_data(token_index)
                        };

                        unsafe {
                            if sq.push(&entry).is_err() {
                                backlog.push_back(entry);
                            }
                        }
                    }
                }
            }
        }
    }

    let cores = get_core_ids().unwrap();

    for i in 1..cores.len() {
        let core = cores[i % cores.len()];
        thread::spawn(move || {
            run(core).expect("Failed to run");
        });
    }

    let core = cores[0];
    run(core)?;

    Ok(())
}

// TODO r
fn std1() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("engine:8081").unwrap();

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        thread::spawn(move || {
            let mut buf = [0;4096];
            loop {
               let n = stream.read(&mut buf).unwrap();
               if n == 0 {
                   break;
               }
               stream.write(&buf[0..n]).unwrap();
            }
        });
    }
}