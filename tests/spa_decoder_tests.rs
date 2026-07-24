use ldpc_rust::matrices::h_256_512::H_256_512;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;

#[test]
fn test_spa_decoder_all_zero_codeword() {
    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(20);

    let llrs = vec![5.0; 512];
    let decoded = decoder.decode(&llrs);

    assert_eq!(decoded.len(), 512);
    assert!(decoded.iter().all(|&b| b == 0));
}

#[test]
fn test_spa_decoder_single_error_recovery() {
    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(30);

    let mut llrs = vec![4.0; 512];
    llrs[42] = -4.0;

    let decoded = decoder.decode(&llrs);

    assert_eq!(decoded.len(), 512);

    let mut syndrome_valid = true;
    for row in H_256_512.iter() {
        let mut sum = 0u8;
        for (j, &h_ij) in row.iter().enumerate() {
            if h_ij == 1 {
                sum ^= decoded[j];
            }
        }
        if sum != 0 {
            syndrome_valid = false;
            break;
        }
    }

    assert!(
        syndrome_valid,
        "SPA decoder failed to correct a single error"
    );
    assert_eq!(decoded[42], 0, "Failed to correct the bit at index 42");
}

#[test]
fn test_spa_decoder_scaling_factor_variations() {
    let mut decoder_nms = SpaDecoderLLR::new(&H_256_512);
    decoder_nms.set_scaling_factor(0.75);
    decoder_nms.set_max_iter(10);

    let mut decoder_ms = SpaDecoderLLR::new(&H_256_512);
    decoder_ms.set_scaling_factor(1.0);
    decoder_ms.set_max_iter(10);

    let llrs = vec![2.0; 512];

    let res_nms = decoder_nms.decode(&llrs);
    let res_ms = decoder_ms.decode(&llrs);

    assert_eq!(res_nms.len(), 512);
    assert_eq!(res_ms.len(), 512);
}

#[test]
fn test_spa_decoder_preserves_dimensions() {
    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    let llrs = vec![1.0; 512];

    let decoded = decoder.decode(&llrs);
    assert_eq!(decoded.len(), 512);
}

#[test]
fn test_spa_decoder_noisy_channel_zero_llrs() {
    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(5);

    let llrs = vec![0.0; 512];
    let decoded = decoder.decode(&llrs);

    assert_eq!(decoded.len(), 512);
}

#[test]
fn test_spa_decoder_zero_iterations() {
    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(0);

    let llrs = vec![2.0; 512];
    let decoded = decoder.decode(&llrs);

    assert_eq!(decoded.len(), 512);
    assert!(decoded.iter().all(|&b| b == 0));
}

#[test]
fn test_spa_decoder_extreme_scaling_factors() {
    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_scaling_factor(0.0);
    decoder.set_max_iter(10);

    let llrs = vec![3.0; 512];
    let decoded = decoder.decode(&llrs);

    assert_eq!(decoded.len(), 512);
}
