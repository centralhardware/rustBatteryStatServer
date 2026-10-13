ALTER TABLE battery_health
    ADD COLUMN design_capacity_mah LowCardinality(UInt16),
    ADD COLUMN max_capacity_mah UInt16
SETTINGS allow_suspicious_low_cardinality_types=1;
