use std::io::{Write};
use crate::{spawn_move, run_global, spawn_coroutine};
use crate::engine::coroutine::{never, yield_now};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::{mem, thread};
use std::net::ToSocketAddrs;
use std::time::Duration;
use crate::engine::net::tcp::TcpListener;
use crate::engine::work_stealing::sleep::sleep::sleep;

pub fn work_stealing_test() {
    let c = Arc::new(AtomicUsize::new(0));
    for i in 0..10 {
        let c = c.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(1));
            for _ in 0..10 {
                let mut socket = std::net::TcpStream::connect("engine:8081").unwrap();
                println!("client {}", c.fetch_add(1, SeqCst) + 1);
                socket.write(b"hello").unwrap();
                println!("wrote");
            }
        });
    }

    run_global!(|| {
        let mut listener = unsafe { mem::zeroed()};
        yield TcpListener::new("engine:8081".to_socket_addrs().unwrap().next().unwrap(), &mut listener);
        let mut r = Arc::new(AtomicUsize::new(0));
        println!("bind ok");
        loop {
            let mut res = unsafe { mem::zeroed() };
            yield listener.accept(&mut res);

            if res.is_err() {
                println!("accept failed, reason: {}", res.err().unwrap());
                continue;
            }

            let mut stream = res.unwrap();
            let r = r.clone();
            spawn_coroutine!(move || {
                let mut slice = unsafe { mem::zeroed() };
                println!("read");
                yield stream.read(&mut slice);
                let slice = slice.unwrap();
                if slice.is_empty() {
                    yield stream.close();
                    println!("closed");
                }
                let r = r.fetch_add(1, SeqCst);
                println!("read {r} ok, len: {}\n {:?}", slice.len(), String::from_utf8(slice.to_vec()).unwrap());
                //let _ = write_tcp!(stream, slice.to_vec());
            });
        }
    });
}

fn test_forever_sleep() {
    println!("no wait");
    run_global!(|| {
        loop {
            println!("1");

            println!("wait 100 micros");
            yield sleep(Duration::from_micros(100));
            println!("2");

            println!("wait 1 seconds");
            yield sleep(Duration::from_secs(1));
            println!("3");

            println!("wait 10 seconds");
            yield sleep(Duration::from_secs(10));
            println!("4");

            yield yield_now();
        }
    });
}

fn bench_count_of_threads() {
    run_global!(|| {
        loop {
            let a = Arc::new(AtomicUsize::new(0));

            for _ in 0..200_000 {
                let a = a.clone();
                spawn_move!({
                    println!("{}", a.fetch_add(1, SeqCst));
                    yield sleep(Duration::from_secs(10000));
                });
            }

            if false {
                yield never();
            }

            thread::sleep(Duration::from_secs(100000));
        }
    });
}