use crate::model::BatteryHealth;
use crate::repository::BatteryRepository;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};
use std::sync::Arc;

pub fn battery_routes(repository: Arc<BatteryRepository>) -> Router {
    Router::new()
        .route("/api/battery/health", post(post_health))
        .with_state(repository)
}

async fn post_health(
    State(repo): State<Arc<BatteryRepository>>,
    Json(health): Json<BatteryHealth>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    repo.save(health).await.map_err(|e| {
        tracing::error!("Failed to save battery health: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"status": "error", "message": e.to_string()})),
        )
    })?;

    Ok((StatusCode::CREATED, Json(json!({"status": "success"}))))
}
