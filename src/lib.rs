//! # wasmworker
//!
//! Parallelize tasks on WebAssembly without `SharedArrayBuffer`.
//!
//! See the [README](https://github.com/paberr/wasmworker) for full documentation
//! including bundler setup and FAQ.
//!
//! ## Quick start
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! wasmworker = { version = "0.2", features = ["macros"] }
//! ```
//!
//! ### Defining worker functions
//!
//! ```no_run
//! use serde::{Deserialize, Serialize};
//! use wasmworker::{webworker, webworker_fn};
//!
//! /// An arbitrary type that is (de)serializable.
//! #[derive(Serialize, Deserialize)]
//! pub struct VecType(Vec<u8>);
//!
//! /// A sort function on a custom type.
//! #[webworker_fn]
//! pub fn sort_vec(mut v: VecType) -> VecType {
//!     v.0.sort();
//!     v
//! }
//!
//! # fn main() {
//! // Obtain a type-safe handle to the function:
//! let ww_sort = webworker!(sort_vec);
//! # }
//! ```
//!
//! ### Running tasks
//!
//! ```no_run
//! # use serde::{Deserialize, Serialize};
//! # use wasmworker::{webworker, webworker_fn, WebWorker};
//! #
//! # #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! # pub struct VecType(Vec<u8>);
//! #
//! # #[webworker_fn]
//! # pub fn sort_vec(mut v: VecType) -> VecType {
//! #     v.0.sort();
//! #     v
//! # }
//! #
//! # async fn example() {
//! let worker = WebWorker::new(None).await.expect("Couldn't create worker");
//! let sorted = worker.run(webworker!(sort_vec), &VecType(vec![3, 1, 2])).await;
//! assert_eq!(sorted.0, vec![1, 2, 3]);
//! # }
//! # fn main() {}
//! ```
//!
//! ```no_run
//! # use serde::{Deserialize, Serialize};
//! # use wasmworker::{webworker, webworker_fn, worker_pool};
//! #
//! # #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! # pub struct VecType(Vec<u8>);
//! #
//! # #[webworker_fn]
//! # pub fn sort_vec(mut v: VecType) -> VecType {
//! #     v.0.sort();
//! #     v
//! # }
//! #
//! # async fn example() {
//! let worker_pool = worker_pool().await;
//! let sorted = worker_pool.run(webworker!(sort_vec), &VecType(vec![3, 1, 2])).await;
//! assert_eq!(sorted.0, vec![1, 2, 3]);
//! # }
//! # fn main() {}
//! ```
//!
//! ### Configuring the worker pool
//!
//! ```no_run
//! # use wasmworker::{init_worker_pool, WorkerPoolOptions};
//! #
//! # async fn startup() {
//! let mut options = WorkerPoolOptions::new();
//! options.num_workers = Some(2); // Default is navigator.hardwareConcurrency
//! init_worker_pool(options).await.expect("Worker pool already initialized");
//! # }
//! # fn main() {}
//! ```

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
#[cfg(feature = "iter-ext")]
pub mod iter_ext;
pub mod pool;
mod webworker;
