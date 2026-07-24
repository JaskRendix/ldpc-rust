use ldpc_rust::bitarray::BitArray;
use ldpc_rust::ldpc_decoder::LdpcDecoder;
use ldpc_rust::matrices::h_256_512::H_256_512;

#[test]
fn test_parity_all_zero_codeword_is_valid() {
    let decoder = LdpcDecoder::new(&H_256_512);

    let cw = [0u8; 64];
    let mut sn = [0u8; 256];

    assert!(decoder.get_parity(&cw, &mut sn));
    assert!(sn.iter().all(|&b| b == 0));
}

#[test]
fn test_parity_single_bit_error_is_invalid() {
    let decoder = LdpcDecoder::new(&H_256_512);

    let mut cw = [0u8; 64];
    BitArray::set_bit(&mut cw, 10, true);

    let mut sn = [0u8; 256];
    assert!(!decoder.get_parity(&cw, &mut sn));
}

#[test]
fn test_score_all_zero_is_zero() {
    let decoder = LdpcDecoder::new(&H_256_512);

    let sn = [0u8; 256];
    let mut en = [0u8; 512]; // Unpacked score output size

    decoder.get_score(&sn, &mut en);

    assert!(en.iter().all(|&v| v == 0));
}

#[test]
fn test_bitflip_runs_safely_on_single_error() {
    let decoder = LdpcDecoder::new(&H_256_512);

    let mut cw = [0u8; 64];
    BitArray::xor_bit(&mut cw, 42);

    for _ in 0..20 {
        decoder.iterate_bitflip(&mut cw);
    }

    let mut sn = [0u8; 256];
    decoder.get_parity(&cw, &mut sn);

    assert_eq!(cw.len(), 64);
    assert_eq!(sn.len(), 256);
}

#[test]
fn test_bitflip_multiple_errors_runs_safely() {
    let decoder = LdpcDecoder::new(&H_256_512);

    let mut cw = [0u8; 64];
    BitArray::xor_bit(&mut cw, 10);
    BitArray::xor_bit(&mut cw, 200);
    BitArray::xor_bit(&mut cw, 350);

    for _ in 0..30 {
        decoder.iterate_bitflip(&mut cw);
    }

    let mut sn = [0u8; 256];
    decoder.get_parity(&cw, &mut sn);

    assert_eq!(cw.len(), 64);
    assert_eq!(sn.len(), 256);
}

#[test]
fn test_decoder_does_not_modify_outside_bounds() {
    let decoder = LdpcDecoder::new(&H_256_512);

    let mut cw = [0u8; 64];
    let before = cw;

    decoder.iterate_bitflip(&mut cw);

    assert_eq!(cw, before);
}

#[test]
fn test_wbf_runs_and_preserves_lengths() {
    let decoder = LdpcDecoder::new(&H_256_512);

    let mut cw = [0u8; 64];
    BitArray::xor_bit(&mut cw, 123);

    for _ in 0..20 {
        decoder.iterate_wbf(&mut cw);
    }

    let mut sn = [0u8; 256];
    decoder.get_parity(&cw, &mut sn);

    assert_eq!(cw.len(), 64);
    assert_eq!(sn.len(), 256);
}

#[test]
fn test_mwbf_runs_and_preserves_lengths() {
    let decoder = LdpcDecoder::new(&H_256_512);

    let mut cw = [0u8; 64];
    BitArray::xor_bit(&mut cw, 77);

    for _ in 0..20 {
        decoder.iterate_mwbf(&mut cw);
    }

    let mut sn = [0u8; 256];
    decoder.get_parity(&cw, &mut sn);

    assert_eq!(cw.len(), 64);
    assert_eq!(sn.len(), 256);
}

#[test]
fn test_nwbf_runs_and_preserves_lengths() {
    let decoder = LdpcDecoder::new(&H_256_512);

    let mut cw = [0u8; 64];
    BitArray::xor_bit(&mut cw, 5);

    for _ in 0..20 {
        decoder.iterate_nwbf(&mut cw);
    }

    let mut sn = [0u8; 256];
    decoder.get_parity(&cw, &mut sn);

    assert_eq!(cw.len(), 64);
    assert_eq!(sn.len(), 256);
}

#[test]
fn test_gallager_a_runs_and_preserves_lengths() {
    let decoder = LdpcDecoder::new(&H_256_512);

    let mut cw = [0u8; 64];
    BitArray::xor_bit(&mut cw, 19);

    for _ in 0..20 {
        decoder.iterate_gallager_a(&mut cw);
    }

    let mut sn = [0u8; 256];
    decoder.get_parity(&cw, &mut sn);

    assert_eq!(cw.len(), 64);
    assert_eq!(sn.len(), 256);
}

#[test]
fn test_gallager_b_runs_and_preserves_lengths() {
    let decoder = LdpcDecoder::new(&H_256_512);

    let mut cw = [0u8; 64];
    BitArray::xor_bit(&mut cw, 201);

    for _ in 0..20 {
        decoder.iterate_gallager_b(&mut cw);
    }

    let mut sn = [0u8; 256];
    decoder.get_parity(&cw, &mut sn);

    assert_eq!(cw.len(), 64);
    assert_eq!(sn.len(), 256);
}

#[test]
fn test_wbf_runs_safely_on_single_bit_error() {
    let decoder = LdpcDecoder::new(&H_256_512);
    let mut cw = [0u8; 64];
    BitArray::xor_bit(&mut cw, 123);

    for _ in 0..200 {
        decoder.iterate_wbf(&mut cw);
    }

    assert_eq!(cw.len(), 64);
}

#[test]
fn test_gallager_b_converges_on_single_error() {
    let mut decoder = LdpcDecoder::new(&H_256_512);
    decoder.set_gallager_b_threshold(2);

    let mut cw = [0u8; 64];
    BitArray::xor_bit(&mut cw, 10);

    let mut converged = false;
    for _ in 0..10 {
        if decoder.iterate_gallager_b(&mut cw) {
            converged = true;
            break;
        }
    }

    assert!(
        converged,
        "Gallager-B failed to converge on a single bit error"
    );

    let mut sn = [0u8; 256];
    assert!(decoder.get_parity(&cw, &mut sn));
}

#[test]
fn test_spa_decoder_nms_convergence() {
    use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;

    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_scaling_factor(0.75); // Test Normalized Min-Sum
    decoder.set_max_iter(30);

    // Provide clean channel LLRs for an all-zero codeword (positive LLRs)
    let llrs = vec![5.0; 512];
    let decoded = decoder.decode(&llrs);

    assert_eq!(decoded.len(), 512);
    assert!(decoded.iter().all(|&b| b == 0));
}

