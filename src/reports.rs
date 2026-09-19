//! Explicit, local-only diagnostic bundles. Networking is intentionally absent: a user chooses
//! later whether to inspect or send a completed bundle through a separately paired relay.

use anyhow::{Context, Result, bail};
use crossbeam_channel::{Receiver, Sender};
use image::RgbImage;
use image::codecs::jpeg::JpegEncoder;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_REPORTS: usize = 5;

struct Capture {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
}

#[derive(Serialize)]
struct Manifest<'a> {
    schema: u8,
    build: String,
    captured_unix: u64,
    width: u32,
    height: u32,
    screenshot: &'a str,
    log: &'a str,
    frame_stats: &'a str,
    upload: &'a str,
}

pub struct ReportWriter {
    sender: Sender<Capture>,
    results: Receiver<String>,
}

impl ReportWriter {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::bounded(1);
        let (result_sender, results) = crossbeam_channel::bounded(2);
        std::thread::Builder::new()
            .name("opennow-report".to_owned())
            .spawn(move || {
                while let Ok(capture) = receiver.recv() {
                    let result = write_report(capture)
                        .map(|path| format!("Diagnostic saved locally: {}", path.display()))
                        .unwrap_or_else(|error| format!("Diagnostic report failed: {error:#}"));
                    let _ = result_sender.send(result);
                }
            })
            .expect("report worker thread");
        Self { sender, results }
    }

    /// The caller is the render thread after presentation. If busy, retain the existing report
    /// rather than queueing game frames or adding streaming pressure.
    pub fn capture_now(&self) -> Result<()> {
        self.sender
            .try_send(capture_display_rgb()?)
            .map_err(|_| anyhow::anyhow!("a diagnostic report is already being written"))
    }

    pub fn try_result(&self) -> Option<String> {
        self.results.try_recv().ok()
    }
}

fn reports_dir() -> PathBuf {
    crate::logger::data_root().join("reports")
}

fn write_report(capture: Capture) -> Result<PathBuf> {
    let root = reports_dir();
    fs::create_dir_all(&root)?;
    prune_reports(&root)?;
    let captured_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let pending = root.join(format!("report-{captured_unix}.partial"));
    let final_dir = root.join(format!("report-{captured_unix}"));
    fs::create_dir_all(&pending)?;

    let image = RgbImage::from_raw(capture.width, capture.height, capture.rgb)
        .context("invalid screenshot buffer")?;
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 80).encode_image(&image)?;
    fs::write(pending.join("screen.jpg.tmp"), jpeg)?;
    fs::rename(pending.join("screen.jpg.tmp"), pending.join("screen.jpg"))?;
    copy_redacted_tail(
        &crate::logger::latest_log_path(),
        &pending.join("opennow.log"),
    )?;
    copy_redacted_tail(
        &crate::logger::frame_stats_path(),
        &pending.join("frame_stats.log"),
    )?;
    let manifest = Manifest {
        schema: 1,
        build: crate::diag::build_id(),
        captured_unix,
        width: capture.width,
        height: capture.height,
        screenshot: "screen.jpg",
        log: "opennow.log",
        frame_stats: "frame_stats.log",
        upload: "not configured; local-only report",
    };
    fs::write(
        pending.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    fs::rename(&pending, &final_dir)?;
    Ok(final_dir)
}

fn copy_redacted_tail(source: &Path, destination: &Path) -> Result<()> {
    let bytes = fs::read(source).unwrap_or_default();
    let start = bytes.len().saturating_sub(64 * 1024);
    let text = String::from_utf8_lossy(&bytes[start..]);
    fs::write(destination, crate::logger::redact(&text))?;
    Ok(())
}

fn prune_reports(root: &Path) -> Result<()> {
    let mut reports: Vec<_> = fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("report-"))
        .collect();
    reports.sort_by_key(|entry| entry.file_name());
    while reports.len() >= MAX_REPORTS {
        let old = reports.remove(0).path();
        fs::remove_dir_all(old)?;
    }
    Ok(())
}

#[cfg(target_os = "vita")]
fn capture_display_rgb() -> Result<Capture> {
    use vitasdk_sys::{
        SCE_DISPLAY_PIXELFORMAT_A8B8G8R8, SCE_DISPLAY_SETBUF_IMMEDIATE, SceDisplayFrameBuf,
        sceDisplayGetFrameBuf,
    };
    let mut frame = SceDisplayFrameBuf {
        size: std::mem::size_of::<SceDisplayFrameBuf>() as _,
        base: std::ptr::null_mut(),
        pitch: 0,
        pixelformat: 0,
        width: 0,
        height: 0,
    };
    let result = unsafe { sceDisplayGetFrameBuf(&mut frame, SCE_DISPLAY_SETBUF_IMMEDIATE) };
    if result < 0 || frame.base.is_null() || frame.pixelformat != SCE_DISPLAY_PIXELFORMAT_A8B8G8R8 {
        bail!("display framebuffer unavailable or unsupported");
    }
    if !(1..=1920).contains(&frame.width)
        || !(1..=1088).contains(&frame.height)
        || frame.pitch < frame.width
        || frame.pitch > 4096
    {
        bail!("display framebuffer dimensions rejected");
    }
    let source_len = frame.pitch as usize * frame.height as usize * 4;
    let source = unsafe { std::slice::from_raw_parts(frame.base.cast::<u8>(), source_len) };
    let mut rgb = Vec::with_capacity(frame.width as usize * frame.height as usize * 3);
    for row in source
        .chunks_exact(frame.pitch as usize * 4)
        .take(frame.height as usize)
    {
        for pixel in row.chunks_exact(4).take(frame.width as usize) {
            rgb.extend_from_slice(&pixel[..3]);
        }
    }
    Ok(Capture {
        width: frame.width,
        height: frame.height,
        rgb,
    })
}

#[cfg(not(target_os = "vita"))]
fn capture_display_rgb() -> Result<Capture> {
    bail!("screenshot capture is available only in the Vita build")
}
