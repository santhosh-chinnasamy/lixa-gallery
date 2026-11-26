//! OS scheduling priority for decode threads that outlived their folder.
//!
//! A folder switch keeps in-flight decodes running so their thumbnails land in
//! the cache, but they must not compete with the folder on screen. Lowering a
//! thread's priority (`nice`) never needs privileges; raising it again does, so
//! callers must run demoted work on a dedicated short-lived thread instead of
//! returning it to a shared pool.
//!
//! Only Linux is wired up today. Other platforms keep the default priority; the
//! measured budgets (`e2e/reports/`) come from Linux/WebKitGTK.

/// How much lower the stale decode priority is. Background-class work.
const NICE_STEP: i32 = 10;

/// Lower the calling thread's priority. The change lasts until it exits, so
/// call this only on a dedicated thread.
#[cfg(target_os = "linux")]
pub fn demote_current_thread() {
    // getpriority legitimately returns -1..19. A -1 can also mean an error, but
    // stepping from -1 by NICE_STEP is still "lower priority", which is the goal.
    let current = unsafe { libc::getpriority(libc::PRIO_PROCESS, 0) };
    let target = (current.saturating_add(NICE_STEP)).clamp(-20, 19);
    if target > current {
        unsafe {
            libc::setpriority(libc::PRIO_PROCESS, 0, target);
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn demote_current_thread() {
    // No portable "lower this thread only" primitive for the platforms we
    // build for. The scheduler still bounds concurrency; only the nice step is
    // skipped.
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn demote_current_thread_lowers_nice() {
        let before = unsafe { libc::getpriority(libc::PRIO_PROCESS, 0) };
        demote_current_thread();
        let after = unsafe { libc::getpriority(libc::PRIO_PROCESS, 0) };
        if before < 19 {
            assert!(
                after > before,
                "nice must drop priority ({before} -> {after})"
            );
        } else {
            assert_eq!(after, 19, "already at the lowest priority");
        }
        assert!(after <= 19);
    }
}
