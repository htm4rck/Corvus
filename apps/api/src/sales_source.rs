use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::{Connection, Executor, Row};
use std::{collections::HashMap, time::Duration};
use tordo_commons_core::{Failure, FailureKind, Result, Scope};
use tordo_corvus_core::sales::{
    contracts::{ExtractSalesSnapshot, IngestionSource},
    domain::*,
    ports::SourceReader,
};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceSettings {
    tenant_id: String,
    url: String,
    deployment_environment: String,
}
pub struct PostgresSalesExportReader {
    sources: HashMap<String, SourceSettings>,
}
fn unavailable() -> Failure {
    Failure::new(
        FailureKind::Unavailable,
        "SOURCE_UNAVAILABLE",
        "La extracción del origen no está disponible",
    )
}
impl PostgresSalesExportReader {
    pub fn from_env() -> Result<Self> {
        let sources = serde_json::from_str(
            &std::env::var("CORVUS_ANALYTICS_SOURCES").unwrap_or_else(|_| "{}".into()),
        )
        .map_err(|_| Failure::invalid("Configuración de orígenes inválida"))?;
        Ok(Self { sources })
    }
    async fn extract(
        &self,
        scope: &Scope,
        c: &ExtractSalesSnapshot,
    ) -> Result<SourceSalesSnapshot> {
        let settings = self
            .sources
            .get(&c.source_code)
            .ok_or_else(Failure::denied)?;
        scope.tenant(&settings.tenant_id)?;
        if settings.deployment_environment != "dev" {
            return Err(Failure::denied());
        }
        let mut connection = sqlx::PgConnection::connect(&settings.url)
            .await
            .map_err(|_| unavailable())?;
        let mut tx = connection.begin().await.map_err(|_| unavailable())?;
        tx.execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .await
            .map_err(|_| unavailable())?;
        tx.execute("SET LOCAL statement_timeout='5000ms'")
            .await
            .map_err(|_| unavailable())?;
        tx.execute("SET LOCAL lock_timeout='250ms'")
            .await
            .map_err(|_| unavailable())?;
        let cut = sqlx::query(
            "SELECT pg_current_snapshot()::text AS cut,transaction_timestamp() AS captured_at",
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| unavailable())?;
        let captured_at: DateTime<Utc> = cut.get("captured_at");
        let source_cut: String = cut.get("cut");
        // Include every invoice linked to the selected order cohort, even outside the billing window.
        // Billing queries independently filter invoice dates; coverage means any approved invoice at the cut.
        let cohort:Vec<Uuid>=sqlx::query_scalar("SELECT id FROM analytics_export.order_headers WHERE business_date BETWEEN $1 AND $2 ORDER BY id LIMIT 5001")
            .bind(c.from).bind(c.to).fetch_all(&mut *tx).await.map_err(|_|unavailable())?;
        if cohort.len() > 5000 {
            return Err(Failure::invalid("El corte supera 5000 pedidos"));
        }
        let rows=sqlx::query("SELECT body FROM analytics_export.invoice_headers WHERE business_date BETWEEN $1 AND $2 OR (body->>'orderId')::uuid=ANY($3) ORDER BY id LIMIT 5001")
            .bind(c.from).bind(c.to).bind(&cohort).fetch_all(&mut *tx).await.map_err(|_|unavailable())?;
        if rows.len() > 5000 {
            return Err(Failure::invalid(
                "El corte supera 5000 comprobantes; restrinja el período",
            ));
        }
        let invoices = rows
            .into_iter()
            .map(|r| {
                serde_json::from_value::<InvoiceFact>(r.get("body"))
                    .map_err(|_| Failure::invalid("Contrato de cabecera inválido"))
            })
            .collect::<Result<Vec<_>>>()?;
        let invoice_ids = invoices.iter().map(|v| v.id).collect::<Vec<Uuid>>();
        let linked_orders = invoices
            .iter()
            .filter_map(|v| v.order_id)
            .collect::<Vec<Uuid>>();
        let rows=sqlx::query("SELECT body FROM analytics_export.invoice_lines WHERE invoice_id=ANY($1) ORDER BY id LIMIT 50001")
            .bind(&invoice_ids).fetch_all(&mut *tx).await.map_err(|_|unavailable())?;
        if rows.len() > 50000 {
            return Err(Failure::invalid("Demasiadas líneas en el corte"));
        }
        let invoice_lines = rows
            .into_iter()
            .map(|r| {
                serde_json::from_value::<InvoiceLineFact>(r.get("body"))
                    .map_err(|_| Failure::invalid("Contrato de línea inválido"))
            })
            .collect::<Result<Vec<_>>>()?;
        let rows=sqlx::query("SELECT body FROM analytics_export.order_headers WHERE business_date BETWEEN $1 AND $2 OR id=ANY($3) ORDER BY id LIMIT 5001")
            .bind(c.from).bind(c.to).bind(&linked_orders).fetch_all(&mut *tx).await.map_err(|_|unavailable())?;
        if rows.len() > 5000 {
            return Err(Failure::invalid("El corte supera 5000 pedidos"));
        }
        let orders = rows
            .into_iter()
            .map(|r| {
                serde_json::from_value::<OrderFact>(r.get("body"))
                    .map_err(|_| Failure::invalid("Contrato de pedido inválido"))
            })
            .collect::<Result<Vec<_>>>()?;
        let order_ids = orders.iter().map(|v| v.id).collect::<Vec<_>>();
        let rows=sqlx::query("SELECT body FROM analytics_export.order_lines WHERE order_id=ANY($1) ORDER BY id LIMIT 50001").bind(&order_ids).fetch_all(&mut *tx).await.map_err(|_|unavailable())?;
        if rows.len() > 50000 {
            return Err(Failure::invalid("Demasiadas líneas de pedido"));
        }
        let order_lines = rows
            .into_iter()
            .map(|r| {
                serde_json::from_value::<OrderLineFact>(r.get("body"))
                    .map_err(|_| Failure::invalid("Contrato de detalle de pedido inválido"))
            })
            .collect::<Result<Vec<_>>>()?;
        tx.rollback().await.map_err(|_| unavailable())?;
        Ok(SourceSalesSnapshot {
            tenant_id: scope.tenant_id.clone(),
            source_code: c.source_code.clone(),
            deployment_environment: settings.deployment_environment.clone(),
            source_cut,
            captured_at,
            completed_at: Utc::now(),
            complete: true,
            invoices,
            invoice_lines,
            orders,
            order_lines,
        })
    }
}
#[async_trait]
impl SourceReader for PostgresSalesExportReader {
    fn sources(&self, s: &Scope) -> Vec<IngestionSource> {
        self.sources
            .iter()
            .filter(|(_, v)| v.tenant_id == s.tenant_id)
            .map(|(k, _)| IngestionSource {
                source_ref: k.clone(),
                connector: "postgresql_sales_export_v1".into(),
            })
            .collect()
    }
    async fn read_snapshot(
        &self,
        scope: &Scope,
        c: &ExtractSalesSnapshot,
    ) -> Result<SourceSalesSnapshot> {
        tokio::time::timeout(Duration::from_secs(25), self.extract(scope, c))
            .await
            .map_err(|_| unavailable())?
    }
}
