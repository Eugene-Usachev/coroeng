/// Creates a new coroutine with the passed block of code.
///
/// # Difference with new_coroutine_move!
///
/// These macros will wrap the block of code in a closure without movement.
#[macro_export]
macro_rules! new_coroutine {
    ($code:block) => {
        || {
            if true {
                $code
            } else {
                yield $crate::engine::coroutine::never();
            }
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
        move || {
            if true {
                $code
            } else {
                yield $crate::engine::coroutine::never();
            }
        }
    }
}

// TODO
#[macro_export]
macro_rules! spawn_coroutine {
    ($coroutine:expr) => {
        $crate::engine::work_stealing::scheduler::SCHEDULER.sched(Box::pin($coroutine));
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
/// spawn_local_coroutine!(|| {
///     println!("Hello");
///     yield crate::engine::sleep::sleep::sleep(Duration::from_secs(1));
///     println!("World");
/// });
/// ```
///
/// # Note
/// Coroutine will be completed regardless of the parent coroutine and will not call context switch.
#[macro_export]
macro_rules! spawn_local_coroutine {
    ($coroutine:expr) => {
        $crate::engine::local::local_scheduler().sched(Box::pin($coroutine));
    }
}

/// Creates a new coroutine and spawns it on the current thread.
///
/// # Panics
///
/// panics if [`LOCAL_SCHEDULER`] is not initialized. To initialize it, use [`run_on_all_cores!`].
///
/// # Example
///
/// ```
/// spawn_local!({
///     println!("Hello, from new coroutine");
/// });
/// ```
///
/// # Note
/// Coroutine will be completed regardless of the parent coroutine and will not call context switch.
#[macro_export]
macro_rules! spawn_local {
    ($code:block) => {
        $crate::spawn_local_coroutine!($crate::new_coroutine!($code))
    };
}

/// Creates a new coroutine and spawns it on the current thread. This macro uses `move` keyword for the coroutine.
///
/// # Panics
///
/// panics if [`LOCAL_SCHEDULER`] is not initialized. To initialize it, use [`run_on_all_cores!`].
///
/// # Example
///
/// ```
/// let name = "Greetings coroutine".to_string();
/// spawn_local!({
///     println!("Hello, from {:?}", name);
/// });
/// ```
///
/// # Note
///
/// Coroutine will be completed regardless of the parent coroutine and will not call context switch.
#[macro_export]
macro_rules! spawn_local_move {
    ($code:block) => {
        $crate::spawn_local_coroutine!($crate::new_coroutine_move!($code))
    };
}

// #[macro_export]
// macro_rules! run_global {
//     ($coroutine:expr) => {
//         if $crate::engine::work_stealing::scheduler::IS_INIT.swap(true, std::sync::atomic::Ordering::SeqCst) {
//             panic!("Global scheduler is already initialize");
//         }
//         $crate::engine::work_stealing::scheduler::SCHEDULER.init();
//         $crate::engine::work_stealing::scheduler::SCHEDULER.run(Box::pin($coroutine));
//     }
// }

/// Run a block of code on all available cores.
///
/// This macro takes a block of code as an argument and runs it on all available cores.
/// It is useful for running tasks in parallel.
///
/// # Example
///
/// ```
/// use crate::run_on_all_cores;
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
    ($func: block) => {
        let cores = core_affinity::get_core_ids().unwrap();
        for i in 1..cores.len() {
            let cores = cores.clone();
            std::thread::spawn(move || {
                core_affinity::set_for_current(cores[i]);
                // TODO accept from cfg
                $crate::utils::BufPool::init(4096);
                $crate::engine::local::Scheduler::init(cores[i]);
                let scheduler = $crate::engine::local::local_scheduler();
                scheduler.run(Box::pin(
                    $crate::new_coroutine!({
                        $func
                    })
                ));
            });
        }

        core_affinity::set_for_current(cores[0]);
        // TODO accept from cfg
        $crate::utils::BufPool::init(4096);
        $crate::engine::local::Scheduler::init(cores[0]);
        let scheduler = $crate::engine::local::local_scheduler();
        scheduler.run(Box::pin(
            $crate::new_coroutine!({
                $func
            })
        ));
    }
}