#![allow(internal_features)]
#![feature(core_intrinsics)]
#![feature(trait_alias)]
#![feature(coroutines, coroutine_trait)]
#![feature(fn_traits)]
#![feature(panic_info_message)]
#![feature(negative_impls)]

pub mod coroutine;
pub mod macros;
pub mod io;
pub mod net;
pub mod local;
pub mod sleep;
pub mod sync;
pub mod utils;
pub mod cfg;
pub mod run;
pub mod buf;
pub mod scheduler;
pub mod fs;
//pub mod work_stealing;

pub use scheduler::local_scheduler;
#[allow(unused_imports)]
pub use macros::*;
pub use run::*;
pub use proc::{test_local, coro, wait, spawn_local};
