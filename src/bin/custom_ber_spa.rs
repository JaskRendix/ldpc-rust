use clap::{Parser, ValueEnum};
use ldpc_rust::channel::{Channel, simulate_llr};
use ldpc_rust::define_custom_matrix;
use ldpc_rust::spa_decoder_llr::{Scheduling, SpaDecoderLLR};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{self, Write};
use std::thread;

// Define a robust custom (8, 16) regular LDPC parity-check matrix using the macro
define_custom_matrix!(
    pub struct CustomMatrix8x16,
    const M = 8,
    const N = 16,
    [
        [1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 1, 1, 1, 1, 0, 1, 0, 0, 0, 0, 0, 0],
        [0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0],
        [0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0],
        [0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0],
        [1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0],
        [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0],
        [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    ]
);

#[derive(ValueEnum, Clone, Copy, Debug)]
enum ChannelArg {
    Awgn,
    Rayleigh,
    Rician,
    Nakagami,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum SchedulingArg {
    Flooding,
    Layered,
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Custom Matrix LDPC SPA Bit Error Rate (BER) Simulation"
)]
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

    #[arg(long, default_value_t = 100_000, help = "Maximum trials per SNR point")]
    max_trials: usize,

    #[arg(
        long,
        default_value_t = 25,
        help = "Maximum decoder iterations per trial"
    )]
    iterations: usize,

    #[arg(long, value_enum, default_value_t = ChannelArg::Awgn, help = "Channel model to simulate")]
    channel: ChannelArg,

    #[arg(
        long,
        value_enum,
        default_value_t = SchedulingArg::Flooding,
        help = "Decoding scheduling strategy"
    )]
    scheduling: SchedulingArg,
}

fn main() {
    let args = Args::parse();
    let smoke = args.smoke;
    let base_seed = args.seed;
    let min_error_bits = args.min_error_bits;
    let max_trials_limit = args.max_trials;
    let max_iter = args.iterations;
    let channel_arg = args.channel;
    let scheduling_arg = args.scheduling;

    let channel_name = match channel_arg {
        ChannelArg::Awgn => "AWGN",
        ChannelArg::Rayleigh => "Rayleigh Fading",
        ChannelArg::Rician => "Rician Fading (K=3.0)",
        ChannelArg::Nakagami => "Nakagami-m Fading (m=1.0)",
    };

    let scheduling_name = match scheduling_arg {
        SchedulingArg::Flooding => "Flooding",
        SchedulingArg::Layered => "Layered",
    };

    let decoder_scheduling = match scheduling_arg {
        SchedulingArg::Flooding => Scheduling::Flooding,
        SchedulingArg::Layered => Scheduling::Layered,
    };

    let snr_points = if smoke {
        vec![0.0]
    } else {
        vec![-2.0, -1.0, 0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0]
    };

    eprintln!(
        "Custom Matrix Simulation [{channel_name} | {scheduling_name}]: code={}x{} seed={base_seed} min_errors={min_error_bits} max_trials={max_trials_limit} max_iter={max_iter} (multithreaded)",
        CustomMatrix8x16::ROWS,
        CustomMatrix8x16::COLS
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
                    let mut decoder: SpaDecoderLLR<
                        { CustomMatrix8x16::ROWS },
                        { CustomMatrix8x16::COLS },
                    > = SpaDecoderLLR::new(&CustomMatrix8x16::DATA);
                    decoder.set_max_iter(max_iter);
                    decoder.set_scheduling(decoder_scheduling);
                    let n = CustomMatrix8x16::COLS;

                    let channel = match channel_arg {
                        ChannelArg::Awgn => Channel::Awgn,
                        ChannelArg::Rayleigh => Channel::Rayleigh,
                        ChannelArg::Rician => Channel::Rician { k: 3.0 },
                        ChannelArg::Nakagami => Channel::Nakagami { m: 1.0 },
                    };

                    let mut total_bits = 0usize;
                    let mut error_bits = 0usize;
                    let mut trials = 0usize;
                    let max_trials = if smoke { 1 } else { max_trials_limit };

                    while trials < max_trials {
                        let cw = vec![0u8; n];
                        let mut llr = vec![0.0f64; n];
                        for i in 0..n {
                            llr[i] = simulate_llr(cw[i], snr_db, channel, &mut rng);
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
