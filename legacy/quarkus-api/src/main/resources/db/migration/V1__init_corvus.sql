CREATE SCHEMA IF NOT EXISTS corvus;

CREATE TABLE IF NOT EXISTS corvus.ingestion_events (
    id UUID PRIMARY KEY,
    tenant_id VARCHAR(80) NOT NULL,
    company_id VARCHAR(80) NOT NULL,
    event_id VARCHAR(120) NOT NULL,
    event_type VARCHAR(120) NOT NULL,
    event_version VARCHAR(40) NOT NULL,
    source_system VARCHAR(80) NOT NULL,
    entity_id VARCHAR(120),
    occurred_at TIMESTAMPTZ NOT NULL,
    ingested_at TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL,
    quality_status VARCHAR(40) NOT NULL,
    UNIQUE (tenant_id, event_id)
);

CREATE TABLE IF NOT EXISTS corvus.import_batches (
    id UUID PRIMARY KEY,
    tenant_id VARCHAR(80) NOT NULL,
    company_id VARCHAR(80) NOT NULL,
    domain VARCHAR(80) NOT NULL,
    source_system VARCHAR(80) NOT NULL,
    period_from DATE,
    period_to DATE,
    aggregation_level VARCHAR(40) NOT NULL,
    original_file VARCHAR(500),
    checksum VARCHAR(128),
    schema_version VARCHAR(40) NOT NULL,
    received_records INTEGER NOT NULL DEFAULT 0,
    valid_records INTEGER NOT NULL DEFAULT 0,
    rejected_records INTEGER NOT NULL DEFAULT 0,
    status VARCHAR(40) NOT NULL,
    uploaded_by VARCHAR(160),
    uploaded_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_corvus_events_company_type
    ON corvus.ingestion_events (tenant_id, company_id, event_type, occurred_at);

CREATE INDEX IF NOT EXISTS idx_corvus_import_batches_company_domain
    ON corvus.import_batches (tenant_id, company_id, domain, status);
