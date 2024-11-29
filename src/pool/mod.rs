use std::borrow::Borrow;

use futures::future::join_all;
use js_sys::wasm_bindgen::{prelude::wasm_bindgen, UnwrapThrowExt};
use scheduler::Scheduler;
pub use scheduler::Strategy;
use serde::{Deserialize, Serialize};
use web_sys::window;

use crate::{error::InitError, func::WebWorkerFn, WebWorker};

mod scheduler;

/// This struct can be used to configure all options of the [`WebWorkerPool`].
///
/// If re-exported, the struct can also be accessed via JavaScript:
/// ```js
/// let options = new WorkerPoolOptions();
/// options.num_workers = 3;
/// ```
#[wasm_bindgen(getter_with_clone)]
#[derive(Default, Clone)]
pub struct WorkerPoolOptions {
    /// The path to the wasm-bindgen glue. By default, this path is inferred.
    /// [`crate::WebWorker::with_path`] lists more details on when this path
    /// should be manually configured.
    pub path: Option<String>,
    /// The strategy to be used by the worker pool.
    pub strategy: Option<Strategy>,
    /// The number of workers that will be spawned. This defaults to `navigator.hardwareConcurrency`.
    pub num_workers: Option<usize>,
}

#[wasm_bindgen]
impl WorkerPoolOptions {
    /// Creates the default options.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Default::default()
    }
}

impl WorkerPoolOptions {
    /// Returns the path to be used.
    fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Returns the configured strategy or the default strategy.
    fn strategy(&self) -> Strategy {
        self.strategy.unwrap_or_default()
    }

    /// Returns the number of workers, which defaults `navigator.hardwareConcurrency`.
    fn num_workers(&self) -> usize {
        self.num_workers.unwrap_or_else(|| {
            window()
                .expect_throw("Window missing")
                .navigator()
                .hardware_concurrency() as usize
        })
    }
}

/// This struct represents a worker pool, i.e., a collection of [`WebWorker`] objects
/// and a scheduler that distributes tasks among those.
///
/// While multiple pools can be spawned, most often it is sufficient to have a single pool.
/// This library already supports one global web worker pool, which can be accessed with
/// [`crate::worker_pool()`].
///
/// Example usage:
/// ```ignore
/// use wasmworker::{webworker, worker_pool};
///
/// let worker_pool = worker_pool().await;
/// let res = worker_pool.run(webworker!(sort_vec), &VecType(vec![5, 2, 8])).await;
/// assert_eq!(res.0, vec![2, 5, 8]);
/// ```
pub struct WebWorkerPool {
    /// The workers that have been spawned.
    workers: Vec<WebWorker>,
    /// The internal scheduler that is used to distribute the tasks.
    scheduler: Scheduler,
}

impl WebWorkerPool {
    /// Initializes a worker pool with default [`WorkerPoolOptions`].
    /// This async function might return an [`InitError`] if one of the workers
    /// cannot be initialized, as described in [`WebWorker::new`].
    pub async fn new() -> Result<Self, InitError> {
        Self::with_options(WorkerPoolOptions::default()).await
    }

    /// Initializes a worker pool with a given strategy and otherwise default [`WorkerPoolOptions`].
    /// This async function might return an [`InitError`] if one of the workers
    /// cannot be initialized, as described in [`WebWorker::new`].
    pub async fn with_strategy(strategy: Strategy) -> Result<Self, InitError> {
        Self::with_options(WorkerPoolOptions {
            strategy: Some(strategy),
            ..Default::default()
        })
        .await
    }

    /// Initializes a worker pool with a given number of workers and otherwise default [`WorkerPoolOptions`].
    /// This async function might return an [`InitError`] if one of the workers
    /// cannot be initialized, as described in [`WebWorker::new`].
    pub async fn with_num_workers(num_workers: usize) -> Result<Self, InitError> {
        Self::with_options(WorkerPoolOptions {
            num_workers: Some(num_workers),
            ..Default::default()
        })
        .await
    }

    /// Initializes a worker pool with a given path and otherwise default [`WorkerPoolOptions`].
    /// This async function might return an [`InitError`] if one of the workers
    /// cannot be initialized, as described in [`WebWorker::new`].
    pub async fn with_path(path: String) -> Result<Self, InitError> {
        Self::with_options(WorkerPoolOptions {
            path: Some(path),
            ..Default::default()
        })
        .await
    }

    /// Initializes a worker pool with the given [`WorkerPoolOptions`].
    /// This async function might return an [`InitError`] if one of the workers
    /// cannot be initialized, as described in [`WebWorker::new`].
    pub async fn with_options(options: WorkerPoolOptions) -> Result<Self, InitError> {
        let worker_inits = (0..options.num_workers()).map(|_| {
            // Do not impose a task limit.
            WebWorker::with_path(options.path(), None)
        });
        let workers = join_all(worker_inits).await;
        let workers = workers.into_iter().collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            workers,
            scheduler: Scheduler::new(options.strategy()),
        })
    }

    /// This is the most general function to outsource a task on a [`WebWorkerPool`].
    /// It will automatically handle serialization of the argument, scheduling of the task on the pool,
    /// and deserialization of the return value.
    ///
    /// The `func`: [`WebWorkerFn`] argument should normally be instantiated using the [`crate::webworker!`] macro.
    /// This ensures type safety and that the function is correctly exposed to the worker.
    ///
    /// Example:
    /// ```ignore
    /// worker_pool().await.run(webworker!(sort_vec), &my_vec).await
    /// ```
    #[cfg(feature = "serde")]
    pub async fn run<T, R>(&self, func: WebWorkerFn<T, R>, arg: &T) -> R
    where
        T: Serialize + for<'de> Deserialize<'de>,
        R: Serialize + for<'de> Deserialize<'de>,
    {
        self.run_internal(func, arg).await
    }

    /// This function can outsource a task on a [`WebWorkerPool`] which has `Box<[u8]>` both as input and output.
    /// (De)serialization of values needs to be handled by the caller.
    /// For more convenient access, make sure the `serde` feature is enabled and use [`WebWorkerPool::run`].
    ///
    /// The `func`: [`WebWorkerFn`] argument should normally be instantiated using the [`crate::webworker!`] macro.
    /// This ensures type safety and that the function is correctly exposed to the worker.
    ///
    /// Example:
    /// ```ignore
    /// worker_pool().await.run_bytes(webworker!(sort), &my_box).await
    /// ```
    pub async fn run_bytes(
        &self,
        func: WebWorkerFn<Box<[u8]>, Box<[u8]>>,
        arg: &Box<[u8]>,
    ) -> Box<[u8]> {
        self.run_internal(func, arg).await
    }

    /// Determines the worker to run the task on using the scheduler
    /// and runs the task.
    pub(crate) async fn run_internal<T, R, A>(&self, func: WebWorkerFn<T, R>, arg: A) -> R
    where
        A: Borrow<T>,
        T: Serialize + for<'de> Deserialize<'de>,
        R: Serialize + for<'de> Deserialize<'de>,
    {
        let worker_id = self.scheduler.schedule(self);
        self.workers[worker_id]
            .run_internal(func, arg.borrow())
            .await
    }

    /// Return the number of tasks currently queued to this worker pool.
    pub fn current_load(&self) -> usize {
        self.workers.iter().map(WebWorker::current_load).sum()
    }

    /// Return the number of workers in the pool.
    pub fn num_workers(&self) -> usize {
        self.workers.len()
    }
}
