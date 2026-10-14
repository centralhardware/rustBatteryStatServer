//! Reads AppleSmartBattery via IOKit and POSTs samples to BatteryStatServer.
//! Runs resident as a root LaunchDaemon (see packaging/): one sample every
//! `IntervalSeconds` (default 10), plus one immediately whenever the power
//! source changes. Each sample also covers any iPhone plugged in by USB (see
//! iphone.rs), reported under its UDID.
//!
//! Server URL: $BATTERY_AGENT_URL, else `ServerURL` in
//! /Library/Preferences/me.centralhardware.battery-agent.plist (the installer
//! asks for it), plus an optional `AuthToken` sent as a bearer token.
//! Preferences are re-read every cycle, so changes need no restart.
//! Samples that can't be delivered wait in an on-disk queue (see queue.rs).
//! `--dry-run` prints one payload without sending it; `--dump` prints the raw
//! AppleSmartBattery properties (`--dump-iphone`: those of USB-connected iPhones).

#[cfg(target_os = "macos")]
mod iokit;

#[cfg(target_os = "macos")]
mod iphone;

#[cfg(target_os = "macos")]
mod queue;

#[cfg(target_os = "macos")]
use battery_model::BatteryHealth;
#[cfg(target_os = "macos")]
use serde::Serialize;
use std::process::ExitCode;
#[cfg(target_os = "macos")]
use std::sync::mpsc;
#[cfg(target_os = "macos")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(target_os = "macos")]
const PREFS_PATH: &str = "/Library/Preferences/me.centralhardware.battery-agent.plist";
#[cfg(target_os = "macos")]
const ENDPOINT: &str = "/api/battery/health";
#[cfg(target_os = "macos")]
const DEFAULT_INTERVAL_SECS: u64 = 10;
#[cfg(target_os = "macos")]
const MIN_INTERVAL_SECS: u64 = 5;

#[cfg(not(target_os = "macos"))]
fn main() -> ExitCode {
    eprintln!("battery-agent only runs on macOS");
    ExitCode::FAILURE
}

