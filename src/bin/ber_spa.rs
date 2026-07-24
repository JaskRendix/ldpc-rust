use ldpc_rust::channel::bpsk_awgn_llr;
use ldpc_rust::matrices::h_256_512::H_256_512;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;

use rand::rngs::StdRng;
use rand::SeedableRng;
use std::thread;

const MIN_ERROR_BITS: usize = 50;
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
    let base_seed = parse_seed(&args);

    let snr_points = if smoke {
        vec![0.0]
    } else {
        vec![-2.0, -1.0, 0.0, 0.5, 1.0, 1.5, 2.0]
    };

    eprintln!("seed={base_seed} min_error_bits={MIN_ERROR_BITS} max_trials={MAX_TRIALS} (multithreaded)");
    println!("snr_db,ber,trials,error_bits,total_bits");

    // Distribute SNR points concurrently across threads using std::thread::scope
    let results: Vec<(f64, f64, usize, usize, usize)> = thread::scope(|s| {
        let handles: Vec<_> = snr_points
            .iter()
            .enumerate()
            .map(|(idx, &snr_db)| {
                // Derive a unique, deterministic seed per SNR point
                let thread_seed = base_seed.wrapping_add(idx as u64 * 0x9E3779B97F4A7C15);

                s.spawn(move || {
                    let mut rng = StdRng::seed_from_u64(thread_seed);
                    let mut decoder = SpaDecoderLLR::new(&H_256_512);
                    let n = 512;

                    let mut total_bits = 0usize;
                    let mut error_bits = 0usize;
                    let mut trials = 0usize;
                    let max_trials = if smoke { 1 } else { MAX_TRIALS };

                    while trials < max_trials {
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
                    (snr_db, ber, trials, error_bits, total_bits)
                })
            })
            .collect();

        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    for (snr_db, ber, trials, error_bits, total_bits) in results {
        println!("{snr_db},{ber},{trials},{error_bits},{total_bits}");
    }
}
