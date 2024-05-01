#[macro_export]
macro_rules! write_ok {
    ($ptr:expr, $res:expr) => {
        unsafe { *$ptr = Ok($res)}
    }
}

#[macro_export]
macro_rules! write_err {
    ($ptr:expr, $err:expr) => {
        unsafe { *$ptr = Err($err)}
    }
}