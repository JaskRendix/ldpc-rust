use ldpc_rust::matrices::h_256_512::H_256_512;

use ldpc_rust::bitarray::BitArray;
use ldpc_rust::ldpc_decoder::LdpcDecoder;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use rand_distr::{Distribution, StandardNormal};

use std::time::Instant;

// TODO: this duplicates bpsk_awgn_llr from bin/ber_spa.rs. Worth pulling
// into a shared `channel` module in the library so both binaries stay in
// sync (the way spa_decoder.rs vs spa_decoder_llr.rs drifting out of sync
// with each other on the hard-decision sign convention already caused a
// real bug - duplicated logic across bins is the same risk).
fn bpsk_awgn_llr(bit: u8, snr_db: f64, rng: &mut StdRng) -> f64 {
    let snr_linear = 10.0_f64.powf(snr_db / 10.0);
    let sigma = (1.0 / (2.0 * snr_linear)).sqrt();

    let s = if bit == 0 { 1.0 } else { -1.0 };
    let n: f64 = StandardNormal.sample(rng);

    let y = s + sigma * n;
    2.0 * y / (sigma * sigma)
}

/// Fixed seed so benchmark runs are reproducible run-to-run and
/// comparable across code changes (e.g. confirming a decoder fix actually
/// changed throughput/convergence rather than the RNG landing differently).
const SEED: u64 = 0xC0FFEE;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let smoke = args.contains(&"--smoke".to_string());

    println!("LDPC Benchmark Harness");
    println!("----------------------------------------");

    benchmark_bitflip(smoke);
    benchmark_spa_llr(smoke);
}

fn benchmark_bitflip(smoke: bool) {
    println!("Bit-Flip Benchmark (256x512):");

    let decoder = LdpcDecoder::new(&H_256_512);
    let mut rng = StdRng::seed_from_u64(SEED);

    let iterations = 20;
    let trials = if smoke { 2 } else { 200 };
    // Number of bit errors injected per trial. Chosen to sit within the
    // decoder's expected correction capability so most trials converge,
    // but not so few that the decoder exits after a single pass - this is
    // meant to exercise real iterative work, not a best-case codeword.
    let error_bits = 3;

    let mut converged_count = 0usize;
    let start = Instant::now();

    for _ in 0..trials {
        let mut cw = [0u8; 64];

        // Inject error_bits distinct random bit errors using packed BitArray helper.
        let mut flipped = std::collections::HashSet::new();
        while flipped.len() < error_bits {
            let idx = rng.random_range(0..512);
            if flipped.insert(idx) {
                BitArray::xor_bit(&mut cw, idx);
            }
        }

        // Mirror production usage (server_router.rs / ber_spa.rs): stop as
        // soon as a valid codeword is reached, rather than always burning
        // the full iteration budget regardless of convergence.
        for _ in 0..iterations {
            if decoder.iterate_bitflip(&mut cw) {
                converged_count += 1;
                break;
            }
        }
    }

    let elapsed = start.elapsed();
    let per_trial = elapsed.as_secs_f64() / trials as f64;

    println!("  Trials: {trials}");
    println!("  Injected errors/trial: {error_bits}");
    println!("  Max iterations/trial: {iterations}");
    println!(
        "  Converged: {converged_count}/{trials} ({:.1}%)",
        100.0 * converged_count as f64 / trials as f64
    );
    println!("  Total time: {:.3} s", elapsed.as_secs_f64());
    println!("  Avg per trial: {:.6} s", per_trial);
    println!("  Throughput: {:.2} trials/s", 1.0 / per_trial);
    println!("----------------------------------------");
}

fn benchmark_spa_llr(smoke: bool) {
    println!("SPA LLR Benchmark (256x512):");

    let mut rng = StdRng::seed_from_u64(SEED);

    let n = 512;
    let iterations = 20;
    let trials = if smoke { 2 } else { 50 };
    // SNR chosen to sit near the waterfall region rather than deep in the
    // error-free zone, so the decoder actually does multi-iteration work
    // instead of converging on essentially the first pass.
    let snr_db = 0.5;

    // One decoder instance reused across all trials, matching how the
    // bit-flip benchmark above only constructs its decoder once. decode()
    // fully resets internal state from the input LLR each call, so this
    // is safe and keeps the comparison to bit-flip apples-to-apples
    // (measuring decode cost only, not allocation cost).
    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(iterations);

    let mut converged_count = 0usize;
    let start = Instant::now();

    for _ in 0..trials {
        let cw = vec![0u8; n];
        let mut llr = vec![0.0f64; n];
        for i in 0..n {
            llr[i] = bpsk_awgn_llr(cw[i], snr_db, &mut rng);
        }

        let hard = decoder.decode(&llr);
        if hard == cw {
            converged_count += 1;
        }
    }

    let elapsed = start.elapsed();
    let per_trial = elapsed.as_secs_f64() / trials as f64;

    println!("  Trials: {trials}");
    println!("  SNR: {snr_db} dB");
    println!("  Max iterations/trial: {iterations}");
    println!(
        "  Converged to correct codeword: {converged_count}/{trials} ({:.1}%)",
        100.0 * converged_count as f64 / trials as f64
    );
    println!("  Total time: {:.3} s", elapsed.as_secs_f64());
    println!("  Avg per trial: {:.6} s", per_trial);
    println!("  Throughput: {:.2} trials/s", 1.0 / per_trial);
    println!("----------------------------------------");
}
