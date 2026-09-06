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

#[test]
fn test_edge_alignment_sparse() {
    let decoder = SpaDecoderLLR::new(&H_256_512);

    for j in 0..decoder.n {
        let check_nodes = &decoder.col_to_rows()[j];
        let edge_idxs = &decoder.col_to_row_edge_idxs()[j];

        assert_eq!(check_nodes.len(), edge_idxs.len());

        for idx in 0..check_nodes.len() {
            let i = check_nodes[idx];
            let k = edge_idxs[idx];

            assert!(k < decoder.row_to_cols()[i].len());
            assert_eq!(
                decoder.row_to_cols()[i][k],
                j,
                "Edge misalignment: row {} edge {} does not map back to col {}",
                i,
                k,
                j
            );
        }
    }
}

#[test]
fn test_sparse_message_dimensions() {
    let decoder = SpaDecoderLLR::new(&H_256_512);

    for i in 0..decoder.m {
        let deg = decoder.row_to_cols()[i].len();

        assert_eq!(decoder.rmn()[i].len(), deg);
        assert_eq!(decoder.qnm()[i].len(), deg);
        assert_eq!(decoder.row_signs()[i].len(), deg);
        assert_eq!(decoder.row_mags()[i].len(), deg);
    }
}

#[test]
fn test_sign_mag_initialization() {
    let decoder = SpaDecoderLLR::new(&H_256_512);

    for i in 0..decoder.m {
        assert!(decoder.row_signs()[i].iter().all(|&s| s == 1));
        assert!(decoder.row_mags()[i].iter().all(|&m| m == 0.0));
    }
}

#[test]
fn test_zero_iteration_behavior_sparse() {
    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(0);

    let llrs = vec![2.0; 512];
    let decoded = decoder.decode(&llrs);

    assert!(decoded.iter().all(|&b| b == 0));

    for i in 0..decoder.m {
        assert!(decoder.rmn()[i].iter().all(|&v| v == 0.0));
    }

    for i in 0..decoder.m {
        for (k, &j) in decoder.row_to_cols()[i].iter().enumerate() {
            assert_eq!(decoder.qnm()[i][k], llrs[j]);
        }
    }
}

#[test]
fn test_variable_node_update_sparse() {
    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(1);

    let llrs = vec![1.0; 512];
    let _decoded = decoder.decode(&llrs);

    for (j, &llr_j) in llrs.iter().enumerate().take(decoder.n) {
        let check_nodes = &decoder.col_to_rows()[j];
        let edge_idxs = &decoder.col_to_row_edge_idxs()[j];

        for (idx, &i) in check_nodes.iter().enumerate() {
            let k = edge_idxs[idx];

            // Compute sum for this variable node
            let mut sum = llr_j;
            for (idx2, &i2) in check_nodes.iter().enumerate() {
                let k2 = edge_idxs[idx2];
                sum += decoder.rmn()[i2][k2];
            }

            assert!(
                (decoder.qnm()[i][k] - (sum - decoder.rmn()[i][k])).abs() < 1e-9,
                "Incorrect qnm update at (i={}, k={})",
                i,
                k
            );
        }
    }
}

#[test]
fn test_syndrome_check_sparse() {
    let decoder = SpaDecoderLLR::new(&H_256_512);

    let cw = vec![0u8; 512];
    assert!(decoder.check_syndrome_public(&cw));

    let mut cw_err = cw.clone();
    cw_err[10] = 1;
    assert!(!decoder.check_syndrome_public(&cw_err));
}

#[test]
fn test_random_llr_stability_sparse() {
    let mut decoder = SpaDecoderLLR::new(&H_256_512);

    let llrs: Vec<f64> = (0..512)
        .map(|_| {
            // rand::random::<u32>() is ALWAYS available
            let x = rand::random::<u32>() as f64 / (u32::MAX as f64);
            -5.0 + x * 10.0 // scale to [-5, 5]
        })
        .collect();

    let decoded = decoder.decode(&llrs);

    assert_eq!(decoded.len(), 512);
    assert!(decoded.iter().all(|&b| b == 0 || b == 1));
}
