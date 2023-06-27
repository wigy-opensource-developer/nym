// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use wasm_bindgen::prelude::*;
use wasm_utils::console_log;

#[cfg(target_arch = "wasm32")]
mod client;
#[cfg(target_arch = "wasm32")]
mod config;
#[cfg(target_arch = "wasm32")]
pub mod encoded_payload_helper;
#[cfg(target_arch = "wasm32")]
pub mod error;
#[cfg(target_arch = "wasm32")]
pub mod gateway_selector;
#[cfg(target_arch = "wasm32")]
mod helpers;
#[cfg(all(target_arch = "wasm32", feature = "mix-fetch"))]
pub mod mix_fetch;
#[cfg(target_arch = "wasm32")]
pub mod storage;
#[cfg(target_arch = "wasm32")]
pub mod tester;
#[cfg(target_arch = "wasm32")]
pub mod topology;
#[cfg(target_arch = "wasm32")]
pub mod validation;

mod constants;

#[wasm_bindgen]
pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[wasm_bindgen(start)]
pub fn main() {
    console_log!("[rust main]: rust module loaded")
}
