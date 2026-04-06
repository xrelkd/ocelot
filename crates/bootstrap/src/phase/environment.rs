pub fn pre(config: &[(String, String)]) {
    for (key, value) in config {
        #[expect(unsafe_code, reason = "Safe in PID 1 single-threaded context")]
        unsafe {
            std::env::set_var(key, value);
        }
        tracing::debug!("Pre-switch: set env {key} = {value}");
    }
}

pub fn post(config: &[(String, String)]) {
    for (key, value) in config {
        #[expect(unsafe_code, reason = "Safe in PID 1 single-threaded context")]
        unsafe {
            std::env::set_var(key, value);
        }
        tracing::debug!("Post-switch: set env {key} = {value}");
    }
}
