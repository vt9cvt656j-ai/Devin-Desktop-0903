//! EPIPE-safe console logging.
//!
//! In `tauri dev` mode stdout/stderr are pipes owned by the cargo/npm parent
//! process. If that parent dies (vite crash, watchdog restart, manual kill),
//! the pipes break and the next bare `eprintln!`/`println!` panics with
//! "failed printing to stderr" → abort → the whole IDE crashes
//! (see crash report michael-ide-2026-07-29-053440.ips: `_eprint → rust_panic`).
//! These macros write through `Write` and discard the error instead.

/// Like `eprintln!`, but a broken stderr pipe is silently ignored.
#[macro_export]
macro_rules! elog {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let _ = writeln!(std::io::stderr().lock(), $($arg)*);
    }};
}

/// Like `println!`, but a broken stdout pipe is silently ignored.
#[macro_export]
macro_rules! olog {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let _ = writeln!(std::io::stdout().lock(), $($arg)*);
    }};
}
