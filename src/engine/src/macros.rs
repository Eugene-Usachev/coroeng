/// Creates a new coroutine with the passed block of code.
///
/// Note
///
/// This macro creates a coroutine from a given block of code by wrapping it in a `move` closure.
#[macro_export]
macro_rules! new_coroutine {
    ($code:block) => {
        Box::pin(#[coroutine] move || {
            $code
        })
    }
}

// TODO
#[macro_export]
macro_rules! spawn_coroutine {
    ($coroutine:expr) => {
        $crate::work_stealing::scheduler::SCHEDULER.sched(Box::pin($coroutine));
    }
}

// TODO
#[macro_export]
macro_rules! spawn {
    ($code:block) => {
        $crate::spawn_coroutine!($crate::new_coroutine!($code))
    };
}

// TODO
#[macro_export]
macro_rules! spawn_move {
    ($code:block) => {
        $crate::spawn_coroutine!($crate::new_coroutine_move!($code))
    };
}