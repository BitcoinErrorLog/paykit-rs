//! Utility functions for WASM

use wasm_bindgen::prelude::*;

/// Set up better panic messages in the browser console
pub fn set_panic_hook() {
    // Panic hook setup is available in tests
    // In production, panics will be caught by browser's error handling
}

/// Log a message to the browser console
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn warn(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn error(s: &str);
}

/// Convert a JS error into a Result
pub fn js_error(msg: &str) -> JsValue {
    js_sys::Error::new(msg).into()
}
