use pot::{Compatibility, Config};
use serde::{Deserialize, Serialize};

const POT_CONFIG: Config = Config::new().compatibility(Compatibility::V4);

/// This wrapper function encapsulates our internal serialization format.
/// It is used internally to prepare values before sending them to a worker
/// or back to the main thread via `postMessage`.
pub fn to_bytes<T: Serialize>(value: &T) -> Box<[u8]> {
    POT_CONFIG
        .serialize(value)
        .expect("WebWorker serialization failed")
        .into()
}

/// This wrapper function encapsulates our internal serialization format.
/// It is used internally to prepare values after receiving them from a worker
/// or the main thread via `postMessage`.
pub fn from_bytes<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> T {
    POT_CONFIG
        .deserialize(bytes)
        .expect("WebWorker deserialization failed")
}
