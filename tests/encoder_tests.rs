use ldpc_rust::encoder::LDPC_ENCODER;
use ldpc_rust::ldpc_decoder::LdpcDecoder;
use ldpc_rust::matrices::h_256_512::H_256_512;
use rand::Rng;
use rand::rngs::ThreadRng;

/// Helper: compute syndrome using existing decoder parity function.
fn syndrome_is_zero(codeword: &[u8; 512]) -> bool {
    let decoder: LdpcDecoder<256, 512> = LdpcDecoder::new(&H_256_512);

    let mut cw_bytes = [0u8; 64];
    for (i, &bit) in codeword.iter().enumerate() {
        if bit == 1 {
            let byte = i / 8;
            let bit_pos = i % 8;
            cw_bytes[byte] |= 1 << bit_pos;
        }
    }

    let mut sn = [0u8; 256];
    decoder.get_parity(&cw_bytes, &mut sn)
}

#[test]
fn test_systematic_property() {
    let mut msg = [0u8; 256];
    for (i, m) in msg.iter_mut().enumerate() {
        *m = (i % 2) as u8;
    }

    let cw = LDPC_ENCODER.encode(&msg);
    assert_eq!(&cw[..256], &msg);
}

#[test]
fn test_all_zero_message_encodes_valid_codeword() {
    let msg = [0u8; 256];
    let cw = LDPC_ENCODER.encode(&msg);

    assert!(syndrome_is_zero(&cw));
    assert!(cw[256..].iter().all(|&b| b == 0));
}

#[test]
fn test_single_bit_message_encodes_valid_codeword() {
    for pos in [0, 1, 42, 127, 200, 255] {
        let mut msg = [0u8; 256];
        msg[pos] = 1;

        let cw = LDPC_ENCODER.encode(&msg);
        assert!(syndrome_is_zero(&cw));
    }
}

#[test]
fn test_random_messages_produce_valid_codewords() {
    let mut rng = ThreadRng::default();

    for _ in 0..50 {
        let mut msg = [0u8; 256];
        for m in msg.iter_mut() {
            *m = (rng.next_u32() & 1) as u8;
        }

        let cw = LDPC_ENCODER.encode(&msg);
        assert!(syndrome_is_zero(&cw));
    }
}

#[test]
fn test_parity_changes_when_message_bit_flips() {
    let mut msg = [0u8; 256];
    msg[10] = 1;

    let cw1 = LDPC_ENCODER.encode(&msg);

    msg[10] = 0;
    let cw2 = LDPC_ENCODER.encode(&msg);

    assert_ne!(&cw1[256..], &cw2[256..]);
}

#[test]
fn test_dense_and_sparse_messages() {
    let mut sparse = [0u8; 256];
    sparse[42] = 1;
    assert!(syndrome_is_zero(&LDPC_ENCODER.encode(&sparse)));

    let dense = [1u8; 256];
    assert!(syndrome_is_zero(&LDPC_ENCODER.encode(&dense)));
}

#[test]
fn test_encoder_lazy_initialization() {
    let msg = [0u8; 256];
    let cw1 = LDPC_ENCODER.encode(&msg);
    let cw2 = LDPC_ENCODER.encode(&msg);

    assert_eq!(cw1, cw2);
}

#[test]
fn test_encode_decode_roundtrip_spa() {
    use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;

    let mut rng = ThreadRng::default();
    let mut msg = [0u8; 256];
    for m in msg.iter_mut() {
        *m = (rng.next_u32() & 1) as u8;
    }

    let cw = LDPC_ENCODER.encode(&msg);

    let mut llrs = vec![0.0f64; 512];
    for (i, &bit) in cw.iter().enumerate() {
        llrs[i] = if bit == 0 { 5.0 } else { -5.0 };
    }

    let mut decoder: SpaDecoderLLR<256, 512> = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(30);

    let decoded = decoder.decode(&llrs);
    assert_eq!(decoded[..256], msg);
}

#[test]
fn test_hard_decision_decoders_roundtrip() {
    use ldpc_rust::bitarray::BitArray;

    let mut msg = [0u8; 256];
    msg[5] = 1;
    msg[100] = 1;
    let cw = LDPC_ENCODER.encode(&msg);

    // Pack into 64 bytes for hard-decision decoders
    let mut cw_bytes = [0u8; 64];
    for (i, &bit) in cw.iter().enumerate() {
        if bit == 1 {
            BitArray::xor_bit(&mut cw_bytes, i);
        }
    }

    let decoder: LdpcDecoder<256, 512> = LdpcDecoder::new(&H_256_512);
    assert!(decoder.iterate_wbf(&mut cw_bytes));
}

#[test]
fn test_spa_syndrome_validation() {
    use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;

    let decoder: SpaDecoderLLR<256, 512> = SpaDecoderLLR::new(&H_256_512);
    let valid_cw = vec![0u8; 512];
    assert!(decoder.check_syndrome_public(&valid_cw));

    let mut invalid_cw = vec![0u8; 512];
    invalid_cw[0] = 1;
    assert!(!decoder.check_syndrome_public(&invalid_cw));
}
