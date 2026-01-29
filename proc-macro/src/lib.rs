#![deny(rustdoc::broken_intra_doc_links, missing_docs)]

//! Procedural macros for exporting functions to WebWorkers.
//!
//! This crate provides two macros:
//! - [`webworker_fn`]: For simple, synchronous functions
//! - [`webworker_channel_fn`]: For async functions with bidirectional channel support

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemFn};

/// A procedural macro that exports a simple function for use with a WebWorker.
///
/// Use this for functions that take a single argument and return a result synchronously.
/// The function will be callable via `WebWorkerFn` and the `webworker!` macro.
///
/// # Example
///
/// ```ignore
/// use wasmworker_proc_macro::webworker_fn;
///
/// #[webworker_fn]
/// fn sort_vec(mut v: Vec<u32>) -> Vec<u32> {
///     v.sort();
///     v
/// }
/// ```
#[proc_macro_attribute]
pub fn webworker_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let wrapper_fn_name = format_ident!("__webworker_{}", fn_name);

    let mod_code = quote! {
        #[doc(hidden)]
        pub mod #fn_name {
            pub const __WEBWORKER: () = ();
            const _: () = {
                #[wasm_bindgen::prelude::wasm_bindgen]
                pub fn #wrapper_fn_name(arg: Box<[u8]>) -> Box<[u8]> {
                    let arg = wasmworker::convert::from_bytes(&arg);
                    let res = super::#fn_name(arg);
                    wasmworker::convert::to_bytes(&res)
                }
            };
        }
    };

    let expanded = quote! {
        #input

        #mod_code
    };

    TokenStream::from(expanded)
}

/// A procedural macro that exports an async function with channel support for use with a WebWorker.
///
/// Use this for functions that need bidirectional communication with the main thread,
/// such as progress reporting or interactive workflows. The function must be async and
/// take a `Channel` as its second parameter.
///
/// The function will be callable via `WebWorkerChannelFn` and the `webworker_channel!` macro.
///
/// # Example
///
/// ```ignore
/// use wasmworker_proc_macro::webworker_channel_fn;
/// use wasmworker::Channel;
///
/// #[webworker_channel_fn]
/// async fn process_with_progress(data: Vec<u8>, channel: Channel) -> Result<Output, Error> {
///     channel.send(&Progress { percent: 50 });
///     let response: UserChoice = channel.recv().await?;
///     // ... process data ...
///     Ok(output)
/// }
/// ```
#[proc_macro_attribute]
pub fn webworker_channel_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let wrapper_fn_name = format_ident!("__webworker_channel_{}", fn_name);

    let mod_code = quote! {
        #[doc(hidden)]
        pub mod #fn_name {
            pub const __WEBWORKER_CHANNEL: () = ();
            const _: () = {
                #[wasm_bindgen::prelude::wasm_bindgen]
                pub async fn #wrapper_fn_name(arg: Box<[u8]>, port: wasm_bindgen::JsValue) -> Box<[u8]> {
                    use wasm_bindgen::JsCast;
                    let arg = wasmworker::convert::from_bytes(&arg);
                    let channel = port
                        .dyn_into::<wasmworker::MessagePort>()
                        .map(wasmworker::Channel::from)
                        .expect("webworker_channel_fn requires a MessagePort");
                    let res = super::#fn_name(arg, channel).await;
                    wasmworker::convert::to_bytes(&res)
                }
            };
        }
    };

    let expanded = quote! {
        #input

        #mod_code
    };

    TokenStream::from(expanded)
}
