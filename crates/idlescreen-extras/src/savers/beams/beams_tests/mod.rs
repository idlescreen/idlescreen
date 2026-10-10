use std::sync::Mutex;

mod behavior;
mod lifecycle;
mod light;
mod math_proptest;

/// Process-global env races under parallel tests.
/// Host reads `IDLE_SECONDARY_MONITOR` (see idle-runner sys_info).
static MONITOR_ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_primary_monitor<R>(f: impl FnOnce() -> R) -> R {
    crate::runner::set_thread_secondary_monitor(Some(false));
    let out = f();
    crate::runner::set_thread_secondary_monitor(None);
    out
}

fn with_secondary_monitor<R>(f: impl FnOnce() -> R) -> R {
    crate::runner::set_thread_secondary_monitor(Some(true));
    let out = f();
    crate::runner::set_thread_secondary_monitor(None);
    out
}
