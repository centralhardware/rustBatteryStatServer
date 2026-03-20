use clickhouse::Row;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

mod bool_as_u8 {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &u8, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(*v)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u8, D::Error> {
        Ok(bool::deserialize(d)? as u8)
    }
}

#[derive(Debug, Deserialize, Serialize, Row)]
#[serde(rename_all = "camelCase")]
pub struct BatteryHealth {
    #[serde(
        default = "OffsetDateTime::now_utc",
        with = "clickhouse::serde::time::datetime"
    )]
    pub date_time: OffsetDateTime,
    pub device_id: String,
    pub cycle_count: u32,
    pub health_percent: u8,
    pub current_charge: u8,
    pub temperature: f32,
    #[serde(with = "bool_as_u8")]
    pub is_charging: u8,
    pub design_capacity_mah: u16,
    pub max_capacity_mah: u16,
    pub voltage_mv: u16,
    pub current_ma: i16,
    pub avg_time_to_empty: u16,
    pub avg_time_to_full: u16,
    pub external_connected: bool,
    pub fully_charged: bool,
    pub nominal_charge_capacity: u16,
    pub raw_current_capacity: u16,
    pub raw_battery_voltage: u16,
    pub virtual_temperature: f32,
    pub cell_voltage_1: u16,
    pub cell_voltage_2: u16,
    pub cell_voltage_3: u16,
    pub at_critical_level: bool,
    pub battery_cell_disconnect_count: u8,
    pub adapter_watts: u16,
    pub adapter_name: String,
    pub adapter_voltage: u32,
    pub design_cycle_count: u16,
}
