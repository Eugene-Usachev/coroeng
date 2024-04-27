/// A macro that yields control to a coroutine after executing the specified coroutine function.
///
/// # Examples
///
/// ```
/// let mut listener = io_yield!(TcpListener::new, "localhost:8081".to_socket_addrs().unwrap().next().unwrap());
///
/// let res = io_yield!(TcpListener::accept, &mut listener);
///
/// if res.is_err() {
///     println!("accept failed, reason: {}", res.err().unwrap());
///     continue;
/// }
///
/// let mut stream: TcpStream = res.unwrap();
/// ```
#[macro_export]
macro_rules! io_yield {
    ($coroutine:expr) => {
        {
            let mut res = unsafe { std::mem::zeroed() };
            yield $coroutine(&mut res);
            res
        }
    };

    ($coroutine:expr, $($arg:tt)*) => {
        {
            let mut res = unsafe { std::mem::zeroed() };
            yield $coroutine($($arg)*, &mut res);
            res
        }
    }
}