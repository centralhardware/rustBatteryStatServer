use battery_model::BatteryHealth;
use crate::repository::BatteryRepository;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::sync::Arc;

/// `token`: when set, POST /api/battery/health requires `Authorization: Bearer <token>`.
pub fn battery_routes(repository: Arc<BatteryRepository>, token: Option<String>) -> Router {
    let token = Arc::new(token);
    let api = Router::new()
        .route("/api/battery/health", post(post_health))
        .route_layer(middleware::from_fn(move |req, next| require_token(token.clone(), req, next)));
    Router::new()
        .merge(api)
        .route("/ping", get(ping))
        .with_state(repository)
}

async fn require_token(token: Arc<Option<String>>, req: Request, next: Next) -> Response {
    let Some(expected) = token.as_deref() else {
        return next.run(req).await;
    };
    let given = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    if given == Some(expected) {
        next.run(req).await
    } else {
        (StatusCode::UNAUTHORIZED, Json(json!({"status": "error", "message": "unauthorized"}))).into_response()
    }
}

async fn ping(
    State(repo): State<Arc<BatteryRepository>>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    repo.ping().await.map_err(|e| {
        tracing::error!("ClickHouse ping failed: {e}");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"status": "error", "message": e.to_string()})),
        )
    })?;

    Ok((StatusCode::OK, Json(json!({"status": "ok"}))))
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
