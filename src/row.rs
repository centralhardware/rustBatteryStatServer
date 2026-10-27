use battery_model::BatteryHealth;
use clickhouse::Row;
use serde::Serialize;

/// A row of ClickHouse `battery_health`. clickhouse 0.15 checks field names
/// against the table, so this keeps the snake_case column names that the
/// camelCase JSON model can't.
#[derive(Debug, Row, Serialize)]
pub struct BatteryRow {
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

impl From<BatteryHealth> for BatteryRow {
    fn from(h: BatteryHealth) -> Self {
        Self {
            date_time: h.date_time,
            device_id: h.device_id,
            cycle_count: h.cycle_count,
            health_percent: h.health_percent,
            current_charge: h.current_charge,
            temperature: h.temperature,
            is_charging: h.is_charging,
            design_capacity_mah: h.design_capacity_mah,
            max_capacity_mah: h.max_capacity_mah,
            voltage_mv: h.voltage_mv,
            current_ma: h.current_ma,
            avg_time_to_empty: h.avg_time_to_empty,
            avg_time_to_full: h.avg_time_to_full,
            external_connected: h.external_connected,
            fully_charged: h.fully_charged,
            nominal_charge_capacity: h.nominal_charge_capacity,
            raw_current_capacity: h.raw_current_capacity,
            raw_battery_voltage: h.raw_battery_voltage,
            virtual_temperature: h.virtual_temperature,
            cell_voltage_1: h.cell_voltage_1,
            cell_voltage_2: h.cell_voltage_2,
            cell_voltage_3: h.cell_voltage_3,
            at_critical_level: h.at_critical_level,
            battery_cell_disconnect_count: h.battery_cell_disconnect_count,
            adapter_watts: h.adapter_watts,
            adapter_name: h.adapter_name,
            adapter_voltage: h.adapter_voltage,
            design_cycle_count: h.design_cycle_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_names_are_snake_case() {
        let names = <BatteryRow as Row>::COLUMN_NAMES;
        assert_eq!(names.len(), 28);
        assert!(names.contains(&"cell_voltage_3"));
        assert!(names.iter().all(|n| !n.chars().any(char::is_uppercase)));
    }
}
