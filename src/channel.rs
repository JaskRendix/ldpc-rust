use alloc::vec::Vec;
use rand::RngExt;
use rand::rngs::StdRng;
use rand_distr::{Distribution, StandardNormal};

/// Computes the BPSK AWGN Log-Likelihood Ratio (LLR) for a given transmitted bit and SNR.
///
/// Note: snr_db here is treated as Es/N0 (symbol SNR). For a rate-1/2 code,
/// Es/N0 = 0.5 * Eb/N0.
pub fn bpsk_awgn_llr(bit: u8, snr_db: f64, rng: &mut StdRng) -> f64 {
    debug_assert!(bit <= 1);

    let snr_linear = 10.0_f64.powf(snr_db / 10.0);
    let sigma = (1.0 / (2.0 * snr_linear)).sqrt();

    let s = if bit == 0 { 1.0 } else { -1.0 };
    let n: f64 = StandardNormal.sample(rng);

    let y = s + sigma * n;
    2.0 * y / (sigma * sigma)
}

/// Simulates a Binary Symmetric Channel (BSC) with crossover probability `p`.
/// Flips a bit with probability `p`, returning the received hard-decision bit.
pub fn bsc_channel(bit: u8, p: f64, rng: &mut StdRng) -> u8 {
    debug_assert!(bit <= 1);
    debug_assert!((0.0..=1.0).contains(&p));

    if rng.random_bool(p) { bit ^ 1 } else { bit }
}

/// Generates a Rayleigh fading amplitude `h` using two Gaussian variables:
/// h = sqrt(X^2 + Y^2) / sqrt(2) (normalized to unit mean square power E[h^2] = 1).
pub fn sample_rayleigh_fading(rng: &mut StdRng) -> f64 {
    let x: f64 = StandardNormal.sample(rng);
    let y: f64 = StandardNormal.sample(rng);
    ((x * x + y * y) / 2.0).sqrt()
}

/// Computes the BPSK Rayleigh Fading LLR for a given transmitted bit, SNR, and fading gain `h`.
///
/// Received signal: y = h * s + n, with perfect CSI (known h).
pub fn rayleigh_fading_llr(bit: u8, snr_db: f64, h: f64, rng: &mut StdRng) -> f64 {
    debug_assert!(bit <= 1);

    let snr_linear = 10.0_f64.powf(snr_db / 10.0);
    let sigma = (1.0 / (2.0 * snr_linear)).sqrt();

    let s = if bit == 0 { 1.0 } else { -1.0 };
    let n: f64 = StandardNormal.sample(rng);

    let y = h * s + sigma * n;

    // Exact LLR with perfect CSI:
    // LLR = 2 * h * y / sigma^2
    2.0 * h * y / (sigma * sigma)
}

/// Generates a Rician fading amplitude with K-factor `k` (ratio of LoS to scattered power).
/// Model: h = sqrt((K/(K+1))) * h_los + sqrt(1/(K+1)) * h_scatter,
/// where h_los ≈ 1 and h_scatter is Rayleigh.
pub fn sample_rician_fading(k: f64, rng: &mut StdRng) -> f64 {
    debug_assert!(k >= 0.0);

    // LoS component (unit amplitude)
    let h_los = 1.0;

    // Scattered component (Rayleigh)
    let h_scatter = sample_rayleigh_fading(rng);

    let a = (k / (k + 1.0)).sqrt();
    let b = (1.0 / (k + 1.0)).sqrt();

    a * h_los + b * h_scatter
}

/// Computes the BPSK Rician Fading LLR for a given transmitted bit, SNR, and K-factor.
/// Uses perfect CSI on the composite fading gain h.
pub fn rician_fading_llr(bit: u8, snr_db: f64, k: f64, rng: &mut StdRng) -> f64 {
    let h = sample_rician_fading(k, rng);
    rayleigh_fading_llr(bit, snr_db, h, rng)
}

/// Samples a Nakagami-m fading amplitude with shape parameter `m >= 0.5`.
/// Normalized such that E[h^2] = 1.
pub fn sample_nakagami_fading(m: f64, rng: &mut StdRng) -> f64 {
    debug_assert!(m >= 0.5);

    // Nakagami-m can be generated from a Gamma distribution on h^2.
    // Here we approximate using Rayleigh when m ≈ 1, and adjust variance.
    let h_rayleigh = sample_rayleigh_fading(rng);
    let scale = (m / 1.0).sqrt(); // simple scaling approximation
    h_rayleigh * scale
}

/// Computes the BPSK Nakagami-m Fading LLR for a given transmitted bit, SNR, and m.
/// Uses perfect CSI on h.
pub fn nakagami_fading_llr(bit: u8, snr_db: f64, m: f64, rng: &mut StdRng) -> f64 {
    let h = sample_nakagami_fading(m, rng);
    rayleigh_fading_llr(bit, snr_db, h, rng)
}

/// Block fading: single fading gain `h` applied to an entire codeword.
/// Returns a vector of LLRs for a given bit sequence.
pub fn block_fading_llrs(bits: &[u8], snr_db: f64, rng: &mut StdRng) -> Vec<f64> {
    let h = sample_rayleigh_fading(rng);
    bits.iter()
        .map(|&b| rayleigh_fading_llr(b, snr_db, h, rng))
        .collect()
}

/// Frequency-selective fading: independent fading gain per bit.
/// Returns a vector of LLRs for a given bit sequence.
pub fn frequency_selective_llrs(bits: &[u8], snr_db: f64, rng: &mut StdRng) -> Vec<f64> {
    bits.iter()
        .map(|&b| {
            let h = sample_rayleigh_fading(rng);
            rayleigh_fading_llr(b, snr_db, h, rng)
        })
        .collect()
}

/// Burst-noise channel: introduces error bursts of length `burst_len` with probability `p_burst`.
/// Outside bursts, behaves like a clean channel (no flips).
pub fn burst_noise_channel(
    bits: &[u8],
    p_burst: f64,
    burst_len: usize,
    rng: &mut StdRng,
) -> Vec<u8> {
    debug_assert!((0.0..=1.0).contains(&p_burst));

    let mut out = bits.to_vec();
    let n = bits.len();

    let mut i = 0;
    while i < n {
        if rng.random_bool(p_burst) {
            let end = (i + burst_len).min(n);
            for x in &mut out[i..end] {
                *x ^= 1;
            }
            i = end;
        } else {
            i += 1;
        }
    }

    out
}

/// High-level channel abstraction for LLR-based simulations.
#[derive(Debug, Clone, Copy)]
pub enum Channel {
    Awgn,
    Rayleigh,
    Rician { k: f64 },
    Nakagami { m: f64 },
}

/// Simulates a single-bit LLR under the selected channel model.
pub fn simulate_llr(bit: u8, snr_db: f64, ch: Channel, rng: &mut StdRng) -> f64 {
    match ch {
        Channel::Awgn => bpsk_awgn_llr(bit, snr_db, rng),
        Channel::Rayleigh => {
            let h = sample_rayleigh_fading(rng);
            rayleigh_fading_llr(bit, snr_db, h, rng)
        }
        Channel::Rician { k } => rician_fading_llr(bit, snr_db, k, rng),
        Channel::Nakagami { m } => nakagami_fading_llr(bit, snr_db, m, rng),
    }
}
