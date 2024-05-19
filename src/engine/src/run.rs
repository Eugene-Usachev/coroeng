use crate::{cfg, local_scheduler};
use crate::coroutine::{CoroutineImpl};
use crate::local::id::set_worker_id_and_core_id;
use crate::local::{Scheduler};
use crate::utils::{BufPool, core};

/// Runs the [`Scheduler`] with the provided coroutine on the current core.
/// This function will block the current thread.
///
/// # Note
/// This function runs only one [`Scheduler`] on the current core and all spawned coroutines will execute on that same core.
/// If you want to use other cores, you can use the [`run_on_all_cores`] function,
/// or you can use this function in different threads with different core ids using [`get_core_ids`](core::get_core_ids).
///
/// # Examples:
///
/// ## Inline coroutine
///
/// ```rust
/// use engine::{run_on_core, new_coroutine};
/// use engine::sleep::sleep;
/// use engine::utils::get_core_ids;
/// use std::time::Duration;
///
/// fn main() {
///     let core = get_core_ids().unwrap()[0];
///     run_on_core(new_coroutine!({
///         let messages = ["Hello", "world", "from", "inline", "coroutine!"];
///         yield sleep(Duration::from_millis(3000));
///         for msg in messages.into_iter() {
///             println!("{}", msg);
///             yield sleep(Duration::from_millis(500));
///         }
///     }), core);
/// }
/// ```
///
/// ## Coroutine creator function
///
/// ```rust
/// use engine::{run_on_core, new_coroutine, coro};
/// use engine::sleep::sleep;
/// use engine::utils::get_core_ids;
/// use std::time::Duration;
///
/// #[coro]
/// fn print_hello(name: String) {
///     let messages = ["Hello".to_string(), "world".to_string(), "from".to_string(), name, "coroutine!".to_string()];
///     for msg in messages.into_iter() {
///         println!("{}", msg);
///         yield sleep(Duration::from_millis(500));
///     }
/// }
///
/// fn main() {
///     let core = get_core_ids().unwrap()[0];
///     run_on_core(print_hello("main".to_string()), core);
/// }
/// ```
pub fn run_on_core(coroutine: CoroutineImpl, core: core::CoreId) {
    core::set_for_current(core);
    set_worker_id_and_core_id(core.id + 1, core.id);
    BufPool::init(cfg::config_buf_len());
    Scheduler::init();
    let scheduler = local_scheduler();
    scheduler.run(coroutine);
}

/// Takes a function that returns a coroutine and call this function on all cores with [`run_on_core`].
/// This function will block the current thread.
///
/// # Note
///
/// For optimal performance, this coroutine should avoid accessing the shared state as much as possible,
/// or at least minimize the frequency of such access.
///
/// # Examples
///
/// ```rust
/// use engine::{coro, run_on_all_cores};
/// use engine::local::get_core_id;
/// use engine::sleep::sleep;
/// use std::time::Duration;
///
/// #[coro]
/// fn greetings_from_different_cores() {
///     loop {
///         println!("Hello from core {}!", get_core_id());
///         yield sleep(Duration::from_millis(3000));
///     }
/// }
///
/// fn main() {
///     run_on_all_cores(greetings_from_different_cores);
/// }
/// ```
pub fn run_on_all_cores<C: 'static + Send + Clone + Fn() -> CoroutineImpl>(creator: C) {
    let cores = core::get_core_ids().unwrap();
    for i in 1..cores.len() {
        let core = cores[i];
        let creator = creator.clone();
        std::thread::Builder::new()
            .name(format!("worker on core: {}", i))
            .spawn(move || {
                run_on_core(creator(), core);
            }).expect("failed to create worker thread");
    }

    run_on_core(creator(), cores[0]);
}