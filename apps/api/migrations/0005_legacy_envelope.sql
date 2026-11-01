-- The first pilot stored an already-normalized envelope. It is not a bronze capture.
-- Keep it for historical evidence, but all new captures use corvus_bronze.batches.
ALTER TABLE corvus.sales_raw RENAME TO legacy_sales_raw;
COMMENT ON TABLE corvus.legacy_sales_raw IS 'Legacy normalized pilot envelopes, retained for history; not original bronze captures.';
