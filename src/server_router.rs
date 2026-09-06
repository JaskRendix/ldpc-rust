use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::bitarray::BitArray;
use crate::ldpc_decoder::LdpcDecoder;
use crate::matrices::h_256_512::H_256_512;
use crate::spa_decoder_llr::{DecoderError, SpaDecoderLLR};

use std::sync::atomic::{AtomicU64, Ordering};

static DECODE_COUNT: AtomicU64 = AtomicU64::new(0);
static LAST_LATENCY_US: AtomicU64 = AtomicU64::new(0);
static LAST_ITERATIONS: AtomicU64 = AtomicU64::new(0);

static BITFLIP_DECODER: LazyLock<LdpcDecoder<256, 512>> =
    LazyLock::new(|| LdpcDecoder::new(&H_256_512));

/// Upper bound on client-supplied iteration counts for both endpoints.
const MAX_ITERATIONS: usize = 200;

pub async fn metrics() -> String {
    format!(
        "ldpc_decode_count {}\nldpc_last_latency_us {}\nldpc_last_iterations {}\n",
        DECODE_COUNT.load(Ordering::Relaxed),
        LAST_LATENCY_US.load(Ordering::Relaxed),
        LAST_ITERATIONS.load(Ordering::Relaxed),
    )
}

pub async fn health() -> &'static str {
    "ok"
}

#[derive(Deserialize)]
pub struct DecodeRequest {
    #[serde(with = "serde_arrays")]
    pub cw: [u8; 512],
    pub iterations: usize,
}

#[derive(Serialize)]
pub struct DecodeResponse {
    pub valid: bool,
    pub cw: Vec<u8>,
    pub syndrome_weight: usize,
}

async fn decode_bitflip(
    Json(payload): Json<DecodeRequest>,
) -> Result<Json<DecodeResponse>, (StatusCode, String)> {
    let start = std::time::Instant::now();

    if let Some((idx, &bad)) = payload.cw.iter().enumerate().find(|&(_, &b)| b > 1) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("cw[{idx}] = {bad}, but bits must be 0 or 1"),
        ));
    }
    if payload.iterations == 0 || payload.iterations > MAX_ITERATIONS {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("iterations must be between 1 and {MAX_ITERATIONS}"),
        ));
    }

    // Pack the 512 individual bits into a 64-byte array
    let mut cw = [0u8; 64];
    for (j, &bit) in payload.cw.iter().enumerate() {
        BitArray::set_bit(&mut cw, j, bit == 1);
    }

    for _ in 0..payload.iterations {
        if BITFLIP_DECODER.iterate_bitflip(&mut cw) {
            break;
        }
    }

    let mut sn = [0u8; 256];
    let valid = BITFLIP_DECODER.get_parity(&cw, &mut sn);
    let syndrome_weight = sn.iter().filter(|&&b| b == 1).count();

    // Unpack using an efficient collector mapping over the fixed range
    let unpacked_cw: Vec<u8> = (0..512).map(|j| BitArray::get_bit(&cw, j)).collect();

    let duration = start.elapsed();
    DECODE_COUNT.fetch_add(1, Ordering::Relaxed);
    LAST_LATENCY_US.store(duration.as_micros() as u64, Ordering::Relaxed);
    LAST_ITERATIONS.store(payload.iterations as u64, Ordering::Relaxed);

    tracing::info!(
        target: "ldpc_decoder",
        algorithm = "bitflip",
        duration_micros = duration.as_micros(),
        iterations = payload.iterations,
        valid = valid,
        syndrome_weight = syndrome_weight,
        "bit-flip decode completed"
    );

    Ok(Json(DecodeResponse {
        valid,
        cw: unpacked_cw,
        syndrome_weight,
    }))
}

#[derive(Deserialize)]
pub struct SpaDecodeRequest {
    #[serde(with = "serde_arrays")]
    pub cw: [f64; 512],
    pub snr_db: f64,
    pub iterations: Option<usize>,
    pub scaling_factor: Option<f64>,
}

#[derive(Serialize)]
pub struct SpaDecodeResponse {
    pub valid: bool,
    pub cw: Vec<u8>, // decoded bits
    pub syndrome_weight: usize,
    pub iterations: usize,
    pub converged: bool,
}

async fn decode_spa(
    Json(req): Json<SpaDecodeRequest>,
) -> Result<Json<SpaDecodeResponse>, (StatusCode, String)> {
    let start = std::time::Instant::now();

    if let Some((idx, &bad)) = req.cw.iter().enumerate().find(|&(_, v)| !v.is_finite()) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("cw[{idx}] = {bad}, but LLR values must be finite"),
        ));
    }
    if req.snr_db.is_nan() {
        return Err((
            StatusCode::BAD_REQUEST,
            "snr_db must not be NaN".to_string(),
        ));
    }
    if !req.snr_db.is_finite() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("snr_db must be finite, got {}", req.snr_db),
        ));
    }
    if req.snr_db < -20.0 || req.snr_db > 50.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("snr_db out of reasonable range [-20, 50]: {}", req.snr_db),
        ));
    }
    let max_iter = req.iterations.unwrap_or(20);
    if max_iter == 0 || max_iter > MAX_ITERATIONS {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("iterations must be between 1 and {MAX_ITERATIONS}"),
        ));
    }

    let mut decoder: SpaDecoderLLR<256, 512> = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(max_iter);

    if let Some(alpha) = req.scaling_factor {
        if alpha.is_nan() || !alpha.is_finite() || alpha < 0.0 {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "scaling_factor must be a finite non-negative number, got {}",
                    alpha
                ),
            ));
        }
        if alpha > 10.0 {
            return Err((
                StatusCode::BAD_REQUEST,
                "scaling_factor too large".to_string(),
            ));
        }
        decoder.set_scaling_factor(alpha);
    }

    let decode_res = decoder.decode(&req.cw).map_err(|e| match e {
        DecoderError::InvalidInputLength => (
            StatusCode::BAD_REQUEST,
            "Invalid input length for SPA decoder".to_string(),
        ),
    })?;

    let decoded = decode_res.codeword;
    let actual_iterations = decode_res.iterations;
    let converged = decode_res.converged;

    // Compute syndrome weight using the decoder's parity method safely
    let mut sn = [0u8; 256];
    let mut cw_bytes = [0u8; 64];
    for (j, &bit) in decoded.iter().enumerate() {
        BitArray::set_bit(&mut cw_bytes, j, bit == 1);
    }
    let valid_syndrome = BITFLIP_DECODER.get_parity(&cw_bytes, &mut sn);
    let syndrome_weight = sn.iter().filter(|&&b| b == 1).count();
    let valid = converged && valid_syndrome && syndrome_weight == 0;

    let duration = start.elapsed();
    DECODE_COUNT.fetch_add(1, Ordering::Relaxed);
    LAST_LATENCY_US.store(duration.as_micros() as u64, Ordering::Relaxed);
    LAST_ITERATIONS.store(actual_iterations as u64, Ordering::Relaxed);

    tracing::info!(
        target: "ldpc_decoder",
        algorithm = "spa",
        duration_micros = duration.as_micros(),
        iterations = actual_iterations,
        converged = converged,
        valid = valid,
        syndrome_weight = syndrome_weight,
        "spa decode completed"
    );

    Ok(Json(SpaDecodeResponse {
        valid,
        cw: decoded,
        syndrome_weight,
        iterations: actual_iterations,
        converged,
    }))
}

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/", get(health))
        .route("/metrics", get(metrics))
        .route("/decode/bitflip", post(decode_bitflip))
        .route("/decode/spa", post(decode_spa))
}
