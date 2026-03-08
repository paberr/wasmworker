# wasmworker

[![Crates.io](https://img.shields.io/crates/v/wasmworker)](https://crates.io/crates/wasmworker)
[![docs.rs](https://img.shields.io/docsrs/wasmworker)](https://docs.rs/wasmworker)
[![CI](https://github.com/paberr/wasmworker/actions/workflows/test.yml/badge.svg)](https://github.com/paberr/wasmworker/actions/workflows/test.yml)
[![Crates.io Downloads](https://img.shields.io/crates/d/wasmworker)](https://crates.io/crates/wasmworker)
[![License](https://img.shields.io/crates/l/wasmworker)](https://github.com/paberr/wasmworker#license)

`wasmworker` is a library that provides easy access to parallelization on web targets when compiled to WebAssembly using [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen).
In contrast to many other libraries like [wasm-bindgen-rayon](https://github.com/RReverser/wasm-bindgen-rayon), this library does not require SharedArrayBuffer support.

- [Usage](#usage)
  - [Setting up](#setting-up)
    - [Serialization codec](#serialization-codec)
  - [Outsourcing tasks](#outsourcing-tasks)
    - [WebWorker](#webworker)
    - [WebWorkerPool](#webworkerpool)
    - [Iterator extension](#iterator-extension)
    - [Async functions with channels](#async-functions-with-channels)
  - [Bundler support (Vite)](#bundler-support-vite)
  - [Idle timeout](#idle-timeout)
- [FAQ](#faq)

## Usage

### Setting up
To use this library, add the following dependency to your `Cargo.toml`.
Enable the `macros` feature to get access to the `#[webworker_fn]` and `#[webworker_channel_fn]` attribute macros.

```toml
[dependencies]
wasmworker = { version = "0.3", features = ["macros"] }
```

Function arguments and return types must implement `serde::Serialize + serde::Deserialize<'de>`.
Alternatively, functions with the type `fn(Box<[u8]>) -> Box<[u8]>` can be used via `run_bytes()` for manual serialization.

The `iter-ext` feature (enabled by default) adds the `par_map` and `try_par_map` iterator extensions for convenient parallel map operations on the default worker pool.

#### Serialization codec
By default, `wasmworker` uses [postcard](https://crates.io/crates/postcard) for internal serialization.
Postcard is compact and fast, making it ideal for the typical WebWorker use case (passing `Vec<T>`, structs, primitives).

For complex types like `Rc<T>` or cyclic structures, you can use [pot](https://crates.io/crates/pot) instead.
Note that pot has significantly higher serialization overhead and larger output sizes, so it should only be used when postcard cannot handle your data types.

```toml
[dependencies]
wasmworker = { version = "0.3", default-features = false, features = ["iter-ext", "macros", "codec-pot"] }
```

You can then start using the library without further setup.
If you plan on using the global `WebWorkerPool` (using the iterator extensions or `worker_pool()`), you can *optionally* configure this pool:
```rust
// Importing it publicly will also expose the function on the JavaScript side.
// You can instantiate the pool both via Rust and JS.
pub use wasmworker::{init_worker_pool, WorkerPoolOptions};

async fn startup() {
    let mut options = WorkerPoolOptions::new();
    options.num_workers = Some(2); // Default is navigator.hardwareConcurrency
    init_worker_pool(options).await.expect("Worker pool already initialized");
}
```

### Outsourcing tasks
The library offers three ways of outsourcing function calls onto concurrent workers:
1. `WebWorker`: a single worker, to which tasks can be queued to.
2. `WebWorkerPool`: a pool of multiple workers, to which tasks are distributed.
3. `par_map`: an extension to regular iterators, which allows to execute a function on every element of the iterator in parallel using the default worker pool.

All approaches require the functions that should be executed to be annotated with the `#[webworker_fn]` macro.
This macro ensures that the functions are available to the web worker instances.
To execute such a function, pass its `WebWorkerFn` handle (obtained via the `webworker!()` macro) to a worker:

```rust
use serde::{Deserialize, Serialize};
use wasmworker::{webworker, webworker_fn};

/// An arbitrary type that is (de)serializable.
#[derive(Serialize, Deserialize)]
pub struct VecType(Vec<u8>);

/// A sort function on a custom type.
#[webworker_fn]
pub fn sort_vec(mut v: VecType) -> VecType {
    v.0.sort();
    v
}

// Obtain a type-safe handle to the function:
let ww_sort = webworker!(sort_vec);
```

#### WebWorker
We can instantiate our own workers and run functions on them:
```rust
use serde::{Deserialize, Serialize};
use wasmworker::{webworker, webworker_fn, WebWorker};

#[derive(Serialize, Deserialize)]
pub struct VecType(Vec<u8>);

#[webworker_fn]
pub fn sort_vec(mut v: VecType) -> VecType {
    v.0.sort();
    v
}

let worker = WebWorker::new(None).await.expect("Couldn't create worker");
let sorted = worker.run(webworker!(sort_vec), &VecType(vec![3, 1, 2])).await;
assert_eq!(sorted.0, vec![1, 2, 3]);
```

#### WebWorkerPool
Most of the time, we probably want to schedule tasks to a pool of workers, though.
The default worker pool is instantiated on first use and can be configured using `init_worker_pool()` as described above.
It uses a round-robin scheduler (with the second option being a load based scheduler), a number of `navigator.hardwareConcurrency` separate workers, and the default inferred path.

```rust
use serde::{Deserialize, Serialize};
use wasmworker::{webworker, webworker_fn, worker_pool};

#[derive(Serialize, Deserialize)]
pub struct VecType(Vec<u8>);

#[webworker_fn]
pub fn sort_vec(mut v: VecType) -> VecType {
    v.0.sort();
    v
}

let worker_pool = worker_pool().await;
let sorted = worker_pool.run(webworker!(sort_vec), &VecType(vec![3, 1, 2])).await;
assert_eq!(sorted.0, vec![1, 2, 3]);
```

#### Iterator extension
Inspired by [Rayon](https://github.com/rayon-rs/rayon), this library also offers a (much simpler and less powerful) method for iterators.
This functionality automatically parallelizes a map operation on the default worker pool.

```rust,ignore
use wasmworker::iter_ext::IteratorExt;

let some_vec = vec![
    VecType(vec![5, 2, 8]),
    // ...
];
let res: Vec<VecType> = some_vec.iter().par_map(webworker!(sort_vec)).await;
```

#### Async functions with channels
For more complex use cases like progress reporting or interactive workflows, you can use async functions with bidirectional channel support.

First, define an async function with the `#[webworker_channel_fn]` macro:

```rust,ignore
use wasmworker::Channel;
use wasmworker::webworker_channel_fn;

#[derive(Serialize, Deserialize)]
pub struct Progress { pub percent: u8 }

#[derive(Serialize, Deserialize)]
pub struct Continue { pub should_continue: bool }

#[webworker_channel_fn]
pub async fn process_with_progress(data: Vec<u8>, channel: Channel) -> ProcessResult {
   // Report progress to main thread
   channel.send(&Progress { percent: 50 });

   // Wait for response from main thread
   let response: Option<Continue> = channel.recv().await;
   if let Some(cont) = response {
      if !cont.should_continue {
            return ProcessResult { was_cancelled: true, .. };
      }
   }

   // Continue processing...
   ProcessResult { was_cancelled: false, .. }
}
```

Then use the `webworker_channel!` macro and `run_channel` method:

```rust,ignore
use wasmworker::{webworker_channel, WebWorker};

let worker = WebWorker::new(None).await?;

// Start the async task — returns a ChannelTask for communication + result
let task = worker
   .run_channel(webworker_channel!(process_with_progress), &data)
   .await;

// Receive progress from worker
let progress: Progress = task.recv().await.unwrap();

// Send response back to worker
task.send(&Continue { should_continue: true });

// Wait for task completion
let result = task.result().await;
```

### Bundler support (Vite)
The recommended approach for Vite is to place the wasm-pack output in Vite's `publicDir`.
This keeps the glue code and WASM binary as static assets, which is required because each
WebWorker needs to import the glue code independently via `import()`.

A working example is in `test/vite-app/`.

**1. Build with wasm-pack:**
```sh
wasm-pack build --target web --out-name myapp --out-dir my-vite-app/pkg
```

**2. Configure Vite** (`vite.config.js`):
```js
import { defineConfig } from 'vite';

export default defineConfig({
  base: './',
  build: {
    target: 'esnext',
    rollupOptions: {
      // Don't try to bundle the wasm-pack glue code
      external: [/\.\/myapp\.js$/],
    },
  },
  // Serve the wasm-pack output as static assets (not bundled)
  publicDir: 'pkg',
});
```

**3. Load in your entry point** (`index.js`):
```js
// Dynamic import with @vite-ignore to skip Vite's module resolution
const { default: init, /* your exports */ } = await import(/* @vite-ignore */ './myapp.js');
await init();
```

No Rust-side changes are needed — `import.meta.url` resolves correctly when the glue code is served as a static asset.

#### Advanced: custom paths

If your build setup places the wasm-bindgen glue or WASM binary at non-standard locations
(e.g., hashed filenames, nested directories), you can override the paths explicitly:

```rust
use wasmworker::{init_worker_pool, WorkerPoolOptions};

let mut options = WorkerPoolOptions::new();
// Path to the wasm-bindgen glue file (used by worker blob's import())
options.path = Some("/assets/myapp.js".to_string());
// Path to the WASM binary (passed to wasm-bindgen's init function)
options.path_bg = Some("/assets/myapp_bg.wasm".to_string());
init_worker_pool(options).await.unwrap();
```

#### Precompiling WASM

To reduce bandwidth (fetch WASM once instead of once per worker), you can precompile and share the module:

```rust
use wasmworker::{init_worker_pool, WorkerPoolOptions};

let mut options = WorkerPoolOptions::new();
options.precompile_wasm = Some(true);
init_worker_pool(options).await.unwrap();
```

### Idle timeout

Workers can be automatically terminated after a period of inactivity and transparently recreated when new tasks arrive. This is useful for freeing resources in applications where worker usage is intermittent:

```rust
use wasmworker::{init_worker_pool, WorkerPoolOptions};

let mut options = WorkerPoolOptions::new();
options.idle_timeout_ms = Some(5000); // Terminate idle workers after 5 seconds
init_worker_pool(options).await.unwrap();
```

You can inspect the pool state using `num_active_workers()` to see how many workers are currently alive.

## FAQ
1. _Why would you not want to use SharedArrayBuffers?_

   The use of SharedArrayBuffers requires cross-origin policy headers to be set, which is not possible in every environment.
   Moreover, most libraries that rely on SharedArrayBuffers, also require a nightly version of Rust at the moment.
   An important goal of this library is to remove these requirements.

2. _Which `wasm-bindgen` targets are supported?_

   So far, this library has only been tested with `--target web`.
   Other targets seem to generally be problematic in that the wasm glue is inaccessible or paths are not correct.
   Both the `Worker` and `WebWorkerPool` have an option to set a custom path, which should make it possible to support other targets dynamically, though.

3. _Can I use bundlers?_

   Yes! Vite is tested and supported. The recommended approach is to serve the wasm-pack
   output as static assets (via Vite's `publicDir`) rather than bundling it. This ensures
   each WebWorker can import the glue code independently.
   See [Bundler support (Vite)](#bundler-support-vite) for a step-by-step guide
   and `test/vite-app/` for a working example.
