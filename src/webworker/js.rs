use js_sys::JsString;
use wasm_bindgen::prelude::wasm_bindgen;

/// The initialization code for the worker,
/// which will be loaded as a blob.
///
/// All wasmworker traffic (init handshake and task dispatch) runs over a
/// dedicated `MessageChannel` port, which is transferred with the first
/// message to the worker. This keeps the worker's global message channel
/// free for the embedded module, so message handlers installed by module
/// code (e.g. in a `#[wasm_bindgen(start)]` function) never interfere
/// with task dispatch, and messages posted by the module on the global
/// scope never reach wasmworker's response callback.
///
/// `{{wasm}}` will be replaced later by an actual path.
pub(crate) const WORKER_JS: &str = r#"
console.debug('Initializing worker');

// Capture the dedicated task port before any module code can run.
const portPromise = new Promise(resolve => {
    const initListener = event => {
        if (event.data && event.data.type === 'init_port') {
            self.removeEventListener('message', initListener);
            resolve(event.ports[0]);
        }
    };
    self.addEventListener('message', initListener);
});

(async () => {
    const port = await portPromise;

    let mod;
    try {
        mod = await import('{{wasm}}');
    } catch (e) {
        console.error('Unable to import module {{wasm}}', e);
        port.postMessage({ success: false, message: e.toString() });
        return;
    }

    await mod.default({{wasm_bg}});
    port.postMessage({ success: true });
    console.debug('Worker started');

    port.onmessage = async event => {
        console.debug('Received worker event');
        const { id, func_name, is_channel, arg } = event.data;

        const prefix = is_channel ? '__webworker_channel_' : '__webworker_';
        const webworker_func_name = `${prefix}${func_name}`;
        const fn = mod[webworker_func_name];
        if (!fn) {
            console.error(`Function '${func_name}' is not exported.`);
            port.postMessage({ id: id, response: null });
            return;
        }

        const worker_result = await fn(arg, event.ports[0]);

        // Send response back to be handled by callback in main thread.
        console.debug('Send worker result');
        port.postMessage({ id: id, response: worker_result });
    };
})();
"#;

/// This function normally returns the path of our wasm-bindgen glue file.
/// It only works in module environments, though.
pub(crate) fn main_js() -> JsString {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(thread_local_v2, js_namespace = ["import", "meta"], js_name = url)]
        static URL: JsString;
    }

    URL.with(Clone::clone)
}

/// The initialization code for workers that receive a pre-compiled WASM module.
///
/// Like [`WORKER_JS`], all wasmworker traffic runs over a dedicated
/// `MessageChannel` port, which arrives with the `wasm_module` init message.
pub(crate) const WORKER_JS_WITH_PRECOMPILED: &str = r#"
console.debug('Initializing worker with pre-compiled WASM');

let mod = null;
let initHandler = null;

// Listen for the pre-compiled WASM module and the dedicated task port
initHandler = async function(event) {
    const data = event.data;

    if (data.type === 'wasm_module') {
        console.debug('Received pre-compiled WASM module');
        const port = event.ports[0];

        // Remove this listener before running module code, so wasmworker
        // no longer listens on the global scope at all.
        self.removeEventListener('message', initHandler);

        // Now initialize with the pre-compiled module
        try {
            mod = await import('{{wasm}}');
            await mod.default({ module_or_path: data.module });
            port.postMessage({ success: true });
            console.debug('Worker started with pre-compiled WASM');
        } catch (e) {
            console.error('Unable to initialize with pre-compiled WASM', e);
            port.postMessage({ success: false, message: e.toString() });
            return;
        }

        // Add the main message handler for tasks
        port.onmessage = async event => {
            console.debug('Received worker event');
            const { id, func_name, is_channel, arg } = event.data;

            const prefix = is_channel ? '__webworker_channel_' : '__webworker_';
            const webworker_func_name = `${prefix}${func_name}`;
            const fn = mod[webworker_func_name];
            if (!fn) {
                console.error(`Function '${func_name}' is not exported.`);
                port.postMessage({ id: id, response: null });
                return;
            }

            const worker_result = await fn(arg, event.ports[0]);

            // Send response back to be handled by callback in main thread.
            console.debug('Send worker result');
            port.postMessage({ id: id, response: worker_result });
        };
    }
};

self.addEventListener('message', initHandler);
"#;
