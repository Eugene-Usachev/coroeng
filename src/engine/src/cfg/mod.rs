#[derive(Copy, Clone)]
pub enum SelectorType {
    Poller,
    Ring
}

pub struct SchedulerCfg {
    buf_len: usize,
    selector: SelectorType
}

impl SchedulerCfg {
    pub const fn default() -> Self {
        Self {
            buf_len: 4096,
            selector: SelectorType::Poller
        }
    }
}

pub(crate) static mut SCHEDULER_CFG: SchedulerCfg = SchedulerCfg::default();

pub fn config_buf_len() -> usize {
    unsafe { SCHEDULER_CFG.buf_len }
}

pub fn config_selector() -> SelectorType {
    unsafe { SCHEDULER_CFG.selector }
}

#[allow(dead_code)]
pub fn set_selector(selector: SelectorType) {
    unsafe { SCHEDULER_CFG.selector = selector }
}

#[allow(dead_code)]
pub fn set_buf_len(buf_len: usize) {
    unsafe { SCHEDULER_CFG.buf_len = buf_len }
}

#[allow(dead_code)]
pub fn set_config(config: SchedulerCfg) {
    unsafe { SCHEDULER_CFG = config }
}