# wasmworker
`wasmworker` is a library that provides easy access to parallelization on web targets when compiled to WebAssembly using [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen).
In contrast to many other libraries like [wasm-bindgen-rayon](https://github.com/RReverser/wasm-bindgen-rayon), this library does not require SharedArrayBuffer support.

- [Usage](#usage)
  - [Setting up](#setting-up)
  - [Outsourcing tasks](#outsourcing-tasks)
    - [WebWorker](#webworker)
    - [WorkerPool](#workerpool)
    - [Iterator extension](#iterator-extension)
  - [Feature detection](#feature-detection)
- [FAQ](#faq)

## Usage
The library consists of two crates:
- `wasmworker`: The main crate that also offers access to the webworker, as well as the worker pool and iterator extensions.
- `wasmworker-proc-macro`: This crate is needed to expose functions towards the web workers via the `#[webworker_fn]` macro.

### Setting up
To use this library, include both dependencies to your `Cargo.toml`.

```toml
[dependencies]
wasmworker = { git = "github.com/paberr/wasmworker" }
wasmworker-proc-macro = { git = "github.com/paberr/wasmworker" }
```

The `wasmworker` crate comes with a default feature called `serde`, which allows running any function on a web worker under the following two conditions:
1. The function takes a single argument, which implements `serde::Serialize + serde::Deserialize<'de>`.
2. The return type implements `serde::Serialize + serde::Deserialize<'de>`.
Without the `serde` feature, only functions with the type `fn(Box<[u8]>) -> Box<[u8]>` can be run on a worker.
This is useful for users that do not want a direct serde dependency. Internally, the library always uses serde, though.

You can then start using the library without further setup.
If you plan on using the global `WorkerPool` (using the iterator extensions or `worker_pool()`), you can *optionally* configure this pool:
```rust
// Importing it publicly will also expose the function on the JavaScript side.
// You can instantiate the pool both via Rust and JS.
pub use wasmworker::init_worker_pool;

async fn startup() {
    init_worker_pool(WorkerPoolOptions {
        num_workers: Some(2), // Default is navigator.hardwareConcurrency
        ..Default::default()
    }).await;
}
```

### Outsourcing tasks
The library offers three ways of outsourcing function calls onto concurrent workers:
1. `WebWorker`: a single worker, to which tasks can be queued to.
2. `WorkerPool`: a pool of multiple workers, to which tasks are distributed.
3. `par_map`: an extension to regular iterators, which allows to execute a function on every element of the iterator in parallel using the default worker pool.

All approaches require the functions that should be executed to be annotated with the `#[webworker_fn]` macro.
This macro ensures that the functions are available to the web worker instances:

```rust
use serde::{Deserialize, Serialize};
use wasmworker_proc_macro::webworker_fn;

/// An arbitrary type that is (de)serializable.
#[derive(Serialize, Deserialize)]
pub struct VecType(Vec<u8>);

/// A sort function on a custom type.
#[webworker_fn]
pub fn sort_vec(mut v: VecType) -> VecType {
    v.0.sort();
    v
}
```

Whenever we want to execute a function, we need to pass the corresponding `WebWorkerFn` object to the worker.
This object describes the function to the worker and can be safely obtained via the `webworker!()` macro:

```rust
use wasmworker::webworker;

let ww_sort = webworker!(sort_vec);
```

#### WebWorker
We can instantiate our own workers and run functions on them:
```rust
use wasmworker::{webworker, WebWorker};

let worker = WebWorker::new(None).await;
let res = worker.run(webworker!(sort_vec), &VecType(vec![5, 2, 8])).await;
assert_eq!(res.0, vec![2, 5, 8]);
```

#### WorkerPool
Most of the time, we probably want to schedule tasks to a pool of workers, though.
The default worker pool is instantiated on first use and can be configured using `init_worker_pool()` as described above.
It uses a round-robin scheduler (with the second option being a load based scheduler), a number of `navigator.hardwareConcurrency` separate workers, and the default inferred path.

```rust
use wasmworker::{webworker, worker_pool};

let worker_pool = worker_pool().await;
let res = worker_pool.run(webworker!(sort_vec), &VecType(vec![5, 2, 8])).await;
assert_eq!(res.0, vec![2, 5, 8]);
```

#### Iterator extension
Inspired by [Rayon](https://github.com/rayon-rs/rayon), this library also offers a (much simpler and less powerful) method for iterators.
This functionality automatically parallelizes a map operation on the default worker pool.

```rust
use wasmworker::iter_ext::IteratorExt;

let some_vec = vec![
    VecType(vec![5, 2, 8]),
    // ...
];
let res: Vec<VecType> = some_vec.iter().par_map(webworker!(sort_vec)).await;
```

## FAQ
1. _Why would you not want to use SharedArrayBuffers?_

    The use of SharedArrayBuffers requires cross-origin policy headers to be set, which is not possible in every environment.
    Moreover, most libraries that rely on SharedArrayBuffers, also require a nightly version of Rust at the moment.
    An important goal of this library is to remove these requirements.

2. _Which `wasm-bindgen` targets are supported?_

    So far, this library has only been tested with `--target web`.
    Other targets seem to generally be problematic in that the wasm glue is inaccessible or paths are not correct.
    Both the `Worker` and `WorkerPool` have an option to set a custom path, which should make it possible to support other targets dynamically, though.

3. _Can I use bundlers?_

    The usage of bundlers has not been officially tested. This might be added in the future.
