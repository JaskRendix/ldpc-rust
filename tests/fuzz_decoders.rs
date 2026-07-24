use ldpc_rust::encoder::LDPC_ENCODER;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;
use ldpc_rust::matrices::h_256_512::H_256_512;
use rand::Rng;

#[test]
fn test_fuzz_encode_decode_pipeline() {
    let mut rng = rand::thread_rng();
    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(30);

    // Run 100 random trials with simulated channel noise
    for trial in 0..100 {
        let mut message = [0u8; 256];
        for b in message.iter_mut() {
            *b = rng.gen::<u8>() & 1;
        }

        // 1. Encode
        let codeword = LDPC_ENCODER.encode(&message);

        // 2. Transmit through channel with minor bit flips (simulate error within correction radius)
        let mut llrs = vec![0.0f64; 512];
        for i in 0..512 {
            let mut bit = codeword[i];
            // Inject random single-bit error with low probability per trial
            if trial % 10 == 0 && i < 10 {
                bit ^= 1;
            }
            llrs[i] = if bit == 0 { 4.0 } else { -4.0 };
        }

        // 3. Decode
        let decoded = decoder.decode(&llrs);

        // 4. Verify roundtrip success for low noise conditions
        if trial % 10 == 0 {
            assert_eq!(
                &decoded[..256],
                &message,
                "Decoder failed to converge on trial {trial}"
            );
        }
    }
}
