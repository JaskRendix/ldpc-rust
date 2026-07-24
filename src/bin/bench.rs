use ldpc_rust::channel::bpsk_awgn_llr;
use ldpc_rust::matrices::h_256_512::H_256_512;

use ldpc_rust::bitarray::BitArray;
use ldpc_rust::ldpc_decoder::LdpcDecoder;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

use std::time::Instant;

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
    let error_bits = 3;

    let mut converged_count = 0usize;
    let start = Instant::now();

    for _ in 0..trials {
        let mut cw = [0u8; 64];

        let mut flipped = std::collections::HashSet::new();
        while flipped.len() < error_bits {
            let idx = rng.random_range(0..512);
            if flipped.insert(idx) {
                BitArray::xor_bit(&mut cw, idx);
            }
        }

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
    let snr_db = 0.5;

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
