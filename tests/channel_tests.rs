use ldpc_rust::channel::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

#[test]
fn test_awgn_llr_basic() {
    let mut rng = StdRng::seed_from_u64(12345);

    let llr0 = bpsk_awgn_llr(0, 1.0, &mut rng);
    let llr1 = bpsk_awgn_llr(1, 1.0, &mut rng);

    assert!(llr0.is_finite());
    assert!(llr1.is_finite());
}

#[test]
fn test_awgn_llr_sign() {
    let mut rng = StdRng::seed_from_u64(999);

    let llr0 = bpsk_awgn_llr(0, 10.0, &mut rng);
    let llr1 = bpsk_awgn_llr(1, 10.0, &mut rng);

    assert!(llr0 > 0.0);
    assert!(llr1 < 0.0);
}

#[test]
fn test_bsc_no_flip() {
    let mut rng = StdRng::seed_from_u64(1);

    for _ in 0..100 {
        let b = bsc_channel(0, 0.0, &mut rng);
        assert_eq!(b, 0);
    }
}

#[test]
fn test_bsc_always_flip() {
    let mut rng = StdRng::seed_from_u64(1);

    for _ in 0..100 {
        let b = bsc_channel(1, 1.0, &mut rng);
        assert_eq!(b, 0);
    }
}

#[test]
fn test_bsc_probabilistic() {
    let mut rng = StdRng::seed_from_u64(123);

    let mut flips = 0;
    for _ in 0..1000 {
        if bsc_channel(0, 0.3, &mut rng) == 1 {
            flips += 1;
        }
    }

    assert!(flips > 200 && flips < 400);
}

#[test]
fn test_rayleigh_sample_positive() {
    let mut rng = StdRng::seed_from_u64(42);

    for _ in 0..100 {
        let h = sample_rayleigh_fading(&mut rng);
        assert!(h >= 0.0);
        assert!(h.is_finite());
    }
}

#[test]
fn test_rayleigh_llr_finite() {
    let mut rng = StdRng::seed_from_u64(777);

    let h = sample_rayleigh_fading(&mut rng);
    let llr = rayleigh_fading_llr(0, 5.0, h, &mut rng);

    assert!(llr.is_finite());
}

#[test]
fn test_rician_sample_basic() {
    let mut rng = StdRng::seed_from_u64(555);

    let h = sample_rician_fading(5.0, &mut rng);
    assert!(h.is_finite());
    assert!(h > 0.0);
}

#[test]
fn test_rician_llr_finite() {
    let mut rng = StdRng::seed_from_u64(888);

    let llr = rician_fading_llr(1, 3.0, 2.0, &mut rng);
    assert!(llr.is_finite());
}

#[test]
fn test_nakagami_sample_basic() {
    let mut rng = StdRng::seed_from_u64(999);

    let h = sample_nakagami_fading(1.0, &mut rng);
    assert!(h.is_finite());
    assert!(h >= 0.0);
}

#[test]
fn test_nakagami_llr_finite() {
    let mut rng = StdRng::seed_from_u64(1234);

    let llr = nakagami_fading_llr(0, 2.0, 1.5, &mut rng);
    assert!(llr.is_finite());
}

#[test]
fn test_block_fading_llrs_length() {
    let mut rng = StdRng::seed_from_u64(2024);

    let bits = vec![0u8; 512];
    let llrs = block_fading_llrs(&bits, 3.0, &mut rng);

    assert_eq!(llrs.len(), 512);
}

#[test]
fn test_block_fading_same_h() {
    let mut rng = StdRng::seed_from_u64(2025);

    let bits = vec![0u8; 10];

    let h = sample_rayleigh_fading(&mut rng);

    let llrs_manual: Vec<f64> = bits
        .iter()
        .map(|&b| rayleigh_fading_llr(b, 3.0, h, &mut rng))
        .collect();

    let mut rng2 = StdRng::seed_from_u64(2025);
    let llrs_block = block_fading_llrs(&bits, 3.0, &mut rng2);

    for (a, b) in llrs_manual.iter().zip(llrs_block.iter()) {
        assert!((a - b).abs() < 1e-12);
    }
}

#[test]
fn test_freq_selective_llrs_length() {
    let mut rng = StdRng::seed_from_u64(3030);

    let bits = vec![1u8; 512];
    let llrs = frequency_selective_llrs(&bits, 2.0, &mut rng);

    assert_eq!(llrs.len(), 512);
}

#[test]
fn test_freq_selective_variation() {
    let mut rng = StdRng::seed_from_u64(3031);

    let bits = vec![0u8; 20];
    let llrs = frequency_selective_llrs(&bits, 2.0, &mut rng);

    // Expect variation due to independent fading per bit
    let unique: std::collections::HashSet<i32> = llrs.iter().map(|v| (v * 1000.0) as i32).collect();

    assert!(unique.len() > 5);
}

#[test]
fn test_burst_noise_no_burst() {
    let mut rng = StdRng::seed_from_u64(4040);

    let bits = vec![0u8; 100];
    let out = burst_noise_channel(&bits, 0.0, 10, &mut rng);

    assert_eq!(out, bits);
}

#[test]
fn test_burst_noise_full_burst() {
    let mut rng = StdRng::seed_from_u64(4041);

    let bits = vec![0u8; 20];
    let out = burst_noise_channel(&bits, 1.0, 20, &mut rng);

    assert!(out.iter().all(|&b| b == 1));
}

#[test]
fn test_burst_noise_partial() {
    let mut rng = StdRng::seed_from_u64(4042);

    let bits = vec![0u8; 200];
    let out = burst_noise_channel(&bits, 0.2, 10, &mut rng);

    // Expect some flips but not all
    let flips = out.iter().filter(|&&b| b == 1).count();

    assert!(flips > 0);
    assert!(flips < 200);
}

#[test]
fn test_channel_enum_awgn() {
    let mut rng = StdRng::seed_from_u64(5050);

    let llr = simulate_llr(0, 5.0, Channel::Awgn, &mut rng);
    assert!(llr.is_finite());
}

#[test]
fn test_channel_enum_rayleigh() {
    let mut rng = StdRng::seed_from_u64(5051);

    let llr = simulate_llr(1, 5.0, Channel::Rayleigh, &mut rng);
    assert!(llr.is_finite());
}

#[test]
fn test_channel_enum_rician() {
    let mut rng = StdRng::seed_from_u64(5052);

    let llr = simulate_llr(0, 5.0, Channel::Rician { k: 3.0 }, &mut rng);
    assert!(llr.is_finite());
}

#[test]
fn test_channel_enum_nakagami() {
    let mut rng = StdRng::seed_from_u64(5053);

    let llr = simulate_llr(1, 5.0, Channel::Nakagami { m: 1.2 }, &mut rng);
    assert!(llr.is_finite());
}
