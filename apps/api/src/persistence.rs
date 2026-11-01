use async_trait::async_trait;
use chrono::Utc;
use serde_json::{Value, json};
use sqlx::Row;
use tordo_commons_core::{Failure, Result};
use tordo_corvus_core::{application::CorvusStore, domain::*};
use tordo_runtime::{database_error, persistence::JsonStore};
use uuid::Uuid;

pub struct PostgresCorvus(pub JsonStore);
fn stored(batch: &Batch) -> Value {
    let mut body = json!(batch);
    body["rows"] = json!(batch.rows);
    body
}
#[async_trait]
impl CorvusStore for PostgresCorvus {
    async fn receive(&self, input: EventInput) -> Result<EventReceipt> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let payload = json!(input);
        let inserted=sqlx::query("INSERT INTO corvus.ingestion_events(id,tenant_id,company_id,event_id,event_type,event_version,source_system,entity_id,occurred_at,ingested_at,payload,quality_status) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'RECEIVED') ON CONFLICT(tenant_id,event_id) DO NOTHING")
            .bind(id).bind(&input.tenant_id).bind(&input.company_id).bind(&input.event_id).bind(&input.event_type)
            .bind(&input.event_version).bind(&input.source_system).bind(&input.entity_id).bind(input.occurred_at)
            .bind(now).bind(&payload).execute(&self.0.pool).await.map_err(database_error)?.rows_affected();
        if inserted == 1 {
            return Ok(EventReceipt {
                id,
                event_id: input.event_id,
                status: "RECEIVED".into(),
                ingested_at: now,
            });
        }
        let row=sqlx::query("SELECT id,payload,ingested_at FROM corvus.ingestion_events WHERE tenant_id=$1 AND event_id=$2")
            .bind(&input.tenant_id).bind(&input.event_id).fetch_one(&self.0.pool).await.map_err(database_error)?;
        // Deserialize to normalize timestamps in events written by the Java implementation.
        let old: EventInput = serde_json::from_value(row.get("payload"))
            .map_err(|_| Failure::conflict("Evento previo incompatible"))?;
        if old != input {
            return Err(Failure::conflict(
                "Misma identidad de evento con contenido diferente",
            ));
        }
        Ok(EventReceipt {
            id: row.get("id"),
            event_id: input.event_id,
            status: "RECEIVED".into(),
            ingested_at: row.get("ingested_at"),
        })
    }
    async fn create(&self, batch: Batch) -> Result<Batch> {
        self.0
            .write(
                &batch.input.tenant_id,
                "batch",
                batch.id,
                stored(&batch),
                None,
            )
            .await?;
        Ok(batch)
    }
    async fn batch(&self, tenant: &str, id: Uuid) -> Result<Batch> {
        self.0.get(tenant, "batch", id).await
    }
    async fn batches(&self, tenant: &str) -> Result<Vec<Batch>> {
        let rows=sqlx::query("SELECT body-'rows' AS body FROM corvus.resources WHERE tenant_id=$1 AND kind='batch' ORDER BY created_at DESC,id LIMIT 100")
            .bind(tenant).fetch_all(&self.0.pool).await.map_err(database_error)?;
        rows.into_iter()
            .map(|r| {
                serde_json::from_value(r.get("body"))
                    .map_err(|_| Failure::invalid("Registro incompatible"))
            })
            .collect()
    }
    async fn save(
        &self,
        batch: Batch,
        expected: i64,
        publication: Option<StructuralSnapshot>,
    ) -> Result<Batch> {
        let body = stored(&batch);
        let tenant = &batch.input.tenant_id;
        let mut tx = self.0.pool.begin().await.map_err(database_error)?;
        let changed=sqlx::query("UPDATE corvus.resources SET body=$3,version=version+1 WHERE tenant_id=$1 AND kind='batch' AND id=$2 AND version=$4")
            .bind(tenant).bind(batch.id).bind(&body).bind(expected).execute(&mut *tx).await.map_err(database_error)?.rows_affected();
        if changed != 1 {
            return Err(Failure::conflict("El lote cambió durante la operación"));
        }
        sqlx::query("INSERT INTO corvus.resource_versions(tenant_id,kind,id,version,body) VALUES($1,'batch',$2,$3,$4)")
            .bind(tenant).bind(batch.id).bind(batch.version).bind(&body).execute(&mut *tx).await.map_err(database_error)?;
        if let Some(snapshot) = publication {
            sqlx::query("INSERT INTO corvus.datasets(tenant_id,id,body) VALUES($1,$2,$3)")
                .bind(tenant)
                .bind(batch.id)
                .bind(json!(snapshot))
                .execute(&mut *tx)
                .await
                .map_err(database_error)?;
        }
        tx.commit().await.map_err(database_error)?;
        Ok(batch)
    }
    async fn datasets(&self, tenant: &str) -> Result<Vec<Value>> {
        let rows=sqlx::query("SELECT body-'rows' AS body FROM corvus.datasets WHERE tenant_id=$1 ORDER BY id LIMIT 100")
            .bind(tenant).fetch_all(&self.0.pool).await.map_err(database_error)?;
        Ok(rows.into_iter().map(|r| r.get("body")).collect())
    }
    async fn dataset(&self, tenant: &str, id: Uuid) -> Result<Value> {
        let row = sqlx::query("SELECT body FROM corvus.datasets WHERE tenant_id=$1 AND id=$2")
            .bind(tenant)
            .bind(id)
            .fetch_optional(&self.0.pool)
            .await
            .map_err(database_error)?
            .ok_or_else(Failure::missing)?;
        Ok(row.get("body"))
    }
}
