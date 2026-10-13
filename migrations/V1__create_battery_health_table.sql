CREATE TABLE IF NOT EXISTS battery_health (
    date_time DateTime,
    device_id String,
    cycle_count UInt32,
    health_percent UInt8,
    manufacture_date Nullable(String)
) ENGINE = MergeTree()
ORDER BY (device_id, date_time);