#[cfg(target_os = "macos")]
fn main() -> ExitCode {
    if std::env::args().any(|a| a == "--dump") {
        let props = iokit::properties("AppleSmartBattery").unwrap_or_default();
        println!("{}", serde_json::to_string_pretty(&props).unwrap());
        return ExitCode::SUCCESS;
    }
    if std::env::args().any(|a| a == "--dump-iphone") {
        let reader = match iphone::Reader::new() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        };
        let dump: serde_json::Map<_, _> = reader
            .read_all()
            .into_iter()
            .map(|r| (r.udid, r.battery.map_or_else(serde_json::Value::String, serde_json::Value::Object)))
            .collect();
        println!("{}", serde_json::to_string_pretty(&dump).unwrap());
        return ExitCode::SUCCESS;
    }
    if std::env::args().any(|a| a == "--dry-run") {
        return match collect() {
            Ok(p) => {
                println!("{}", serde_json::to_string_pretty(&p).unwrap());
                if let Ok(reader) = iphone::Reader::new() {
                    let mut errors = std::collections::HashMap::new();
                    for p in collect_iphones(&reader, &mut errors) {
                        println!("{}", serde_json::to_string_pretty(&p).unwrap());
                    }
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        };
    }

    // Fail fast on a machine without a battery; launchd won't restart a clean exit.
    if let Err(e) = collect() {
        eprintln!("error: {e}");
        return ExitCode::SUCCESS;
    }

    let tls = match native_tls::TlsConnector::new() {
        Ok(t) => std::sync::Arc::new(t),
        Err(e) => {
            eprintln!("TLS setup failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    // native-tls uses the macOS Security framework and the system keychain.
    let agent = ureq::AgentBuilder::new()
        .tls_connector(tls)
        .timeout(Duration::from_secs(20))
        .build();

    // Sending runs on its own thread, so a hung or offline network never
    // delays sampling; samples it can't deliver wait in the on-disk queue.
    let (samples, outbox) = mpsc::channel::<BatteryHealth>();
    std::thread::spawn(move || deliver(&agent, &outbox));

    let iphones = iphone::Reader::new().map_err(|e| eprintln!("iPhone sampling off: {e}")).ok();
    // Last error per iPhone, so a locked or untrusted phone is logged once, not every cycle.
    let mut iphone_errors = std::collections::HashMap::new();

    let (tx, rx) = mpsc::channel();
    iokit::on_power_source_change(tx);
    println!("started, sampling every {}s", interval().as_secs());

    let mut collect_failed = false;
    loop {
        match collect() {
            Ok(sample) => {
                collect_failed = false;
                if samples.send(sample).is_err() {
                    eprintln!("sender thread died");
                    return ExitCode::FAILURE;
                }
            }
            Err(e) if !collect_failed => {
                eprintln!("{e}");
                collect_failed = true;
            }
            Err(_) => {}
        }
        for sample in iphones.iter().flat_map(|r| collect_iphones(r, &mut iphone_errors)) {
            if samples.send(sample).is_err() {
                eprintln!("sender thread died");
                return ExitCode::FAILURE;
            }
        }
        // Wake on the timer or a power event; collapse bursts of events into one sample.
        let _ = rx.recv_timeout(interval());
        while rx.try_recv().is_ok() {}
    }
}

/// Sends samples as they arrive, oldest first: queued ones before new ones.
#[cfg(target_os = "macos")]
fn deliver(agent: &ureq::Agent, outbox: &mpsc::Receiver<BatteryHealth>) {
    let mut failing = false;
    while let Ok(first) = outbox.recv() {
        // Everything that piled up while the previous send was hanging.
        let batch: Vec<_> = std::iter::once(first).chain(outbox.try_iter()).collect();
        let result = match server_url().ok_or_else(|| format!("ServerURL is not set in {PREFS_PATH}")) {
            Err(e) => {
                batch.iter().for_each(queue::push);
                Err(e)
            }
            Ok(url) if queue::is_empty() => {
                let mut result = Ok(());
                for (i, sample) in batch.iter().enumerate() {
                    if let Err(e) = send(agent, &url, sample) {
                        batch[i..].iter().for_each(queue::push);
                        result = Err(e);
                        break;
                    }
                }
                result
            }
            Ok(url) => {
                batch.iter().for_each(queue::push);
                queue::flush(|q| send(agent, &url, q))
            }
        };
        match result {
            Ok(()) if failing => {
                println!("reporting recovered");
                failing = false;
            }
            Ok(()) => {}
            // Log only the first failure of a streak to keep the log small.
            Err(e) if !failing => {
                eprintln!("{e}");
                failing = true;
            }
            Err(_) => {}
        }
    }
}

#[cfg(target_os = "macos")]
fn send(agent: &ureq::Agent, url: &str, payload: &impl Serialize) -> Result<(), String> {
    let mut req = agent.post(url);
    if let Some(token) = pref("AuthToken").and_then(|v| v.into_string()).filter(|t| !t.is_empty()) {
        req = req.set("Authorization", &format!("Bearer {token}"));
    }
    match req.send_json(payload) {
        Ok(resp) if resp.status() == 201 => Ok(()),
        Ok(resp) => Err(format!("unexpected HTTP {} from {url}", resp.status())),
        Err(e) => Err(format!("failed to report to {url}: {e}")),
    }
}

#[cfg(target_os = "macos")]
fn pref(key: &str) -> Option<plist::Value> {
    plist::Value::from_file(PREFS_PATH).ok()?.as_dictionary()?.get(key).cloned()
}

#[cfg(target_os = "macos")]
fn server_url() -> Option<String> {
    std::env::var("BATTERY_AGENT_URL")
        .ok()
        .or_else(|| pref("ServerURL")?.into_string().filter(|u| !u.is_empty()))
        .map(|url| endpoint(&url))
}

#[cfg(target_os = "macos")]
fn interval() -> Duration {
    let secs = pref("IntervalSeconds")
        .and_then(|v| v.as_unsigned_integer())
        .unwrap_or(DEFAULT_INTERVAL_SECS);
    Duration::from_secs(secs.max(MIN_INTERVAL_SECS))
}

/// Accepts either the server's base URL or the full endpoint URL.
#[cfg(target_os = "macos")]
fn endpoint(url: &str) -> String {
    let base = url.trim_end_matches('/');
    if base.ends_with(ENDPOINT) { base.to_owned() } else { format!("{base}{ENDPOINT}") }
}

#[cfg(target_os = "macos")]
fn collect() -> Result<BatteryHealth, String> {
    let device_id = iokit::string_property("IOPlatformExpertDevice", "IOPlatformUUID")
        .ok_or("cannot read IOPlatformUUID")?;
    let b = iokit::properties("AppleSmartBattery").ok_or("no AppleSmartBattery (is this a laptop?)")?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as u32);
    battery_agent::payload::build(&device_id, &b, now)
}

/// Samples of the iPhones connected by USB. A device that can't be read is
/// logged when its error changes and logged again once it recovers.
#[cfg(target_os = "macos")]
fn collect_iphones(
    reader: &iphone::Reader,
    errors: &mut std::collections::HashMap<String, String>,
) -> Vec<BatteryHealth> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as u32);
    let mut samples = Vec::new();
    for reading in reader.read_all() {
        let sample = reading
            .battery
            .and_then(|b| battery_agent::payload::build(&reading.udid, &b, now));
        match sample {
            Ok(s) => {
                if errors.remove(&reading.udid).is_some() {
                    println!("iPhone {}: reading recovered", reading.udid);
                }
                samples.push(s);
            }
            Err(e) => {
                if errors.get(&reading.udid) != Some(&e) {
                    eprintln!("iPhone {}: {e}", reading.udid);
                    errors.insert(reading.udid, e);
                }
            }
        }
    }
    samples
}
