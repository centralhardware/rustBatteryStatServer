//! Reads AppleSmartBattery via IOKit and POSTs one sample to BatteryStatServer.
//! launchd runs it periodically as a root LaunchDaemon (see packaging/).
//!
//! Server URL: $BATTERY_AGENT_URL, else `ServerURL` in
//! /Library/Preferences/me.centralhardware.battery-agent.plist, else the default.
//! `--dry-run` prints the payload without sending it.

mod iokit;

use serde_json::{Map, Value, json};
use std::process::ExitCode;
use std::time::Duration;

const DEFAULT_URL: &str = "https://bs.centralhardware.me/api/battery/health";
const PREFS_PATH: &str = "/Library/Preferences/me.centralhardware.battery-agent.plist";
/// The gas gauge reports 65535 when a time estimate is unavailable.
const NO_ESTIMATE: i64 = 65535;

fn main() -> ExitCode {
    let dry_run = std::env::args().any(|a| a == "--dry-run");

    let payload = match collect() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if dry_run {
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
        return ExitCode::SUCCESS;
    }

    let url = server_url();
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
        .timeout(Duration::from_secs(30))
        .build();
    match agent.post(&url).send_json(&payload) {
        Ok(resp) if resp.status() == 201 => {
            println!("reported: charge {}%, health {}%", payload["currentCharge"], payload["healthPercent"]);
            ExitCode::SUCCESS
        }
        Ok(resp) => {
            eprintln!("unexpected HTTP {} from {url}", resp.status());
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("failed to report to {url}: {e}");
            ExitCode::FAILURE
        }
    }
}

fn server_url() -> String {
    if let Ok(url) = std::env::var("BATTERY_AGENT_URL") {
        return url;
    }
    plist::Value::from_file(PREFS_PATH)
        .ok()
        .and_then(|v| v.as_dictionary()?.get("ServerURL")?.as_string().map(str::to_owned))
        .unwrap_or_else(|| DEFAULT_URL.to_owned())
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
