use clap::Parser;
use ldpc_rust::channel::bpsk_awgn_llr;
use ldpc_rust::matrices::h_256_512::H_256_512;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;

use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{self, Write};
use std::thread;

#[derive(Parser, Debug)]
#[command(author, version, about = "LDPC SPA Bit Error Rate (BER) Simulation")]
struct Args {
    #[arg(
        long,
        default_value_t = false,
        help = "Run a quick smoke test with reduced trials"
    )]
    smoke: bool,

    #[arg(
        long,
        default_value_t = 0xC0FFEE,
        help = "Base RNG seed for simulation"
    )]
    seed: u64,

    #[arg(
        long,
        default_value_t = 50,
        help = "Minimum error bits before convergence breakout"
    )]
    min_error_bits: usize,

    #[arg(long, default_value_t = 200_000, help = "Maximum trials per SNR point")]
    max_trials: usize,
}

fn main() {
    let args = Args::parse();
    let smoke = args.smoke;
    let base_seed = args.seed;
    let min_error_bits = args.min_error_bits;
    let max_trials_limit = args.max_trials;

    let snr_points = if smoke {
        vec![0.0]
    } else {
        vec![-2.0, -1.0, 0.0, 0.5, 1.0, 1.5, 2.0]
    };

    eprintln!(
        "seed={base_seed} min_error_bits={min_error_bits} max_trials={max_trials_limit} (multithreaded)"
    );
    println!("snr_db,ber,trials,error_bits,total_bits");

    let results: Vec<(f64, f64, usize, usize, usize)> = thread::scope(|s| {
        let handles: Vec<_> = snr_points
            .iter()
            .enumerate()
            .map(|(idx, &snr_db)| {
                let thread_seed = base_seed.wrapping_add(idx as u64 * 0x9E3779B97F4A7C15);

                s.spawn(move || {
                    let mut rng = StdRng::seed_from_u64(thread_seed);
                    let mut decoder: SpaDecoderLLR<256, 512> = SpaDecoderLLR::new(&H_256_512);
                    let n = 512;

                    let mut total_bits = 0usize;
                    let mut error_bits = 0usize;
                    let mut trials = 0usize;
                    let max_trials = if smoke { 1 } else { max_trials_limit };

                    while trials < max_trials {
                        let cw = vec![0u8; n];
                        let mut llr = vec![0.0f64; n];
                        for i in 0..n {
                            llr[i] = bpsk_awgn_llr(cw[i], snr_db, &mut rng);
                        }

                        let hard = match decoder.decode(&llr) {
                            Ok(res) => res.codeword,
                            Err(_) => vec![1u8; n],
                        };

                        for i in 0..n {
                            total_bits += 1;
                            if hard[i] != cw[i] {
                                error_bits += 1;
                            }
                        }
                        trials += 1;

                        if trials.is_multiple_of(100) && !smoke {
                            let current_ber = error_bits as f64 / total_bits as f64;
                            eprint!("\r[SNR {snr_db:+.1}dB] Trials: {trials}/{max_trials} | Errors: {error_bits}/{min_error_bits} | BER: {current_ber:.5}   ");
                            let _ = io::stderr().flush();
                        }

                        if !smoke && error_bits >= min_error_bits {
                            break;
                        }
                    }

                    eprintln!("\r[SNR {snr_db:+.1}dB] Completed: trials={trials} errors={error_bits}                  ");

                    let ber = error_bits as f64 / total_bits as f64;
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
