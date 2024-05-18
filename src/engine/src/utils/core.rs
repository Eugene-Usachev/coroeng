pub type CoreId = core_affinity::CoreId;

pub fn get_core_ids() -> Option<Vec<CoreId>> {
    core_affinity::get_core_ids()
}

pub fn set_for_current(core_id: CoreId) {
    core_affinity::set_for_current(core_id);
}