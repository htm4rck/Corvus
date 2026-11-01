CREATE SCHEMA IF NOT EXISTS corvus;
-- Compatible with the previous Flyway V1 event layout; migration is explicit, never in a request.
CREATE TABLE IF NOT EXISTS corvus.ingestion_events (
    id UUID PRIMARY KEY, tenant_id VARCHAR(80) NOT NULL, company_id VARCHAR(80) NOT NULL,
    event_id VARCHAR(120) NOT NULL,event_type VARCHAR(120) NOT NULL,event_version VARCHAR(40) NOT NULL,
    source_system VARCHAR(80) NOT NULL,entity_id VARCHAR(120),occurred_at TIMESTAMPTZ NOT NULL,
    ingested_at TIMESTAMPTZ NOT NULL,payload JSONB NOT NULL,quality_status VARCHAR(40) NOT NULL,
    UNIQUE(tenant_id,event_id)
);
CREATE INDEX IF NOT EXISTS idx_corvus_events_company_type
    ON corvus.ingestion_events(tenant_id,company_id,event_type,occurred_at);
CREATE TABLE corvus.resources (
    tenant_id TEXT NOT NULL,kind TEXT NOT NULL,id UUID NOT NULL,body JSONB NOT NULL,
    version BIGINT NOT NULL DEFAULT 1 CHECK(version>0),created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY(tenant_id,kind,id)
);
CREATE TABLE corvus.resource_versions (
    tenant_id TEXT NOT NULL,kind TEXT NOT NULL,id UUID NOT NULL,version BIGINT NOT NULL,
    body JSONB NOT NULL,PRIMARY KEY(tenant_id,kind,id,version),
    FOREIGN KEY(tenant_id,kind,id) REFERENCES corvus.resources(tenant_id,kind,id)
);
CREATE TABLE corvus.datasets (
    tenant_id TEXT NOT NULL,id UUID NOT NULL,body JSONB NOT NULL,
    PRIMARY KEY(tenant_id,id)
);

