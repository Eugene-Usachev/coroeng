/// A type of the [`Selector`](crate::io::selector::Selector).
/// It can be `Poller` or `Ring`.
///
/// Ring based on `io-uring` for Linux.
///
/// Poller based on `epoll` for Linux.
#[derive(Copy, Clone)]
pub enum SelectorType {
    Poller,
    Ring
}

/// The configuration of the scheduler.
pub struct SchedulerCfg {
    buf_len: usize,
    selector: SelectorType
}

impl SchedulerCfg {
    /// Creates a new SchedulerCfg with default values.
    /// We can't use [`Default`] trait, because [`Default::default`] is not `const fn`.
    pub const fn default() -> Self {
        Self {
            buf_len: 4096,
            selector: SelectorType::Poller
        }
    }
}

/// The configuration of the scheduler.
/// This will be read only on [`run_on_core`](crate::run::run_on_core) and [`run_on_all_cores`](crate::run::run_on_all_cores).
pub(crate) static mut SCHEDULER_CFG: SchedulerCfg = SchedulerCfg::default();

/// Getter for [`SCHEDULER_CFG::buf_len`].
pub fn config_buf_len() -> usize {
    unsafe { SCHEDULER_CFG.buf_len }
}

/// Getter for [`SCHEDULER_CFG::selector`].
pub fn config_selector() -> SelectorType {
    unsafe { SCHEDULER_CFG.selector }
}

/// Setter for [`SCHEDULER_CFG::selector`].
#[allow(dead_code)]
pub fn set_selector(selector: SelectorType) {
    unsafe { SCHEDULER_CFG.selector = selector }
}

/// Setter for [`SCHEDULER_CFG::buf_len`].
#[allow(dead_code)]
pub fn set_buf_len(buf_len: usize) {
    unsafe { SCHEDULER_CFG.buf_len = buf_len }
}

/// Setter for [`SCHEDULER_CFG`].
#[allow(dead_code)]
pub fn set_config(config: SchedulerCfg) {
    unsafe { SCHEDULER_CFG = config }
}