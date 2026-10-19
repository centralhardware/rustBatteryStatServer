use battery_model::BatteryHealth;
use crate::repository::Store;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::sync::Arc;

/// `token`: when set, POST /api/battery/health requires `Authorization: Bearer <token>`.
pub fn battery_routes<S: Store>(repository: Arc<S>, token: Option<String>) -> Router {
    let token = Arc::new(token);
    let api = Router::new()
        .route("/api/battery/health", post(post_health::<S>))
        .route_layer(middleware::from_fn(move |req, next| require_token(token.clone(), req, next)));
    Router::new()
        .merge(api)
        .route("/ping", get(ping::<S>))
        .with_state(repository)
}

async fn require_token(token: Arc<Option<String>>, req: Request, next: Next) -> Response {
    let Some(expected) = token.as_deref() else {
        return next.run(req).await;
    };
    // The scheme is case-insensitive (RFC 7235); the token itself is not.
    let given = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, t)| t.trim());
    if given.is_some_and(|g| constant_time_eq(g.as_bytes(), expected.as_bytes())) {
        next.run(req).await
    } else {
        (StatusCode::UNAUTHORIZED, Json(json!({"status": "error", "message": "unauthorized"}))).into_response()
    }
}

/// Equality that takes as long however early the inputs differ. Only the
/// length can leak, and the length of a random token tells nothing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// The database's own error stays in the server log: `/ping` is open to anyone,
/// and the message can name hosts, tables and users.
fn db_error(status: StatusCode) -> (StatusCode, Json<Value>) {
    (status, Json(json!({"status": "error", "message": "database unavailable"})))
}

async fn ping<S: Store>(
    State(repo): State<Arc<S>>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    repo.ping().await.map_err(|e| {
        tracing::error!("ClickHouse ping failed: {e}");
        db_error(StatusCode::SERVICE_UNAVAILABLE)
    })?;

    Ok((StatusCode::OK, Json(json!({"status": "ok"}))))
}

async fn post_health<S: Store>(
    State(repo): State<Arc<S>>,
    Json(health): Json<BatteryHealth>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    repo.save(health).await.map_err(|e| {
        tracing::error!("Failed to save battery health: {e}");
        db_error(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    Ok((StatusCode::CREATED, Json(json!({"status": "success"}))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::sync::Mutex;
    use tower::ServiceExt;

    #[derive(Default)]
    struct Fake {
        saved: Mutex<Vec<BatteryHealth>>,
        down: bool,
    }

    impl Store for Fake {
        async fn ping(&self) -> Result<(), String> {
            if self.down { Err("secret-host:9000 auth failed for user x".into()) } else { Ok(()) }
        }
        async fn save(&self, health: BatteryHealth) -> Result<(), String> {
            if self.down {
                return Err("secret-host:9000 auth failed for user x".into());
            }
            self.saved.lock().unwrap().push(health);
            Ok(())
        }
    }

    fn body() -> Body {
        Body::from(
            serde_json::to_string(&BatteryHealth { device_id: "D".into(), ..Default::default() }).unwrap(),
        )
    }

    async fn post(fake: Arc<Fake>, token: Option<&str>, auth: Option<&str>) -> (StatusCode, String) {
        let mut req = Request::post("/api/battery/health").header("content-type", "application/json");
        if let Some(a) = auth {
            req = req.header("authorization", a);
        }
        let resp = battery_routes(fake, token.map(str::to_owned))
            .oneshot(req.body(body()).unwrap())
            .await
            .unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn token_checks() {
        let fake = Arc::new(Fake::default());
        assert_eq!(post(fake.clone(), Some("t0k"), None).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(post(fake.clone(), Some("t0k"), Some("Bearer nope")).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(post(fake.clone(), Some("t0k"), Some("Basic t0k")).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(post(fake.clone(), Some("t0k"), Some("Bearer t0k")).await.0, StatusCode::CREATED);
        assert_eq!(post(fake.clone(), Some("t0k"), Some("bearer t0k")).await.0, StatusCode::CREATED);
        assert_eq!(post(fake.clone(), None, None).await.0, StatusCode::CREATED);
        assert_eq!(fake.saved.lock().unwrap().len(), 3);
        assert_eq!(fake.saved.lock().unwrap()[0].device_id, "D");
    }

    #[tokio::test]
    async fn ping_is_open_and_hides_db_errors() {
        let ok = battery_routes(Arc::new(Fake::default()), Some("t".into()))
            .oneshot(Request::get("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK);

        let down = battery_routes(Arc::new(Fake { down: true, ..Default::default() }), None)
            .oneshot(Request::get("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(down.status(), StatusCode::SERVICE_UNAVAILABLE);
        let text = axum::body::to_bytes(down.into_body(), usize::MAX).await.unwrap();
        assert!(!String::from_utf8_lossy(&text).contains("secret-host"));
    }

    #[tokio::test]
    async fn save_error_is_generic() {
        let (status, text) = post(Arc::new(Fake { down: true, ..Default::default() }), None, None).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!text.contains("secret-host"));
    }

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }
}
