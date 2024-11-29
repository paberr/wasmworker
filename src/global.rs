use send_wrapper::SendWrapper;
use tokio::sync::OnceCell;
use wasm_bindgen::{prelude::wasm_bindgen, UnwrapThrowExt};

use crate::pool::{WebWorkerPool, WorkerPoolOptions};

static WORKER_POOL: OnceCell<SendWrapper<WebWorkerPool>> = OnceCell::const_new();

/// This function can be called before the first use of the global worker pool to configure it.
/// It takes a [`WorkerPoolOptions`] configuration object. Note that this function is async.
///
/// ```ignore
/// # use wasmworker::{init_worker_pool, WorkerPoolOptions};
/// init_worker_pool(WorkerPoolOptions {
///     num_workers: Some(2),
///     ..Default::default()
/// }).await
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
pub async fn init_worker_pool(options: WorkerPoolOptions) {
    WORKER_POOL
        .get_or_init(|| async move {
            SendWrapper::new(
                WebWorkerPool::with_options(options)
                    .await
                    .expect_throw("Couldn't instantiate worker pool"),
            )
        })
        .await;
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
                WebWorkerPool::new()
                    .await
                    .expect_throw("Couldn't instantiate worker pool"),
            )
        })
        .await
}
