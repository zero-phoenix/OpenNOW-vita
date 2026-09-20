//! Persistent, lossless diagnostic evidence. Captures stay local while a game is streaming;
//! an explicit pre-launch or post-disconnect flush uploads the accumulated evidence to GitHub.

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use crossbeam_channel::{Receiver, Sender};
use image::RgbImage;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

const OWNER: &str = "zero-phoenix";
// Reports live in the user's existing repository, isolated on a branch rather than another repo.
const REPO: &str = "OpenNOW-vita";
const BRANCH: &str = "diagnostics-reports";
const CAPTURE_INTERVAL: Duration = Duration::from_secs(15);

struct Capture {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
    reason: &'static str,
}
#[derive(Serialize, Deserialize)]
struct Manifest {
    schema: u8,
    build: String,
    captured_unix: u64,
    reason: String,
    width: u32,
    height: u32,
    screenshot: String,
    log: String,
    frame_stats: String,
    sha256: String,
    uploaded: bool,
    github_path: Option<String>,
}

pub struct ReportWriter {
    sender: Sender<Capture>,
    results: Receiver<String>,
    streaming: Arc<AtomicBool>,
    tokens: Arc<Mutex<Option<crate::github::Tokens>>>,
    uploading: Arc<AtomicBool>,
    pending_captures: Arc<AtomicUsize>,
}
impl ReportWriter {
    pub fn new() -> Self {
        // The durable on-disk queue is unlimited by design. Never silently discard evidence
        // because a PNG encoder happened to still be working on the previous frame.
        let (sender, receiver) = crossbeam_channel::unbounded();
        let (result_sender, results) = crossbeam_channel::bounded(8);
        let pending_captures = Arc::new(AtomicUsize::new(0));
        let worker_pending = pending_captures.clone();
        std::thread::Builder::new()
            .name("opennow-report".into())
            .spawn(move || {
                while let Ok(capture) = receiver.recv() {
                    let result = write_report(capture)
                        .map(|p| format!("Evidence stored: {}", p.display()))
                        .unwrap_or_else(|e| format!("Evidence capture failed: {e:#}"));
                    let _ = result_sender.send(result);
                    worker_pending.fetch_sub(1, Ordering::Release);
                }
            })
            .expect("report worker thread");
        Self {
            sender,
            results,
            streaming: Arc::new(AtomicBool::new(false)),
            tokens: Arc::new(Mutex::new(crate::github::load())),
            uploading: Arc::new(AtomicBool::new(false)),
            pending_captures,
        }
    }
    pub fn set_streaming(&self, active: bool) {
        self.streaming.store(active, Ordering::Release);
    }
    pub fn capture_now(&self) -> Result<()> {
        self.capture("manual")
    }
    pub fn capture_automatic(&self) -> Result<()> {
        self.capture("automatic-15s")
    }
    fn capture(&self, reason: &'static str) -> Result<()> {
        let capture = capture_display_rgb(reason)?;
        self.pending_captures.fetch_add(1, Ordering::Release);
        if self.sender.try_send(capture).is_err() {
            self.pending_captures.fetch_sub(1, Ordering::Release);
            bail!("diagnostic capture worker is unavailable")
        }
        Ok(())
    }
    pub fn try_result(&self) -> Option<String> {
        self.results.try_recv().ok()
    }
    pub fn has_github_login(&self) -> bool {
        self.tokens
            .lock()
            .ok()
            .and_then(|v| v.as_ref().cloned())
            .is_some()
    }
    pub fn set_github_login(&self, tokens: crate::github::Tokens) -> Result<()> {
        crate::github::save(&tokens)?;
        *self.tokens.lock().expect("report token lock") = Some(tokens);
        Ok(())
    }
    pub fn clear_github_login(&self) {
        crate::github::clear();
        *self.tokens.lock().expect("report token lock") = None;
    }
    /// Starts one durable worker. It retries pending evidence indefinitely, but checks the
    /// streaming gate before every request, so it never uploads during a game.
    pub fn flush_upload(&self) {
        if self.streaming.load(Ordering::Acquire) || self.uploading.swap(true, Ordering::AcqRel) {
            return;
        }
        let streaming = self.streaming.clone();
        let tokens = self.tokens.clone();
        let uploading = self.uploading.clone();
        let pending_captures = self.pending_captures.clone();
        std::thread::Builder::new()
            .name("opennow-github-upload".into())
            .spawn(move || {
                loop {
                    if streaming.load(Ordering::Acquire) {
                        std::thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                    // A boundary capture (pre-launch or post-disconnect) must reach storage
                    // before the scan, otherwise it could be left for a later session.
                    if pending_captures.load(Ordering::Acquire) != 0 {
                        std::thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    let result = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(anyhow::Error::from)
                        .and_then(|rt| rt.block_on(refresh_and_upload(&streaming, &tokens)));
                    match result {
                        Ok(true) => break,
                        Ok(false) => std::thread::sleep(Duration::from_secs(15)),
                        Err(error) => {
                            crate::log_warn!("GitHub evidence upload will retry: {error:#}");
                            std::thread::sleep(Duration::from_secs(15));
                        }
                    }
                }
                uploading.store(false, Ordering::Release);
            })
            .expect("GitHub upload worker");
    }
}

async fn refresh_and_upload(
    streaming: &AtomicBool,
    tokens: &Arc<Mutex<Option<crate::github::Tokens>>>,
) -> Result<bool> {
    if streaming.load(Ordering::Acquire) {
        return Ok(false);
    }
    let current = tokens
        .lock()
        .ok()
        .and_then(|value| value.clone())
        .context("GitHub reports are not signed in")?;
    let token = if crate::github::needs_refresh(&current) {
        let refreshed = crate::github::refresh(&reqwest::Client::new(), &current).await?;
        crate::github::save(&refreshed)?;
        *tokens.lock().expect("report token lock") = Some(refreshed.clone());
        refreshed
    } else {
        current
    };
    if streaming.load(Ordering::Acquire) {
        return Ok(false);
    }
    upload_pending(streaming, &token).await
}
pub fn capture_interval() -> Duration {
    CAPTURE_INTERVAL
}
fn reports_dir() -> PathBuf {
    crate::logger::data_root().join("reports")
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|x| x.as_secs())
        .unwrap_or(0)
}
fn write_report(capture: Capture) -> Result<PathBuf> {
    let root = reports_dir();
    fs::create_dir_all(&root)?;
    let stamp = now();
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let pending = root.join(format!("report-{stamp}-{nonce}.partial"));
    let final_dir = root.join(format!("report-{stamp}-{nonce}"));
    fs::create_dir_all(&pending)?;
    let image = RgbImage::from_raw(capture.width, capture.height, capture.rgb)
        .context("invalid screenshot buffer")?;
    image.save_with_format(pending.join("screen.png"), image::ImageFormat::Png)?;
    copy_redacted_tail(
        &crate::logger::latest_log_path(),
        &pending.join("opennow.log"),
    )?;
    copy_redacted_tail(
        &crate::logger::frame_stats_path(),
        &pending.join("frame_stats.log"),
    )?;
    let png = fs::read(pending.join("screen.png"))?;
    let manifest = Manifest {
        schema: 2,
        build: crate::diag::build_id(),
        captured_unix: stamp,
        reason: capture.reason.into(),
        width: capture.width,
        height: capture.height,
        screenshot: "screen.png".into(),
        log: "opennow.log".into(),
        frame_stats: "frame_stats.log".into(),
        sha256: sha256(&png),
        uploaded: false,
        github_path: None,
    };
    fs::write(
        pending.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    fs::rename(&pending, &final_dir)?;
    Ok(final_dir)
}
fn copy_redacted_tail(source: &Path, destination: &Path) -> Result<()> {
    let data = fs::read(source).unwrap_or_default();
    let at = data.len().saturating_sub(64 * 1024);
    fs::write(
        destination,
        crate::logger::redact(&String::from_utf8_lossy(&data[at..])),
    )?;
    Ok(())
}
fn sha256(input: &[u8]) -> String {
    use ring::digest::{SHA256, digest};
    digest(&SHA256, input)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
async fn upload_pending(streaming: &AtomicBool, tokens: &crate::github::Tokens) -> Result<bool> {
    let root = reports_dir();
    if !root.exists() {
        return Ok(true);
    };
    let mut pending: Vec<PathBuf> = fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && !p.extension().is_some_and(|x| x == "partial"))
        .collect();
    pending.sort();
    for report in pending {
        if streaming.load(Ordering::Acquire) {
            return Ok(false);
        };
        upload_report(streaming, tokens, &report).await?;
    }
    Ok(true)
}
async fn upload_report(
    streaming: &AtomicBool,
    tokens: &crate::github::Tokens,
    dir: &Path,
) -> Result<()> {
    let manifest_path = dir.join("manifest.json");
    let mut manifest: Manifest = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    if manifest.uploaded {
        return Ok(());
    };
    let id = dir
        .file_name()
        .context("report without name")?
        .to_string_lossy();
    let base = format!("reports/{id}");
    for name in ["opennow.log", "frame_stats.log", "screen.png"] {
        if streaming.load(Ordering::Acquire) {
            return Ok(());
        };
        let path = dir.join(name);
        if path.exists() {
            put_file(
                streaming,
                tokens,
                &format!("{base}/{name}"),
                &fs::read(&path)?,
            )
            .await?;
        }
    }
    manifest.uploaded = true;
    manifest.github_path = Some(format!(
        "https://github.com/{OWNER}/{REPO}/tree/{BRANCH}/{base}"
    ));
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    if streaming.load(Ordering::Acquire) {
        return Ok(());
    };
    put_file(streaming, tokens, &format!("{base}/manifest.json"), &bytes).await?;
    // Never delete logs. A PNG is deleted only after GitHub accepted its file and final manifest.
    let png = dir.join("screen.png");
    if png.exists() {
        fs::remove_file(png)?
    };
    fs::write(manifest_path, bytes)?;
    Ok(())
}
async fn put_file(
    streaming: &AtomicBool,
    tokens: &crate::github::Tokens,
    path: &str,
    bytes: &[u8],
) -> Result<()> {
    if streaming.load(Ordering::Acquire) {
        bail!("upload paused because streaming became active")
    }
    let url = format!("https://api.github.com/repos/{OWNER}/{REPO}/contents/{path}");
    let body = serde_json::json!({"message":format!("diagnostics: {path}"),"content":STANDARD.encode(bytes),"branch":BRANCH});
    let response = reqwest::Client::new()
        .put(url)
        .header("User-Agent", "OpenNOW-Vita")
        .bearer_auth(&tokens.access_token)
        .json(&body)
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::UNPROCESSABLE_ENTITY {
        ensure_branch(tokens).await?;
        if streaming.load(Ordering::Acquire) {
            bail!("upload paused because streaming became active")
        }
        let retry = reqwest::Client::new()
            .put(format!(
                "https://api.github.com/repos/{OWNER}/{REPO}/contents/{path}"
            ))
            .header("User-Agent", "OpenNOW-Vita")
            .bearer_auth(&tokens.access_token)
            .json(&body)
            .send()
            .await?;
        // A prior attempt can have completed this exact immutable path and crashed before its
        // local manifest was marked. Report paths contain a UUID, so an existing path is our own
        // already-accepted object, not an overwrite of another report.
        if retry.status() != reqwest::StatusCode::UNPROCESSABLE_ENTITY {
            retry.error_for_status()?;
        }
    } else {
        response.error_for_status()?;
    }
    Ok(())
}
async fn ensure_branch(tokens: &crate::github::Tokens) -> Result<()> {
    let c = reqwest::Client::new();
    let repo: serde_json::Value = c
        .get(format!("https://api.github.com/repos/{OWNER}/{REPO}"))
        .header("User-Agent", "OpenNOW-Vita")
        .bearer_auth(&tokens.access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let default = repo["default_branch"]
        .as_str()
        .context("GitHub default branch missing")?;
    let reference: serde_json::Value = c
        .get(format!(
            "https://api.github.com/repos/{OWNER}/{REPO}/git/ref/heads/{default}"
        ))
        .header("User-Agent", "OpenNOW-Vita")
        .bearer_auth(&tokens.access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let sha = reference["object"]["sha"]
        .as_str()
        .context("GitHub head missing")?;
    let r = c
        .post(format!(
            "https://api.github.com/repos/{OWNER}/{REPO}/git/refs"
        ))
        .header("User-Agent", "OpenNOW-Vita")
        .bearer_auth(&tokens.access_token)
        .json(&serde_json::json!({"ref":format!("refs/heads/{BRANCH}"),"sha":sha}))
        .send()
        .await?;
    if !(r.status().is_success() || r.status() == reqwest::StatusCode::UNPROCESSABLE_ENTITY) {
        r.error_for_status()?;
    };
    Ok(())
}

#[cfg(target_os = "vita")]
fn capture_display_rgb(reason: &'static str) -> Result<Capture> {
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
    let r = unsafe { sceDisplayGetFrameBuf(&mut frame, SCE_DISPLAY_SETBUF_IMMEDIATE) };
    if r < 0 || frame.base.is_null() || frame.pixelformat != SCE_DISPLAY_PIXELFORMAT_A8B8G8R8 {
        bail!("display framebuffer unavailable")
    };
    if !(1..=1920).contains(&frame.width)
        || !(1..=1088).contains(&frame.height)
        || frame.pitch < frame.width
        || frame.pitch > 4096
    {
        bail!("display framebuffer dimensions rejected")
    };
    let data = unsafe {
        std::slice::from_raw_parts(
            frame.base.cast::<u8>(),
            frame.pitch as usize * frame.height as usize * 4,
        )
    };
    let mut rgb = Vec::with_capacity(frame.width as usize * frame.height as usize * 3);
    for row in data
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
        reason,
    })
}
#[cfg(not(target_os = "vita"))]
fn capture_display_rgb(_reason: &'static str) -> Result<Capture> {
    bail!("screenshot capture is available only in the Vita build")
}
