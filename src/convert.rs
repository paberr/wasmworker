use serde::{Deserialize, Serialize};

/// This wrapper function encapsulates our internal serialization format.
/// It is used internally to prepare values before sending them to a worker
/// or back to the main thread via `postMessage`.
#[cfg(feature = "codec-postcard")]
pub fn to_bytes<T: Serialize>(value: &T) -> Box<[u8]> {
    postcard::to_allocvec(value)
        .expect("WebWorker serialization failed")
        .into()
}

/// This wrapper function encapsulates our internal serialization format.
/// It is used internally to prepare values after receiving them from a worker
/// or the main thread via `postMessage`.
#[cfg(feature = "codec-postcard")]
pub fn from_bytes<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> T {
    postcard::from_bytes(bytes).expect("WebWorker deserialization failed")
}

#[cfg(feature = "codec-pot")]
pub const POT_CONFIG: pot::Config = pot::Config::new().compatibility(pot::Compatibility::V4);

/// This wrapper function encapsulates our internal serialization format.
/// It is used internally to prepare values before sending them to a worker
/// or back to the main thread via `postMessage`.
#[cfg(feature = "codec-pot")]
pub fn to_bytes<T: Serialize>(value: &T) -> Box<[u8]> {
    POT_CONFIG
        .serialize(self)
        .expect("WebWorker serialization failed")
        .into()
}

/// This wrapper function encapsulates our internal serialization format.
/// It is used internally to prepare values after receiving them from a worker
/// or the main thread via `postMessage`.
#[cfg(feature = "codec-pot")]
pub fn from_bytes<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> T {
    POT_CONFIG
        .deserialize(bytes)
        expect("WebWorker deserialization failed")
}
