//! Samples that couldn't be delivered (offline, server down), one JSON per
//! line. Sent oldest first once the server answers again.

use battery_model::BatteryHealth;
use std::fs;
use std::io::Write;

const PATH: &str = "/Library/Application Support/battery-agent/queue.jsonl";
/// About a week of samples at the default 10s interval; the oldest are dropped beyond it.
const MAX_LINES: usize = 60_000;
/// Per cycle, so a long backlog doesn't stall fresh sampling.
const FLUSH_BATCH: usize = 500;

pub fn is_empty() -> bool {
    fs::metadata(PATH).map_or(true, |m| m.len() == 0)
}

pub fn push(sample: &BatteryHealth) {
    let _ = fs::create_dir_all(std::path::Path::new(PATH).parent().unwrap());
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(PATH)
        && let Ok(line) = serde_json::to_string(sample)
    {
        let _ = writeln!(f, "{line}");
    }
}

/// Sends queued samples in order until one fails; keeps the rest.
pub fn flush(send: impl Fn(&BatteryHealth) -> Result<(), String>) -> Result<(), String> {
    let Ok(text) = fs::read_to_string(PATH) else { return Ok(()) };
    let mut lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    if lines.len() > MAX_LINES {
        lines.drain(..lines.len() - MAX_LINES);
    }
    let mut sent = 0;
    let mut result = Ok(());
    for line in lines.iter().take(FLUSH_BATCH) {
        let Ok(sample) = serde_json::from_str::<BatteryHealth>(line) else {
            sent += 1; // unreadable line: drop it
            continue;
        };
        if let Err(e) = send(&sample) {
            result = Err(e);
            break;
        }
        sent += 1;
    }
    let rest = &lines[sent..];
    let _ = if rest.is_empty() {
        fs::remove_file(PATH)
    } else {
        fs::write(PATH, rest.join("\n") + "\n")
    };
    result
}
