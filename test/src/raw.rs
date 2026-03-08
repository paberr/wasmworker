use wasm_bindgen::{throw_str, UnwrapThrowExt};
use wasm_bindgen_futures::JsFuture;
use wasmworker::webworker_fn;
use wasmworker::{
    error::InitError, webworker, worker_pool, WebWorker, WebWorkerPool, WorkerPoolOptions,
};

use crate::js_assert_eq;

async fn sleep_ms(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap_throw()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32)
            .unwrap_throw();
    });
    JsFuture::from(promise).await.unwrap_throw();
}

#[webworker_fn]
pub fn sort(mut v: Box<[u8]>) -> Box<[u8]> {
    v.sort();
    v
}

pub(crate) async fn can_handle_invalid_paths() {
    let worker = WebWorker::with_path(Some("something"), None, None).await;
    if !matches!(worker, Err(InitError::WebWorkerModuleLoading(_))) {
        throw_str("Should have failed initialization with wrong path");
    }
}

pub(crate) async fn can_run_task_bytes() {
    let worker = WebWorker::new(None).await.expect("Couldn't create worker");

    // Prepare input and output.
    let vec = vec![8, 1, 5, 0, 4];
    let mut sorted_vec = vec.clone();
    sorted_vec.sort();
    let vec = vec.into();
    let sorted_vec = sorted_vec.into();

    // Test try run.
    let res1 = worker
        .try_run_bytes(webworker!(sort), &vec)
        .await
        .expect("Should not be full");
    js_assert_eq!(res1, sorted_vec, "Raw try run failed");

    // Test run.
    let res2 = worker.run_bytes(webworker!(sort), &vec).await;
    js_assert_eq!(res2, sorted_vec, "Raw run failed");
}

pub(crate) async fn can_limit_tasks_bytes() {
    let worker = WebWorker::new(Some(0))
        .await
        .expect("Couldn't create worker");

    // Prepare input.
    let vec = vec![8, 1, 5, 0, 4];
    let vec = vec.into();

    // Test try run.
    let res1 = worker.try_run_bytes(webworker!(sort), &vec).await;
    if res1.is_ok() {
        wasm_bindgen::throw_str("Should not be able to obtain permit")
    }
}

pub(crate) async fn can_schedule_task_bytes() {
    let pool = worker_pool().await;

    // Prepare input and output.
    let vec = vec![8, 1, 5, 0, 4];
    let mut sorted_vec = vec.clone();
    sorted_vec.sort();
    let vec = vec.into();
    let sorted_vec = sorted_vec.into();

    // Test run.
    let res2 = pool.run_bytes(webworker!(sort), &vec).await;
    js_assert_eq!(res2, sorted_vec);
}

/// Test that a worker pool with precompiled WASM works correctly.
/// This also tests the path_bg option indirectly.
pub(crate) async fn can_use_precompiled_wasm() {
    // Create a pool with precompiled WASM
    let pool = WebWorkerPool::with_precompiled_wasm()
        .await
        .expect("Couldn't create pool with precompiled WASM");

    // Prepare input and output.
    let vec = vec![8, 1, 5, 0, 4];
    let mut sorted_vec = vec.clone();
    sorted_vec.sort();
    let vec: Box<[u8]> = vec.into();
    let sorted_vec: Box<[u8]> = sorted_vec.into();

    // Test run.
    let res = pool.run_bytes(webworker!(sort), &vec).await;
    js_assert_eq!(res, sorted_vec, "Precompiled WASM run failed");
}

/// Test that custom WorkerPoolOptions work.
pub(crate) async fn can_use_custom_pool_options() {
    let mut options = WorkerPoolOptions::new();
    options.num_workers = Some(2);

    let pool = WebWorkerPool::with_options(options)
        .await
        .expect("Couldn't create pool with custom options");

    js_assert_eq!(pool.num_workers(), 2, "Should have 2 workers");

    // Prepare input and output.
    let vec = vec![8, 1, 5, 0, 4];
    let mut sorted_vec = vec.clone();
    sorted_vec.sort();
    let vec: Box<[u8]> = vec.into();
    let sorted_vec: Box<[u8]> = sorted_vec.into();

    // Test run.
    let res = pool.run_bytes(webworker!(sort), &vec).await;
    js_assert_eq!(res, sorted_vec, "Custom options run failed");
}

/// Test that idle timeout terminates workers and transparently recreates them.
pub(crate) async fn can_use_idle_timeout() {
    let mut options = WorkerPoolOptions::new();
    options.num_workers = Some(2);
    options.idle_timeout_ms = Some(300);

    let pool = WebWorkerPool::with_options(options)
        .await
        .expect("Couldn't create pool with idle timeout");

    // All workers should be active after creation.
    js_assert_eq!(
        pool.num_active_workers(),
        2,
        "Should start with 2 active workers"
    );

    // Run a task to verify workers work.
    let vec: Box<[u8]> = vec![3, 1, 2].into();
    let sorted: Box<[u8]> = vec![1, 2, 3].into();
    let res = pool.run_bytes(webworker!(sort), &vec).await;
    js_assert_eq!(res, sorted, "Task should succeed");

    // Poll until all workers are idle-terminated (avoids flakiness from browser timer throttling).
    let deadline = js_sys::Date::now() + 10_000.0;
    while pool.num_active_workers() > 0 && js_sys::Date::now() < deadline {
        sleep_ms(50).await;
    }

    // Workers should be terminated.
    js_assert_eq!(
        pool.num_active_workers(),
        0,
        "Workers should be terminated after idle"
    );

    // Run another task — should transparently recreate a worker.
    let res = pool.run_bytes(webworker!(sort), &vec).await;
    js_assert_eq!(res, sorted, "Task should succeed after recreation");

    // At least one worker should be active now.
    js_assert_eq!(
        pool.num_active_workers() >= 1,
        true,
        "Should have at least one active worker after recreation"
    );
}
