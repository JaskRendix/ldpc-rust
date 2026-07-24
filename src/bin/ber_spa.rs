use ldpc_rust::matrices::h_256_512::H_256_512;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;

use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Distribution, StandardNormal};

// TODO: confirm whether `snr_db` here is meant as Eb/N0 or Es/N0. sigma is
// currently derived directly from snr_db as if it's the coded/symbol SNR
// (Es/N0). For a rate-1/2 code (k/n = 256/512), Es/N0 = 0.5 * Eb/N0 - if
// this is meant to be plotted against Eb/N0 (the usual waterfall-curve
// convention), that scaling needs to be applied before computing sigma,
// and the CSV header/README should say explicitly which one it is.
fn bpsk_awgn_llr(bit: u8, snr_db: f64, rng: &mut StdRng) -> f64 {
    let snr_linear = 10.0_f64.powf(snr_db / 10.0);
    let sigma = (1.0 / (2.0 * snr_linear)).sqrt();

    let s = if bit == 0 { 1.0 } else { -1.0 };
    let n: f64 = StandardNormal.sample(rng);

    let y = s + sigma * n;
    2.0 * y / (sigma * sigma)
}

/// Stop simulating a given SNR point once this many bit errors have been
/// observed. Fixed-trial-count sampling produces unreliable (often
/// exactly-zero) BER estimates at high SNR, where true error rates can be
/// far rarer than a small fixed number of trials could ever detect.
/// Error-count stopping spends more trials exactly where errors are rare.
const MIN_ERROR_BITS: usize = 50;

/// Hard ceiling so a very clean SNR point (or a decoder that never
/// errors) can't run forever chasing MIN_ERROR_BITS.
const MAX_TRIALS: usize = 200_000;

fn parse_seed(args: &[String]) -> u64 {
    args.iter()
        .position(|a| a == "--seed")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0xC0FFEE)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let smoke = args.contains(&"--smoke".to_string());
    let seed = parse_seed(&args);

    let snr_points = if smoke {
        vec![0.0]
    } else {
        vec![-2.0, -1.0, 0.0, 0.5, 1.0, 1.5, 2.0]
    };

    let n = 512;

    // Seeded so runs are reproducible - important for confirming a code
    // change (e.g. a decoder bugfix) actually moved the curve, rather than
    // the RNG just landing differently.
    let mut rng = StdRng::seed_from_u64(seed);

    eprintln!("seed={seed} min_error_bits={MIN_ERROR_BITS} max_trials={MAX_TRIALS}");
    println!("snr_db,ber,trials,error_bits,total_bits");

    for &snr_db in &snr_points {
        let mut total_bits = 0usize;
        let mut error_bits = 0usize;
        let mut trials = 0usize;

        // One decoder per SNR point, not per trial: decode() fully resets
        // its internal state (qnm) from the input LLR before use each
        // call, so reuse across trials is safe and avoids re-cloning
        // h_matrix and reallocating qnm/rmn on every single trial.
        let mut decoder = SpaDecoderLLR::new(&H_256_512);

        let max_trials = if smoke { 1 } else { MAX_TRIALS };

        while trials < max_trials {
            // All-zero codeword: valid for BER estimation here because
            // linear codes decoded with a symmetric decoder (which this
            // min-sum/SPA decoder is) have error behavior independent of
            // which codeword was actually sent - only the noise pattern
            // matters. Simulating any single fixed codeword is equivalent
            // to averaging over all of them.
            let cw = vec![0u8; n];

            let mut llr = vec![0.0f64; n];
            for i in 0..n {
                llr[i] = bpsk_awgn_llr(cw[i], snr_db, &mut rng);
            }

            let hard = decoder.decode(&llr);

            for i in 0..n {
                total_bits += 1;
                if hard[i] != cw[i] {
                    error_bits += 1;
                }
            }
            trials += 1;

            if !smoke && error_bits >= MIN_ERROR_BITS {
                break;
            }
        }

        let ber = error_bits as f64 / total_bits as f64;
        eprintln!(
            "snr={snr_db}dB trials={trials} error_bits={error_bits} total_bits={total_bits} ber={ber}"
        );
        println!("{snr_db},{ber},{trials},{error_bits},{total_bits}");
    }
}
