use battery_agent::payload;
use serde_json::{Map, Value, json};

/// The JSON the server receives.
fn build(device_id: &str, b: &Map<String, Value>, now: u32) -> Result<Value, String> {
    payload::build(device_id, b, now).map(|p| serde_json::to_value(p).unwrap())
}

fn fixture(name: &str) -> Map<String, Value> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// macOS 27 keeps capacities in BatteryData and publishes no temperature,
/// cell voltages or instant current (from a real MacBook Air, 2026-10-13).
#[test]
fn macos_27_reads_battery_data_and_nulls_missing_readings() {
    let p = build("DEVICE", &fixture("macos-27.2-macbook-air.json"), 1_791_918_000).unwrap();

    assert_eq!(p["date_time"], 1_791_918_000);
    assert_eq!(p["cycle_count"], 241);
    assert_eq!(p["health_percent"], 92, "System Information shows 92%");
    assert_eq!(p["current_charge"], 80);
    assert_eq!(p["design_capacity_mah"], 4629);
    assert_eq!(p["max_capacity_mah"], 4097);
    assert_eq!(p["nominal_charge_capacity"], 4224);
    assert_eq!(p["raw_current_capacity"], 3211);
    assert_eq!(p["avg_time_to_empty"], 0, "65535 means no estimate");
    assert_eq!(p["external_connected"], true);
    assert_eq!(p["adapter_name"], "35W USB-C Power Adapter");
    for key in ["temperature", "virtualTemperature", "currentMa", "cellVoltage1", "cellVoltage2", "cellVoltage3"] {
        assert_eq!(p[key], Value::Null, "{key}");
    }
}

/// Older macOS publishes the readings at the top level.
#[test]
fn top_level_readings() {
    let b = json!({
        "CycleCount": 10, "CurrentCapacity": 50, "MaxCapacity": 100,
        "Temperature": 3050, "InstantAmperage": -1500, "MaximumCapacityPercent": 99,
        "BatteryData": {"CellVoltage": [4100, 4101, 4102]}
    });
    let p = build("D", b.as_object().unwrap(), 0).unwrap();
    assert_eq!(p["temperature"], 30.5);
    assert_eq!(p["current_ma"], -1500);
    assert_eq!(p["health_percent"], 99);
    assert_eq!(p["cell_voltage_3"], 4102);
}

#[test]
fn no_cycle_count_is_an_error() {
    assert!(build("D", &Map::new(), 0).is_err());
}

/// An iPhone read over USB (`--dump-iphone`, 2026-10-14, identifiers and the
/// bulky diagnostic blocks trimmed): the same keys as macOS 27, capacities in
/// BatteryData, no temperature, and an AdapterDetails without a Name.
#[test]
fn iphone_over_usb() {
    let p = build("UDID", &fixture("ios-iphone-usb.json"), 1_791_965_095).unwrap();

    assert_eq!(p["device_id"], "UDID");
    assert_eq!(p["cycle_count"], 569);
    assert_eq!(p["current_charge"], 73);
    assert_eq!(p["design_capacity_mah"], 3544);
    assert_eq!(p["max_capacity_mah"], 3250);
    assert_eq!(p["nominal_charge_capacity"], 3170);
    assert_eq!(p["health_percent"], 90, "3170 / 3544 mAh, rounded up");
    assert_eq!(p["raw_current_capacity"], 2365);
    assert_eq!(p["voltage_mv"], 4244);
    assert_eq!(p["current_ma"], 1490);
    assert_eq!(p["avg_time_to_empty"], 876);
    assert_eq!(p["is_charging"], true);
    assert_eq!(p["external_connected"], true);
    assert_eq!(p["adapter_watts"], 15);
    assert_eq!(p["adapter_voltage"], 5000);
    assert_eq!(p["adapter_name"], "");
    assert_eq!(p["temperature"], Value::Null);
}
