use serde::{Deserialize, Serialize};

/// This wrapper function encapsules our internal serialization format.
/// It is used internally to prepare values before sending them to a worker
/// or back to the main thread via `postMessage`.
pub fn to_bytes<T: Serialize>(value: &T) -> Box<[u8]> {
    postcard::to_allocvec(value)
        .expect("WebWorker serialization failed")
        .into()
}

/// This wrapper function encapsules our internal serialization format.
/// It is used internally to prepare values after receiving them from a worker
/// or the main thread via `postMessage`.
pub fn from_bytes<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> T {
    postcard::from_bytes(bytes).expect("WebWorker deserialization failed")
}
