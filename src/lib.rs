#![doc = include_str!("../README.md")]
#![allow(clippy::borrowed_box)]
pub use channel::Channel;
pub use channel_task::ChannelTask;
pub use global::{
    has_worker_pool, init_optimized_worker_pool, init_worker_pool, worker_pool, AlreadyInitialized,
};
pub use pool::WorkerPoolOptions;
pub use webworker::WebWorker;

#[doc(hidden)]
pub use web_sys::MessagePort;

// Re-export WebWorkerPool from pool module
pub use pool::WebWorkerPool;

#[cfg(feature = "macros")]
pub use wasmworker_proc_macro::*;

mod channel;
mod channel_task;
pub mod convert;
pub mod error;
pub mod func;
mod global;
#[cfg(feature = "serde")]
pub mod iter_ext;
pub mod pool;
mod webworker;
