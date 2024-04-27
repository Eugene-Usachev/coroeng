#[allow(dead_code)]
#[inline]
pub const fn ns_to_ms(ns: u64) -> u64 {
    (ns + 1_000_000 - 1) / 1_000_000
}