use std::borrow::Borrow;

use futures::future::join_all;
use js_sys::wasm_bindgen::{prelude::wasm_bindgen, UnwrapThrowExt};
use scheduler::Scheduler;
pub use scheduler::Strategy;
use serde::{Deserialize, Serialize};

use wasm_bindgen_futures::JsFuture;
use web_sys::{window, MessagePort};

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
#[non_exhaustive]
pub struct WorkerPoolOptions {
    /// The path to the wasm-bindgen glue. By default, this path is inferred.
    /// [`crate::WebWorker::with_path`] lists more details on when this path
    /// should be manually configured.
    pub path: Option<String>,
    pub path_bg: Option<String>,
    /// The strategy to be used by the worker pool.
    pub strategy: Option<Strategy>,
    /// The number of workers that will be spawned. This defaults to `navigator.hardwareConcurrency`.
    pub num_workers: Option<usize>,
    /// Whether to precompile and share the WASM module across workers for bandwidth optimization.
    /// This reduces the number of WASM fetches from N (one per worker) to 1 (shared across all workers).
    pub precompile_wasm: Option<bool>,
    /// Pre-compiled WASM module to share across workers. Internal use only.
    pub(crate) wasm_module: Option<js_sys::WebAssembly::Module>,
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

    fn path_bg(&self) -> Option<&str> {
        self.path_bg.as_deref()
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
    /// Pre-compiled WASM module shared across workers (kept alive to prevent dropping)
    #[allow(dead_code)]
    wasm_module: Option<js_sys::WebAssembly::Module>,
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
    pub async fn with_options(mut options: WorkerPoolOptions) -> Result<Self, InitError> {
        // Pre-compile WASM module if explicitly requested or not already provided
        let wasm_module =
            if options.wasm_module.is_none() && options.precompile_wasm.unwrap_or(false) {
                Some(Self::precompile_wasm(&options).await?)
            } else {
                options.wasm_module.take()
            };

        let worker_inits = (0..options.num_workers()).map(|_| {
            // Do not impose a task limit.
            WebWorker::with_path_and_module(
                options.path(),
                options.path_bg(),
                None,
                wasm_module.clone(),
            )
        });
        let workers = join_all(worker_inits).await;
        let workers = workers.into_iter().collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            workers,
            scheduler: Scheduler::new(options.strategy()),
            wasm_module,
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
    pub async fn run<T, R>(&self, func: WebWorkerFn<T, R>, arg: &T) -> R
    where
        T: Serialize + for<'de> Deserialize<'de>,
        R: Serialize + for<'de> Deserialize<'de>,
    {
        self.run_internal(func, arg, None).await
    }

    #[cfg(feature = "serde")]
    pub async fn run_with_channel<T, R>(
        &self,
        func: WebWorkerFn<T, R>,
        arg: &T,
        port: MessagePort,
    ) -> R
    where
        T: Serialize + for<'de> Deserialize<'de>,
        R: Serialize + for<'de> Deserialize<'de>,
    {
        self.run_internal(func, arg, Some(port)).await
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
        self.run_internal(func, arg, None).await
    }

    /// Determines the worker to run the task on using the scheduler
    /// and runs the task.
    pub(crate) async fn run_internal<T, R, A>(
        &self,
        func: WebWorkerFn<T, R>,
        arg: A,
        port: Option<MessagePort>,
    ) -> R
    where
        A: Borrow<T>,
        T: Serialize + for<'de> Deserialize<'de>,
        R: Serialize + for<'de> Deserialize<'de>,
    {
        let worker_id = self.scheduler.schedule(self);
        self.workers[worker_id]
            .run_internal(func, arg.borrow(), port)
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

    /// Create a worker pool with a pre-compiled WASM module for optimal bandwidth usage.
    /// This method pre-compiles the WASM module once and shares it across all workers,
    /// reducing bandwidth usage compared to each worker loading the WASM independently.
    pub async fn with_precompiled_wasm() -> Result<Self, InitError> {
        let mut options = WorkerPoolOptions::new();
        options.precompile_wasm = Some(true);
        Self::with_options(options).await
    }

    /// Pre-compile the WASM module for sharing across workers.
    ///
    /// This function fetches and compiles the WASM module once, which can then be
    /// shared across all workers to reduce bandwidth usage.
    ///
    /// Path resolution:
    /// - If `path_bg` is provided, it should be the full URL to the WASM file
    /// - If `path` is provided, assumes standard wasm-bindgen naming (_bg.wasm suffix)
    /// - Otherwise, infers path from the current module location
    async fn precompile_wasm(
        options: &WorkerPoolOptions,
    ) -> Result<js_sys::WebAssembly::Module, InitError> {
        use wasm_bindgen::JsCast;

        // Get the WASM path - if path_bg is provided, use it directly since it should be the WASM URL
        let wasm_path = if let Some(bg_path) = options.path_bg() {
            // path_bg should already be the WASM URL (e.g., "http://localhost:8080/webapp_bg.wasm")
            bg_path.to_string()
        } else if let Some(js_path) = options.path() {
            // Convert main JS path to WASM path (typically add _bg.wasm)
            if js_path.ends_with(".js") {
                js_path.replace(".js", "_bg.wasm")
            } else {
                format!("{}_bg.wasm", js_path)
            }
        } else {
            // Use default path inference from the main JS module
            let js_path = crate::webworker::js::main_js().as_string().unwrap_throw();
            if js_path.ends_with(".js") {
                js_path.replace(".js", "_bg.wasm")
            } else {
                format!("{}_bg.wasm", js_path)
            }
        };

        // Fetch the WASM file
        use wasm_bindgen::UnwrapThrowExt;
        let window = web_sys::window().unwrap_throw();
        let resp_value = JsFuture::from(window.fetch_with_str(&wasm_path))
            .await
            .map_err(|e| {
                InitError::WebWorkerModuleLoading(format!(
                    "Failed to fetch WASM from '{}': {:?}. Check that path_bg points to the correct WASM file URL.",
                    wasm_path, e
                ))
            })?;
        let resp: web_sys::Response = resp_value.unchecked_into();

        let array_buffer = JsFuture::from(resp.array_buffer().unwrap_throw())
            .await
            .map_err(|e| {
                InitError::WebWorkerModuleLoading(format!(
                    "Failed to read WASM bytes from '{}': {:?}",
                    wasm_path, e
                ))
            })?;

        // Compile the WASM module
        let compile_promise = js_sys::WebAssembly::compile(&array_buffer);
        let module_value = JsFuture::from(compile_promise).await.map_err(|e| {
            InitError::WebWorkerModuleLoading(format!(
                "Failed to compile WASM from '{}': {:?}. This usually means the file is not a valid WASM binary or the URL returned an error page.",
                wasm_path, e
            ))
        })?;

        Ok(module_value.into())
    }
}
