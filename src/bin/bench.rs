use clap::{Parser, ValueEnum};
use ldpc_rust::bitarray::BitArray;
use ldpc_rust::channel::{Channel, simulate_llr};
use ldpc_rust::ldpc_decoder::LdpcDecoder;
use ldpc_rust::matrices::h_256_512::H_256_512;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::time::Instant;

const SEED: u64 = 0xC0FFEE;

#[derive(ValueEnum, Clone, Copy, Debug)]
enum ChannelArg {
    Awgn,
    Rayleigh,
    Rician,
    Nakagami,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "LDPC Benchmark Harness")]
struct Args {
    #[arg(
        long,
        default_value_t = false,
        help = "Run a quick smoke test with reduced trials"
    )]
    smoke: bool,

    #[arg(
        long,
        default_value_t = 0.5,
        help = "Target SNR in dB for SPA LLR benchmark"
    )]
    snr: f64,

    #[arg(long, default_value_t = 20, help = "Max iterations per trial")]
    iterations: usize,

    #[arg(long, help = "Override trial count for bit-flip and SPA benchmarks")]
    trials: Option<usize>,

    #[arg(long, value_enum, default_value_t = ChannelArg::Awgn, help = "Channel model to simulate")]
    channel: ChannelArg,
}

fn main() {
    let args = Args::parse();

    println!("LDPC Benchmark Harness");
    println!("----------------------------------------");

    benchmark_bitflip(args.smoke, args.iterations, args.trials);
    benchmark_spa_llr(
        args.smoke,
        args.snr,
        args.iterations,
        args.trials,
        args.channel,
    );
}

fn benchmark_bitflip(smoke: bool, iterations: usize, custom_trials: Option<usize>) {
    println!("Bit-Flip Benchmark (256x512):");

    let decoder: LdpcDecoder<256, 512> = LdpcDecoder::new(&H_256_512);
    let mut rng = StdRng::seed_from_u64(SEED);

    let trials = custom_trials.unwrap_or(if smoke { 2 } else { 200 });
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

fn benchmark_spa_llr(
    smoke: bool,
    snr_db: f64,
    iterations: usize,
    custom_trials: Option<usize>,
    channel_arg: ChannelArg,
) {
    let channel_name = match channel_arg {
        ChannelArg::Awgn => "AWGN",
        ChannelArg::Rayleigh => "Rayleigh Fading",
        ChannelArg::Rician => "Rician Fading (K=3.0)",
        ChannelArg::Nakagami => "Nakagami-m Fading (m=1.0)",
    };

    println!("SPA LLR Benchmark (256x512) over {channel_name}:");

    let mut rng = StdRng::seed_from_u64(SEED);
    let n = 512;
    let trials = custom_trials.unwrap_or(if smoke { 2 } else { 50 });

    let mut decoder: SpaDecoderLLR<256, 512> = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(iterations);

    let channel = match channel_arg {
        ChannelArg::Awgn => Channel::Awgn,
        ChannelArg::Rayleigh => Channel::Rayleigh,
        ChannelArg::Rician => Channel::Rician { k: 3.0 },
        ChannelArg::Nakagami => Channel::Nakagami { m: 1.0 },
    };

    let mut converged_count = 0usize;
    let start = Instant::now();

    for _ in 0..trials {
        let cw = vec![0u8; n];
        let mut llr = vec![0.0f64; n];
        for i in 0..n {
            llr[i] = simulate_llr(cw[i], snr_db, channel, &mut rng);
        }

        match decoder.decode(&llr) {
            Ok(res) if res.converged && res.codeword == cw => {
                converged_count += 1;
            }
            _ => {}
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
