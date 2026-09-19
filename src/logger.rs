use anyhow::Result;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);
const MAX_LOG_BYTES: u64 = 512 * 1024;
const MAX_FRAME_STATS_BYTES: u64 = 1024 * 1024;

pub fn data_root() -> PathBuf {
    if cfg!(target_os = "vita") {
        PathBuf::from("ux0:data/opennow-vita")
    } else {
        PathBuf::from("opennow-vita")
    }
}

pub fn logs_dir() -> PathBuf {
    data_root().join("logs")
}

pub fn frame_stats_path() -> PathBuf {
    data_root().join("frame_stats.log")
}

pub fn latest_log_path() -> PathBuf {
    logs_dir().join("opennow_latest.log")
}

pub fn reset_frame_stats_log() {
    let path = frame_stats_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, "");
}

/// Frame telemetry goes to its own file, and only there.
///
/// It used to be copied into the main log as well. At one block every two seconds that is most of
/// what the log contains - 94 KB of it in the last Vita3K session - and the four lines that
/// actually mattered (the renderer name, the CDRAM reservation) were buried inside it. A log whose
/// signal is that far below its noise is not a diagnostic, and this is the file the player is
/// asked to send back.
pub fn write_frame_stats(message: &str) {
    let path = frame_stats_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0) >= MAX_FRAME_STATS_BYTES {
        let _ = fs::write(&path, "[truncated: frame statistics size limit reached]\n");
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(file, "{message}");
    }
}

pub fn init() -> Result<()> {
    let dir = logs_dir();
    fs::create_dir_all(&dir)?;

    let latest_path = latest_log_path();
    let previous_path = dir.join("opennow_previous.log");

    if latest_path.exists() {
        let _ = fs::rename(&latest_path, &previous_path);
    }

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&latest_path)?;

    if let Ok(mut guard) = LOG_FILE.lock() {
        *guard = Some(file);
    }

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("[FATAL PANIC] {info}\n");
        eprintln!("{msg}");
        // Do not re-enter the normal logger while a panic may have interrupted it holding the
        // mutex. The stderr line above remains available to Vita3K/the platform debugger.
        default_hook(info);
    }));

    write_log("INFO", "OpenNOW-vita logger initialized");
    Ok(())
}

pub fn write_log(level: &str, message: &str) {
    let timestamp = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => 0,
    };

    let line = format!("[{timestamp}] [{level}] {}\n", redact(message));
    eprint!("{line}");

    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut file) = *guard {
            if file.metadata().map(|meta| meta.len()).unwrap_or(0) >= MAX_LOG_BYTES {
                return;
            }
            let _ = file.write_all(line.as_bytes());
            if matches!(level, "ERROR" | "FATAL") {
                let _ = file.flush();
            }
        }
    }
}

/// Deliberately conservative scrubber for diagnostics that can leave the device.
/// Protocol payloads are removed at their source; this is a final guard for accidental tokens.
pub fn redact(message: &str) -> String {
    let mut result = String::with_capacity(message.len());
    let mut redact_words = 0u8;
    for word in message.split_whitespace() {
        let lower = word.to_ascii_lowercase();
        let authorization = lower.contains("authorization:");
        let marker = authorization
            || lower == "bearer"
            || lower == "gfnjwt"
            || lower.contains("refresh_token")
            || lower.contains("access_token");
        let secret =
            redact_words > 0 || marker || (word.matches('.').count() == 2 && word.len() > 24);
        if redact_words > 0 {
            redact_words -= 1;
        }
        if authorization {
            // "Authorization: Bearer <token>" is three separate whitespace-delimited words.
            redact_words = 2;
        } else if lower == "bearer" || lower == "gfnjwt" {
            redact_words = 1;
        }
        if secret {
            result.push_str("[redacted]");
        } else {
            result.push_str(word);
        }
        result.push(' ');
    }
    result.trim_end().to_owned()
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logger::write_log("INFO", &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logger::write_log("WARN", &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::write_log("ERROR", &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_stream {
    ($($arg:tt)*) => {
        $crate::logger::write_log("STREAM", &format!($($arg)*))
    };
}
