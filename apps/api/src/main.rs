mod http;
mod persistence;
mod sales_persistence;
mod sales_queries;
mod sales_source;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
};
use sqlx::postgres::PgPoolOptions;
use std::{sync::Arc, time::Duration};
use tordo_corvus_core::application::Corvus;
use tordo_corvus_core::sales::use_cases::SalesUseCases;
use tordo_runtime::{
    auth::{Auth, protect},
    persistence::JsonStore,
};
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();
    let url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL requerida")?;
    let db = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await?;
    if std::env::args().any(|a| a == "migrate") {
        sqlx::migrate!("./migrations").run(&db).await?;
        return Ok(());
    }
    let central = if let Ok(url) = std::env::var("CENTRAL_DATABASE_URL") {
        Some(
            PgPoolOptions::new()
                .max_connections(2)
                .connect(&url)
                .await?,
        )
    } else {
        None
    };
    let auth = Auth::from_env(central).map_err(|e| e.message)?;
    let sales_store = Arc::new(sales_persistence::PostgresSales { pool: db.clone() });
    let sales = SalesUseCases::new(
        Arc::new(sales_source::PostgresSalesExportReader::from_env().map_err(|e| e.message)?),
        sales_store.clone(),
        sales_store.clone(),
        sales_store.clone(),
        Arc::new(sales_queries::PostgresSalesQuery {
            pool: db.clone(),
            catalog: (*sales_store).clone(),
        }),
        sales_store.clone(),
        sales_store.clone(),
    );
    let app = http::App {
        sales,
        corvus: Corvus::new(Arc::new(persistence::PostgresCorvus(JsonStore::new(
            db.clone(),
            "corvus",
        )))),
        db,
    };
    let protected = Router::new()
        .route(
            "/api/v1/pipelines",
            get(http::pipelines).post(http::create_pipeline),
        )
        .route(
            "/api/v1/pipelines/{id}/dataset",
            get(http::pipeline_dataset),
        )
        .route(
            "/api/v1/pipelines/{id}/ingestions",
            axum::routing::post(http::run_pipeline),
        )
        .route("/api/v1/pipelines/{id}/runs", get(http::pipeline_runs))
        .route("/api/v1/ingestion-sources", get(http::ingestion_sources))
        .route("/api/v1/ingestion-domains", get(http::ingestion_domains))
        .route("/api/v1/models", get(http::models))
        .route("/api/v1/sales/ingestions", post(http::extract_sales))
        .route("/api/v1/sales/dataset", get(http::sales_manifest))
        .route("/api/v1/sales/query", post(http::sales_query))
        .route("/api/v1/events", post(http::receive))
        .route(
            "/api/v1/import-batches",
            get(http::batches).post(http::create),
        )
        .route("/api/v1/import-batches/{id}", get(http::batch))
        .route("/api/v1/import-batches/{id}/validate", post(http::validate))
        .route("/api/v1/import-batches/{id}/confirm", post(http::confirm))
        .route("/api/v1/datasets", get(http::datasets))
        .route("/api/v1/datasets/{id}", get(http::dataset))
        .route("/analytics/v1/companies/{company}/kpis", get(http::kpis))
        .route_layer(middleware::from_fn_with_state(auth, protect));
    let ui = std::env::var("UI_DIR").unwrap_or_else(|_| "apps/ui/dist/corvus-ui/browser".into());
    let routes = Router::new()
        .merge(protected)
        .route("/health", get(http::health))
        .route("/api/v1/capabilities", get(http::capabilities))
        .with_state(app)
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .fallback_service(
            ServeDir::new(&ui).not_found_service(ServeFile::new(format!("{ui}/index.html"))),
        );
    let address = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8086".into());
    tracing::info!(%address,"Corvus listening");
    axum::serve(tokio::net::TcpListener::bind(address).await?, routes)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
