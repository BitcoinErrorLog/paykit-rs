//! Utility functions for WASM

use wasm_bindgen::prelude::*;

/// Get current Unix timestamp in seconds
///
/// On WASM targets, uses `js_sys::Date::now()` (browser API).
/// On native targets, uses `std::time::SystemTime` for testing.
pub fn current_timestamp_secs() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as i64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}

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
