use send_wrapper::SendWrapper;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, UnwrapThrowExt};
use wasmworker::{iter_ext::IteratorExt, webworker, worker_pool, WebWorker};
use wasmworker_proc_macro::webworker_fn;
use web_sys::{HtmlElement, HtmlInputElement};

/// A wrapper type to demonstrate serde functionality.
#[derive(Serialize, Deserialize)]
pub struct VecType(Vec<u8>);

/// A sort function on a custom type.
#[webworker_fn]
pub fn sort_vec(mut v: VecType) -> VecType {
    v.0.sort();
    v
}

/// Initialises a worker.
async fn worker() -> &'static WebWorker {
    static WORKER: OnceCell<SendWrapper<WebWorker>> = OnceCell::const_new();
    WORKER
        .get_or_init(move || async {
            SendWrapper::new(
                WebWorker::with_path(None, None)
                    .await
                    .expect_throw("Couldn't instantiate WebWorker"),
            )
        })
        .await
}

/// Run task on a simple worker.
#[wasm_bindgen(js_name = runWorker)]
pub async fn run_worker() {
    let num_values = get_input();
    let mut values = VecType(vec![]);
    for _ in 0..num_values {
        values.0.push(rand::random());
    }

    let worker = worker().await;

    // Run sorting.
    let res = worker.run(webworker!(sort_vec), &values).await;

    set_result(&format!("{:?} -> {:?}", values.0, res.0));
}

/// Demonstrate worker pool functionality.
#[wasm_bindgen(js_name = runPool)]
pub async fn run_pool() {
    let num_values = get_input();
    let mut values = VecType(vec![]);
    for _ in 0..num_values {
        values.0.push(rand::random());
    }

    let pool = worker_pool().await;

    // Run sorting.
    let res = pool.run(webworker!(sort_vec), &values).await;

    set_result(&format!("{:?} -> {:?}", values.0, res.0));
}

/// Demonstrate iterator extensions.
#[wasm_bindgen(js_name = runParMap)]
pub async fn run_par_map() {
    let num_values = get_input();
    let mut vecs = vec![];
    for _ in 0..10 {
        let mut values: Vec<u8> = vec![];
        for _ in 0..num_values {
            values.push(rand::random());
        }
        vecs.push(VecType(values));
    }

    // Run `par_map`.
    let sorted_vecs = vecs.iter().par_map(webworker!(sort_vec)).await;

    // Build result.
    let mut text = String::new();
    for (vec, sorted) in vecs.iter().zip(sorted_vecs.iter()) {
        text.push_str(&format!("{:?} -> {:?}<br>", vec.0, sorted.0));
    }

    set_result(&text);
}

/// Read input.
fn get_input() -> usize {
    let document = web_sys::window().unwrap().document().unwrap();
    let input_field = document
        .get_element_by_id("num_values")
        .expect("#num_keys should exist");
    let input_field = input_field
        .dyn_ref::<HtmlInputElement>()
        .expect("#num_keys should be a HtmlInputElement");
    input_field.value().parse::<usize>().unwrap_or(1)
}

/// Set output.
fn set_result(s: &str) {
    let document = web_sys::window().unwrap().document().unwrap();
    let result_field = document
        .get_element_by_id("result")
        .expect("#result should exist");
    let result_field = result_field
        .dyn_ref::<HtmlElement>()
        .expect("#result should be a HtmlInputElement");
    result_field.set_inner_html(s);
}
