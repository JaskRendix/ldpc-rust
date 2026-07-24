use ldpc_rust::encoder::LDPC_ENCODER;
use ldpc_rust::ldpc_decoder::LdpcDecoder;
use ldpc_rust::matrices::h_256_512::H_256_512;

/// Helper: compute syndrome using existing decoder parity function.
fn syndrome_is_zero(codeword: &[u8; 512]) -> bool {
    let decoder = LdpcDecoder::new(&H_256_512);

    // decoder.get_parity() expects packed 64-byte cw, but we have 512 bits.
    // Repack 512 bits into 64 bytes.
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

    assert_eq!(&cw[..256], &msg, "Systematic part must match message");
}

#[test]
fn test_all_zero_message_encodes_valid_codeword() {
    let msg = [0u8; 256];
    let cw = LDPC_ENCODER.encode(&msg);

    assert!(
        syndrome_is_zero(&cw),
        "All-zero message must produce valid codeword"
    );

    assert!(cw[256..].iter().all(|&b| b == 0));
}

#[test]
fn test_single_bit_message_encodes_valid_codeword() {
    for pos in [0, 1, 42, 127, 200, 255] {
        let mut msg = [0u8; 256];
        msg[pos] = 1;

        let cw = LDPC_ENCODER.encode(&msg);

        assert!(
            syndrome_is_zero(&cw),
            "Single-bit message at position {pos} must produce valid codeword"
        );
    }
}

#[test]
fn test_random_messages_produce_valid_codewords() {
    use rand::Rng;

    let mut rng = rand::thread_rng();
    for _ in 0..50 {
        let mut msg = [0u8; 256];
        for m in msg.iter_mut() {
            *m = rng.gen::<u8>() & 1;
        }

        let cw = LDPC_ENCODER.encode(&msg);

        assert!(
            syndrome_is_zero(&cw),
            "Random message must produce valid codeword"
        );
    }
}

#[test]
fn test_parity_changes_when_message_bit_flips() {
    let mut msg = [0u8; 256];
    msg[10] = 1;

    let cw1 = LDPC_ENCODER.encode(&msg);

    msg[10] = 0;
    let cw2 = LDPC_ENCODER.encode(&msg);

    assert_ne!(
        &cw1[256..],
        &cw2[256..],
        "Parity must change when a message bit flips"
    );
}

#[test]
fn test_dense_and_sparse_messages() {
    // Sparse: only one bit set
    let mut sparse = [0u8; 256];
    sparse[42] = 1;
    let cw_sparse = LDPC_ENCODER.encode(&sparse);
    assert!(syndrome_is_zero(&cw_sparse));

    // Dense: all ones
    let dense = [1u8; 256];
    let cw_dense = LDPC_ENCODER.encode(&dense);
    assert!(syndrome_is_zero(&cw_dense));
}

#[test]
fn test_encoder_lazy_initialization() {
    let msg = [0u8; 256];
    let cw1 = LDPC_ENCODER.encode(&msg);
    let cw2 = LDPC_ENCODER.encode(&msg);

    assert_eq!(cw1, cw2, "LazyLock must produce consistent results");
}

#[test]
fn test_encode_decode_roundtrip_spa() {
    use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;
    use rand::Rng;

    let mut rng = rand::thread_rng();
    let mut msg = [0u8; 256];
    for m in msg.iter_mut() {
        *m = rng.gen::<u8>() & 1;
    }

    let cw = LDPC_ENCODER.encode(&msg);

    // Convert codeword bits to strong LLRs
    let mut llrs = vec![0.0f64; 512];
    for (i, &bit) in cw.iter().enumerate() {
        llrs[i] = if bit == 0 { 5.0 } else { -5.0 };
    }

    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(30);

    let decoded = decoder.decode(&llrs);

    assert_eq!(
        decoded[..256],
        msg,
        "SPA decoder must recover original message from encoded codeword"
    );
}
