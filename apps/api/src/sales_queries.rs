use crate::sales_persistence::PostgresSales;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use sqlx::{PgPool, Row};
use tordo_analytics_contracts::{Column, DatasetQueryRequest, ResultProvenance, TableResult};
use tordo_commons_core::{Failure, Result, Scope};
use tordo_corvus_core::sales::ports::{AnalyticalQuery, DatasetCatalog};
use tordo_runtime::database_error;
pub struct PostgresSalesQuery {
    pub pool: PgPool,
    pub catalog: PostgresSales,
}
#[async_trait]
impl AnalyticalQuery for PostgresSalesQuery {
    async fn query(&self, s: &Scope, r: &DatasetQueryRequest) -> Result<TableResult> {
        let m = self.catalog.manifest(s, r.snapshot_id).await?;
        let f = &r.query.filters;
        let (sql, specs): (&str, Vec<(&str, &str)>) = match r.query.query_id.as_str() {
            "sales.billing.daily.v1" => (
                r#"SELECT business_date::text AS "invoiceBusinessDate",business_date::text||' · '||origin_application||' · '||document_type AS "dayChannelLabel",origin_application AS "originApplication",sales_channel AS "salesChannel",currency,document_type AS "documentType",fiscal_environment AS "fiscalEnvironment",sum(document_count)::text AS "documentCount",sum(tax_base)::text AS "taxBase",sum(tax_amount)::text AS "taxAmount",sum(billed_total)::text AS "billedTotal",round(sum(billed_total)/NULLIF(sum(document_count),0),2)::text AS "averageDocumentAmount" FROM corvus_gold.billing_daily WHERE tenant_id=$1 AND snapshot_id=$2 AND ($3::date IS NULL OR business_date >= $3) AND ($4::date IS NULL OR business_date <= $4) AND ($5::text IS NULL OR currency=$5) AND ($6::text IS NULL OR origin_application=$6) AND ($7::text IS NULL OR document_type=$7) AND ($8::text IS NULL OR fiscal_environment=$8) GROUP BY business_date,origin_application,sales_channel,currency,document_type,fiscal_environment ORDER BY business_date,origin_application,document_type LIMIT 1001"#,
                vec![
                    ("invoiceBusinessDate", "date"),
                    ("dayChannelLabel", "text"),
                    ("originApplication", "text"),
                    ("salesChannel", "text"),
                    ("currency", "text"),
                    ("documentType", "text"),
                    ("fiscalEnvironment", "text"),
                    ("documentCount", "integer"),
                    ("taxBase", "decimal"),
                    ("taxAmount", "decimal"),
                    ("billedTotal", "decimal"),
                    ("averageDocumentAmount", "decimal"),
                ],
            ),
            "sales.billing.products.v1" => (
                r#"SELECT product_code AS "productCode",max(product_name) AS "productLabel",unit_code AS "unitCode",currency,origin_application AS "originApplication",line_class AS "lineClass",sum(quantity)::text AS "invoicedQuantity",sum(tax_base)::text AS "taxBase",sum(tax_amount)::text AS "taxAmount",sum(billed_total)::text AS "billedLineTotal" FROM corvus_gold.billing_products WHERE tenant_id=$1 AND snapshot_id=$2 AND ($3::date IS NULL OR business_date >= $3) AND ($4::date IS NULL OR business_date <= $4) AND ($5::text IS NULL OR currency=$5) AND ($6::text IS NULL OR origin_application=$6) AND ($7::text IS NULL OR document_type=$7) AND ($8::text IS NULL OR fiscal_environment=$8) GROUP BY product_code,unit_code,currency,origin_application,line_class ORDER BY line_class,sum(billed_total) DESC,product_code LIMIT 1001"#,
                vec![
                    ("productCode", "text"),
                    ("productLabel", "text"),
                    ("unitCode", "text"),
                    ("currency", "text"),
                    ("originApplication", "text"),
                    ("lineClass", "text"),
                    ("invoicedQuantity", "decimal"),
                    ("taxBase", "decimal"),
                    ("taxAmount", "decimal"),
                    ("billedLineTotal", "decimal"),
                ],
            ),
            "sales.orders.invoice-coverage.v1" => {
                if f.document_type.is_some() || f.fiscal_environment.is_some() {
                    return Err(Failure::invalid(
                        "Tipo y ambiente fiscal sólo aplican a comprobantes",
                    ));
                }
                (
                    r#"SELECT origin_application AS "originApplication",sales_channel AS "salesChannel",currency,coverage AS "invoiceCoverage",origin_application||' · '||sales_channel||' · '||coverage AS "channelCoverageLabel",sum(order_count)::text AS "orderCount",sum(intent_amount)::text AS "orderIntentAmount",COALESCE(sum(order_count) FILTER(WHERE coverage='WITH_APPROVED_INVOICE'),0)::text AS "ordersWithApprovedInvoice",COALESCE(sum(order_count) FILTER(WHERE coverage='WITHOUT_APPROVED_INVOICE'),0)::text AS "ordersWithoutApprovedInvoice",NULL::text AS "shareWithAnyApprovedInvoice" FROM corvus_gold.order_coverage WHERE tenant_id=$1 AND snapshot_id=$2 AND ($3::date IS NULL OR business_date >= $3) AND ($4::date IS NULL OR business_date <= $4) AND ($5::text IS NULL OR currency=$5) AND ($6::text IS NULL OR origin_application=$6) AND $7::text IS NULL AND $8::text IS NULL GROUP BY origin_application,sales_channel,currency,coverage ORDER BY origin_application,coverage LIMIT 1001"#,
                    vec![
                        ("originApplication", "text"),
                        ("salesChannel", "text"),
                        ("currency", "text"),
                        ("invoiceCoverage", "text"),
                        ("channelCoverageLabel", "text"),
                        ("orderCount", "integer"),
                        ("orderIntentAmount", "decimal"),
                        ("ordersWithApprovedInvoice", "integer"),
                        ("ordersWithoutApprovedInvoice", "integer"),
                        ("shareWithAnyApprovedInvoice", "decimal"),
                    ],
                )
            }
            _ => return Err(Failure::invalid("Modelo no registrado")),
        };
        let mut tx = self.pool.begin().await.map_err(database_error)?;
        sqlx::query("SET TRANSACTION READ ONLY")
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
        sqlx::query("SET LOCAL statement_timeout='2000ms'")
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
        let rows = sqlx::query(sql)
            .bind(&s.tenant_id)
            .bind(r.snapshot_id)
            .bind(f.from)
            .bind(f.to)
            .bind(&f.currency)
            .bind(&f.origin_application)
            .bind(&f.document_type)
            .bind(&f.fiscal_environment)
            .fetch_all(&mut *tx)
            .await
            .map_err(database_error)?;
        tx.rollback().await.map_err(database_error)?;
        let truncated = rows.len() > 1000;
        let rows = rows
            .iter()
            .take(1000)
            .map(|row| {
                specs
                    .iter()
                    .map(|(name, _)| {
                        row.try_get::<Option<String>, _>(*name)
                            .map(|v| v.map_or(Value::Null, Value::String))
                            .map_err(database_error)
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let columns = specs
            .into_iter()
            .enumerate()
            .map(|(ordinal, (name, logical_type))| Column {
                ordinal,
                name: name.into(),
                logical_type: logical_type.into(),
                nullable: true,
            })
            .collect();
        let mut warnings = m.quality.warnings.clone();
        if truncated {
            warnings.push("Vista previa limitada; no es una exportación completa".into());
        }
        if r.query.query_id == "sales.orders.invoice-coverage.v1" {
            warnings.push("Importe de pedido: intención comercial; presencia de comprobante no acredita facturación completa".into());
        }
        let result = TableResult {
            columns,
            rows,
            completeness: if truncated { "preview" } else { "complete" }.into(),
            generated_at: Utc::now(),
            warnings,
            provenance: ResultProvenance {
                dataset_code: Some(m.dataset_code),
                snapshot_id: Some(m.snapshot_id),
                pipeline_id: m.pipeline_id,
                dataset_version: Some(m.version),
                observed_through: Some(m.observed_through),
                published_at: Some(m.published_at),
            },
        };
        if serde_json::to_vec(&result)
            .map_err(|_| Failure::invalid("Resultado incompatible"))?
            .len()
            > 1024 * 1024
        {
            return Err(Failure::invalid("El resultado supera el límite de un MiB"));
        }
        Ok(result)
    }
}
