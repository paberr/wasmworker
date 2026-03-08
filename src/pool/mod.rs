use std::{borrow::Borrow, cell::RefCell, rc::Rc};

use futures::future::join_all;
use js_sys::wasm_bindgen::{prelude::wasm_bindgen, UnwrapThrowExt};
use scheduler::Scheduler;
pub use scheduler::Strategy;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;

use wasm_bindgen_futures::JsFuture;
use web_sys::window;

use crate::{
    channel_task::ChannelTask,
    error::InitError,
    func::{WebWorkerChannelFn, WebWorkerFn},
    WebWorker,
};

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
    /// The path to the wasm-bindgen glue JS file. By default, this path is inferred
    /// from `import.meta.url`.
    /// [`crate::WebWorker::with_path`] lists more details on when this path
    /// should be manually configured.
    pub path: Option<String>,
    /// The path to the WASM binary file. When set, this is passed as `module_or_path`
    /// to the wasm-bindgen `init()` function inside the worker.
    /// By default, the glue code resolves this automatically relative to itself.
    /// Set this when your build setup places the WASM binary at a non-standard location.
    pub path_bg: Option<String>,
    /// The strategy to be used by the worker pool.
    pub strategy: Option<Strategy>,
    /// The number of workers that will be spawned. This defaults to `navigator.hardwareConcurrency`.
    pub num_workers: Option<usize>,
    /// Whether to precompile and share the WASM module across workers for bandwidth optimization.
    /// This reduces the number of WASM fetches from N (one per worker) to 1 (shared across all workers).
    pub precompile_wasm: Option<bool>,
    /// Idle timeout in milliseconds. Workers with no pending tasks will be terminated
    /// after being idle for this duration. They are transparently recreated when new tasks arrive.
    /// Default: `None` (no timeout, workers live for the pool's lifetime).
    pub idle_timeout_ms: Option<u32>,
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
/// ```no_run
/// # use serde::{Serialize, Deserialize};
/// # use wasmworker_proc_macro::webworker_fn;
/// # #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// # struct VecType(Vec<u32>);
/// # #[webworker_fn]
/// # pub fn sort_vec(mut v: VecType) -> VecType { v.0.sort(); v }
/// use wasmworker::{webworker, worker_pool};
///
/// # async fn example() {
/// let worker_pool = worker_pool().await;
/// let res = worker_pool.run(webworker!(sort_vec), &VecType(vec![5, 2, 8])).await;
/// assert_eq!(res.0, vec![2, 5, 8]);
/// # }
/// # fn main() {}
/// ```
/// The state of a single worker slot in the pool.
enum WorkerSlot {
    /// Worker is active and can accept tasks.
    Active(WebWorker),
    /// Worker is being created (prevents duplicate creation during async init).
    Creating,
    /// Worker was terminated by idle timeout and can be recreated.
    Empty,
}

pub struct WebWorkerPool {
    /// The worker slots (per-slot RefCell for independent borrowing).
    slots: Rc<Vec<RefCell<WorkerSlot>>>,
    /// The total number of slots (pool capacity).
    num_slots: usize,
    /// The internal scheduler that is used to distribute the tasks.
    scheduler: Scheduler,
    /// Pre-compiled WASM module shared across workers (kept alive to prevent dropping)
    #[allow(dead_code)]
    wasm_module: Option<js_sys::WebAssembly::Module>,
    /// Config retained for worker re-creation.
    pool_path: Option<String>,
    pool_path_bg: Option<String>,
    /// Idle checker setInterval closure (prevent GC).
    _idle_checker_cb: Option<Closure<dyn FnMut()>>,
    /// Idle checker interval ID (for clearInterval on Drop).
    _idle_checker_id: Option<i32>,
    /// Notify waiting tasks when a worker becomes available after creation.
    worker_ready: tokio::sync::Notify,
}

impl Drop for WebWorkerPool {
    fn drop(&mut self) {
        if let Some(id) = self._idle_checker_id {
            if let Some(w) = web_sys::window() {
                w.clear_interval_with_handle(id);
            }
        }
    }
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

