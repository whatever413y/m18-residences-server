//! Logging that reaches the right place on each target: the Workers console
//! (`wrangler tail`, Workers Logs) in production, stdout/stderr in native tests.
//! Use the macros: `log_ok!` (✅), `log_warn!` (⚠️), `log_error!` (❌).

pub fn ok(message: &str) {
    #[cfg(target_arch = "wasm32")]
    worker::console_log!("✅ {message}");
    #[cfg(not(target_arch = "wasm32"))]
    println!("✅ {message}");
}

pub fn warn(message: &str) {
    #[cfg(target_arch = "wasm32")]
    worker::console_warn!("⚠️ {message}");
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("⚠️ {message}");
}

pub fn error(message: &str) {
    #[cfg(target_arch = "wasm32")]
    worker::console_error!("❌ {message}");
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("❌ {message}");
}

#[macro_export]
macro_rules! log_ok {
    ($($arg:tt)*) => { $crate::log::ok(&format!($($arg)*)) };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => { $crate::log::warn(&format!($($arg)*)) };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => { $crate::log::error(&format!($($arg)*)) };
}
