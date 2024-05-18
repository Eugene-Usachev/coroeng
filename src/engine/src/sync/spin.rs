use std::hint;

const SPIN_LIMIT: usize = 6;


pub(crate) fn spin(step: usize) {
    if step <= SPIN_LIMIT {
        for _ in 0..1 << step {
            hint::spin_loop();
        }
    } else {
        for _ in 0..1 << SPIN_LIMIT {
            hint::spin_loop();
        }
    }
}