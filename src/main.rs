#![feature(core_intrinsics)]
#![feature(trait_alias)]
#![feature(coroutines, coroutine_trait)]
#![feature(fn_traits)]


mod engine;
mod utils;

use crate::engine::local::test::local_test;

fn main() {
    local_test();
}