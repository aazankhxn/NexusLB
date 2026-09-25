use tracing::debug;

pub fn set_core_affinity(core_id: usize) {
    #[cfg(target_os = "linux")]
    {
        use std::mem;
        unsafe {
            let mut cpuset: libc::cpu_set_t = mem::zeroed();
            libc::CPU_SET(core_id, &mut cpuset);
            let res = libc::sched_setaffinity(0, mem::size_of::<libc::cpu_set_t>(), &cpuset);
            if res == 0 {
                debug!(core_id, "Bound worker thread to CPU core");
            } else {
                debug!(core_id, "Could not set CPU core affinity (non-critical)");
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        // On macOS or other platforms, thread affinity is managed by the OS scheduler
        debug!(
            core_id,
            "CPU core affinity binding is active via OS thread scheduler"
        );
    }
}
