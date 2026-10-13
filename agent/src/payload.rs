//! Turns AppleSmartBattery registry properties into a [`BatteryHealth`] sample.
//! Pure, so it is tested on any platform against saved `--dump` output.

use battery_model::BatteryHealth;
use serde_json::{Map, Value};

/// The gas gauge reports 65535 when a time estimate is unavailable.
const NO_ESTIMATE: i64 = 65535;

/// `b`: AppleSmartBattery properties; `now`: sampling time, unix seconds.
/// Readings this macOS doesn't publish are `None`, not 0.
pub fn build(device_id: &str, b: &Map<String, Value>, now: u32) -> Result<BatteryHealth, String> {
    let empty = Map::new();
    // macOS 27 moved most gauge readings from the top level into BatteryData.
    let data = b.get("BatteryData").and_then(Value::as_object).unwrap_or(&empty);
    let get = |key: &str| int(b, key).or_else(|| int(data, key));
    let adapter = b.get("AdapterDetails").and_then(Value::as_object).unwrap_or(&empty);

    let cycle_count = get("CycleCount").ok_or("no CycleCount")?;
    let design = get("DesignCapacity").unwrap_or(0);
    let raw_max = get("AppleRawMaxCapacity").unwrap_or(0);

    let current = get("CurrentCapacity").unwrap_or(0);
    let max = get("MaxCapacity").unwrap_or(0);
    let current_charge = if max > 0 { current * 100 / max } else { 0 };

    let cells: Vec<i64> = data
        .get("CellVoltage")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default();
    let cell = |i: usize| cells.get(i).copied().filter(|&v| v > 0).map(fit);

    let time_estimate = |key| fit(get(key).filter(|&v| v != NO_ESTIMATE).unwrap_or(0));

    Ok(BatteryHealth {
        date_time: now,
        device_id: device_id.to_owned(),
        cycle_count: fit(cycle_count),
        health_percent: fit(health_percent(&get, design, raw_max)),
        current_charge: fit(current_charge),
        temperature: centi(b, "Temperature"),
        is_charging: flag(b, "IsCharging"),
        design_capacity_mah: fit(design),
        max_capacity_mah: fit(raw_max),
        voltage_mv: fit(get("Voltage").unwrap_or(0)),
        current_ma: get("InstantAmperage").map(fit),
        avg_time_to_empty: time_estimate("AvgTimeToEmpty"),
        avg_time_to_full: time_estimate("AvgTimeToFull"),
        external_connected: flag(b, "ExternalConnected"),
        fully_charged: flag(b, "FullyCharged"),
        nominal_charge_capacity: fit(get("NominalChargeCapacity").unwrap_or(0)),
        raw_current_capacity: fit(get("AppleRawCurrentCapacity").unwrap_or(0)),
        raw_battery_voltage: fit(get("AppleRawBatteryVoltage").unwrap_or(0)),
        virtual_temperature: centi(b, "VirtualTemperature"),
        cell_voltage_1: cell(0),
        cell_voltage_2: cell(1),
        cell_voltage_3: cell(2),
        at_critical_level: flag(b, "AtCriticalLevel"),
        battery_cell_disconnect_count: fit(get("BatteryCellDisconnectCount").unwrap_or(0)),
        adapter_watts: fit(int(adapter, "Watts").unwrap_or(0)),
        adapter_name: adapter.get("Name").and_then(Value::as_str).unwrap_or("").trim().to_owned(),
        adapter_voltage: fit(int(adapter, "AdapterVoltage").unwrap_or(0)),
        design_cycle_count: fit(get("DesignCycleCount9C").unwrap_or(0)),
    })
}

/// System Information's "Maximum Capacity": newer macOS publishes it directly,
/// otherwise it is nominal / design capacity rounded up (4224 / 4629 mAh → 92%).
fn health_percent(get: &impl Fn(&str) -> Option<i64>, design: i64, raw_max: i64) -> i64 {
    get("MaximumCapacityPercent").unwrap_or_else(|| {
        let nominal = get("NominalChargeCapacity").filter(|&n| n > 0).unwrap_or(raw_max);
        if design > 0 { ((nominal * 100 + design - 1) / design).min(100) } else { 0 }
    })
}

/// Narrows a registry integer to the column type, clamping out-of-range values.
fn fit<T: TryFrom<i64> + Bounded>(v: i64) -> T {
    T::try_from(v).unwrap_or(if v < 0 { T::MIN } else { T::MAX })
}

trait Bounded {
    const MIN: Self;
    const MAX: Self;
}

macro_rules! bounded {
    ($($t:ty),*) => { $(impl Bounded for $t { const MIN: Self = <$t>::MIN; const MAX: Self = <$t>::MAX; })* };
}
bounded!(u8, u16, u32, i32);

fn int(m: &Map<String, Value>, key: &str) -> Option<i64> {
    m.get(key)?.as_i64()
}

fn flag(m: &Map<String, Value>, key: &str) -> bool {
    m.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// Temperatures are reported in hundredths of a degree Celsius.
fn centi(m: &Map<String, Value>, key: &str) -> Option<f32> {
    int(m, key).map(|v| v as f32 / 100.0)
}
