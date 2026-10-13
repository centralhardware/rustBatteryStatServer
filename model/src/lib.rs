//! One battery sample: the JSON body of `POST /api/battery/health` (camelCase)
//! and a row of ClickHouse `battery_health` (snake_case, field order = columns).
//! `None` marks a reading this Mac doesn't publish.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "clickhouse", derive(clickhouse::Row))]
#[serde(rename_all = "camelCase")]
pub struct BatteryHealth {
    /// Sampling time, unix seconds (ClickHouse `DateTime`). The agent may deliver
    /// queued samples late; 0 or missing means "now" to the server.
    #[serde(default)]
    pub date_time: u32,
    pub device_id: String,
    pub cycle_count: u32,
    pub health_percent: u8,
    pub current_charge: u8,
    pub temperature: Option<f32>,
    pub is_charging: bool,
    pub design_capacity_mah: u16,
    pub max_capacity_mah: u16,
    pub voltage_mv: u16,
    pub current_ma: Option<i32>,
    pub avg_time_to_empty: u16,
    pub avg_time_to_full: u16,
    pub external_connected: bool,
    pub fully_charged: bool,
    pub nominal_charge_capacity: u16,
    pub raw_current_capacity: u16,
    pub raw_battery_voltage: u16,
    pub virtual_temperature: Option<f32>,
    pub cell_voltage_1: Option<u16>,
    pub cell_voltage_2: Option<u16>,
    pub cell_voltage_3: Option<u16>,
    pub at_critical_level: bool,
    pub battery_cell_disconnect_count: u8,
    pub adapter_watts: u16,
    pub adapter_name: String,
    pub adapter_voltage: u32,
    pub design_cycle_count: u16,
}
