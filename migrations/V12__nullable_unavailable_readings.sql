-- macOS 27 no longer exposes temperature, cell voltages or instant current;
-- store them as NULL instead of a fake 0. current_ma widened past ±32 A,
-- is_charging made Bool like the other flags.
ALTER TABLE battery_health
    MODIFY COLUMN is_charging Bool,
    MODIFY COLUMN current_ma Nullable(Int32),
    MODIFY COLUMN temperature Nullable(Float32),
    MODIFY COLUMN virtual_temperature Nullable(Float32),
    MODIFY COLUMN cell_voltage_1 Nullable(UInt16),
    MODIFY COLUMN cell_voltage_2 Nullable(UInt16),
    MODIFY COLUMN cell_voltage_3 Nullable(UInt16);

-- A temperature or cell voltage of exactly 0 was always "not reported".
ALTER TABLE battery_health
    UPDATE temperature = NULL, virtual_temperature = NULL
    WHERE temperature = 0
    SETTINGS mutations_sync = 1;
ALTER TABLE battery_health
    UPDATE cell_voltage_1 = NULL, cell_voltage_2 = NULL, cell_voltage_3 = NULL
    WHERE cell_voltage_1 = 0
    SETTINGS mutations_sync = 1;
