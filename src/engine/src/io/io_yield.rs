/// TODO update docs
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
///     panic!("accept failed, reason: {}", res.err().unwrap());
/// }
///
/// let mut stream: TcpStream = res.unwrap();
/// ```
#[macro_export]
macro_rules! ret_yield {
    ($coroutine:expr) => {
        unsafe {
            let mut res = std::mem::MaybeUninit::uninit();
            yield $coroutine(res.as_mut_ptr());
            res.assume_init()
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