/* tslint:disable */
/* eslint-disable */
/**
 * @param {Uint8Array} arg
 * @returns {Uint8Array}
 */
export function __webworker_sort_vec(arg: Uint8Array): Uint8Array;
/**
 * Run task on a simple worker.
 * @returns {Promise<void>}
 */
export function runWorker(): Promise<void>;
/**
 * Demonstrate worker pool functionality.
 * @returns {Promise<void>}
 */
export function runPool(): Promise<void>;
/**
 * Demonstrate iterator extensions.
 * @returns {Promise<void>}
 */
export function runParMap(): Promise<void>;
/**
 * This function can be called before the first use of the global worker pool to configure it.
 * It takes a [`WorkerPoolOptions`] configuration object. Note that this function is async.
 *
 * ```ignore
 * # use wasmworker::{init_worker_pool, WorkerPoolOptions};
 * init_worker_pool(WorkerPoolOptions {
 *     num_workers: Some(2),
 *     ..Default::default()
 * }).await
 * ```
 *
 * This function can also be called from JavaScript:
 * ```js
 * // Make sure to use the correct path.
 * import init, { initWorkerPool, WorkerPoolOptions } from "./pkg/wasmworker_demo.js";
 *
 * await init();
 * let options = new WorkerPoolOptions();
 * options.num_workers = 3;
 * await initWorkerPool(options);
 * ```
 * @param {WorkerPoolOptions} options
 * @returns {Promise<void>}
 */
export function initWorkerPool(options: WorkerPoolOptions): Promise<void>;
/**
 * This enumeration contains the supported strategies for distributing
 * tasks within the worker pool.
 *
 * If re-exported, the strategy can also be accessed from JavaScript.
 * Rust:
 * ```rust
 * pub use wasmworker::pool::Strategy;
 * ```
 */
export enum Strategy {
  /**
   * The round-robin strategy will allocate tasks in a round-robin fashion
   * to the workers in the pool.
   */
  RoundRobin = 0,
  /**
   * The load-based strategy will allocate a task always to the worker with
   * the lowest number of tasks already scheduled.
   * If more than one worker has the same number of tasks scheduled, the first
   * one is chosen.
   */
  LoadBased = 1,
}
/**
 * This struct can be used to configure all options of the [`WebWorkerPool`].
 *
 * If re-exported, the struct can also be accessed via JavaScript:
 * ```js
 * let options = new WorkerPoolOptions();
 * options.num_workers = 3;
 * ```
 */
export class WorkerPoolOptions {
  free(): void;
  /**
   * Creates the default options.
   */
  constructor();
/**
 * The number of workers that will be spawned. This defaults to `navigator.hardwareConcurrency`.
 */
  num_workers?: number;
/**
 * The path to the wasm-bindgen glue. By default, this path is inferred.
 * [`crate::WebWorker::with_path`] lists more details on when this path
 * should be manually configured.
 */
  path?: string;
/**
 * The strategy to be used by the worker pool.
 */
  strategy?: Strategy;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __webworker_sort_vec: (a: number, b: number, c: number) => void;
  readonly runWorker: () => number;
  readonly runPool: () => number;
  readonly runParMap: () => number;
  readonly initWorkerPool: (a: number) => number;
  readonly __wbg_workerpooloptions_free: (a: number, b: number) => void;
  readonly __wbg_get_workerpooloptions_path: (a: number, b: number) => void;
  readonly __wbg_set_workerpooloptions_path: (a: number, b: number, c: number) => void;
  readonly __wbg_get_workerpooloptions_strategy: (a: number) => number;
  readonly __wbg_set_workerpooloptions_strategy: (a: number, b: number) => void;
  readonly __wbg_get_workerpooloptions_num_workers: (a: number, b: number) => void;
  readonly __wbg_set_workerpooloptions_num_workers: (a: number, b: number, c: number) => void;
  readonly workerpooloptions_new: () => number;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hb3f0da974b301f4e: (a: number, b: number, c: number) => void;
  readonly _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h1ed3f61ce670585a: (a: number, b: number, c: number) => void;
  readonly _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h36e1010f860e4e87: (a: number, b: number, c: number) => void;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly wasm_bindgen__convert__closures__invoke2_mut__h6d1a51fb0f116f48: (a: number, b: number, c: number, d: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
