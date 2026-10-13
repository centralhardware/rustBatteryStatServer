//! Reads AppleSmartBattery via IOKit and POSTs samples to BatteryStatServer.
//! Runs resident as a root LaunchDaemon (see packaging/): one sample every
//! 30 seconds, plus one immediately whenever the power source changes.
//!
//! Server URL: $BATTERY_AGENT_URL, else `ServerURL` in
//! /Library/Preferences/me.centralhardware.battery-agent.plist (the installer
//! asks for it). It is re-read every cycle, so changing it needs no restart.
//! `--dry-run` prints one payload without sending it.

mod iokit;

use serde_json::{Map, Value, json};
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::Duration;

const PREFS_PATH: &str = "/Library/Preferences/me.centralhardware.battery-agent.plist";
const INTERVAL: Duration = Duration::from_secs(30);
/// The gas gauge reports 65535 when a time estimate is unavailable.
const NO_ESTIMATE: i64 = 65535;

fn main() -> ExitCode {
    if std::env::args().any(|a| a == "--dry-run") {
        return match collect() {
            Ok(p) => {
                println!("{}", serde_json::to_string_pretty(&p).unwrap());
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

    let (tx, rx) = mpsc::channel();
    iokit::on_power_source_change(tx);
    println!("started, sampling every {}s", INTERVAL.as_secs());

    let mut failing = false;
    loop {
        let result = server_url()
            .ok_or_else(|| format!("ServerURL is not set in {PREFS_PATH}"))
            .and_then(|url| collect().and_then(|p| send(&agent, &url, &p)));
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
        // Wake on the timer or a power event; collapse bursts of events into one sample.
        let _ = rx.recv_timeout(INTERVAL);
        while rx.try_recv().is_ok() {}
    }
}

fn send(agent: &ureq::Agent, url: &str, payload: &Value) -> Result<(), String> {
    match agent.post(url).send_json(payload) {
        Ok(resp) if resp.status() == 201 => Ok(()),
        Ok(resp) => Err(format!("unexpected HTTP {} from {url}", resp.status())),
        Err(e) => Err(format!("failed to report to {url}: {e}")),
    }
}

fn server_url() -> Option<String> {
    std::env::var("BATTERY_AGENT_URL").ok().or_else(|| {
        plist::Value::from_file(PREFS_PATH)
            .ok()?
            .as_dictionary()?
            .get("ServerURL")?
            .as_string()
            .filter(|u| !u.is_empty())
            .map(str::to_owned)
    })
}

fn collect() -> Result<Value, String> {
    let device_id = iokit::string_property("IOPlatformExpertDevice", "IOPlatformUUID")
        .ok_or("cannot read IOPlatformUUID")?;
    let b = iokit::properties("AppleSmartBattery").ok_or("no AppleSmartBattery (is this a laptop?)")?;
    let empty = Map::new();
    let data = b.get("BatteryData").and_then(Value::as_object).unwrap_or(&empty);
    let adapter = b.get("AdapterDetails").and_then(Value::as_object).unwrap_or(&empty);

    let cycle_count = int(&b, "CycleCount").ok_or("no CycleCount")?;
    let design = int(&b, "DesignCapacity").unwrap_or(0);
    let raw_max = int(&b, "AppleRawMaxCapacity").unwrap_or(0);

    let current = int(&b, "CurrentCapacity").unwrap_or(0);
    let max = int(&b, "MaxCapacity").unwrap_or(0);
    let current_charge = if max > 0 { current * 100 / max } else { 0 };

    let cells: Vec<i64> = data
        .get("CellVoltage")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default();
    let cell = |i: usize| cells.get(i).copied().unwrap_or(0);

    let time_estimate = |key| int(&b, key).filter(|&v| v != NO_ESTIMATE).unwrap_or(0);

    Ok(json!({
        "deviceId": device_id,
        "cycleCount": cycle_count,
        "healthPercent": health_percent(&b, data, design, raw_max),
        "currentCharge": current_charge,
        "temperature": centi(&b, "Temperature"),
        "isCharging": flag(&b, "IsCharging"),
        "designCapacityMah": design,
        "maxCapacityMah": raw_max,
        "voltageMv": int(&b, "Voltage").unwrap_or(0),
        "currentMa": int(&b, "InstantAmperage").unwrap_or(0),
        "avgTimeToEmpty": time_estimate("AvgTimeToEmpty"),
        "avgTimeToFull": time_estimate("AvgTimeToFull"),
        "externalConnected": flag(&b, "ExternalConnected"),
        "fullyCharged": flag(&b, "FullyCharged"),
        "nominalChargeCapacity": int(&b, "NominalChargeCapacity").unwrap_or(0),
        "rawCurrentCapacity": int(&b, "AppleRawCurrentCapacity").unwrap_or(0),
        "rawBatteryVoltage": int(&b, "AppleRawBatteryVoltage").unwrap_or(0),
        "virtualTemperature": centi(&b, "VirtualTemperature"),
        "cellVoltage1": cell(0),
        "cellVoltage2": cell(1),
        "cellVoltage3": cell(2),
        "atCriticalLevel": flag(&b, "AtCriticalLevel"),
        "batteryCellDisconnectCount": int(&b, "BatteryCellDisconnectCount").unwrap_or(0),
        "adapterWatts": int(adapter, "Watts").unwrap_or(0),
        "adapterName": adapter.get("Name").and_then(Value::as_str).unwrap_or("").trim(),
        "adapterVoltage": int(adapter, "AdapterVoltage").unwrap_or(0),
        "designCycleCount": int(&b, "DesignCycleCount9C").unwrap_or(0),
    }))
}

/// The "Maximum Capacity" figure System Information shows. Newer macOS
/// publishes it directly; otherwise derive it from raw vs design capacity.
fn health_percent(b: &Map<String, Value>, data: &Map<String, Value>, design: i64, raw_max: i64) -> i64 {
    int(b, "MaximumCapacityPercent")
        .or_else(|| int(data, "MaximumCapacityPercent"))
        .unwrap_or_else(|| {
            let nominal = int(b, "NominalChargeCapacity").filter(|&n| n > 0).unwrap_or(raw_max);
            if design > 0 { ((nominal * 100 + design / 2) / design).min(100) } else { 0 }
        })
}

fn int(m: &Map<String, Value>, key: &str) -> Option<i64> {
    m.get(key)?.as_i64()
}

fn flag(m: &Map<String, Value>, key: &str) -> bool {
    m.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// Temperatures are reported in hundredths of a degree Celsius.
fn centi(m: &Map<String, Value>, key: &str) -> f64 {
    int(m, key).map(|v| v as f64 / 100.0).unwrap_or(0.0)
}
