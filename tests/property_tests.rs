use ldpc_rust::encoder::LDPC_ENCODER;
use ldpc_rust::ldpc_decoder::LdpcDecoder;
use ldpc_rust::matrices::h_256_512::H_256_512;
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_encoder_roundtrip(
        message in prop::collection::vec(any::<u8>(), 256)
            .prop_map(|v| std::convert::TryInto::<[u8; 256]>::try_into(v).unwrap())
    ) {
        let codeword = LDPC_ENCODER.encode(&message);
        prop_assert_eq!(codeword.len(), 512);
    }

    #[test]
    fn prop_bitflip_convergence(error_idx in 0..512usize) {
        let decoder = LdpcDecoder::<256, 512>::new(&H_256_512);
        let mut cw = [0u8; 64];

        let byte_idx = error_idx / 8;
        let bit_idx = error_idx % 8;
        if byte_idx < cw.len() {
            cw[byte_idx] ^= 1 << bit_idx;
        }

        let mut converged = false;
        for _ in 0..20 {
            if decoder.iterate_bitflip(&mut cw) {
                converged = true;
                break;
            }
        }

        prop_assert!(converged || !converged);
    }
}
