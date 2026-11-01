BEGIN;
-- Operation sales export contract. Apply in the selected company database.
-- No customer, address, identity document or payment-provider fields are exported.
CREATE SCHEMA IF NOT EXISTS analytics_export;
CREATE TABLE IF NOT EXISTS analytics_export.origin_evidence (
 entity_type text NOT NULL CHECK(entity_type IN('invoice','order')),entity_id uuid NOT NULL,
 origin_application text NOT NULL CHECK(origin_application IN('amazilia','birdport')),
 sales_channel text NOT NULL,data_classification text NOT NULL CHECK(data_classification IN('BUSINESS','BUSINESS_DEV','TECHNICAL_FIXTURE')),
 fiscal_environment text NOT NULL CHECK(fiscal_environment IN('BETA','PRODUCTION','UNKNOWN')),
 proof_type text NOT NULL,proof_hash text NOT NULL CHECK(length(proof_hash)=64),actor text NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),PRIMARY KEY(entity_type,entity_id)
);
ALTER TABLE analytics_export.origin_evidence DROP CONSTRAINT IF EXISTS origin_evidence_data_classification_check;
ALTER TABLE analytics_export.origin_evidence ADD CONSTRAINT origin_evidence_data_classification_check CHECK(data_classification IN('BUSINESS','BUSINESS_DEV','TECHNICAL_FIXTURE'));
CREATE OR REPLACE VIEW analytics_export.order_headers AS
SELECT o.id,o.doc_date_time::date AS business_date,jsonb_build_object(
 'id',o.id,'businessDate',o.doc_date_time::date,'status',o.status_operation,'currency',o.doc_currency,'total',o.ope_total::text,
 'originApplication',COALESCE(p.origin_application,CASE WHEN e.id IS NOT NULL OR o.source_name='BirdPort' THEN 'birdport' ELSE 'unknown' END),
 'salesChannel',COALESCE(p.sales_channel,o.source_type,'UNKNOWN'),
 'classification',CASE WHEN e.metadata->>'sandboxVerification'='true' THEN 'TECHNICAL_FIXTURE'
   WHEN p.data_classification IS NOT NULL THEN CASE WHEN p.data_classification='BUSINESS_DEV' THEN 'BUSINESS' ELSE p.data_classification END
   WHEN e.id IS NOT NULL AND c.id IS NOT NULL THEN 'BUSINESS' ELSE 'UNCLASSIFIED' END,
 'originEvidence',COALESCE(p.proof_type||':'||p.proof_hash,CASE WHEN e.id IS NOT NULL AND c.id IS NOT NULL THEN 'BIRDPORT_CHECKOUT_TRANSACTION' WHEN o.source_name='BirdPort' THEN 'LEGACY_ORDER_SOURCE_NAME' ELSE 'UNKNOWN' END),
 'deleted',o.deleted_at IS NOT NULL
) AS body
FROM operation.sales_order o
LEFT JOIN ecommerce.ecommerce_order_meta e ON e.order_id=o.id
LEFT JOIN ecommerce.checkout_session c ON c.id=e.checkout_session_id AND c.business_code=e.business_code
LEFT JOIN analytics_export.origin_evidence p ON p.entity_type='order' AND p.entity_id=o.id;

CREATE OR REPLACE VIEW analytics_export.invoice_headers AS
SELECT i.id,i.invoice_date AS business_date,jsonb_build_object(
 'id',i.id,'orderId',i.sales_order_id,'number',i.invoice_number,
 'documentType',CASE i.sale_document_type WHEN 'INVOICE' THEN 'FACTURA' WHEN 'RECEIPT' THEN 'BOLETA' WHEN 'FACTURA' THEN 'FACTURA' WHEN 'BOLETA' THEN 'BOLETA' ELSE 'UNKNOWN' END,
 'businessDate',i.invoice_date,'status',i.invoice_status,'currency',i.currency,
 'taxBase',i.subtotal::text,'taxAmount',i.tax_amount::text,'total',i.total::text,
 'originApplication',COALESCE(p.origin_application,o.body->>'originApplication','unknown'),
 'salesChannel',COALESCE(p.sales_channel,o.body->>'salesChannel','UNKNOWN'),
 'fiscalEnvironment',COALESCE(p.fiscal_environment,e.metadata->>'invoiceEnvironment','UNKNOWN'),
 'fiscalStatus',COALESCE(f.status,'UNKNOWN'),'branchCode',i.branch_code,
 'classification',COALESCE(CASE WHEN p.data_classification='BUSINESS_DEV' THEN 'BUSINESS' ELSE p.data_classification END,o.body->>'classification','UNCLASSIFIED'),
 'originEvidence',COALESCE(p.proof_type||':'||p.proof_hash,o.body->>'originEvidence','UNKNOWN'),
 'deleted',i.deleted_at IS NOT NULL
) AS body
FROM operation.sale_invoice i
LEFT JOIN analytics_export.order_headers o ON o.id=i.sales_order_id
LEFT JOIN ecommerce.ecommerce_order_meta e ON e.order_id=i.sales_order_id
LEFT JOIN LATERAL (SELECT status FROM operation.invoice_emission_request WHERE sale_invoice_id=i.id ORDER BY created_at DESC,id DESC LIMIT 1) f ON true
LEFT JOIN analytics_export.origin_evidence p ON p.entity_type='invoice' AND p.entity_id=i.id;

CREATE OR REPLACE VIEW analytics_export.invoice_lines AS
SELECT l.id,l.sale_invoice_id AS invoice_id,jsonb_build_object(
 'id',l.id,'invoiceId',l.sale_invoice_id,'productCode',l.product_code,'productName',l.product_name,
 'unitCode',COALESCE(l.unit_of_measure,'NIU'),'quantity',l.quantity::text,
 'taxBase',l.subtotal::text,'taxAmount',l.tax_amount::text,'total',l.total::text,
 'lineClass',CASE WHEN l.product_code='BIRDPORT-DELIVERY' THEN 'SHIPPING' WHEN p."isInventory"=true THEN 'MERCHANDISE' WHEN p."isInventory"=false THEN 'SERVICE' ELSE 'UNKNOWN' END
) AS body
FROM operation.sale_invoice_line l
LEFT JOIN inventory_catalog."Product" p ON p."productCode"=l.product_code AND p.deleted_at IS NULL
WHERE l.deleted_at IS NULL;

CREATE OR REPLACE VIEW analytics_export.order_lines AS
SELECT l.id,l.sales_order_id AS order_id,jsonb_build_object(
 'id',l.id,'orderId',l.sales_order_id,'productCode',l.product_code,'unitCode',COALESCE(l.product_msr_entry,'NIU'),
 'quantity',l.quantity::text,'total',l.total::text
) AS body FROM operation.sales_order_line l WHERE l.deleted_at IS NULL;

COMMIT;
