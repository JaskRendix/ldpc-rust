use rand::rngs::StdRng;
use rand_distr::{Distribution, StandardNormal};

/// Computes the BPSK AWGN Log-Likelihood Ratio (LLR) for a given transmitted bit and SNR.
/// 
/// Note: `snr_db` here is treated as Es/N0 (symbol SNR). For a rate-1/2 code, 
/// Es/N0 = 0.5 * Eb/N0.
pub fn bpsk_awgn_llr(bit: u8, snr_db: f64, rng: &mut StdRng) -> f64 {
    let snr_linear = 10.0_f64.powf(snr_db / 10.0);
    let sigma = (1.0 / (2.0 * snr_linear)).sqrt();

    let s = if bit == 0 { 1.0 } else { -1.0 };
    let n: f64 = StandardNormal.sample(rng);

    let y = s + sigma * n;
    2.0 * y / (sigma * sigma)
}
