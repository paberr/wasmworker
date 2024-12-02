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

        const worker_result = await fn(arg);

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
        #[wasm_bindgen(thread_local, js_namespace = ["import", "meta"], js_name = url)]
        static URL: JsString;
    }

    URL.with(Clone::clone)
}
