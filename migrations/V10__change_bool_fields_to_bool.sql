ALTER TABLE battery_health
    MODIFY COLUMN external_connected Bool,
    MODIFY COLUMN fully_charged Bool,
    MODIFY COLUMN at_critical_level Bool;
