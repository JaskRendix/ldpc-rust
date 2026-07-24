use ldpc_rust::channel::bpsk_awgn_llr;
use ldpc_rust::matrices::h_256_512::H_256_512;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;

use rand::rngs::StdRng;
use rand::SeedableRng;

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
    let seed = parse_seed(&args);

    let snr_points = if smoke {
        vec![0.0]
    } else {
        vec![-2.0, -1.0, 0.0, 0.5, 1.0, 1.5, 2.0]
    };

    let n = 512;
    let mut rng = StdRng::seed_from_u64(seed);

    eprintln!("seed={seed} min_error_bits={MIN_ERROR_BITS} max_trials={MAX_TRIALS}");
    println!("snr_db,ber,trials,error_bits,total_bits");

    for &snr_db in &snr_points {
        let mut total_bits = 0usize;
        let mut error_bits = 0usize;
        let mut trials = 0usize;

        let mut decoder = SpaDecoderLLR::new(&H_256_512);
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
        println!("{snr_db},{ber},{trials},{error_bits},{total_bits}");
    }
}
