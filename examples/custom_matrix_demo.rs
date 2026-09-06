use ldpc_rust::channel::{Channel, simulate_llr};
use ldpc_rust::define_custom_matrix;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;
use rand::SeedableRng;
use rand::rngs::StdRng;

// Define a custom (4, 8) parity-check matrix using the macro
define_custom_matrix!(
    pub struct CustomMatrix4x8,
    const M = 4,
    const N = 8,
    [
        [1, 1, 0, 0, 1, 0, 0, 0],
        [0, 1, 1, 0, 0, 1, 0, 0],
        [0, 0, 1, 1, 0, 0, 1, 0],
        [1, 0, 0, 1, 0, 0, 0, 1],
    ]
);

fn main() {
    println!(
        "Running end-to-end simulation with custom (4, 8) matrix over Rician fading channel..."
    );

    let n = CustomMatrix4x8::COLS;
    let mut rng = StdRng::seed_from_u64(42);

    // Initialize the SPA decoder with the custom matrix data
    let mut decoder: SpaDecoderLLR<{ CustomMatrix4x8::ROWS }, { CustomMatrix4x8::COLS }> =
        SpaDecoderLLR::new(&CustomMatrix4x8::DATA);
    decoder.set_max_iter(20);

    // Assume an all-zero codeword for testing transmission
    let sent_codeword = vec![0u8; n];

    // Select a Rician fading channel model (K-factor = 3.0) via the high-level abstraction
    let channel = Channel::Rician { k: 3.0 };
    let snr_db = 3.5;

    let mut llr = vec![0.0f64; n];
    for i in 0..n {
        llr[i] = simulate_llr(sent_codeword[i], snr_db, channel.clone(), &mut rng);
    }

    // Attempt to decode the noisy LLR measurements
    match decoder.decode(&llr) {
        Ok(result) => {
            println!("Decoded successfully in {} iterations.", result.iterations);
            println!("Converged: {}", result.converged);
            println!(
                "Recovered codeword matches sent: {}",
                result.codeword == sent_codeword
            );
        }
        Err(e) => {
            println!("Decoding failed: {e:?}");
        }
    }
}
