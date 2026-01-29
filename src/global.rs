use send_wrapper::SendWrapper;
use tokio::sync::OnceCell;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue, UnwrapThrowExt};

use crate::pool::{WebWorkerPool, WorkerPoolOptions};

static WORKER_POOL: OnceCell<SendWrapper<WebWorkerPool>> = OnceCell::const_new();

/// Error returned when [`init_worker_pool`] is called after the worker pool has already been initialized.
#[derive(Debug, Clone, Copy)]
pub struct AlreadyInitialized;

impl std::fmt::Display for AlreadyInitialized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Worker pool has already been initialized")
    }
}

impl std::error::Error for AlreadyInitialized {}

impl From<AlreadyInitialized> for JsValue {
    fn from(err: AlreadyInitialized) -> Self {
        JsValue::from_str(&err.to_string())
    }
}

/// This function can be called before the first use of the global worker pool to configure it.
/// It takes a [`WorkerPoolOptions`] configuration object. Note that this function is async.
///
/// Returns an error if the worker pool has already been initialized (options would be ignored).
///
/// ```ignore
/// # use wasmworker::{init_worker_pool, WorkerPoolOptions};
/// let mut options = WorkerPoolOptions::new();
/// options.num_workers = Some(2);
/// init_worker_pool(options).await.expect("Worker pool already initialized");
/// ```
///
/// This function can also be called from JavaScript:
/// ```js
/// // Make sure to use the correct path.
/// import init, { initWorkerPool, WorkerPoolOptions } from "./pkg/wasmworker_demo.js";
///
/// await init();
/// let options = new WorkerPoolOptions();
/// options.num_workers = 3;
/// await initWorkerPool(options);
/// ```
#[wasm_bindgen(js_name = initWorkerPool)]
pub async fn init_worker_pool(options: WorkerPoolOptions) -> Result<(), AlreadyInitialized> {
    let pool = SendWrapper::new(
        WebWorkerPool::with_options(options)
            .await
            .expect_throw("Couldn't instantiate worker pool"),
    );
    WORKER_POOL.set(pool).map_err(|_| AlreadyInitialized)
}

/// JavaScript-accessible function to initialize an optimized worker pool globally.
/// This creates a worker pool that precompiles and shares WASM across all workers
/// for optimal bandwidth usage.
///
/// ```js
/// import init, { initOptimizedWorkerPool } from "./pkg/webapp.js";
///
/// await init();
/// await initOptimizedWorkerPool();
/// ```
#[wasm_bindgen(js_name = initOptimizedWorkerPool)]
pub async fn init_optimized_worker_pool() -> Result<(), AlreadyInitialized> {
    let mut options = WorkerPoolOptions::new();
    options.precompile_wasm = Some(true);
    init_worker_pool(options).await
}

/// This function accesses the default worker pool.
/// If [`init_worker_pool`] has not been manually called,
/// this function will initialize the worker pool prior to returning it.
///
/// It will use the options provided by [`WorkerPoolOptions::default()`].
pub async fn worker_pool() -> &'static WebWorkerPool {
    WORKER_POOL
        .get_or_init(|| async {
            SendWrapper::new(
                WebWorkerPool::with_options(WorkerPoolOptions::default())
                    .await
                    .expect_throw("Couldn't instantiate worker pool"),
            )
        })
        .await
}

/// This function checks if the worker pool has been initialized.
pub fn has_worker_pool() -> bool {
    WORKER_POOL.initialized()
}
