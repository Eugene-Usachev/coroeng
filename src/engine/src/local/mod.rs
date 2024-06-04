pub mod id;
pub mod local;

pub use id::{
    get_core_id,
    get_worker_id
};

pub use local::*;