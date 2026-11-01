use async_trait::async_trait;
use serde_json::json;
use sqlx::{PgPool, Postgres, Row, Transaction};
use tordo_analytics_contracts::SnapshotManifest;
use tordo_commons_core::{AuditEntry, AuditTrail, Failure, Result, Scope};
use tordo_corvus_core::sales::{
    contracts::Pipeline,
    domain::{SalesPublicationPlan, SourceSalesSnapshot, hash},
    ports::*,
};
use tordo_runtime::database_error;
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresSales {
    pub pool: PgPool,
}
pub struct SalesTransaction {
    tenant: String,
    pipeline_id: Uuid,
    tx: Transaction<'static, Postgres>,
}
fn decode_manifest(v: serde_json::Value) -> Result<SnapshotManifest> {
    serde_json::from_value(v).map_err(|_| Failure::invalid("Manifiesto almacenado incompatible"))
}
#[async_trait]
impl DatasetCatalog for PostgresSales {
    async fn current(&self, s: &Scope, pipeline_id: Uuid) -> Result<Option<SnapshotManifest>> {
        let row=sqlx::query("SELECT s.manifest FROM corvus.sales_dataset_heads h JOIN corvus.sales_snapshots s ON s.tenant_id=h.tenant_id AND s.id=h.snapshot_id WHERE h.tenant_id=$1 AND h.pipeline_id=$2").bind(&s.tenant_id).bind(pipeline_id).fetch_optional(&self.pool).await.map_err(database_error)?;
        row.map(|r| decode_manifest(r.get("manifest"))).transpose()
    }
    async fn manifest(&self, s: &Scope, id: Uuid) -> Result<SnapshotManifest> {
        let row =
            sqlx::query("SELECT manifest FROM corvus.sales_snapshots WHERE tenant_id=$1 AND id=$2")
                .bind(&s.tenant_id)
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(database_error)?
                .ok_or_else(Failure::missing)?;
        decode_manifest(row.get("manifest"))
    }
}
#[async_trait]
impl IngestionRunStore for PostgresSales {
    async fn claim(&self, s: &Scope, pipeline_id: Uuid, key: &str, hash: &str) -> Result<RunClaim> {
        let id = Uuid::new_v4();
        let attempt = Uuid::new_v4();
        let inserted=sqlx::query("INSERT INTO corvus.ingestion_runs(tenant_id,run_key,id,request_hash,attempt,state,lease_until,pipeline_id) VALUES($1,$2,$3,$4,$5,'running',now()+interval '60 seconds',$6) ON CONFLICT DO NOTHING")
            .bind(&s.tenant_id).bind(key).bind(id).bind(hash).bind(attempt).bind(pipeline_id).execute(&self.pool).await.map_err(database_error)?.rows_affected();
        if inserted == 1 {
            return Ok(RunClaim::Claimed {
                run_id: id,
                attempt,
            });
        }
        let row=sqlx::query("SELECT id,request_hash,state,snapshot_id,lease_until FROM corvus.ingestion_runs WHERE tenant_id=$1 AND run_key=$2 AND pipeline_id=$3")
            .bind(&s.tenant_id).bind(key).bind(pipeline_id).fetch_one(&self.pool).await.map_err(database_error)?;
        if row.get::<String, _>("request_hash") != hash {
            return Err(Failure::conflict(
                "Identidad de ingesta reutilizada con otra solicitud",
            ));
        }
        if row.get::<String, _>("state") == "succeeded" {
            return Ok(RunClaim::Published(
                self.manifest(s, row.get("snapshot_id")).await?,
            ));
        }
        let id: Uuid = row.get("id");
        let changed=sqlx::query("UPDATE corvus.ingestion_runs SET attempt=$3,state='running',lease_until=now()+interval '60 seconds',error_code=NULL,updated_at=now() WHERE tenant_id=$1 AND id=$2 AND (state='failed' OR lease_until<now())")
            .bind(&s.tenant_id).bind(id).bind(attempt).execute(&self.pool).await.map_err(database_error)?.rows_affected();
        if changed != 1 {
            return Err(Failure::conflict("La ingesta ya está en ejecución"));
        }
        Ok(RunClaim::Claimed {
            run_id: id,
            attempt,
        })
    }
    async fn fail(&self, s: &Scope, id: Uuid, attempt: Uuid, code: &str) -> Result<()> {
        sqlx::query("UPDATE corvus.ingestion_runs SET state='failed',error_code=$4,updated_at=now() WHERE tenant_id=$1 AND id=$2 AND attempt=$3 AND state='running'")
            .bind(&s.tenant_id).bind(id).bind(attempt).bind(code).execute(&self.pool).await.map_err(database_error)?;
        Ok(())
    }
}
#[async_trait]
impl CorvusUnitOfWork for PostgresSales {
    async fn begin(
        &self,
        s: &Scope,
        pipeline_id: Uuid,
        expected: Option<Uuid>,
    ) -> Result<Box<dyn CorvusTransaction>> {
        let mut tx = self.pool.begin().await.map_err(database_error)?;
        sqlx::query(
            "INSERT INTO corvus.sales_dataset_heads(tenant_id,pipeline_id) VALUES($1,$2) ON CONFLICT DO NOTHING",
        )
        .bind(&s.tenant_id).bind(pipeline_id)
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;
        let current: Option<Uuid> = sqlx::query_scalar(
            "SELECT snapshot_id FROM corvus.sales_dataset_heads WHERE tenant_id=$1 AND pipeline_id=$2 FOR UPDATE",
        )
        .bind(&s.tenant_id).bind(pipeline_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(database_error)?;
        if current != expected {
            return Err(Failure::conflict(
                "Cambió el snapshot vigente; vuelva a extraer",
            ));
        }
        Ok(Box::new(SalesTransaction {
            tenant: s.tenant_id.clone(),
            pipeline_id,
            tx,
        }))
    }
}
#[async_trait]
impl SalesProjectionWriter for SalesTransaction {
    async fn write_projections(&mut self, p: &SalesPublicationPlan) -> Result<()> {
        if p.manifest.pipeline_id != Some(self.pipeline_id)
            || p.manifest.tenant_id != self.tenant
            || p.source.tenant_id != self.tenant
        {
            return Err(Failure::denied());
        }
        let id = p.manifest.snapshot_id;
        sqlx::query(
            "INSERT INTO corvus.sales_snapshots(tenant_id,id,version,manifest,pipeline_id) VALUES($1,$2,$3,$4,$5)",
        )
        .bind(&self.tenant)
        .bind(id)
        .bind(p.manifest.version)
        .bind(json!(p.manifest)).bind(self.pipeline_id)
        .execute(&mut *self.tx)
        .await
        .map_err(database_error)?;
        sqlx::query(r#"INSERT INTO corvus_silver.invoice_headers SELECT $1,$2,r.id,r."orderId",r.number,r."documentType",r."businessDate",r.status,r.currency,r."taxBase",r."taxAmount",r.total,r."originApplication",r."salesChannel",r."fiscalEnvironment",r."fiscalStatus",r."branchCode",r.classification,r."originEvidence",r.deleted,r."includedInBilling" FROM jsonb_to_recordset($3) AS r(id uuid,"orderId" uuid,number text,"documentType" text,"businessDate" date,status text,currency text,"taxBase" numeric,"taxAmount" numeric,total numeric,"originApplication" text,"salesChannel" text,"fiscalEnvironment" text,"fiscalStatus" text,"branchCode" text,classification text,"originEvidence" text,deleted boolean,"includedInBilling" boolean)"#)
            .bind(&self.tenant).bind(id).bind(json!(p.source.invoices)).execute(&mut *self.tx).await.map_err(database_error)?;
        sqlx::query(r#"INSERT INTO corvus_silver.invoice_lines SELECT $1,$2,r.id,r."invoiceId",r."productCode",r."productName",r."unitCode",r.quantity,r."taxBase",r."taxAmount",r.total,r."lineClass" FROM jsonb_to_recordset($3) AS r(id uuid,"invoiceId" uuid,"productCode" text,"productName" text,"unitCode" text,quantity numeric,"taxBase" numeric,"taxAmount" numeric,total numeric,"lineClass" text)"#)
            .bind(&self.tenant).bind(id).bind(json!(p.source.invoice_lines)).execute(&mut *self.tx).await.map_err(database_error)?;
        sqlx::query(r#"INSERT INTO corvus_silver.order_headers SELECT $1,$2,r.id,r."businessDate",r.status,r.currency,r.total,r."originApplication",r."salesChannel",r.classification,r."originEvidence",r.deleted,r.coverage FROM jsonb_to_recordset($3) AS r(id uuid,"businessDate" date,status text,currency text,total numeric,"originApplication" text,"salesChannel" text,classification text,"originEvidence" text,deleted boolean,coverage text)"#)
            .bind(&self.tenant).bind(id).bind(json!(p.source.orders)).execute(&mut *self.tx).await.map_err(database_error)?;
        sqlx::query(r#"INSERT INTO corvus_silver.order_lines SELECT $1,$2,r.id,r."orderId",r."productCode",r."unitCode",r.quantity,r.total FROM jsonb_to_recordset($3) AS r(id uuid,"orderId" uuid,"productCode" text,"unitCode" text,quantity numeric,total numeric)"#)
            .bind(&self.tenant).bind(id).bind(json!(p.source.order_lines)).execute(&mut *self.tx).await.map_err(database_error)?;
        sqlx::query("INSERT INTO corvus_silver.order_invoice_links SELECT tenant_id,snapshot_id,order_id,id FROM corvus_silver.invoice_headers WHERE tenant_id=$1 AND snapshot_id=$2 AND order_id IS NOT NULL")
            .bind(&self.tenant).bind(id).execute(&mut *self.tx).await.map_err(database_error)?;
        Ok(())
    }
    async fn write_gold_models(&mut self, p: &SalesPublicationPlan) -> Result<()> {
        if p.manifest.tenant_id != self.tenant || p.manifest.pipeline_id != Some(self.pipeline_id) {
            return Err(Failure::denied());
        }
        sqlx::query("SELECT corvus.build_sales_gold($1,$2)")
            .bind(&self.tenant)
            .bind(p.manifest.snapshot_id)
            .execute(&mut *self.tx)
            .await
            .map_err(database_error)?;
        Ok(())
    }
}
#[async_trait]
impl SnapshotWriter for SalesTransaction {
    async fn publish_manifest(
        &mut self,
        p: &SalesPublicationPlan,
        run_id: Uuid,
        attempt: Uuid,
    ) -> Result<()> {
        if p.manifest.tenant_id != self.tenant || p.manifest.snapshot_id != run_id {
            return Err(Failure::denied());
        }
        let changed=sqlx::query("UPDATE corvus.ingestion_runs SET state='succeeded',snapshot_id=$2,updated_at=now() WHERE tenant_id=$1 AND id=$2 AND attempt=$3 AND pipeline_id=$4 AND state='running' AND lease_until>now()")
            .bind(&self.tenant).bind(run_id).bind(attempt).bind(self.pipeline_id).execute(&mut *self.tx).await.map_err(database_error)?.rows_affected();
        if changed != 1 {
            return Err(Failure::conflict("La ingesta perdió su lease"));
        }
        sqlx::query("UPDATE corvus.sales_dataset_heads SET snapshot_id=$2 WHERE tenant_id=$1 AND pipeline_id=$3")
            .bind(&self.tenant)
            .bind(run_id).bind(self.pipeline_id)
            .execute(&mut *self.tx)
            .await
            .map_err(database_error)?;
        sqlx::query("INSERT INTO corvus.sales_checkpoints(tenant_id,source_code,snapshot_id,source_cut,observed_through,pipeline_id) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(tenant_id,pipeline_id) DO UPDATE SET snapshot_id=excluded.snapshot_id,source_cut=excluded.source_cut,observed_through=excluded.observed_through")
            .bind(&self.tenant).bind(&p.manifest.source_code).bind(run_id).bind(&p.manifest.source_cut).bind(p.manifest.observed_through).bind(self.pipeline_id).execute(&mut *self.tx).await.map_err(database_error)?;
        Ok(())
    }
}
#[async_trait]
impl AuditTrail for SalesTransaction {
    async fn record_audit(&mut self, e: &AuditEntry) -> Result<()> {
        if e.tenant_id != self.tenant {
            return Err(Failure::denied());
        }
        sqlx::query("INSERT INTO corvus.audit_log(id,tenant_id,actor,action,subject,correlation_id,occurred_at) VALUES($1,$2,$3,$4,$5,$6,$7)")
            .bind(e.id).bind(&self.tenant).bind(&e.actor).bind(&e.action).bind(&e.subject).bind(&e.correlation_id).bind(e.occurred_at).execute(&mut *self.tx).await.map_err(database_error)?;
        Ok(())
    }
}
#[async_trait]
impl CorvusTransaction for SalesTransaction {
    fn tenant_id(&self) -> &str {
        &self.tenant
    }
    async fn commit(self: Box<Self>) -> Result<()> {
        self.tx.commit().await.map_err(database_error)
    }
}

#[async_trait]
impl PipelineCatalog for PostgresSales {
    async fn pipelines(&self, s: &Scope) -> Result<Vec<Pipeline>> {
        let values: Vec<serde_json::Value> = sqlx::query_scalar(
            "SELECT body FROM corvus.pipelines WHERE tenant_id=$1 ORDER BY id LIMIT 100",
        )
        .bind(&s.tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)?;
        values
            .into_iter()
            .map(|v| {
                serde_json::from_value(v)
                    .map_err(|_| Failure::invalid("Definición de ingesta incompatible"))
            })
            .collect()
    }
    async fn pipeline(&self, s: &Scope, id: Uuid) -> Result<Pipeline> {
        let value: serde_json::Value =
            sqlx::query_scalar("SELECT body FROM corvus.pipelines WHERE tenant_id=$1 AND id=$2")
                .bind(&s.tenant_id)
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(database_error)?
                .ok_or_else(Failure::missing)?;
        serde_json::from_value(value)
            .map_err(|_| Failure::invalid("Definición de ingesta incompatible"))
    }
    async fn create_pipeline(&self, s: &Scope, p: &Pipeline, audit: &AuditEntry) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(database_error)?;
        sqlx::query("INSERT INTO corvus.pipelines(tenant_id,id,body) VALUES($1,$2,$3)")
            .bind(&s.tenant_id)
            .bind(p.id)
            .bind(json!(p))
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
        if audit.tenant_id != s.tenant_id {
            return Err(Failure::denied());
        }
        sqlx::query("INSERT INTO corvus.audit_log(id,tenant_id,actor,action,subject,correlation_id,occurred_at) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(audit.id).bind(&s.tenant_id).bind(&audit.actor).bind(&audit.action).bind(&audit.subject).bind(&audit.correlation_id).bind(audit.occurred_at).execute(&mut *tx).await.map_err(database_error)?;
        tx.commit().await.map_err(database_error)?;
        Ok(())
    }
}
#[async_trait]
impl BronzeStore for PostgresSales {
    async fn capture(
        &self,
        s: &Scope,
        pipeline_id: Uuid,
        run: Uuid,
        attempt: Uuid,
        source: &SourceSalesSnapshot,
    ) -> Result<()> {
        s.tenant(&source.tenant_id)?;
        let changed=sqlx::query("INSERT INTO corvus_bronze.batches(tenant_id,pipeline_id,run_id,attempt,body,content_hash,captured_at) SELECT $1,$2,$3,$4,$5,$6,$7 WHERE EXISTS(SELECT 1 FROM corvus.ingestion_runs WHERE tenant_id=$1 AND pipeline_id=$2 AND id=$3 AND attempt=$4 AND state='running' AND lease_until>now())")
            .bind(&s.tenant_id).bind(pipeline_id).bind(run).bind(attempt).bind(json!(source)).bind(hash(source)?).bind(source.captured_at).execute(&self.pool).await.map_err(database_error)?.rows_affected();
        if changed != 1 {
            return Err(Failure::conflict(
                "El intento de ingesta perdió su lease antes de capturar bronce",
            ));
        }
        Ok(())
    }
    async fn runs(&self, s: &Scope, pipeline_id: Uuid) -> Result<Vec<serde_json::Value>> {
        let rows=sqlx::query("SELECT id,run_key,state,error_code,snapshot_id,created_at,EXISTS(SELECT 1 FROM corvus_bronze.batches b WHERE b.tenant_id=r.tenant_id AND b.run_id=r.id) AS bronze FROM corvus.ingestion_runs r WHERE tenant_id=$1 AND pipeline_id=$2 ORDER BY created_at DESC LIMIT 100").bind(&s.tenant_id).bind(pipeline_id).fetch_all(&self.pool).await.map_err(database_error)?;
        Ok(rows.into_iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"runKey":r.get::<String,_>("run_key"),"state":r.get::<String,_>("state"),"errorCode":r.get::<Option<String>,_>("error_code"),"snapshotId":r.get::<Option<Uuid>,_>("snapshot_id"),"createdAt":r.get::<chrono::DateTime<chrono::Utc>,_>("created_at"),"bronzeCaptured":r.get::<bool,_>("bronze")})).collect())
    }
}
