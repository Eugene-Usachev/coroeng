/// Creates a new coroutine with the passed block of code.
///
/// # Difference with new_coroutine_move!
///
/// These macros will wrap the block of code in a closure without movement.
#[macro_export]
macro_rules! new_coroutine {
    ($code:block) => {
        #[coroutine] || {
            $code
        }
    }
}

/// Creates a new coroutine with the passed block of code.
///
/// # Difference with new_coroutine!
///
/// These macros will wrap the block of code in a move closure.
#[macro_export]
macro_rules! new_coroutine_move {
    ($code:block) => {
        #[coroutine] move || {
            $code
        }
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

/// These macros spawns provided coroutine on the current thread.
///
/// # Panics
///
/// panics if [`LOCAL_SCHEDULER`] is not initialized. To initialize it, use [`run_on_all_cores!`].
///
/// # Example
///
/// ```
/// use engine::{new_coroutine, spawn_local};
/// use engine::sleep::sleep;
/// use std::time::Duration;
///
/// spawn_local!(new_coroutine!({
///     println!("Hello");
///     yield sleep(Duration::from_secs(1));
///     println!("World");
/// }));
/// ```
///
/// # Note
/// Coroutine will be completed regardless of the parent coroutine and will not call context switch.
#[macro_export]
macro_rules! spawn_local {
    ($coroutine:expr) => {
        $crate::local_scheduler().sched(Box::pin($coroutine))
    };

    ($code:block) => {
        $crate::spawn_local!($crate::new_coroutine!($code))
    };
}

#[macro_export]
macro_rules! spawn_local_move {
    ($code:block) => {
        $crate::spawn_local!($crate::new_coroutine_move!($code))
    };

    ($coroutine:expr) => {
        $crate::local_scheduler().sched(Box::pin($coroutine))
    };
}

// #[macro_export]
// macro_rules! run_global {
//     ($coroutine:expr) => {
//         if $crate::work_stealing::scheduler::IS_INIT.swap(true, std::sync::atomic::Ordering::SeqCst) {
//             panic!("Global scheduler is already initialize");
//         }
//         $crate::work_stealing::scheduler::SCHEDULER.init();
//         $crate::work_stealing::scheduler::SCHEDULER.run(Box::pin($coroutine));
//     }
// }

// TODO docs
#[macro_export]
macro_rules! run_on_core {
    ($code: block, $core: expr) => {
        $crate::utils::core::set_for_current($core);
        // TODO accept from cfg
        $crate::utils::BufPool::init($crate::cfg::config_buf_len());
        $crate::local::Scheduler::init();
        let scheduler = $crate::local::local_scheduler();
        scheduler.run(Box::pin(
            $crate::new_coroutine!({
                $code
            })
        ));
    }
}

/// Run a block of code on all available cores.
///
/// This macro takes a block of code as an argument and runs it on all available cores.
/// It is useful for running tasks in parallel.
///
/// # Example
///
/// ```
/// use engine::run_on_all_cores;
///
/// run_on_all_cores!({
///     // do some work here
/// });
/// ```
///
/// # Note
///
/// For better performance, it is better to make the code inside the block independent of the shared state.
#[macro_export]
macro_rules! run_on_all_cores {
    ($code: block) => {
        let cores = $crate::utils::core::get_core_ids().unwrap();
        for i in 1..cores.len() {
            let cores = cores.clone();
            std::thread::Builder::new()
                .name(format!("worker on core: {}", i))
                .spawn(move || {
                    $crate::run_on_core!($code, cores[i]);
                });
        }

        $crate::run_on_core!($code, cores[0]);
    }
}

#[macro_export]
macro_rules! wait {
    ($coroutine:expr) => {
        loop {
            match $coroutine.as_mut().resume(()) {
                std::ops::CoroutineState::Yielded(state) => {
                    println!("state is: {state:?}");
                    yield state;
                },
                std::ops::CoroutineState::Complete(_) => break,
            }
        }
    };
}