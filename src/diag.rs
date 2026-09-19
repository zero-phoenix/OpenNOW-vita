//! Diagnostics that a real console can actually deliver.
//!
//! Every render, memory and decoder diagnostic in this client used to be an `eprintln!`. On a PS
//! Vita that goes to `tty0:`, which has no sink unless a debug cable or an emulator is attached -
//! so the messages existed, but only Vita3K could read them. That is how 0.6.0 shipped with a
//! broken interface: the evidence was being printed the whole time, into nothing.
//!
//! `diag!` writes the same text to the log file on the memory card, which the player can send back.
//! It still prints to stderr, so nothing is lost where a TTY does exist.
//!
//! Folding matters as much as routing. A texture that fails to allocate fails again on the next
//! frame, and sixty identical lines a second would bury the log exactly the way the frame stats
//! did. Repeats from the same call site inside a short window are counted, not written.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long repeats from one call site are folded before the next line is written.
const REPEAT_WINDOW: Duration = Duration::from_secs(5);

struct Repeat {
    window_started_at: Instant,
    suppressed: u32,
}

static SITES: Mutex<Option<HashMap<&'static str, Repeat>>> = Mutex::new(None);

/// Records a diagnostic, folding bursts from the same call site.
///
/// `site` is a `file!():line!()` string the macro supplies, so the fold survives a message whose
/// numbers change every time ("no room for a 2048x512 texture" then "...2048x1024...").
pub fn record(site: &'static str, message: &str) {
    let now = Instant::now();
    let folded = {
        let Ok(mut guard) = SITES.lock() else {
            // A poisoned lock must not cost us the message - that is the whole point of this module.
            crate::logger::write_log("DIAG", message);
            return;
        };
        let sites = guard.get_or_insert_with(HashMap::new);
        match sites.get_mut(site) {
            Some(seen) if now.duration_since(seen.window_started_at) < REPEAT_WINDOW => {
                seen.suppressed += 1;
                return;
            }
            Some(seen) => {
                let suppressed = std::mem::replace(&mut seen.suppressed, 0);
                seen.window_started_at = now;
                suppressed
            }
            None => {
                sites.insert(
                    site,
                    Repeat {
                        window_started_at: now,
                        suppressed: 0,
                    },
                );
                0
            }
        }
    };

    if folded > 0 {
        crate::logger::write_log("DIAG", &format!("{message} (+{folded} repetidos)"));
    } else {
        crate::logger::write_log("DIAG", message);
    }
}

/// A diagnostic that reaches the memory card, not just a TTY nobody is holding.
///
/// Drop-in for `eprintln!`: same formatting, same stderr output, plus the log file.
#[macro_export]
macro_rules! diag {
    ($($arg:tt)*) => {
        $crate::diag::record(
            concat!(file!(), ":", line!()),
            &format!($($arg)*),
        )
    };
}

/// Free memory as the kernel reports it, or why it could not be read.
///
/// The three partitions answer three different questions: `user` is the app's own LPDDR2, `cdram`
/// is the 128 MB of video memory that SDL's GXM backend allocates every texture from, and
/// `phycont` is the physically contiguous pool the video decoder needs.
pub fn free_memory() -> String {
    #[cfg(target_os = "vita")]
    {
        use vitasdk_sys::{SceKernelFreeMemorySizeInfo, sceKernelGetFreeMemorySize};
        unsafe {
            let mut info = SceKernelFreeMemorySizeInfo {
                size: size_of::<SceKernelFreeMemorySizeInfo>() as i32,
                size_user: 0,
                size_cdram: 0,
                size_phycont: 0,
            };
            let ret = sceKernelGetFreeMemorySize(&mut info);
            if ret < 0 {
                return format!("free=unavailable({ret:#x})");
            }
            format!(
                "free user={} KiB cdram={} KiB phycont={} KiB",
                info.size_user / 1024,
                info.size_cdram / 1024,
                info.size_phycont / 1024,
            )
        }
    }
    #[cfg(not(target_os = "vita"))]
    {
        "free=n/a (host build)".to_owned()
    }
}

/// The build that is actually running, so a log can be matched to a commit.
///
/// `OPENNOW_BUILD_REV` is stamped in by `build.rs`; without it we would be guessing which of two
/// same-looking VPKs produced a log, which is precisely the ambiguity that made the 0.6.0 release
/// impossible to reason about.
pub fn build_id() -> String {
    format!(
        "{} rev={} built={}",
        env!("CARGO_PKG_VERSION"),
        option_env!("OPENNOW_BUILD_REV").unwrap_or("unknown"),
        option_env!("OPENNOW_BUILD_TIME").unwrap_or("unknown"),
    )
}

/// One block at startup with everything needed to interpret the rest of the log.
pub fn startup_report() {
    crate::logger::write_log("DIAG", &format!("build: {}", build_id()));
    crate::logger::write_log("DIAG", &format!("memory at startup: {}", free_memory()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_burst_from_one_site_is_folded_into_one_line() {
        // Not a rendering test - it pins the contract the render path depends on: a failure that
        // repeats every frame must not be able to fill the memory card.
        let site = "src/diag.rs:test-burst";
        record(site, "first");
        for _ in 0..1000 {
            record(site, "repeat");
        }
        let guard = SITES.lock().expect("uncontended in a test");
        let sites = guard.as_ref().expect("record() initialised the map");
        let seen = sites.get(site).expect("the site was recorded");
        assert_eq!(
            seen.suppressed, 1000,
            "every repeat inside the window should have been counted, not written"
        );
    }
}
