use js_sys::JsString;
use wasm_bindgen::prelude::wasm_bindgen;

/// The initialization code for the worker,
/// which will be loaded as a blob.
///
/// `{{wasm}}` will be replaced later by an actual path.
pub(crate) const WORKER_JS: &str = r#"
console.debug('Initializing worker');

(async () => {
    let mod;
    try {
        mod = await import('{{wasm}}');
    } catch (e) {
        console.error('Unable to import module {{wasm}}', e);
        self.postMessage({ success: false, message: e.toString() });
        return;
    }

    await mod.default({{wasm_bg}});
    self.postMessage({ success: true });
    console.debug('Worker started');

    self.addEventListener('message', async event => {
        console.debug('Received worker event');
        const { id, func_name, arg } = event.data;

        const webworker_func_name = `__webworker_${func_name}`;
        const fn = mod[webworker_func_name];
        if (!fn) {
            console.error(`Function '${func_name}' is not exported.`);
            self.postMessage({ id: id, response: null });
            return;
        }

        const worker_result = await fn(arg, event.ports[0]);

        // Send response back to be handled by callback in main thread.
        console.debug('Send worker result');
        self.postMessage({ id: id, response: worker_result });
    });
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

/// The initialization code for workers that receive a pre-compiled WASM module
pub(crate) const WORKER_JS_WITH_PRECOMPILED: &str = r#"
console.debug('Initializing worker with pre-compiled WASM');

let wasmModule = null;
let mod = null;
let initHandler = null;

// Listen for the pre-compiled WASM module
initHandler = async function(event) {
    const data = event.data;

    if (data.type === 'wasm_module') {
        console.debug('Received pre-compiled WASM module');
        wasmModule = data.module;

        // Now initialize with the pre-compiled module
        try {
            mod = await import('{{wasm}}');
            await mod.default({ module_or_path: wasmModule });
            self.postMessage({ success: true });
            console.debug('Worker started with pre-compiled WASM');
        } catch (e) {
            console.error('Unable to initialize with pre-compiled WASM', e);
            self.postMessage({ success: false, message: e.toString() });
            return;
        }

        // Remove this listener and add the task handler
        self.removeEventListener('message', initHandler);

        // Add the main message handler for tasks
        self.addEventListener('message', async event => {
            console.debug('Received worker event');
            const { id, func_name, arg } = event.data;

            const webworker_func_name = `__webworker_${func_name}`;
            const fn = mod[webworker_func_name];
            if (!fn) {
                console.error(`Function '${func_name}' is not exported.`);
                self.postMessage({ id: id, response: null });
                return;
            }

            const worker_result = await fn(arg, event.ports[0]);

            // Send response back to be handled by callback in main thread.
            console.debug('Send worker result');
            self.postMessage({ id: id, response: worker_result });
        });
    }
};

self.addEventListener('message', initHandler);
"#;
