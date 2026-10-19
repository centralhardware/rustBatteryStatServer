//! Samples that couldn't be delivered (offline, server down), one JSON per
//! line. Sent oldest first once the server answers again.

use battery_model::BatteryHealth;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// About a week of samples at the default 10s interval; the oldest are dropped beyond it.
pub const MAX_LINES: usize = 60_000;
/// Per cycle, so a long backlog doesn't stall fresh sampling.
const FLUSH_BATCH: usize = 500;
/// Past this size `push` trims the file back to `MAX_LINES`. A sample line is
/// well under 2 KiB, so this sits a little above `MAX_LINES` worth; the queue
/// stays bounded even when nothing ever flushes it (no ServerURL set).
const TRIM_BYTES: u64 = 120 * 1024 * 1024;

/// Why a send failed.
#[derive(Debug, PartialEq)]
pub enum SendError {
    /// Worth trying again later: network, 5xx, 401/403 (token being changed), 429.
    Retry(String),
    /// The server will never take this sample (other 4xx); drop it.
    Permanent(String),
}

impl SendError {
    pub fn from_status(status: u16, msg: String) -> Self {
        match status {
            401 | 403 | 408 | 429 => Self::Retry(msg),
            400..=499 => Self::Permanent(msg),
            _ => Self::Retry(msg),
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Retry(m) | Self::Permanent(m) => m,
        }
    }
}

pub struct Queue {
    path: PathBuf,
}

impl Queue {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn is_empty(&self) -> bool {
        fs::metadata(&self.path).map_or(true, |m| m.len() == 0)
    }

    pub fn push(&self, sample: &BatteryHealth) {
        if let Some(dir) = self.path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&self.path)
            && let Ok(line) = serde_json::to_string(sample)
        {
            let _ = writeln!(f, "{line}");
        }
        if fs::metadata(&self.path).is_ok_and(|m| m.len() > TRIM_BYTES) {
            self.trim();
        }
    }

    /// Drops the oldest lines beyond `MAX_LINES`.
    fn trim(&self) {
        let Ok(text) = fs::read_to_string(&self.path) else { return };
        let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        if lines.len() > MAX_LINES {
            self.rewrite(&lines[lines.len() - MAX_LINES..]);
        }
    }

    /// Sends queued samples in order until one fails with a retryable error;
    /// keeps the rest. A sample the server rejects for good is dropped (and
    /// logged) so it cannot block everything queued behind it.
    pub fn flush(&self, send: impl Fn(&BatteryHealth) -> Result<(), SendError>) -> Result<(), String> {
        let Ok(text) = fs::read_to_string(&self.path) else { return Ok(()) };
        let mut lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        if lines.len() > MAX_LINES {
            lines.drain(..lines.len() - MAX_LINES);
        }
        let mut done = 0;
        let mut result = Ok(());
        for line in lines.iter().take(FLUSH_BATCH) {
            let Ok(sample) = serde_json::from_str::<BatteryHealth>(line) else {
                done += 1; // unreadable line: drop it
                continue;
            };
            match send(&sample) {
                Ok(()) => {}
                Err(SendError::Permanent(e)) => eprintln!("dropping a queued sample: {e}"),
                Err(SendError::Retry(e)) => {
                    result = Err(e);
                    break;
                }
            }
            done += 1;
        }
        if done > 0 || lines.len() < text.lines().count() {
            self.rewrite(&lines[done..]);
        }
        result
    }

    /// Replaces the file through a temp file and a rename, so a crash mid-write
    /// leaves either the old queue or the new one, never half of one.
    fn rewrite(&self, lines: &[&str]) {
        if lines.is_empty() {
            let _ = fs::remove_file(&self.path);
            return;
        }
        let tmp = tmp_path(&self.path);
        if fs::write(&tmp, lines.join("\n") + "\n").is_ok() {
            let _ = fs::rename(&tmp, &self.path);
        }
    }
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}