        let num_slots = options.num_workers().max(1);
        let worker_inits = (0..num_slots).map(|_| {
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

        let slots: Rc<Vec<RefCell<WorkerSlot>>> = Rc::new(
            workers
                .into_iter()
                .map(|w| RefCell::new(WorkerSlot::Active(w)))
                .collect(),
        );

        // Set up idle timeout checker if configured.
        let (idle_checker_cb, idle_checker_id) = if let Some(timeout) = options.idle_timeout_ms {
            let slots_clone = Rc::clone(&slots);
            let cb = Closure::<dyn FnMut()>::new(move || {
                let now = js_sys::Date::now();
                for i in 0..slots_clone.len() {
                    let should_terminate = {
                        let s = slots_clone[i].borrow();
                        matches!(&*s, WorkerSlot::Active(ref w)
                            if w.current_load() == 0 && (now - w.last_active()) >= timeout as f64)
                    };
                    if should_terminate {
                        *slots_clone[i].borrow_mut() = WorkerSlot::Empty;
                    }
                }
            });
            let id = window()
                .expect_throw("Window missing")
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    (timeout / 2).max(1).min(i32::MAX as u32) as i32,
                )
                .expect_throw("Could not set interval");
            (Some(cb), Some(id))
        } else {
            (None, None)
        };

        Ok(Self {
            slots,
            num_slots,
            scheduler: Scheduler::new(options.strategy()),
            wasm_module,
            pool_path: options.path.clone(),
            pool_path_bg: options.path_bg.clone(),
            _idle_checker_cb: idle_checker_cb,
            _idle_checker_id: idle_checker_id,
            worker_ready: tokio::sync::Notify::new(),
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
        self.run_internal(func, arg).await
    }

    /// Run an async function with bidirectional channel support on this [`WebWorkerPool`].
    ///
    /// Returns a [`ChannelTask`] that provides both the communication channel and the
    /// task result. The `MessageChannel` is created internally.
    ///
    /// The `func`: [`WebWorkerChannelFn`] argument should normally be instantiated using the
    /// [`crate::webworker_channel!`] macro. This ensures type safety and that the function
    /// is correctly exposed to the worker.
    ///
    /// Example:
    /// ```ignore
    /// let task = worker_pool().await
    ///     .run_channel(webworker_channel!(process_with_progress), &data)
    ///     .await;
    ///
    /// let progress: Progress = task.recv().await.expect("progress");
    /// task.send(&Continue { should_continue: true });
    /// let result: ProcessResult = task.result().await;
    /// ```
    pub async fn run_channel<T, R>(&self, func: WebWorkerChannelFn<T, R>, arg: &T) -> ChannelTask<R>
    where
        T: Serialize + for<'de> Deserialize<'de>,
        R: Serialize + for<'de> Deserialize<'de>,
    {
        self.run_channel_internal(func, arg).await
    }

    /// This function can outsource a task on a [`WebWorkerPool`] which has `Box<[u8]>` both as input and output.
    /// (De)serialization of values needs to be handled by the caller.
    /// For more convenient access, use [`WebWorkerPool::run`] instead.
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

    /// Acquires an active worker slot, recreating a terminated worker if needed.
    async fn acquire_worker(&self) -> usize {
        loop {
            let loads = self.compute_loads();
            if let Some(id) = self.scheduler.schedule(&loads) {
                return id;
            }

            // No active workers. Find first Empty slot and recreate.
            let empty_slot = self
                .slots
                .iter()
                .position(|slot| matches!(&*slot.borrow(), WorkerSlot::Empty));
            if let Some(i) = empty_slot {
                *self.slots[i].borrow_mut() = WorkerSlot::Creating;
            }

            if let Some(slot_id) = empty_slot {
                let worker_result = WebWorker::with_path_and_module(
                    self.pool_path.as_deref(),
                    self.pool_path_bg.as_deref(),
                    None,
                    self.wasm_module.clone(),
                )
                .await;
                match worker_result {
                    Ok(worker) => {
                        *self.slots[slot_id].borrow_mut() = WorkerSlot::Active(worker);
                        self.worker_ready.notify_waiters();
                        return slot_id;
                    }
                    Err(_) => {
                        *self.slots[slot_id].borrow_mut() = WorkerSlot::Empty;
                        self.worker_ready.notify_waiters();
                        panic!("Couldn't recreate worker");
                    }
                }
            }

            // All slots are Creating — wait for one to finish.
            self.worker_ready.notified().await;
        }
    }

    /// Compute per-slot loads for the scheduler.
    fn compute_loads(&self) -> Vec<Option<usize>> {
        self.slots
            .iter()
            .map(|slot| match &*slot.borrow() {
                WorkerSlot::Active(w) => Some(w.current_load()),
                _ => None,
            })
            .collect()
    }

    /// Determines the worker to run a simple task on using the scheduler
    /// and runs the task.
    // Per-slot RefCell: holding a borrow across await is safe because
    // the idle checker only terminates slots with zero load (i.e., not borrowed).
    #[allow(clippy::await_holding_refcell_ref)]
    pub(crate) async fn run_internal<T, R, A>(&self, func: WebWorkerFn<T, R>, arg: A) -> R
    where
        A: Borrow<T>,
        T: Serialize + for<'de> Deserialize<'de>,
        R: Serialize + for<'de> Deserialize<'de>,
    {
        let worker_id = self.acquire_worker().await;
        let slot = self.slots[worker_id].borrow();
        match &*slot {
            WorkerSlot::Active(worker) => worker.run_internal(func, arg.borrow()).await,
            _ => unreachable!("acquire_worker guarantees Active slot"),
        }
    }

    /// Determines the worker to run a channel task on using the scheduler
    /// and runs the task.
    // Per-slot RefCell: holding a borrow across await is safe because
    // the idle checker only terminates slots with zero load (i.e., not borrowed).
    #[allow(clippy::await_holding_refcell_ref)]
    pub(crate) async fn run_channel_internal<T, R>(
        &self,
        func: WebWorkerChannelFn<T, R>,
        arg: &T,
    ) -> ChannelTask<R>
    where
        T: Serialize + for<'de> Deserialize<'de>,
        R: Serialize + for<'de> Deserialize<'de>,
    {
        let worker_id = self.acquire_worker().await;
        let slot = self.slots[worker_id].borrow();
        match &*slot {
            WorkerSlot::Active(worker) => worker.run_channel_internal(func, arg).await,
            _ => unreachable!("acquire_worker guarantees Active slot"),
        }
    }

    /// Return the number of tasks currently queued to this worker pool.
    pub fn current_load(&self) -> usize {
        self.slots
            .iter()
            .map(|slot| match &*slot.borrow() {
                WorkerSlot::Active(w) => w.current_load(),
                _ => 0,
            })
            .sum()
    }

    /// Return the total number of worker slots in the pool (pool capacity).
    pub fn num_workers(&self) -> usize {
        self.num_slots
    }

    /// Return the number of currently active (non-terminated) workers.
    pub fn num_active_workers(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| matches!(&*RefCell::borrow(s), WorkerSlot::Active(_)))
            .count()
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
