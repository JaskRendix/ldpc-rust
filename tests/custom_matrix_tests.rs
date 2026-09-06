use ldpc_rust::define_custom_matrix;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;

define_custom_matrix!(
    pub struct TestMatrix4x8,
    const M = 4,
    const N = 8,
    [
        [1, 1, 0, 0, 1, 0, 0, 0],
        [0, 1, 1, 0, 0, 1, 0, 0],
        [0, 0, 1, 1, 0, 0, 1, 0],
        [1, 0, 0, 1, 0, 0, 0, 1],
    ]
);

#[test]
fn test_custom_matrix_dimensions() {
    assert_eq!(TestMatrix4x8::ROWS, 4);
    assert_eq!(TestMatrix4x8::COLS, 8);
    assert_eq!(TestMatrix4x8::DATA.len(), 4);
    assert_eq!(TestMatrix4x8::DATA[0].len(), 8);
}

#[test]
fn test_custom_matrix_decoding_convergence() {
    let mut decoder: SpaDecoderLLR<{ TestMatrix4x8::ROWS }, { TestMatrix4x8::COLS }> =
        SpaDecoderLLR::new(&TestMatrix4x8::DATA);
    decoder.set_max_iter(10);

    let llr = vec![10.0f64; TestMatrix4x8::COLS];
    let result = decoder.decode(&llr);

    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(res.converged);
    assert_eq!(res.codeword, vec![0u8; TestMatrix4x8::COLS]);
}

#[test]
fn test_edge_case_high_noise_non_convergence() {
    let mut decoder: SpaDecoderLLR<{ TestMatrix4x8::ROWS }, { TestMatrix4x8::COLS }> =
        SpaDecoderLLR::new(&TestMatrix4x8::DATA);
    decoder.set_max_iter(5);

    // Feed extremely hostile/noisy LLRs (strongly pointing to incorrect bits)
    let adversarial_llr = vec![-50.0f64; TestMatrix4x8::COLS];
    let result = decoder.decode(&adversarial_llr);

    // The decoder should gracefully handle non-convergence without panicking or crashing
    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(
        !res.converged,
        "Decoder should fail to converge under heavy adversarial noise"
    );
}

#[test]
fn test_edge_case_zero_iterations() {
    let mut decoder: SpaDecoderLLR<{ TestMatrix4x8::ROWS }, { TestMatrix4x8::COLS }> =
        SpaDecoderLLR::new(&TestMatrix4x8::DATA);
    decoder.set_max_iter(0);

    let llr = vec![5.0f64; TestMatrix4x8::COLS];
    let result = decoder.decode(&llr);

    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(
        !res.converged,
        "Decoder with 0 iterations should not mark convergence"
    );
}
