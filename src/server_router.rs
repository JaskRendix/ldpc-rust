use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::bitarray::BitArray;
use crate::ldpc_decoder::LdpcDecoder;
use crate::matrices::h_256_512::H_256_512;
use crate::spa_decoder_llr::SpaDecoderLLR;

use std::sync::atomic::{AtomicU64, Ordering};

static DECODE_COUNT: AtomicU64 = AtomicU64::new(0);
static LAST_LATENCY_US: AtomicU64 = AtomicU64::new(0);
static LAST_ITERATIONS: AtomicU64 = AtomicU64::new(0);

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

//
// ------------------------------------------------------------
// BIT-FLIP (hard decision, FULL 512 bits)
// ------------------------------------------------------------
//

#[derive(Deserialize)]
pub struct DecodeRequest {
    pub cw: Vec<u8>, // HARD bits, length MUST be 512, values MUST be 0 or 1
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

    if payload.cw.len() != 512 {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "cw must have exactly 512 elements, got {}",
                payload.cw.len()
            ),
        ));
    }
    if let Some((idx, &bad)) = payload.cw.iter().enumerate().find(|(_, &b)| b > 1) {
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

    let decoder = LdpcDecoder::new(&H_256_512);

    // Pack the 512 individual bits into a 64-byte array
    let mut cw = [0u8; 64];
    for (j, &bit) in payload.cw.iter().enumerate() {
        BitArray::set_bit(&mut cw, j, bit == 1);
    }

    for _ in 0..payload.iterations {
        if decoder.iterate_bitflip(&mut cw) {
            break;
        }
    }

    let mut sn = [0u8; 256];
    let valid = decoder.get_parity(&cw, &mut sn);
    let syndrome_weight = sn.iter().map(|b| *b as usize).sum();

    // Unpack the 64-byte array back into a 512-element Vec<u8> for the response
    let mut unpacked_cw = Vec::with_capacity(512);
    for j in 0..512 {
        unpacked_cw.push(BitArray::get_bit(&cw, j));
    }

    DECODE_COUNT.fetch_add(1, Ordering::Relaxed);
    LAST_LATENCY_US.store(start.elapsed().as_micros() as u64, Ordering::Relaxed);
    LAST_ITERATIONS.store(payload.iterations as u64, Ordering::Relaxed);

    Ok(Json(DecodeResponse {
        valid,
        cw: unpacked_cw,
        syndrome_weight,
    }))
}

//
// ------------------------------------------------------------
// SPA (soft decision, LLR-domain, FULL 512 bits)
// ------------------------------------------------------------
//

#[derive(Deserialize)]
pub struct SpaDecodeRequest {
    pub cw: Vec<f64>, // LLRs, length MUST be 512, values MUST be finite
    pub snr_db: f64,
    pub iterations: Option<usize>,
    pub scaling_factor: Option<f64>, // Optional NMS scaling factor (e.g. 0.75)
}

#[derive(Serialize)]
pub struct SpaDecodeResponse {
    pub valid: bool,
    pub cw: Vec<u8>, // decoded bits
    pub syndrome_weight: usize,
}

async fn decode_spa(
    Json(req): Json<SpaDecodeRequest>,
) -> Result<Json<SpaDecodeResponse>, (StatusCode, String)> {
    let start = std::time::Instant::now();

    if req.cw.len() != 512 {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("cw must have exactly 512 LLR values, got {}", req.cw.len()),
        ));
    }
    if let Some((idx, &bad)) = req.cw.iter().enumerate().find(|(_, v)| !v.is_finite()) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("cw[{idx}] = {bad}, but LLR values must be finite"),
        ));
    }
    if !req.snr_db.is_finite() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("snr_db must be finite, got {}", req.snr_db),
        ));
    }
    let max_iter = req.iterations.unwrap_or(20);
    if max_iter == 0 || max_iter > MAX_ITERATIONS {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("iterations must be between 1 and {MAX_ITERATIONS}"),
        ));
    }

    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(max_iter);

    if let Some(alpha) = req.scaling_factor {
        if !alpha.is_finite() || alpha < 0.0 {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("scaling_factor must be a finite non-negative number, got {}", alpha),
            ));
        }
        decoder.set_scaling_factor(alpha);
    }

    let decoded = decoder.decode(&req.cw);

    // Compute syndrome weight
    let mut syndrome_weight = 0usize;

    for row in H_256_512.iter() {
        let mut sum = 0u8;
        for (j, &h_ij) in row.iter().enumerate() {
            if h_ij == 1 {
                sum ^= decoded[j];
            }
        }
        if sum != 0 {
            syndrome_weight += 1;
        }
    }

    DECODE_COUNT.fetch_add(1, Ordering::Relaxed);
    LAST_LATENCY_US.store(start.elapsed().as_micros() as u64, Ordering::Relaxed);
    LAST_ITERATIONS.store(max_iter as u64, Ordering::Relaxed);

    Ok(Json(SpaDecodeResponse {
        valid: syndrome_weight == 0,
        cw: decoded,
        syndrome_weight,
    }))
}

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/decode/bitflip", post(decode_bitflip))
        .route("/decode/spa", post(decode_spa))
}
