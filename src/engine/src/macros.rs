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

// TODO r this need only for docs.
// / These macros spawns provided coroutine in current [`Scheduler`](crate::scheduler::Scheduler).
// ///
// /// # Panics
// ///
// /// panics if [`LOCAL_SCHEDULER`](crate::scheduler::LOCAL_SCHEDULER) is not initialized.
// /// To initialize it, use [`run_on_all_cores`](crate::run::run_on_all_cores) or [`run_on_core`](crate::run::run_on_core).
// ///
// /// # Example
// ///
// /// ```
// /// use engine::{new_coroutine, spawn_local};
// /// use engine::sleep::sleep;
// /// use std::time::Duration;
// ///
// /// spawn_local!(new_coroutine!({
// ///     println!("Hello");
// ///     yield sleep(Duration::from_secs(1));
// ///     println!("World");
// /// }));
// /// ```
// ///
// /// # Note
// ///
// /// Coroutine will be completed regardless of the parent coroutine and will not call context switch.
// #[macro_export]
// macro_rules! spawn_local {
//     ($code:block) => {
//         $crate::spawn_local!($crate::new_coroutine!($code))
//     };
//
//     ($coroutine:expr) => {
//         $crate::local_scheduler().sched($coroutine)
//     };
// }