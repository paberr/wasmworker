#![deny(rustdoc::broken_intra_doc_links, missing_docs)]

//! A crate to export a function to the webworker.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, token::Async, ItemFn};

/// A procedural macro that exports a function for use with a webworker.
#[proc_macro_attribute]
pub fn webworker_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;

    // make the input function always async
    input.sig.asyncness = Some(Async::default());

    // the core function should have 2 inputs always
    let output = if input.sig.inputs.len() == 1 {
        let func_vis = &input.vis; // like pub
        let func_block = &input.block; // { some statement or expression here }
        let func_name = &input.sig.ident; // function nameinput
        let func_generics = &input.sig.generics;
        let func_inputs = &input.sig.inputs;
        let func_output = &input.sig.output;
        quote! {
            #func_vis async fn #func_name #func_generics(#func_inputs, channel: Option<wasmworker::Channel>) #func_output {
                #func_block
            }
        }
    } else {
        quote! {
            #input
        }
    };

    // Generate a module with the wrapper function
    let wrapper_fn_name = format_ident!("__webworker_{}", fn_name);
    let mod_code = quote! {
        pub mod #fn_name {
            pub const __WEBWORKER: () = ();
            const _: () = {
                #[wasm_bindgen::prelude::wasm_bindgen]
                pub async fn #wrapper_fn_name(arg: Box<[u8]>, port: wasm_bindgen::JsValue) -> Box<[u8]> {
                    use wasm_bindgen::JsCast;
                    let arg = wasmworker::convert::from_bytes(&arg);
                    let channel = port.dyn_into::<wasmworker::MessagePort>().ok().map(wasmworker::Channel::from);
                    let res = super::#fn_name(arg, channel).await;
                    let res = wasmworker::convert::to_bytes(&res);
                    res
                }
            };
        }
    };

    // Combine everything into the final output
    let expanded = quote! {
        #output

        #mod_code
    };

    TokenStream::from(expanded)
}
