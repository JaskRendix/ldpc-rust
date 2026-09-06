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
    assert!(parsed.get("iterations").is_some());
    assert!(parsed.get("converged").is_some());
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
        "scaling_factor": -0.5
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

#[tokio::test]
async fn test_bitflip_invalid_bit() {
    let app = ldpc_rust::server_router::router();

    let mut cw = vec![0u8; 512];
    cw[42] = 2; // invalid bit

    let payload = json!({
        "cw": cw,
        "iterations": 5
    });

    let request = Request::builder()
        .method("POST")
        .uri("/decode/bitflip")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_bitflip_wrong_length() {
    let app = ldpc_rust::server_router::router();

    let payload = json!({
        "cw": vec![0u8; 511],
        "iterations": 5
    });

    let request = Request::builder()
        .method("POST")
        .uri("/decode/bitflip")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // serde_arrays deserialization failure returns UNPROCESSABLE_ENTITY (422)
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_spa_nan_llr() {
    let app = ldpc_rust::server_router::router();

    // Construct raw JSON string since serde_json turns f64::NAN into null
    let mut cw_str = String::from("[");
    for i in 0..512 {
        if i == 10 {
            cw_str.push_str("\"NaN\"");
        } else {
            cw_str.push_str("0.0");
        }
        if i < 511 {
            cw_str.push(',');
        }
    }
    cw_str.push(']');

    let body_str = format!(r#"{{"cw": {}, "snr_db": 1.0, "iterations": 5}}"#, cw_str);

    let request = Request::builder()
        .method("POST")
        .uri("/decode/spa")
        .header("content-type", "application/json")
        .body(Body::from(body_str))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_spa_snr_out_of_range() {
    let app = ldpc_rust::server_router::router();

    let payload = json!({
        "cw": vec![0.0f64; 512],
        "snr_db": 100.0,
        "iterations": 5
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

#[tokio::test]
async fn test_spa_zero_iterations() {
    let app = ldpc_rust::server_router::router();

    let payload = json!({
        "cw": vec![0.0f64; 512],
        "snr_db": 1.0,
        "iterations": 0
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

#[tokio::test]
async fn test_spa_default_iterations() {
    let app = ldpc_rust::server_router::router();

    let payload = json!({
        "cw": vec![0.0f64; 512],
        "snr_db": 1.0
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
    // Verify that iterations finished successfully within the max limit (<= 20)
    let iters = parsed["iterations"].as_u64().unwrap();
    assert!((1..=20).contains(&iters));
}

#[tokio::test]
async fn test_spa_scaling_factor_too_large() {
    let app = ldpc_rust::server_router::router();

    let payload = json!({
        "cw": vec![0.0f64; 512],
        "snr_db": 1.0,
        "iterations": 5,
        "scaling_factor": 50.0
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

#[tokio::test]
async fn test_spa_wrong_length() {
    let app = ldpc_rust::server_router::router();

    let payload = json!({
        "cw": vec![0.0f64; 511],
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
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_spa_too_long() {
    let app = ldpc_rust::server_router::router();

    let payload = json!({
        "cw": vec![0.0f64; 600],
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
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_spa_scaling_factor_nan() {
    let app = ldpc_rust::server_router::router();

    let body_str = r#"{"cw": [0.0], "snr_db": 1.0, "scaling_factor": "NaN"}"#;
    // Will fail length or deserialize correctly as bad request / unprocessable entity
    let request = Request::builder()
        .method("POST")
        .uri("/decode/spa")
        .header("content-type", "application/json")
        .body(Body::from(body_str))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(response.status().is_client_error());
}

#[tokio::test]
async fn test_timeout_layer_integration() {
    use std::time::Duration;
    use tower_http::timeout::TimeoutLayer;

    let app = ldpc_rust::server_router::router().layer(TimeoutLayer::with_status_code(
        StatusCode::GATEWAY_TIMEOUT,
        Duration::from_secs(10),
    ));

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
