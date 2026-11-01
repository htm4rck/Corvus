-- Preserve the existing pilot facts while replacing tenant-named product concepts.
CREATE TABLE corvus.pipelines(tenant_id text NOT NULL,id uuid NOT NULL,body jsonb NOT NULL,version bigint NOT NULL DEFAULT 1,PRIMARY KEY(tenant_id,id));
INSERT INTO corvus.pipelines(tenant_id,id,body)
SELECT DISTINCT ON(tenant_id) tenant_id,md5('sales-documents:'||tenant_id)::uuid,
 jsonb_build_object('id',md5('sales-documents:'||tenant_id)::uuid,'name','Comprobantes de venta','domain','sales_documents','sourceRef',manifest->>'sourceCode','version',1,'createdAt',manifest->>'publishedAt')
FROM corvus.sales_snapshots ORDER BY tenant_id,version DESC;
ALTER TABLE corvus.sales_snapshots ADD COLUMN pipeline_id uuid;
UPDATE corvus.sales_snapshots SET pipeline_id=md5('sales-documents:'||tenant_id)::uuid;
ALTER TABLE corvus.sales_snapshots ALTER COLUMN pipeline_id SET NOT NULL;
ALTER TABLE corvus.sales_snapshots ADD FOREIGN KEY(tenant_id,pipeline_id) REFERENCES corvus.pipelines(tenant_id,id);
ALTER TABLE corvus.sales_snapshots DROP CONSTRAINT sales_snapshots_tenant_id_version_key;
ALTER TABLE corvus.sales_snapshots ADD UNIQUE(tenant_id,pipeline_id,version);
-- Metadata correction only: preserve snapshot IDs, facts, captured times and hashes.
UPDATE corvus.sales_snapshots SET manifest=jsonb_set(jsonb_set(manifest,'{datasetCode}','"sales.v1"'),'{pipelineId}',to_jsonb(pipeline_id));
ALTER TABLE corvus.sales_dataset_heads ADD COLUMN pipeline_id uuid;
UPDATE corvus.sales_dataset_heads SET pipeline_id=md5('sales-documents:'||tenant_id)::uuid;
DELETE FROM corvus.sales_dataset_heads h WHERE NOT EXISTS(SELECT 1 FROM corvus.pipelines p WHERE p.tenant_id=h.tenant_id AND p.id=h.pipeline_id);
ALTER TABLE corvus.sales_dataset_heads ALTER COLUMN pipeline_id SET NOT NULL;
ALTER TABLE corvus.sales_dataset_heads DROP CONSTRAINT sales_dataset_heads_pkey;
ALTER TABLE corvus.sales_dataset_heads ADD PRIMARY KEY(tenant_id,pipeline_id);
ALTER TABLE corvus.sales_dataset_heads ADD FOREIGN KEY(tenant_id,pipeline_id) REFERENCES corvus.pipelines(tenant_id,id);
ALTER TABLE corvus.ingestion_runs ADD COLUMN pipeline_id uuid;
UPDATE corvus.ingestion_runs SET pipeline_id=md5('sales-documents:'||tenant_id)::uuid WHERE EXISTS(SELECT 1 FROM corvus.pipelines p WHERE p.tenant_id=ingestion_runs.tenant_id);
-- Failed legacy requests that never had a pipeline remain historical, not executable definitions.
ALTER TABLE corvus.sales_checkpoints ADD COLUMN pipeline_id uuid;
UPDATE corvus.sales_checkpoints SET pipeline_id=md5('sales-documents:'||tenant_id)::uuid;
ALTER TABLE corvus.sales_checkpoints ALTER COLUMN pipeline_id SET NOT NULL;
ALTER TABLE corvus.sales_checkpoints DROP CONSTRAINT sales_checkpoints_pkey;
ALTER TABLE corvus.sales_checkpoints ADD PRIMARY KEY(tenant_id,pipeline_id);
CREATE SCHEMA corvus_bronze;
CREATE TABLE corvus_bronze.batches(
 tenant_id text NOT NULL,pipeline_id uuid NOT NULL,run_id uuid NOT NULL,attempt uuid NOT NULL,
 body jsonb NOT NULL,content_hash text NOT NULL,captured_at timestamptz NOT NULL,stored_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(tenant_id,run_id,attempt),FOREIGN KEY(tenant_id,pipeline_id) REFERENCES corvus.pipelines(tenant_id,id)
);
CREATE SCHEMA corvus_silver;
ALTER TABLE corvus.invoice_headers SET SCHEMA corvus_silver;
ALTER TABLE corvus.invoice_lines SET SCHEMA corvus_silver;
ALTER TABLE corvus.order_headers SET SCHEMA corvus_silver;
ALTER TABLE corvus.order_lines SET SCHEMA corvus_silver;
ALTER TABLE corvus.order_invoice_links SET SCHEMA corvus_silver;
CREATE SCHEMA corvus_gold;
CREATE TABLE corvus_gold.billing_daily(
 tenant_id text NOT NULL,snapshot_id uuid NOT NULL,business_date date NOT NULL,origin_application text NOT NULL,sales_channel text NOT NULL,
 currency text NOT NULL,document_type text NOT NULL,fiscal_environment text NOT NULL,document_count bigint NOT NULL,tax_base numeric NOT NULL,tax_amount numeric NOT NULL,billed_total numeric NOT NULL,
 PRIMARY KEY(tenant_id,snapshot_id,business_date,origin_application,sales_channel,currency,document_type,fiscal_environment),
 FOREIGN KEY(tenant_id,snapshot_id) REFERENCES corvus.sales_snapshots(tenant_id,id)
);
CREATE TABLE corvus_gold.billing_products(
 tenant_id text NOT NULL,snapshot_id uuid NOT NULL,business_date date NOT NULL,product_code text NOT NULL,product_name text NOT NULL,unit_code text NOT NULL,
 currency text NOT NULL,origin_application text NOT NULL,document_type text NOT NULL,fiscal_environment text NOT NULL,line_class text NOT NULL,
 quantity numeric NOT NULL,tax_base numeric NOT NULL,tax_amount numeric NOT NULL,billed_total numeric NOT NULL,
 PRIMARY KEY(tenant_id,snapshot_id,business_date,product_code,unit_code,currency,origin_application,document_type,fiscal_environment,line_class),
 FOREIGN KEY(tenant_id,snapshot_id) REFERENCES corvus.sales_snapshots(tenant_id,id)
);
CREATE TABLE corvus_gold.order_coverage(
 tenant_id text NOT NULL,snapshot_id uuid NOT NULL,business_date date NOT NULL,origin_application text NOT NULL,sales_channel text NOT NULL,currency text NOT NULL,
 coverage text NOT NULL,order_count bigint NOT NULL,intent_amount numeric NOT NULL,
 PRIMARY KEY(tenant_id,snapshot_id,business_date,origin_application,sales_channel,currency,coverage),
 FOREIGN KEY(tenant_id,snapshot_id) REFERENCES corvus.sales_snapshots(tenant_id,id)
);
CREATE FUNCTION corvus.build_sales_gold(p_tenant text,p_snapshot uuid) RETURNS void LANGUAGE sql AS $$
 INSERT INTO corvus_gold.billing_daily
 SELECT tenant_id,snapshot_id,business_date,origin_application,sales_channel,currency,document_type,fiscal_environment,count(*),sum(tax_base),sum(tax_amount),sum(total)
 FROM corvus_silver.invoice_headers WHERE tenant_id=p_tenant AND snapshot_id=p_snapshot AND included_in_billing
 GROUP BY tenant_id,snapshot_id,business_date,origin_application,sales_channel,currency,document_type,fiscal_environment;
 INSERT INTO corvus_gold.billing_products
 SELECT h.tenant_id,h.snapshot_id,h.business_date,l.product_code,max(l.product_name),l.unit_code,h.currency,h.origin_application,h.document_type,h.fiscal_environment,l.line_class,sum(l.quantity),sum(l.tax_base),sum(l.tax_amount),sum(l.total)
 FROM corvus_silver.invoice_lines l JOIN corvus_silver.invoice_headers h ON h.tenant_id=l.tenant_id AND h.snapshot_id=l.snapshot_id AND h.id=l.invoice_id
 WHERE h.tenant_id=p_tenant AND h.snapshot_id=p_snapshot AND h.included_in_billing
 GROUP BY h.tenant_id,h.snapshot_id,h.business_date,l.product_code,l.unit_code,h.currency,h.origin_application,h.document_type,h.fiscal_environment,l.line_class;
 INSERT INTO corvus_gold.order_coverage
 SELECT tenant_id,snapshot_id,business_date,origin_application,sales_channel,currency,coverage,count(*),sum(total)
 FROM corvus_silver.order_headers WHERE tenant_id=p_tenant AND snapshot_id=p_snapshot AND coverage<>'EXCLUDED'
 GROUP BY tenant_id,snapshot_id,business_date,origin_application,sales_channel,currency,coverage;
$$;
SELECT corvus.build_sales_gold(tenant_id,id) FROM corvus.sales_snapshots;
