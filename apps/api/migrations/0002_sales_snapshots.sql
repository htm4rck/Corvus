CREATE TABLE corvus.ingestion_runs (
 tenant_id text NOT NULL, run_key text NOT NULL, id uuid NOT NULL UNIQUE,
 request_hash text NOT NULL, attempt uuid NOT NULL, state text NOT NULL,
 lease_until timestamptz NOT NULL, error_code text, snapshot_id uuid,
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(tenant_id,run_key), CHECK(state IN('running','failed','succeeded'))
);
CREATE TABLE corvus.sales_snapshots (
 tenant_id text NOT NULL,id uuid NOT NULL,version bigint NOT NULL,manifest jsonb NOT NULL,
 PRIMARY KEY(tenant_id,id),UNIQUE(tenant_id,version),CHECK(version>0)
);
CREATE TABLE corvus.sales_dataset_heads (
 tenant_id text PRIMARY KEY,snapshot_id uuid,
 FOREIGN KEY(tenant_id,snapshot_id) REFERENCES corvus.sales_snapshots(tenant_id,id)
);
CREATE TABLE corvus.sales_raw (
 tenant_id text NOT NULL,snapshot_id uuid NOT NULL,body jsonb NOT NULL,
 PRIMARY KEY(tenant_id,snapshot_id),FOREIGN KEY(tenant_id,snapshot_id) REFERENCES corvus.sales_snapshots(tenant_id,id)
);
CREATE TABLE corvus.invoice_headers (
 tenant_id text NOT NULL,snapshot_id uuid NOT NULL,id uuid NOT NULL,order_id uuid,
 number text NOT NULL,document_type text NOT NULL,business_date date NOT NULL,status text NOT NULL,
 currency text NOT NULL,tax_base numeric(18,2) NOT NULL,tax_amount numeric(18,2) NOT NULL,total numeric(18,2) NOT NULL,
 origin_application text NOT NULL,sales_channel text NOT NULL,fiscal_environment text NOT NULL,fiscal_status text NOT NULL,
 branch_code text,classification text NOT NULL,origin_evidence text NOT NULL,deleted boolean NOT NULL,included_in_billing boolean NOT NULL,
 PRIMARY KEY(tenant_id,snapshot_id,id),FOREIGN KEY(tenant_id,snapshot_id) REFERENCES corvus.sales_snapshots(tenant_id,id),
 CHECK(tax_base>=0 AND tax_amount>=0 AND total>=0)
);
CREATE TABLE corvus.invoice_lines (
 tenant_id text NOT NULL,snapshot_id uuid NOT NULL,id uuid NOT NULL,invoice_id uuid NOT NULL,
 product_code text NOT NULL,product_name text NOT NULL,unit_code text NOT NULL,quantity numeric(18,6) NOT NULL,
 tax_base numeric(18,2) NOT NULL,tax_amount numeric(18,2) NOT NULL,total numeric(18,2) NOT NULL,line_class text NOT NULL,
 PRIMARY KEY(tenant_id,snapshot_id,id),FOREIGN KEY(tenant_id,snapshot_id,invoice_id) REFERENCES corvus.invoice_headers(tenant_id,snapshot_id,id)
);
CREATE TABLE corvus.order_headers (
 tenant_id text NOT NULL,snapshot_id uuid NOT NULL,id uuid NOT NULL,business_date date NOT NULL,status text NOT NULL,
 currency text NOT NULL,total numeric(18,2) NOT NULL,origin_application text NOT NULL,sales_channel text NOT NULL,
 classification text NOT NULL,origin_evidence text NOT NULL,deleted boolean NOT NULL,coverage text NOT NULL,
 PRIMARY KEY(tenant_id,snapshot_id,id),FOREIGN KEY(tenant_id,snapshot_id) REFERENCES corvus.sales_snapshots(tenant_id,id)
);
CREATE TABLE corvus.order_lines (
 tenant_id text NOT NULL,snapshot_id uuid NOT NULL,id uuid NOT NULL,order_id uuid NOT NULL,
 product_code text NOT NULL,unit_code text NOT NULL,quantity numeric(18,6) NOT NULL,total numeric(18,2) NOT NULL,
 PRIMARY KEY(tenant_id,snapshot_id,id),FOREIGN KEY(tenant_id,snapshot_id,order_id) REFERENCES corvus.order_headers(tenant_id,snapshot_id,id)
);
CREATE TABLE corvus.order_invoice_links (
 tenant_id text NOT NULL,snapshot_id uuid NOT NULL,order_id uuid NOT NULL,invoice_id uuid NOT NULL,
 PRIMARY KEY(tenant_id,snapshot_id,order_id,invoice_id),
 FOREIGN KEY(tenant_id,snapshot_id,order_id) REFERENCES corvus.order_headers(tenant_id,snapshot_id,id),
 FOREIGN KEY(tenant_id,snapshot_id,invoice_id) REFERENCES corvus.invoice_headers(tenant_id,snapshot_id,id)
);
CREATE TABLE corvus.sales_checkpoints (
 tenant_id text NOT NULL,source_code text NOT NULL,snapshot_id uuid NOT NULL,source_cut text NOT NULL,observed_through timestamptz NOT NULL,
 PRIMARY KEY(tenant_id,source_code),FOREIGN KEY(tenant_id,snapshot_id) REFERENCES corvus.sales_snapshots(tenant_id,id)
);
CREATE TABLE corvus.audit_log (
 id uuid PRIMARY KEY,tenant_id text NOT NULL,actor text NOT NULL,action text NOT NULL,
 subject text NOT NULL,correlation_id text NOT NULL,occurred_at timestamptz NOT NULL
);
CREATE INDEX invoice_scope ON corvus.invoice_headers(tenant_id,snapshot_id,business_date,currency);
CREATE INDEX order_scope ON corvus.order_headers(tenant_id,snapshot_id,business_date,currency);
