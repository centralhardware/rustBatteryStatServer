ALTER TABLE battery_health
    ADD COLUMN avg_time_to_empty UInt16,
    ADD COLUMN avg_time_to_full UInt16,
    ADD COLUMN external_connected LowCardinality(UInt8),
    ADD COLUMN fully_charged LowCardinality(UInt8),
    ADD COLUMN nominal_charge_capacity UInt16,
    ADD COLUMN raw_current_capacity UInt16,
    ADD COLUMN raw_battery_voltage UInt16,
    ADD COLUMN virtual_temperature Float32,
    ADD COLUMN cell_voltage_1 UInt16,
    ADD COLUMN cell_voltage_2 UInt16,
    ADD COLUMN cell_voltage_3 UInt16,
    ADD COLUMN at_critical_level LowCardinality(UInt8),
    ADD COLUMN battery_cell_disconnect_count LowCardinality(UInt16),
    ADD COLUMN adapter_watts LowCardinality(UInt16),
    ADD COLUMN adapter_name LowCardinality(String),
    ADD COLUMN adapter_voltage UInt32,
    ADD COLUMN design_cycle_count LowCardinality(UInt16)
SETTINGS allow_suspicious_low_cardinality_types=1;
