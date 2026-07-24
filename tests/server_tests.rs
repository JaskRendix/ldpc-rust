use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn test_health() {
    let app = ldpc_rust::server_router::router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_bitflip() {
    let app = ldpc_rust::server_router::router();

    // MUST be 512 bits — server asserts this
    let payload = json!({
        "cw": vec![0u8; 512],
        "iterations": 5
    });

    let request = Request::builder()
        .method("POST")
        .uri("/decode/bitflip")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_spa_decode() {
    let app = ldpc_rust::server_router::router();

    // MUST be 512 LLRs
    let payload = json!({
        "cw": vec![0.0f64; 512],
        "snr_db": 1.0,
        "iterations": 5
    });

    let request = Request::builder()
        .method("POST")
        .uri("/decode/spa")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(parsed.get("cw").is_some());
    assert!(parsed.get("syndrome_weight").is_some());
    assert!(parsed.get("valid").is_some());
}

#[tokio::test]
async fn test_metrics() {
    let app = ldpc_rust::server_router::router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let text = String::from_utf8(body.to_vec()).unwrap();

    assert!(text.contains("ldpc_decode_count"));
    assert!(text.contains("ldpc_last_latency_us"));
    assert!(text.contains("ldpc_last_iterations"));
}

#[tokio::test]
async fn test_spa_decode_with_scaling_factor() {
    let app = ldpc_rust::server_router::router();

    let payload = json!({
        "cw": vec![2.0f64; 512],
        "snr_db": 2.5,
        "iterations": 10,
        "scaling_factor": 0.75
    });

    let request = Request::builder()
        .method("POST")
        .uri("/decode/spa")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_spa_decode_invalid_scaling_factor() {
    let app = ldpc_rust::server_router::router();

    let payload = json!({
        "cw": vec![1.0f64; 512],
        "snr_db": 1.0,
        "scaling_factor": -0.5 // Invalid negative factor
    });

    let request = Request::builder()
        .method("POST")
        .uri("/decode/spa")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
