use js_sys::wasm_bindgen::JsValue;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("WebWorker capacity reached")]
pub struct Full;

#[derive(Debug, Error)]
pub enum InitError {
    #[error("WebWorker creation error: {0:?}")]
    WebWorkerCreation(JsValue),
    #[error("WebWorker module loading error: {0:?}")]
    WebWorkerModuleLoading(String),
}
