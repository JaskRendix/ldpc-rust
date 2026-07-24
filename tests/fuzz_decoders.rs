use ldpc_rust::encoder::LDPC_ENCODER;
use ldpc_rust::matrices::h_256_512::H_256_512;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;
use rand::rngs::ThreadRng;
use rand::RngCore;

#[test]
fn test_fuzz_encode_decode_pipeline() {
    let mut rng = ThreadRng::default();
    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(30);

    for trial in 0..100 {
        let mut message = [0u8; 256];
        for b in message.iter_mut() {
            *b = (rng.next_u32() & 1) as u8;
        }

        let codeword = LDPC_ENCODER.encode(&message);

        let mut llrs = vec![0.0f64; 512];
        for (i, llr) in llrs.iter_mut().enumerate() {
            let mut bit = codeword[i];

            if trial % 10 == 0 && i < 10 {
                bit ^= 1;
            }

            *llr = if bit == 0 { 4.0 } else { -4.0 };
        }

        let decoded = decoder.decode(&llrs);

        if trial % 10 == 0 {
            assert_eq!(&decoded[..256], &message);
        }
    }
}
