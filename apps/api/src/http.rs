use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use tordo_commons_core::{Failure, Scope};
use tordo_corvus_core::{application::Corvus, domain::*};
use tordo_runtime::{ApiError, database_error};
use uuid::Uuid;

#[derive(Clone)]
pub struct App {
    pub sales: tordo_corvus_core::sales::use_cases::SalesUseCases,
    pub corvus: Corvus,
    pub db: PgPool,
}
pub async fn extract_sales(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
    Json(command): Json<tordo_corvus_core::sales::contracts::ExtractSalesSnapshot>,
) -> Result<Json<tordo_analytics_contracts::SnapshotManifest>, ApiError> {
    Ok(Json(a.sales.extract(&s, command).await?))
}
pub async fn sales_manifest(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
) -> Result<Json<tordo_analytics_contracts::SnapshotManifest>, ApiError> {
    Ok(Json(a.sales.current(&s, None).await?))
}
pub async fn sales_query(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
    Json(request): Json<tordo_analytics_contracts::DatasetQueryRequest>,
) -> Result<Json<tordo_analytics_contracts::TableResult>, ApiError> {
    Ok(Json(a.sales.query(&s, request).await?))
}
pub async fn health(State(app): State<App>) -> (StatusCode, Json<Value>) {
    let ready = sqlx::query("SELECT 1 FROM corvus.resources LIMIT 1")
        .execute(&app.db)
        .await
        .is_ok();
    (
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(
            json!({"service":"corvus","status":if ready{"ready"}else{"not_ready"},"backend":"rust"}),
        ),
    )
}
pub async fn capabilities() -> Json<Value> {
    Json(json!({"component":"Corvus","phase":"composable_analytics",
    "implemented":["events","compatible_replay","batches","structural_validation","atomic_snapshot","tenant_scope","sales_domain_pipeline","bronze_silver_gold","registered_query_serving","atomic_audit","snapshot_cas"],
    "planned":["incremental_ingestion","other_domains","parquet","analytical_sql","lineage","durable_workers"],
    "layers":["bronze","silver","gold"]}))
}
pub async fn receive(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
    Json(v): Json<EventInput>,
) -> Result<(StatusCode, Json<EventReceipt>), ApiError> {
    Ok((StatusCode::ACCEPTED, Json(a.corvus.receive(&s, v).await?)))
}
pub async fn create(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
    Json(v): Json<CreateBatch>,
) -> Result<(StatusCode, Json<Batch>), ApiError> {
    Ok((StatusCode::CREATED, Json(a.corvus.create(&s, v).await?)))
}
pub async fn batches(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
) -> Result<Json<Vec<Batch>>, ApiError> {
    Ok(Json(a.corvus.batches(&s).await?))
}
pub async fn batch(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
    Path(id): Path<Uuid>,
) -> Result<Json<Batch>, ApiError> {
    Ok(Json(a.corvus.batch(&s, id).await?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rows {
    rows: Vec<Value>,
}
pub async fn validate(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
    Path(id): Path<Uuid>,
    Json(v): Json<Rows>,
) -> Result<Json<Batch>, ApiError> {
    Ok(Json(a.corvus.validate(&s, id, v.rows).await?))
}
pub async fn confirm(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
    Path(id): Path<Uuid>,
) -> Result<Json<Batch>, ApiError> {
    Ok(Json(a.corvus.confirm(&s, id).await?))
}
pub async fn datasets(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
) -> Result<Json<Vec<Value>>, ApiError> {
    Ok(Json(a.corvus.datasets(&s).await?))
}
pub async fn dataset(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(a.corvus.dataset(&s, id).await?))
}
#[derive(Deserialize)]
pub struct Range {
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}
pub async fn kpis(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
    Path(company): Path<String>,
    Query(range): Query<Range>,
) -> Result<Json<Value>, ApiError> {
    s.require("corvus:read")?;
    s.tenant(&company)?;
    if range.from.zip(range.to).is_some_and(|(f, t)| f > t) {
        return Err(Failure::invalid("Rango de fechas inválido").into());
    }
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM corvus.ingestion_events WHERE tenant_id=$1 AND company_id=$1 AND ($2::date IS NULL OR occurred_at >= $2::date) AND ($3::date IS NULL OR occurred_at < ($3::date+1))")
        .bind(&company).bind(range.from).bind(range.to).fetch_one(&a.db).await.map_err(database_error)?;
    Ok(Json(
        json!({"companyId":company,"from":range.from,"to":range.to,"kpis":[{"code":"EVENTS_RECEIVED","value":count.to_string(),"unit":"events","status":"RAW_ONLY"}]}),
    ))
}

pub async fn pipelines(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
) -> Result<Json<Vec<tordo_corvus_core::sales::contracts::Pipeline>>, ApiError> {
    Ok(Json(a.sales.pipelines(&s).await?))
}
pub async fn create_pipeline(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
    Json(v): Json<tordo_corvus_core::sales::contracts::PipelineInput>,
) -> Result<
    (
        StatusCode,
        Json<tordo_corvus_core::sales::contracts::Pipeline>,
    ),
    ApiError,
> {
    Ok((
        StatusCode::CREATED,
        Json(a.sales.create_pipeline(&s, v).await?),
    ))
}
pub async fn ingestion_sources(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
) -> Result<Json<Vec<tordo_corvus_core::sales::contracts::IngestionSource>>, ApiError> {
    Ok(Json(a.sales.sources(&s).await?))
}
pub async fn pipeline_dataset(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
    Path(id): Path<Uuid>,
) -> Result<Json<tordo_analytics_contracts::SnapshotManifest>, ApiError> {
    Ok(Json(a.sales.current(&s, Some(id)).await?))
}
pub async fn run_pipeline(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
    Path(id): Path<Uuid>,
    Json(v): Json<tordo_corvus_core::sales::contracts::RunPipeline>,
) -> Result<Json<tordo_analytics_contracts::SnapshotManifest>, ApiError> {
    Ok(Json(a.sales.run_pipeline(&s, id, v).await?))
}
pub async fn pipeline_runs(
    State(a): State<App>,
    Extension(s): Extension<Scope>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Value>>, ApiError> {
    Ok(Json(a.sales.runs(&s, id).await?))
}
pub async fn models(
    Extension(s): Extension<Scope>,
) -> Result<Json<Vec<tordo_analytics_contracts::AnalyticalModel>>, ApiError> {
    s.require("corvus:query")?;
    Ok(Json(tordo_analytics_contracts::sales_models()))
}
pub async fn ingestion_domains(Extension(s): Extension<Scope>) -> Result<Json<Value>, ApiError> {
    s.require("corvus:read")?;
    Ok(Json(
        json!([{"code":"sales_documents","name":"Comprobantes de venta y vínculo con pedidos","datasetCode":"sales.v1","layers":["bronze","silver","gold"]}]),
    ))
}
