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
    fn prop_single_bit_error_not_worse(error_idx in 0..512usize) {
        let decoder = LdpcDecoder::<256, 512>::new(&H_256_512);

        let mut cw = [0u8; 64];

        let mut syndrome_before = [0u8; 256];
        let mut syndrome_after = [0u8; 256];

        let byte_idx = error_idx / 8;
        let bit_idx = error_idx % 8;

        cw[byte_idx] ^= 1 << bit_idx;

        decoder.get_parity(&cw, &mut syndrome_before);
        let before = syndrome_before.iter().filter(|&&x| x == 1).count();

        let mut working = cw;
        decoder.iterate_bitflip(&mut working);

        decoder.get_parity(&working, &mut syndrome_after);
        let after = syndrome_after.iter().filter(|&&x| x == 1).count();

        prop_assert!(after <= before);
    }
}
