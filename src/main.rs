mod model;
mod repository;
mod routes;

use crate::repository::BatteryRepository;
use crate::routes::battery_routes;
use std::env;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let clickhouse_url =
        env::var("CLICKHOUSE_URL").unwrap_or_else(|_| "http://localhost:8123".into());
    let clickhouse_db =
        env::var("CLICKHOUSE_DB").unwrap_or_else(|_| "default".into());
    let clickhouse_user =
        env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".into());
    let clickhouse_password =
        env::var("CLICKHOUSE_PASSWORD").unwrap_or_default();
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    tracing::info!("Starting Battery Stats Server on port {port}");
    tracing::info!("ClickHouse URL: {clickhouse_url}, DB: {clickhouse_db}");

    let repository = BatteryRepository::new(&clickhouse_url, &clickhouse_db, &clickhouse_user, &clickhouse_password);

    let app = battery_routes(Arc::new(repository));

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("failed to bind");

    tracing::info!("Listening on 0.0.0.0:{port}");

    axum::serve(listener, app).await.expect("server error");
}
