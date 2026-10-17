-- Agents post one row per sample; a Buffer in front of battery_health batches
-- them in memory so MergeTree gets fewer, larger parts. Flushes after 10–100 s,
-- 10k–1M rows or 10–100 MB, whichever hits first. Rows not yet flushed are lost
-- if ClickHouse dies. Reads through the buffer see both flushed and pending rows.
-- The setting is needed because battery_health already has LowCardinality(UInt16)
-- columns (V4), which CREATE refuses by default.
CREATE TABLE IF NOT EXISTS battery_health_buffer AS battery_health
ENGINE = Buffer(currentDatabase(), battery_health, 1, 10, 100, 10000, 1000000, 10000000, 100000000)
SETTINGS allow_suspicious_low_cardinality_types = 1;
