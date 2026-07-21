//! Regression test for conflicting message handlers in workers.
//!
//! The `#[wasm_bindgen(start)]` function below installs hostile message
//! handlers in every worker (start functions run during `mod.default()`),
//! both via the `onmessage` property and via `addEventListener`. Since all
//! wasmworker traffic runs over a dedicated `MessageChannel` port, these
//! handlers must never fire, and messages the module posts on the global
//! scope must never reach wasmworker's response callback.

use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsCast, JsValue, UnwrapThrowExt};
use wasmworker::{webworker, webworker_fn, WebWorker, WebWorkerPool};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

use crate::{js_assert_eq, raw::sort};

/// Runs on every module initialization, both on the main page and inside
/// workers. Inside workers, install hostile message handlers on the global
/// scope: if one of them ever fires, it posts a message to the main thread
/// that cannot be deserialized as a task response.
#[wasm_bindgen(start)]
fn register_conflicting_handlers() {
    let Ok(scope) = js_sys::global().dyn_into::<DedicatedWorkerGlobalScope>() else {
        // Main page: `self` is a `Window`, nothing to do.
        return;
    };

    // Via the `onmessage` property, as e.g. wasm-bindgen glue may set it.
    let post = scope.clone();
    let handler = Closure::<dyn FnMut(MessageEvent)>::new(move |_: MessageEvent| {
        let _ = post.post_message(&JsValue::from_str("conflicting onmessage fired"));
    });
    scope.set_onmessage(Some(handler.as_ref().unchecked_ref()));
    handler.forget();

    // Via `addEventListener`.
    let post = scope.clone();
    let listener = Closure::<dyn FnMut(MessageEvent)>::new(move |_: MessageEvent| {
        let _ = post.post_message(&JsValue::from_str("conflicting listener fired"));
    });
    scope
        .add_event_listener_with_callback("message", listener.as_ref().unchecked_ref())
        .expect_throw("Could not add hostile listener");
    listener.forget();
}

/// Runs inside a worker: posts garbage on the global scope (as a module might
/// do for its own purposes) and returns its argument sorted.
#[webworker_fn]
fn post_garbage(mut v: Box<[u8]>) -> Box<[u8]> {
    js_sys::global()
        .dyn_into::<DedicatedWorkerGlobalScope>()
        .expect_throw("Not in a worker")
        .post_message(&JsValue::from_str("garbage from module code"))
        .expect_throw("Could not post garbage");
    v.sort();
    v
}

/// Hostile message handlers registered by a `#[wasm_bindgen(start)]` function
/// must not break task dispatch, and module messages posted on the global
/// scope must not reach wasmworker's response callback. Covers both worker
/// blobs: the regular one (via [`WebWorker`]) and the precompiled-WASM one
/// (via [`WebWorkerPool::with_precompiled_wasm`]).
pub(crate) async fn can_run_task_with_conflicting_onmessage() {
    let vec: Box<[u8]> = vec![8, 1, 5, 0, 4].into();
    let sorted: Box<[u8]> = vec![0, 1, 4, 5, 8].into();

    // Regular worker blob.
    let worker = WebWorker::new(None).await.expect("Couldn't create worker");

    let res = worker.run_bytes(webworker!(sort), &vec).await;
    js_assert_eq!(
        res,
        sorted,
        "Task should complete despite conflicting handlers"
    );

    // Worker -> main direction: garbage on the global scope must not disturb
    // the response path.
    let res = worker.run_bytes(webworker!(post_garbage), &vec).await;
    js_assert_eq!(res, sorted, "Task posting garbage should complete");

    let res = worker.run_bytes(webworker!(sort), &vec).await;
    js_assert_eq!(res, sorted, "Tasks should still work after posted garbage");

    // Precompiled WASM blob.
    let pool = WebWorkerPool::with_precompiled_wasm()
        .await
        .expect("Couldn't create pool with precompiled WASM");

    let res = pool.run_bytes(webworker!(sort), &vec).await;
    js_assert_eq!(
        res,
        sorted,
        "Task should complete despite conflicting handlers (precompiled)"
    );

    let res = pool.run_bytes(webworker!(post_garbage), &vec).await;
    js_assert_eq!(
        res,
        sorted,
        "Task posting garbage should complete (precompiled)"
    );

    let res = pool.run_bytes(webworker!(sort), &vec).await;
    js_assert_eq!(
        res,
        sorted,
        "Tasks should still work after posted garbage (precompiled)"
    );
}
