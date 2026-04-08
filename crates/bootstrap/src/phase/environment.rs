/// Environment variable phase functions.
///
/// These functions set environment variables during the bootstrap process.
use std::collections::HashMap;

/// Sets environment variables before `switch_root`.
pub fn pre(config: &HashMap<String, String>) {
    for (key, value) in config {
        #[expect(unsafe_code, reason = "Safe in PID 1 single-threaded context")]
        unsafe {
            std::env::set_var(key, value);
        }
        tracing::debug!("Pre-switch: set env {key} = {value}");
    }
}

/// Sets environment variables after `switch_root`.
pub fn post(config: &HashMap<String, String>) {
    for (key, value) in config {
        #[expect(unsafe_code, reason = "Safe in PID 1 single-threaded context")]
        unsafe {
            std::env::set_var(key, value);
        }
        tracing::debug!("Post-switch: set env {key} = {value}");
    }
}
