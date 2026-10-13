ALTER TABLE battery_health
    ADD COLUMN current_charge UInt8,
    ADD COLUMN temperature Int16,
    ADD COLUMN is_charging UInt8;
