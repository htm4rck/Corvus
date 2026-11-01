-- A run key belongs to a pipeline within a company, not to the whole installation.
ALTER TABLE corvus.ingestion_runs DROP CONSTRAINT ingestion_runs_pkey;
ALTER TABLE corvus.ingestion_runs DROP CONSTRAINT ingestion_runs_id_key;
ALTER TABLE corvus.ingestion_runs ADD PRIMARY KEY(id);
ALTER TABLE corvus.ingestion_runs ADD UNIQUE(tenant_id,pipeline_id,run_key);
ALTER TABLE corvus.ingestion_runs ADD UNIQUE(tenant_id,pipeline_id,id);
ALTER TABLE corvus.sales_snapshots ADD UNIQUE(tenant_id,pipeline_id,id);
ALTER TABLE corvus.sales_dataset_heads ADD FOREIGN KEY(tenant_id,pipeline_id,snapshot_id) REFERENCES corvus.sales_snapshots(tenant_id,pipeline_id,id);
ALTER TABLE corvus.sales_checkpoints ADD FOREIGN KEY(tenant_id,pipeline_id,snapshot_id) REFERENCES corvus.sales_snapshots(tenant_id,pipeline_id,id);
ALTER TABLE corvus.ingestion_runs ADD FOREIGN KEY(tenant_id,pipeline_id,snapshot_id) REFERENCES corvus.sales_snapshots(tenant_id,pipeline_id,id);
ALTER TABLE corvus_bronze.batches ADD FOREIGN KEY(tenant_id,pipeline_id,run_id) REFERENCES corvus.ingestion_runs(tenant_id,pipeline_id,id);
